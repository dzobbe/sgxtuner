/// ///////////////////////////////////////////////////////////////////////////
///  File: annealing/solver/spis.rs
/// ///////////////////////////////////////////////////////////////////////////
///  Copyright 2017 Giovanni Mazzeo
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

/// ****************************************************************************
/// *****************************************************************************
/// **
/// Simultaneous Periodically Interacting Simulated Annealing (SPISA)
/// *
/// *****************************************************************************
/// ****************************************************************************
use annealing::solver::Solver;
use annealing::problem::Problem;
use annealing::cooler::{Cooler, StepsCooler, TimeCooler};
use annealing::solver::common;
use annealing::solver::common::MrResult;
use annealing::solver::common::IntermediateResults;
use res_emitters;
use res_emitters::Emitter;

use shared::TunerParameter;


use time;
use CoolingSchedule;
use EnergyType;
use pbr;
use rand;
use libc;
use num_cpus;

use rand::{Rng, thread_rng};
use rand::distributions::{Range, IndependentSample};
use ansi_term::Colour::Green;
use std::collections::HashMap;
use pbr::{ProgressBar, MultiBar};
use std::thread;
use std::sync::mpsc::channel;
use std::time::Duration;


#[derive(Debug, Clone)]
pub struct Spisa {
    pub tuner_params: TunerParameter,
    pub res_emitter: Emitter,
}

impl Solver for Spisa {
    fn solve(&mut self, problem: &mut Problem, num_workers: usize) -> MrResult {


        let cooler = StepsCooler {
            max_steps: self.tuner_params.max_step,
            min_temp: self.tuner_params.min_temp.unwrap(),
            max_temp: self.tuner_params.max_temp.unwrap(),
        };


        ("{}",Green.paint("\n-------------------------------------------------------------------------------------------------------------------"));
        println!(
            "{} Initialization Phase: Evaluation of Energy for Default Parameters",
            Green.paint("[TUNER]")
        );
        println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------"));

        let mut start_time = time::precise_time_ns();
        let mut rng = thread_rng();

        let mut rng_c = rng.clone();
        let mut master_state = problem.initial_state();
        let mut master_energy = match problem.energy(&master_state.clone(), 0) {
            Some(nrg) => nrg,
            None => panic!("The initial configuration does not allow to calculate the energy"),
        };

        let mut elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;
        let time_2_complete_hrs = ((elapsed_time as f64) * self.tuner_params.max_step as f64) /
            3600.0;


        let mut cpu_time = 0.0;
        let mut elapsed_steps = common::SharedGenericCounter::new();
        let mut accepted = common::SharedGenericCounter::new();
        let mut subsequent_rej = common::SharedGenericCounter::new();

        let mut temperature = common::Temperature::new(
            self.tuner_params.max_temp.unwrap(),
            cooler,
            self.clone().tuner_params.cooling,
        );


        // Channel for receiving results from worker threads and send them to the file writer.
        let (tx, rx) = channel::<IntermediateResults>();
        // Spawn the thread that will take care of writing results into a CSV file
        let (elapsed_steps_c, temperature_c) = (elapsed_steps.clone(), temperature.clone());
        let mut res_emitter = self.clone().res_emitter;
        thread::spawn(move || loop {
            elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;
            match rx.recv() {
                Ok(res) => {
                    res_emitter.send_update(
                        temperature_c.get(),
                        elapsed_time,
                        cpu_time,
                        res.last_nrg,
                        &res.last_state,
                        res.best_nrg,
                        &res.best_state,
                        elapsed_steps_c.get(),
                        res.tid,
                    );
                }
                Err(e) => {} 
            }
        });


        // *********************************************************************************************************
        start_time = time::precise_time_ns();
        'outer: loop {




            if elapsed_steps.get() > self.tuner_params.max_step {
                break 'outer;
            }

            if subsequent_rej.get() > 300 {
                println!("{} Convergence Reached!!!", Green.paint("[TUNER]"));
                break 'outer;
            }
            elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;

            println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------"));
            println!(
                "{} Completed Steps: {:.2} - Percentage of Completion: {:.2}% - Estimated \
                      time to Complete: {:.2} Hrs",
                Green.paint("[TUNER]"),
                elapsed_steps.get(),
                (elapsed_steps.get() as f64 / self.tuner_params.max_step as f64) * 100.0,
                time_2_complete_hrs as usize
            );
            println!(
                "{} Total Accepted: {:?} - Subsequent Rejected: {:?} - Current Temperature: \
                      {:.2} - Elapsed Time: {:.2} s",
                Green.paint("[TUNER]"),
                accepted.get(),
                subsequent_rej.get(),
                temperature.get(),
                elapsed_time
            );
            println!(
                "{} Accepted State: {:?}",
                Green.paint("[TUNER]"),
                master_state
            );
            println!(
                "{} Accepted Energy: {:.4}",
                Green.paint("[TUNER]"),
                master_energy
            );

            println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------"));


            // Create the Pool of Neighborhoods
            let neigh_space = problem.neigh_space(&master_state);
            let neigh_pool = common::NeighborhoodsPool::new(neigh_space);

            let threads_res = common::ThreadsResults::new();

            let mut mb = MultiBar::new();




            /// *********************************************************************************************************
            let handles: Vec<_> = (0..num_workers).map(|worker_nr| {
	 				let mut pb=mb.create_bar(neigh_pool.size()/num_workers as u64);
 			        pb.show_message = true;
		            					
					let (mut master_state_c, mut problem_c) = (master_state.clone(), problem.clone());
	            	let (elapsed_steps_c, temperature_c,
	            		 neigh_pool_c, accepted_c,
	            		 subsequent_rej_c,threads_res_c) = (elapsed_steps.clone(),
	            		 							  temperature.clone(),
	            		 							  neigh_pool.clone(), 
	            		 							  accepted.clone(), 
	            		 							  subsequent_rej.clone(),
	            		 							  threads_res.clone());

					let nrg_type = self.clone().tuner_params.energy;
					
					let tx_c=tx.clone();


					/************************************************************************************************************/
		            thread::spawn(move || {

  			 			
							let mut worker_nrg=master_energy.clone();
							let mut worker_state=master_state_c.clone();
  					        let range = Range::new(0.0, 1.0);
		  					let mut rng = thread_rng();
 							
							let mut last_nrg=master_energy.clone();
							let mut last_state=master_state_c.clone();
							
				            loop{
				            	
				            	
				            	pb.message(&format!("TID [{}] - Neigh. Exploration Status - ", worker_nr));

				            	worker_state = {
	            	
						                let next_state = match neigh_pool_c.remove_one(){
							            		Some(res) => res,
							            		None 	  => break,
						            	};
										
										last_state=next_state.clone();
									
										let accepted_state = match problem_c.energy(&next_state.clone(), worker_nr) {
						                    Some(new_energy) => {
						            			println!("Thread : {:?} - Step: {:?} - State: {:?} - Energy: {:?}",worker_nr, elapsed_steps_c.get(),next_state,new_energy);
												last_nrg=new_energy;
						                        let de = match nrg_type {
						                            EnergyType::throughput => new_energy - worker_nrg,
						                            EnergyType::latency => -(new_energy - worker_nrg), 
						                        };
						
						                        if de > 0.0 || range.ind_sample(&mut rng) <= (de / temperature_c.get()).exp() {
						                            accepted_c.increment();
						                        	
						                            worker_nrg = new_energy;
						
						                            if de > 0.0 {
														subsequent_rej_c.reset();
						                            }
						                          
						                            next_state
						
						                        } else {
													subsequent_rej_c.increment();					                        	
							                           
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
                                                            
				            	   let intermediate_res=IntermediateResults{
				            			last_nrg: last_nrg,
				            			last_state:last_state.clone(),
				            			best_nrg: worker_nrg,
				            			best_state: worker_state.clone(),
				            			tid: worker_nr,
				            		};
				            		
				            		tx_c.send(intermediate_res);
				            		
					            	elapsed_steps_c.increment();
 									pb.inc();	            	
									temperature_c.update(elapsed_steps_c.get());	
							}
				            
				            let res=common::MrResult{
				            	energy: worker_nrg,
				            	state: worker_state,
				            };
				            

				            threads_res_c.push(res);	
    		            	pb.finish_print(&format!("Child Thread [{}] Terminated the Execution", worker_nr));
	                	
		            })

		        }).collect();

            mb.listen();

            // Wait for all threads to complete before start a search in a new set of neighborhoods.
            for h in handles {
                h.join().unwrap();
            }


            /// *********************************************************************************************************

            // Get results of worker threads (each one will put its best evaluated energy) and
            // choose between them which one will be the best
            let mut workers_res = threads_res.get_coll();
            let first_elem = workers_res.pop().unwrap();

            let mut best_workers_nrg = first_elem.energy;
            let mut best_workers_state = first_elem.state;

            for elem in workers_res.iter() {
                let diff = match self.tuner_params.energy {
                    EnergyType::throughput => elem.energy - best_workers_nrg,
                    EnergyType::latency => -(elem.energy - best_workers_nrg), 
                };
                if diff > 0.0 {
                    best_workers_nrg = elem.clone().energy;
                    best_workers_state = elem.clone().state;
                }
            }

            let de = match self.tuner_params.energy {
                EnergyType::throughput => best_workers_nrg - master_energy,
                EnergyType::latency => -(best_workers_nrg - master_energy), 
            };
            let range = Range::new(0.0, 1.0);

            if de > 0.0 || range.ind_sample(&mut rng_c) <= (de / temperature.get()).exp() {
                master_energy = best_workers_nrg;
                master_state = best_workers_state;
                if de > 0.0 {
                    subsequent_rej.reset();
                }

            } else {
                subsequent_rej.increment();
            }


        }

        MrResult {
            energy: master_energy,
            state: master_state,
        }
    }
}
