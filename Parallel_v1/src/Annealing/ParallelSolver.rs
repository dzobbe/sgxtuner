/// ///////////////////////////////////////////////////////////////////////////
///  File: neil/solver.rs
/// ///////////////////////////////////////////////////////////////////////////
///  Copyright 2016 Giovanni Mazzeo
///
///  Licensed under the Apache License, Version 2.0 (the "License");
///  you may not use this file except in compliance with the License.
///  You may obtain a copy of the License at
///
///      http://www.apache.org/licenses/LICENSE-2.0
///
///  Unless required by applicable law or agreed to in writing, software
///  distributed under the License is distributed on an "AS IS" BASIS,
///  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
///  See the License for the specific language governing permissions and
///  limitations under the License.
/// ///////////////////////////////////////////////////////////////////////////

use time;
use CoolingSchedule;
use EnergyType;
use hwloc;
use libc;
use TerminationCriteria;
use super::Shared;
use super::super::ResultsEmitter;
use ResultsEmitter::Emitter;
use rand::thread_rng;
use rand::distributions::{Range, IndependentSample};
use ansi_term::Colour::Green;
use super::Problem::Problem;
use super::Cooler::{Cooler, StepsCooler, TimeCooler};
use std::fs::{File, OpenOptions};
use std::collections::HashMap;
use hwloc::{Topology, ObjectType, CPUBIND_THREAD, CpuSet};
use std::thread;
use std::sync::{Arc, Mutex};



/**
 * A solver will take a problem and use simulated annealing
 * to try and find an optimal state.
 */
#[derive(Debug, Clone)]
pub struct Solver {
	
    /**
     * The termination criteria (maximum time or maximum number of steps)
     */
    pub termination_criteria: TerminationCriteria,

    /**
     * The minimum temperature of the process.
     */
    pub min_temperature: f64,

    /**
     * The maximum temperature of the process.
     */
    pub max_temperature: f64,

    /**
     * The Cooling Schedule procedure to select
     */
    pub cooling_schedule: CoolingSchedule,

    /**
     * The Energy metric to evaluate (Throughput or Latency)
     */
    pub energy_type: EnergyType,
}

impl Solver {
    /**
     * Construct the new default solver.
     */
    pub fn new() -> Solver {
        Default::default()
    }

    /** 
     * Construct a new solver with a given builder function.
     */
    pub fn build_new<F>(builder: F) -> Solver
        where F: FnOnce(&mut Solver)
    {
        let mut solver = Solver::new();
        builder(&mut solver);
        solver
    }

    /**
     * Run the solver
     */
    pub fn solve(&self, problem: &mut Problem) -> HashMap<String, u32> {
        let best_configuration = match self.termination_criteria {
            TerminationCriteria::Max_Steps(value) => {
                self.solve_step_based(problem,
                                      value,
                                      StepsCooler {
                                          max_steps: value,
                                          min_temp: self.min_temperature,
                                          max_temp: self.max_temperature,
                                      })
            }
            TerminationCriteria::Max_Time_Seconds(value) => {
                self.solve_time_based(problem,
                                      value,
                                      TimeCooler {
                                          max_time: value,
                                          min_temp: self.min_temperature,
                                          max_temp: self.max_temperature,
                                      })
            }
        };

        return best_configuration;
    }


    fn solve_step_based(&self,
                        problem: &mut Problem,
                        max_steps: usize,
                        cooler: StepsCooler)
                        -> HashMap<String, u32> {
                        	
        let mut results_emitter = ResultsEmitter::Emitter2File::new();

        println!("{}",Green.paint("\n-------------------------------------------------------------------------------------------------------------------"));
        println!("{} Initialization Phase: Evaluation of Energy for Default Paramters",
                 Green.paint("[TUNER]"));
        println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------"));

        let mut start_time = time::precise_time_ns();

        let mut master_state = problem.initial_state();
        let mut master_energy = match problem.energy(master_state.clone(), self.energy_type.clone(),0) {
            Some(nrg) => nrg,
            None => panic!("The initial configuration does not allow to calculate the energy"),
        };

        let mut elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;
        let time_2_complete_hrs = ((elapsed_time as f64) * max_steps as f64) / 3600.0;

        
		let mut elapsed_steps = Shared::ElapsedSteps::new();
		let mut accepted = Shared::AcceptedStates::new();
        let mut rejected = Shared::SubsequentRejStates::new();
		let mut temperature = Shared::Temperature::new(self.max_temperature, cooler, self.clone().cooling_schedule);
		
        let mut attempted = 0;
        let mut total_improves = 0;
        let mut subsequent_improves = 0;

	    
        start_time = time::precise_time_ns();
        
        'outer: loop {
        	
        		if elapsed_steps.get() == max_steps{
        			break 'outer;
        		}
	        	elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;
	
	            println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------"));
	            println!("{} Completed Steps: {:.2} - Percentage of Completion: {:.2}% - Estimated \
	                      time to Complete: {:.2} Hrs",
	                     Green.paint("[TUNER]"),
	                     elapsed_steps.get(),
	                     (elapsed_steps.get() as f64 / max_steps as f64) * 100.0,
	                     time_2_complete_hrs as usize);
	            println!("{} Total Accepted Solutions: {:?} - Current Temperature: {:.2} - Elapsed \
	                      Time: {:.2} s",
	                     Green.paint("[TUNER]"),
	                     accepted.get(),
	                     temperature.get(),
	                     elapsed_time);
	            println!("{} Accepted State: {:?}", Green.paint("[TUNER]"), master_state);
	            println!("{} Accepted Energy: {:.4} - Last Measured Energy: {:.4}",
	                     Green.paint("[TUNER]"),
	                     2,
	                     3);
	            println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------"));
	
	
				//Create the Pool of Neighborhoods
				let neigh_space=problem.neigh_space(&master_state);
				let neigh_pool=Shared::NeighborhoodsPool::new(neigh_space);
				
				
				let threads_res=Shared::ThreadsResults::new();
				
				let cpu_topology = Arc::new(Mutex::new(Topology::new()));//get_cpu_cores(cpu_topology.clone())
		    	// Spawn one thread for each and pass the topology down into scope.
	 			let handles: Vec<_> = (0..2).map(|core| {
		            let child_topo = cpu_topology.clone();
		            					
					let (mut master_state_c, mut problem_c) = (master_state.clone(), problem.clone());
	            	let (elapsed_steps_c, temperature_c,
	            		 neigh_pool_c, accepted_c,
	            		 rejected_c,threads_res_c) = (elapsed_steps.clone(),
	            		 							  temperature.clone(),
	            		 							  neigh_pool.clone(), 
	            		 							  accepted.clone(), 
	            		 							  rejected.clone(),
	            		 							  threads_res.clone());

					let nrg_type = self.clone().energy_type;
					
		            thread::spawn(move || {
							let mut worker_nrg=master_energy;
							let mut worker_state=master_state_c;
  					        let range = Range::new(0.0, 1.0);
		  					let mut rng = thread_rng();

			                bind_2_cpu_core(core, child_topo);
			                
				            loop{
				            	
				            	worker_state = {
	            	
						                let next_state = match neigh_pool_c.remove_one(){
							            		Some(res) => res,
							            		None 	  => break,
						            	};
						                
										let accepted_state = match problem_c.energy(next_state.clone(), nrg_type.clone(), core+1) {
						                    Some(new_energy) => {
						            			println!("Thread : {:?} - Step: {:?} - State: {:?} - Energy: {:?}",core, elapsed_steps_c.get(),next_state,new_energy);
						 
						                        let de = match nrg_type {
						                            EnergyType::throughput => new_energy - worker_nrg,
						                            EnergyType::latency => -(new_energy - worker_nrg), 
						                        };
						
						                        if de > 0.0 || range.ind_sample(&mut rng) <= (-de / temperature_c.get()).exp() {
						                            accepted_c.increment();
						                        	rejected_c.reset();
						                        	
						                            worker_nrg = new_energy;
						
						                           /* if de > 0.0 {
						                                total_improves = total_improves + 1;
						                                subsequent_improves = subsequent_improves + 1;
						                            }*/
						
						                            /*results_emitter.send_update(new_energy,
						                                                &next_state,
						                                                energy,
						                                                &next_state,
						                                                elapsed_steps_c.get());*/
						                            next_state
						
						                        } else {
						                        	rejected_c.increment();
						                        	
						                        	if rejected_c.get()==100{
						                        		break;
						                        	} 
						                        		
						                           // subsequent_improves = 0;
						                            /*results_emitter.send_update(new_energy,
						                                                &next_state,
						                                                energy,
						                                                &state,
						                                                elapsed_steps_c.get());*/
						                            worker_state
						                        }
						                    }
						                    None => {
						                        println!("{} The current configuration parameters cannot be evaluated. \
						                                  Skip!",
						                                 Green.paint("[TUNER]"));
						                        worker_state
						                    }
						                };
						                
						                accepted_state
						            };
										            	
					            	elapsed_steps_c.increment();
									temperature_c.update(elapsed_steps_c.get());	
							}
				            
				            let res=Shared::MrResult{
				            	energy: worker_nrg,
				            	state: worker_state,
				            };
				            threads_res_c.push(res);		                	
		            })
		            
		            
		        }).collect();
				
		        // Wait for all threads to complete before start a search in a new set of neighborhoods.
		        for h in handles {
		            h.join().unwrap();
		        }
		        
		        //Get results of worker threads (each one will put its best evaluated energy) and 
		        //choose between them which one will be the best one.
		        let mut workers_res = threads_res.get_coll();
		       	let first_elem = workers_res.pop().unwrap();
		       	
		       	master_energy = first_elem.energy;
		       	master_state  = first_elem.state;
		       	
		       	for elem in workers_res.iter() {
		       		let diff=match self.energy_type {
                            EnergyType::throughput => {
                            	 elem.energy-master_energy
                            },
                            EnergyType::latency => {
                            	-(elem.energy-master_energy)
                            } 
                        };
		       		if diff > 0.0 {
		       			master_energy=elem.clone().energy;
		       			master_state=elem.clone().state;
		       		}
		       	}
		       
			}

        master_state
    } 



    fn solve_time_based(&self,
                        problem: &mut Problem,
                        max_time: usize,
                        cooler: TimeCooler)
                        -> HashMap<String, u32> {
                        	
        let mut rng = thread_rng();
        let range = Range::new(0.0, 1.0);

        let mut state = problem.initial_state();

        return state;
    }



    /**
     * Automatic Evaluation of Tmin and Tmax based on certain number 
     * of algorithm executions
     */
    pub fn auto_param_evaluation(&self) {}
}

impl Default for Solver {
    fn default() -> Solver {
        Solver {
            termination_criteria: TerminationCriteria::Max_Steps(10000),
            min_temperature: 2.5,
            max_temperature: 1000.0,
            cooling_schedule: CoolingSchedule::exponential,
            energy_type: EnergyType::throughput,
        }
    }
}
 

fn get_cpu_cores(topology: Arc<Mutex<hwloc::Topology>>) -> usize {
	/**Extract the number of CPU cores Available**/
	let num_cores = {
	    let topo_rc = topology.clone();
	    let topo_locked = topo_rc.lock().unwrap();
	    (*topo_locked).objects_with_type(&ObjectType::Core).unwrap().len()
    };
	return num_cores;
	
}

fn bind_2_cpu_core(core: usize, topology: Arc<Mutex<hwloc::Topology>>){
	// Get the current thread id and lock the topology to use.
    let tid = get_thread_id();
    let mut locked_topo = topology.lock().unwrap();
    // Thread binding before explicit set.
    let before = locked_topo.get_cpubind_for_thread(tid, CPUBIND_THREAD);
    // load the cpuset for the given core index.
    let bind_to = cpuset_for_core(&*locked_topo, core);
    // Set the binding.
    locked_topo.set_cpubind_for_thread(tid, bind_to, CPUBIND_THREAD).unwrap();
}


/// Load the CpuSet for the given core index.
fn cpuset_for_core(topology: &Topology, idx: usize) -> CpuSet {
    let cores = (*topology).objects_with_type(&ObjectType::Core).unwrap();
    match cores.get(idx) {
        Some(val) => val.cpuset().unwrap(),
        None => panic!("No Core found with id {}", idx)
    }
}

/// Helper method to get the thread id through libc, with current rust stable (1.5.0) its not
/// possible otherwise I think.
fn get_thread_id() -> libc::pthread_t {
    unsafe { libc::pthread_self() }
}
