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
use MrResult;
use hwloc;
use pbr;
use libc;
use num_cpus;
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
use pbr::{ProgressBar,MultiBar};


/**
 * A solver will take a problem and use simulated annealing
 * to try and find an optimal state.
 */
#[derive(Debug, Clone)]
pub struct Solver {
	
    pub max_steps: usize,

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
 

	/***********************************************************************************************************
	************************************************************************************************************
    /// **
    /// Sequential Solver
    /// *  
	************************************************************************************************************
	************************************************************************************************************/
    pub fn solve_sequential(&self,
                        problem: &mut Problem)
                        -> MrResult {
                        	        	
    	let cooler=StepsCooler {
                      max_steps: self.max_steps,
                      min_temp: self.min_temperature,
                      max_temp: self.max_temperature,
                      };
    	
       	let mut results_emitter = ResultsEmitter::Emitter2File::new();
        let mut rng = thread_rng();
        let range = Range::new(0.0, 1.0);

        println!("{}",Green.paint("\n-------------------------------------------------------------------------------------------------------------------"));
        println!("{} Initialization Phase: Evaluation of Energy for Default Parameters",
                 Green.paint("[TUNER]"));
        println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------"));

        let mut start_time = time::precise_time_ns();

        let mut state = problem.initial_state();
        let mut energy = match problem.energy(&state, self.energy_type.clone(),0) {
            Some(nrg) => nrg,
            None => panic!("The initial configuration does not allow to calculate the energy"),
        };

        let mut elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;
        let estimated_time_2_complete = ((elapsed_time as f64) * self.max_steps as f64) / 3600.0;

        let mut temperature: f64 = self.max_temperature;
        let mut attempted = 0;
        let mut accepted = 0;
        let mut rejected = 0;
        let mut total_improves = 0;
        let mut subsequent_improves = 0;
        let mut last_nrg = energy;


        start_time = time::precise_time_ns();

        for elapsed_steps in 0..self.max_steps {

            elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;

            println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------"));
            println!("{} Completed Steps: {:.2} - Percentage of Completion: {:.2}% - Estimated \
                      time to Complete: {:.2} Hrs",
                     Green.paint("[TUNER]"),
                     elapsed_steps,
                     (elapsed_steps as f64 / cooler.max_steps as f64) * 100.0,
                     estimated_time_2_complete as usize);
            println!("{} Total Accepted Solutions: {:?} - Current Temperature: {:.2} - Elapsed \
                      Time: {:.2} s",
                     Green.paint("[TUNER]"),
                     accepted,
                     temperature,
                     elapsed_time);
            println!("{} Accepted State: {:?}", Green.paint("[TUNER]"), state);
            println!("{} Accepted Energy: {:.4} - Last Measured Energy: {:.4}",
                     Green.paint("[TUNER]"),
                     energy,
                     last_nrg);
            println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------"));


            state = {
            	
                let next_state = match problem.new_state(&state, self.max_steps, elapsed_steps) {
                    // There is a neighborhood available
                    Some(n_s) => n_s,
                    // No neighborhood available, all states have been visited
                    None => {
                        println!("{} Any Neighborhood Available - Terminate the Annealing",
                                 Green.paint("[TUNER]"));
                        break;
                    }
                }; 

                let accepted_state = match problem.energy(&next_state, self.clone().energy_type,0) {
                    Some(new_energy) => {
                        last_nrg = new_energy;

                        let de = match self.energy_type {
                            EnergyType::throughput => new_energy - energy,
                            EnergyType::latency => -(new_energy - energy), 
                        };

                        if de > 0.0 || range.ind_sample(&mut rng) <= (de / temperature).exp() {
                            accepted += 1;
                            energy = new_energy;

                            if de > 0.0 {
                                total_improves = total_improves + 1;
                                subsequent_improves = subsequent_improves + 1;
                            }

                            results_emitter.send_update(new_energy,
                                                &next_state,
                                                energy,
                                                &next_state,
                                                elapsed_steps);
                            next_state

                        } else {
                            subsequent_improves = 0;
                            results_emitter.send_update(new_energy,
                                                &next_state,
                                                energy,
                                                &state,
                                                elapsed_steps);
                            state
                        }
                    }
                    None => {
                        println!("{} The current configuration parameters cannot be evaluated. \
                                  Skip!",
                                 Green.paint("[TUNER]"));
                        state
                    }
                };

                accepted_state
            };


            temperature = match self.cooling_schedule {
                CoolingSchedule::linear => cooler.linear_cooling(elapsed_steps),
                CoolingSchedule::exponential => cooler.exponential_cooling(elapsed_steps),
                CoolingSchedule::basic_exp_cooling => cooler.basic_exp_cooling(temperature),
            }; 
        }
		
		MrResult {
                  energy: energy,
                  state: state,
                  }
        
    }



	/***********************************************************************************************************
	************************************************************************************************************
    /// **
    /// Parallel Solver v1
    /// *  
	************************************************************************************************************
	************************************************************************************************************/
	pub fn solve_parallel_v1(&self,
                        problem: &mut Problem)
                        -> MrResult {
                        	
        let cooler=StepsCooler {
                      max_steps: self.max_steps,
                      min_temp: self.min_temperature,
                      max_temp: self.max_temperature,
                      };
                        	                	
        let mut results_emitter = ResultsEmitter::Emitter2File::new();

        ("{}",Green.paint("\n-------------------------------------------------------------------------------------------------------------------"));
        println!("{} Initialization Phase: Evaluation of Energy for Default Parameters",
                 Green.paint("[TUNER]"));
        println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------"));

        let mut start_time = time::precise_time_ns();

        let mut master_state = problem.initial_state();
        let mut master_energy = match problem.energy(&master_state.clone(), self.energy_type.clone(),0) {
            Some(nrg) => nrg,
            None => panic!("The initial configuration does not allow to calculate the energy"),
        };

        let mut elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;
        let time_2_complete_hrs = ((elapsed_time as f64) * self.max_steps as f64) / 3600.0;

        
		let mut elapsed_steps = Shared::ElapsedSteps::new();
		let mut accepted = Shared::AcceptedStates::new();
        let mut rejected = Shared::SubsequentRejStates::new();
		let mut temperature = Shared::Temperature::new(self.max_temperature, cooler, self.clone().cooling_schedule);
		
        let mut attempted = 0;
        let mut total_improves = 0;
        let mut subsequent_improves = 0;

 		/************************************************************************************************************/
        start_time = time::precise_time_ns();
        'outer: loop {
        	
        		if elapsed_steps.get() > self.max_steps{
        			break 'outer;
        		}
	        	elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;
	
	            println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------"));
	            println!("{} Completed Steps: {:.2} - Percentage of Completion: {:.2}% - Estimated \
	                      time to Complete: {:.2} Hrs",
	                     Green.paint("[TUNER]"),
	                     elapsed_steps.get(),
	                     (elapsed_steps.get() as f64 / self.max_steps as f64) * 100.0,
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
	                     master_energy,
	                     3);
	            println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------"));
	
				
				//Create the Pool of Neighborhoods
				let neigh_space=problem.neigh_space(&master_state);
				let neigh_pool=Shared::NeighborhoodsPool::new(neigh_space);
				
				let threads_res=Shared::ThreadsResults::new();
				
				
				let cpu_topology = Arc::new(Mutex::new(Topology::new()));

		    	
 				let mut mb = MultiBar::new();
 
 				//Get the number of physical cpu cores
			 	let num_cores = {
			        let topo_rc = cpu_topology.clone();
			        let topo_locked = topo_rc.lock().unwrap();
			        (*topo_locked).objects_with_type(&ObjectType::Core).unwrap().len()
			    };		
 				/************************************************************************************************************/
	 			let handles: Vec<_> = (0..num_cores).map(|core| {
	 				let mut pb=mb.create_bar(neigh_pool.size()/num_cores as u64);
 			        pb.show_message = true;
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
					
					
					/************************************************************************************************************/
		            thread::spawn(move || {

							let mut worker_nrg=master_energy;
							let mut worker_state=master_state_c;
  					        let range = Range::new(0.0, 1.0);
		  					let mut rng = thread_rng();

    						let tid = get_thread_id();
			                //bind_2_cpu_core(core, tid, child_topo);
			                
				            loop{
				            	pb.message(&format!("TID [{}] - Neigh. Exploration Status - ", core));

				            	worker_state = {
	            	
						                let next_state = match neigh_pool_c.remove_one(){
							            		Some(res) => res,
							            		None 	  => break,
						            	};

										let accepted_state = match problem_c.energy(&next_state.clone(), nrg_type.clone(), core) {
						                    Some(new_energy) => {
						            			println!("Thread : {:?} - Step: {:?} - State: {:?} - Energy: {:?}",core, elapsed_steps_c.get(),next_state,new_energy);

						                        let de = match nrg_type {
						                            EnergyType::throughput => new_energy - worker_nrg,
						                            EnergyType::latency => -(new_energy - worker_nrg), 
						                        };
						
						                        if de > 0.0 || range.ind_sample(&mut rng) <= (de / temperature_c.get()).exp() {
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
						                        	
						                        	if rejected_c.get()==50{
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
 									pb.inc();	            	
									temperature_c.update(elapsed_steps_c.get());	
							}
				            
				            let res=Shared::MrResult{
				            	energy: worker_nrg,
				            	state: worker_state,
				            };

				            threads_res_c.push(res);	
    		            	pb.finish_print(&format!("Child Thread [{}] Terminated the Execution", core));
	                	
		            })

		        }).collect();
				
				mb.listen(); 
		        // Wait for all threads to complete before start a search in a new set of neighborhoods.
		        for h in handles {
		            h.join().unwrap();
		        }
		        
		        
				/************************************************************************************************************/	
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

		MrResult {
                  energy: master_energy,
                  state: master_state,
                  }
    } 


	/***********************************************************************************************************
	************************************************************************************************************
    /// **
    /// Parallel Solver v2
    /// *  
	************************************************************************************************************
	************************************************************************************************************/
	pub fn solve_parallel_v2(&self,
                        problem: &mut Problem)
                        -> MrResult {
                        	
                	
    	let cooler=StepsCooler {
                      max_steps: self.max_steps,
                      min_temp: self.min_temperature,
                      max_temp: self.max_temperature,
                      };
                        	
        let mut results_emitter = ResultsEmitter::Emitter2File::new();

        ("{}",Green.paint("\n-------------------------------------------------------------------------------------------------------------------"));
        println!("{} Initialization Phase: Evaluation of Energy for Default Parameters",
                 Green.paint("[TUNER]"));
        println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------"));


		let cpu_topology = Arc::new(Mutex::new(Topology::new()));
        //Get the number of physical cpu cores
	 	let num_cores = {
	        let topo_rc = cpu_topology.clone();
	        let topo_locked = topo_rc.lock().unwrap();
	        (*topo_locked).objects_with_type(&ObjectType::Core).unwrap().len()
	    };
	 	
        
		let mut elapsed_steps = Shared::ElapsedSteps::new();
		
		//Creation of the pool of Initial States. It will be composed by the initial default state
		//given by the user and by other num_cores-1 states generated in a random way
		let mut initial_state = problem.initial_state();
		let mut initial_states_pool = Shared::InitialStatesPool::new();
        initial_states_pool.push(initial_state.clone()); 
        for i in 1..num_cores{
	        initial_states_pool.push(problem.rand_state(&initial_state.clone()));    		
        }  		 

 		//Create a muti-bar
 		let mut mb = MultiBar::new();

		let threads_res=Shared::ThreadsResults::new();

	 	let mut overall_start_time = time::precise_time_ns();
 		let handles: Vec<_> = (0..num_cores).map(|core| {
 				
				let mut pb=mb.create_bar((self.max_steps/num_cores) as u64);
 			    pb.show_message = true;
		        
		       
		        				
				let (mut master_state_c, mut problem_c) = (initial_state.clone(), problem.clone());
	        	let (elapsed_steps_c,
	        		initial_states_pool_c,
            		threads_res_c) = 	(elapsed_steps.clone(),
            		 					initial_states_pool.clone(),
    		 							threads_res.clone());

				let nrg_type = self.clone().energy_type;
				let max_temp = self.clone().max_temperature;
				let max_steps= self.clone().max_steps;
				let cooling_sched= self.clone().cooling_schedule;
				let cooler_c=cooler.clone();
				 
 	 			/************************************************************************************************************/
 				thread::spawn(move || {
				 	
			        let mut attempted = 0;
			        let mut total_improves = 0;
			        let mut subsequent_improves = 0;
					let mut accepted = 0;
			        let mut rejected = 0;
					let mut temperature = max_temp;
					let mut worker_elapsed_steps=0;		
						
        			let mut start_time = time::precise_time_ns(); 
        			
					let mut worker_state=initial_states_pool_c.remove_one().unwrap();
				 	let mut worker_nrg = match problem_c.energy(&worker_state.clone(), nrg_type.clone(), core) {
			            Some(nrg) => nrg,
			            None => panic!("The initial configuration does not allow to calculate the energy"),
			        };
					
					
					let mut elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;
        			let time_2_complete_hrs = ((elapsed_time as f64) * max_steps as f64) / 3600.0;
  					
  					let range = Range::new(0.0, 1.0);
  					let mut rng = thread_rng();
					let tid = get_thread_id();
			 		
			 		
			 		/************************************************************************************************************/			    	
			    	
					let threads_res=Shared::ThreadsResults::new();
 

		            loop{
		            	pb.message(&format!("TID [{}] - Status - ", core));
						pb.inc();	            	

		            	worker_state = {
        	
				                let next_state = problem_c.new_state(&worker_state,max_steps,worker_elapsed_steps).unwrap();
				                
								let accepted_state = match problem_c.energy(&next_state.clone(), nrg_type.clone(), core) {
				                    Some(new_energy) => {
				            			println!("Thread : {:?} - Step: {:?} - State: {:?} - Energy: {:?}",core, elapsed_steps_c.get(),next_state,new_energy);
				 
				                        let de = match nrg_type {
				                            EnergyType::throughput => new_energy - worker_nrg,
				                            EnergyType::latency => -(new_energy - worker_nrg), 
				                        }; 
				
				                        if de > 0.0 || range.ind_sample(&mut rng) <= (de / temperature).exp() {
				                            accepted+=1;
				                        	rejected=0;
				                        	
				                            worker_nrg = new_energy;
				
				                            if de > 0.0 {
				                                total_improves = total_improves + 1;
				                                subsequent_improves = subsequent_improves + 1;
				                            }
				
				                            /*results_emitter.send_update(new_energy,
				                                                &next_state,
				                                                energy,
				                                                &next_state,
				                                                elapsed_steps_c.get());*/
				                            next_state
				
				                        } else {
				                        	rejected+=1;
				                        	
				                        	if rejected==100{
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
			            	worker_elapsed_steps+=1;
			            	
							temperature=match cooling_sched {
					                CoolingSchedule::linear => cooler_c.linear_cooling(worker_elapsed_steps),
					                CoolingSchedule::exponential => cooler_c.exponential_cooling(worker_elapsed_steps),
					                CoolingSchedule::basic_exp_cooling => cooler_c.basic_exp_cooling(temperature),
		           				 };   
					}
		            
		            let res=Shared::MrResult{
		            	energy: worker_nrg,
		            	state: worker_state,
		            };
		            threads_res_c.push(res);	
	            	pb.finish_print(&format!("Child Thread [{}] Terminated the Execution", core));
            	
	            })

	        }).collect();
			
			mb.listen();
	        // Wait for all threads to complete before start a search in a new set of neighborhoods.
	        for h in handles {
	            h.join().unwrap();
	        }
	        
	         
			/************************************************************************************************************/	
	        //Get results of worker threads (each one will put its best evaluated energy) and 
	        //choose between them which one will be the best one.
	        let mut workers_res = threads_res.get_coll();
	       	let first_elem = workers_res.pop().unwrap();
	       	
	       	let mut best_energy = first_elem.energy;
	       	let mut best_state  = first_elem.state;
	       	
	       	for elem in workers_res.iter() {
	       		let diff=match self.energy_type {
                        EnergyType::throughput => {
                        	 elem.energy-best_energy
                        },
                        EnergyType::latency => {
                        	-(elem.energy-best_energy)
                        } 
                    };
	       		if diff > 0.0 {
	       			best_energy=elem.clone().energy;
	       			best_state=elem.clone().state;
	       		}
	       	}
		       
			

		MrResult {
          energy: best_energy,
          state: best_state,
          }
    } 	
	
}

impl Default for Solver {
    fn default() -> Solver {
        Solver {
            max_steps: 10000,
            min_temperature: 2.5,
            max_temperature: 1000.0,
            cooling_schedule: CoolingSchedule::exponential,
            energy_type: EnergyType::throughput,
        }
    }
}
 

fn get_cpu_cores(topology: Arc<Mutex<hwloc::Topology>>) -> usize {
	/**Extract the number of CPU cores Available**/
  	let topo_rc = topology.clone();
    let topo_locked = topo_rc.lock().unwrap();
    (*topo_locked).objects_with_type(&ObjectType::Core).unwrap().len()	
}

fn bind_2_cpu_core(core: usize, tid: libc::pthread_t, topology: Arc<Mutex<hwloc::Topology>>){
	// Get the current thread id and lock the topology to use.
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
