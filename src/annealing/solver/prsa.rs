/// ///////////////////////////////////////////////////////////////////////////
///  File: annealing/solver/prsa.rs
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
/// Parallel Recombinative Simulated Annealing (PRSA)
/// *
/// *****************************************************************************
/// ****************************************************************************
use annealing::solver::Solver;
use annealing::problem::Problem;
use annealing::cooler::{Cooler, StepsCooler, TimeCooler};
use annealing::solver::common;
use annealing::solver::common::{MrResult,IntermediateResults};
use results_emitter;
use results_emitter::{Emitter, Emitter2File};

use time;
use CoolingSchedule;
use EnergyType;
use hwloc;
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
use std::thread::JoinHandle;
use State;
use std::sync::mpsc::channel;


#[derive(Debug, Clone)]
pub struct Prsa {
    pub min_temp: f64,
    pub max_temp: f64,
    pub max_steps: usize,
    pub population_size: usize,
    pub cooling_schedule: CoolingSchedule,
    pub energy_type: EnergyType,
}

impl Solver for Prsa {
    fn solve(&mut self, problem: &mut Problem, num_workers: usize) -> MrResult {

        let cooler = StepsCooler {
            max_steps: self.max_steps,
            min_temp: self.min_temp,
            max_temp: self.max_temp,
        };


        // Generate a Population of specified size with different configurations randomly selected
        // from the space state
        let mut population =
            common::StatesPool::new_with_val(problem.get_population(self.population_size));

		let mut subsequent_rejected=0;
   		
        let mut elapsed_steps = common::SharedGenericCounter::new();
        let mut temperature =
            common::Temperature::new(self.max_temp, cooler.clone(), self.cooling_schedule.clone());
        let threads_res = common::ThreadsResults::new();

        /// *********************************************************************************************************
        let mut start_time = time::precise_time_ns();
        let mut rng=rand::thread_rng();
        let mut initial_state = problem.initial_state();
        let mut nrg = match problem.energy(&initial_state.clone(), 0, rng.clone()) {
            Some(nrg) => nrg,
            None => panic!("The initial configuration does not allow to calculate the energy"),
        };
        
        let mut elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;

        let mut final_best_res = MrResult {
            energy: nrg,
            state: initial_state,
        };
        
        // Channel for receiving results from worker threads and send them to the file writer.
        let (tx, rx) = channel::<IntermediateResults>();
        let mut results_emitter = Emitter2File::new();
        // Spawn the thread that will take care of writing results into a CSV file
        let (elapsed_steps_c, temperature_c) = (elapsed_steps.clone(), temperature.clone());
        thread::spawn(move || loop {
            elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;
            match rx.recv() {
                Ok(res) => {
                    results_emitter.send_update(temperature_c.get(),
                                                elapsed_time,
                                                0.0,
                                                res.last_nrg,
                                                &res.last_state,
                                                res.best_nrg,
                                                &res.best_state,
                                                elapsed_steps_c.get());
                }
                Err(e) => {} 
            }
        });
        
        
        'outer: loop {
        	
        	let mut mb = MultiBar::new();


            if elapsed_steps.get() > self.max_steps || subsequent_rejected > 200{
                break 'outer;
            }

            elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;

            println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------------------------------------------"));
            println!("{} Completed Steps: {:.2} - Percentage of Completion: {:.2}% - Estimated \
                      time to Complete: {:.2} Hrs",
                     Green.paint("[TUNER]"),
                     elapsed_steps.get(),
                     elapsed_steps.get() as f64 / (self.max_steps as f64) * 100.0,
                     elapsed_time);
        	println!("{} Best Energy: {:.4}",
                     Green.paint("[TUNER]"),
                     final_best_res.energy);
            println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------------------------------------------"));


            // Shuffle pointers of population elements
            population.shuffle();

            let mut th_handlers: Vec<JoinHandle<_>> = Vec::with_capacity(num_workers);

            // Divide the population in num_workers chunks
            let chunk_size = (self.population_size as f64 / num_workers as f64).floor() as usize;
            let mut chunks: Vec<Vec<State>> =
                (0..num_workers).map(|_| Vec::with_capacity(num_workers)).collect();
            for i in 0..num_workers {
                for j in 0..chunk_size {
                    match population.pop() {
                        Some(v) => {
                            chunks[i].push(v);
                        }
                        None => break,
                    };
                }
            }


            for core in 0..num_workers {

                let sub_population = chunks[core].clone();


                let mut problem_c = problem.clone();
                let elapsed_steps_c = elapsed_steps.clone();

                let nrg_type = self.clone().energy_type;
                let max_steps = self.clone().max_steps;
                let cooling_sched = self.clone().cooling_schedule;
                let cooler_c = cooler.clone();
                let population_c = population.clone();
                let temperature_c = temperature.clone();
                let sub_population_c = sub_population.clone();
                let threads_res_c = threads_res.clone();
                let len_subpop = sub_population_c.len();



                let mut pb = mb.create_bar((len_subpop / 2) as u64);
				let tx_c=tx.clone();
	       		let mut final_best_res_c=final_best_res.clone();
                /*********************************************************************************************************/
                th_handlers.push(thread::spawn(move || {
                    pb.show_message = true;
       				let mut rng_c = rand::thread_rng();

                    let mut new_sub_population: Vec<State> = Vec::with_capacity(len_subpop);

                    let mut results: Vec<common::MrResult> = Vec::new();

                    for step in 0..len_subpop / 2 {
                        pb.message(&format!("TID [{}] - Sub-Population Exploration Status - ",
                                            core));

                        let (parent_1, parent_2) = get_parents(&mut sub_population_c.to_vec(),rng_c.clone());

                        let cost_parent_1 = problem_c.energy(&parent_1, core, rng_c.clone())
                            .unwrap();

 						let intermediate_res=IntermediateResults{
				            			last_nrg: cost_parent_1,
				            			last_state: parent_1.clone(),
				            			best_nrg: final_best_res_c.clone().energy,
				            			best_state: final_best_res_c.clone().state,
	            		};
	            		tx_c.send(intermediate_res);

                        let cost_parent_2 = problem_c.energy(&parent_2, core, rng_c.clone())
                            .unwrap();

						let intermediate_res=IntermediateResults{
				            			last_nrg: cost_parent_2,
				            			last_state: parent_2.clone(),
				            			best_nrg: final_best_res_c.clone().energy,
				            			best_state: final_best_res_c.clone().state,
	            		};
	            		tx_c.send(intermediate_res);
	            		
                        let (mut child_1, mut child_2) =
                            generate_children(&mut problem_c, &parent_1, &parent_2, rng_c.clone());

                        let cost_child_1 = problem_c.energy(&child_1, core, rng_c.clone())
                            .unwrap();
                            
                        let intermediate_res=IntermediateResults{
				            			last_nrg: cost_child_1,
				            			last_state: child_1.clone(),
				            			best_nrg: final_best_res_c.clone().energy,
				            			best_state: final_best_res_c.clone().state,
	            		};
	            		tx_c.send(intermediate_res);
	            		    
                        let cost_child_2 = problem_c.energy(&child_2, core, rng_c.clone())
                            .unwrap();

						let intermediate_res=IntermediateResults{
				            			last_nrg: cost_child_2,
				            			last_state: child_2.clone(),
				            			best_nrg: final_best_res_c.clone().energy,
				            			best_state: final_best_res_c.clone().state,
	            		};
	            		tx_c.send(intermediate_res);
	            		
                        // Compare cost of parent_1 with cost of child_2
                        let range = Range::new(0.0, 1.0);

                        let de_p1_c2 = match nrg_type {
                            EnergyType::throughput => cost_parent_1 - cost_child_2,
                            EnergyType::latency => -(cost_parent_1 - cost_child_2), 
                        };

                        let (best_state_1, best_cost_1) = {
                            if range.ind_sample(&mut rng_c) <
                               1.0 / (1.0 + (de_p1_c2 / temperature_c.get()).exp()) {
                                (parent_1, cost_parent_1)
                            } else {
                                (child_2, cost_child_2)
                            }
                        };



                        // Compare cost of parent_2 with cost of child_1
                        let de_p2_c1 = match nrg_type {
                            EnergyType::throughput => cost_parent_2 - cost_child_1,
                            EnergyType::latency => -(cost_parent_2 - cost_child_1), 
                        };

                        let (best_state_2, best_cost_2) = {
                            if range.ind_sample(&mut rng_c) <
                               1.0 / (1.0 + (de_p2_c1 / temperature_c.get()).exp()) {
                                (parent_2, cost_parent_2)
                            } else {
                                (child_1, cost_child_1)
                            }
                        };

                        new_sub_population.push(best_state_1.clone());
                        new_sub_population.push(best_state_2.clone());


                        let (iter_best_state, iter_best_cost) = match nrg_type {
                            EnergyType::throughput => {
                                if best_cost_1 > best_cost_2 {
                                    (best_state_1, best_cost_1)
                                } else {
                                    (best_state_2, best_cost_2)
                                }
                            }
                            EnergyType::latency => {
                                if best_cost_1 > best_cost_2 {
                                    (best_state_2, best_cost_2)
                                } else {
                                    (best_state_1, best_cost_1)
                                }
                            } 
                        };

                        results.push(common::MrResult {
                            energy: iter_best_cost,
                            state: iter_best_state.clone(),
                        });
                        
                       
                        
                        temperature_c.update(elapsed_steps_c.get());
                        elapsed_steps_c.increment();
                   		pb.inc();

                    }

                    let chunk_best_res = eval_best_res(&mut results, nrg_type);
                    threads_res_c.push(chunk_best_res);

                    pb.finish_print(&format!("Child Thread [{}] Terminated the Execution", core));

                    population_c.push_bulk(&mut new_sub_population);
                }));

            }

            mb.listen();

            for h in th_handlers {
                h.join().unwrap();
            }

            let best_subpop_res = eval_best_res(&mut threads_res.get_coll(), self.energy_type);


			let de = match self.energy_type {
                EnergyType::throughput => best_subpop_res.energy - final_best_res.energy,
                EnergyType::latency => -(best_subpop_res.energy - final_best_res.energy), 
            };
			
            let range = Range::new(0.0, 1.0);
            if de > 0.0 || range.ind_sample(&mut rng) <= (de / temperature.get()).exp() {
                final_best_res=best_subpop_res;
                if de > 0.0 {
					subsequent_rejected+=1;
                }

            } else {
				subsequent_rejected=0;		
            }

			
        }


        final_best_res
    }
}

fn eval_best_res(workers_res: &mut Vec<MrResult>, nrg_type: EnergyType) -> MrResult {
    // Get results of worker threads (each one will put its best evaluated energy) and
    // choose between them which one will be the best
    let first_elem = workers_res.pop().unwrap();

    let mut best_state = first_elem.state;
    let mut best_cost = first_elem.energy;

    for elem in workers_res.iter() {
        let diff = match nrg_type {
            EnergyType::throughput => elem.energy - best_cost,
            EnergyType::latency => -(elem.energy - best_cost), 
        };
        if diff > 0.0 {
            best_cost = elem.clone().energy;
            best_state = elem.clone().state;
        }
    }

    MrResult {
        energy: best_cost,
        state: best_state,
    }
}


fn get_parents(sub_population: &mut Vec<State>, mut rng: rand::ThreadRng) -> (State, State) {
    let len = sub_population.len();
    let parent_1 = sub_population.swap_remove(rng.gen_range(0, len - 1));
    let parent_2 = sub_population.swap_remove(rng.gen_range(0, len - 1));
    return (parent_1, parent_2);
}

fn generate_children(problem: &mut Problem, parent_1: &State, parent_2: &State, mut rng: rand::ThreadRng) -> (State, State) {

    // Enforce Crossover between parent_1 and parent_2 configurations
    let cutting_point = ((0.4 * parent_1.len() as f64).floor()) as usize;

    let mut child_1 = HashMap::with_capacity(parent_1.len());
    let mut child_2 = HashMap::with_capacity(parent_1.len());

    let mut p1_iter = parent_1.iter();

    for i in 0..parent_1.len() {
        let (mut key_p1, mut val_p1) = p1_iter.next().unwrap();
        let mut key_p2 = key_p1;
        let mut val_p2 = parent_2.get(key_p2).unwrap();


        if i < cutting_point {
            child_1.insert(key_p1.clone(), val_p1.clone());
            child_2.insert(key_p2.clone(), val_p2.clone());
        } else {
            child_1.insert(key_p2.clone(), val_p2.clone());
            child_2.insert(key_p1.clone(), val_p1.clone());
        }
    }

    // Enforce Uniform Mutation on Child_1: This operator replaces the value of the chosen "gene" (configuration parameter) with a
    // uniform random value selected between the upper and lower bounds for that gene (into the space state of the configuration parameter).
    let mut keys: Vec<_> = child_1.keys()
        .map(|arg| arg.clone())
        .collect();

    let keys_c = keys.clone();
    let mut random_gene = rng.choose(&keys_c).unwrap();
    let mut gen_space_state = &problem.params_configurator.params_space_state.get(random_gene);

    let mut new_value = rng.choose(&*gen_space_state.unwrap()).unwrap();
    *(child_1).get_mut(random_gene).unwrap() = *new_value;


    // Enforce Mutation on Child_2
    keys = child_2.keys()
        .map(|arg| arg.clone())
        .collect();
    random_gene = rng.choose(&keys).unwrap();

    let mut gen_space_state_2 = &problem.params_configurator.params_space_state.get(random_gene);

    new_value = rng.choose(&*gen_space_state_2.unwrap()).unwrap();
    *(child_2).get_mut(random_gene).unwrap() = *new_value;



    return (child_1, child_2);

}
