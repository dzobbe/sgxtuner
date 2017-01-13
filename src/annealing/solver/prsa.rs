/// ///////////////////////////////////////////////////////////////////////////
///  File: Annealing/Solver/PRSA.rs
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
use annealing::solver::common::MrResult;
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
    fn solve(&mut self, problem: &mut Problem) -> MrResult {

        let cooler = StepsCooler {
            max_steps: self.max_steps,
            min_temp: self.min_temp,
            max_temp: self.max_temp,
        };

        // Get num_cores initial different populations
        let num_cores = common::get_num_cores();

        // Generate a Population of specified size with different configurations randomly selected
        // from the space state
        let mut population = common::StatesPool::new_with_val(problem.get_population(self.population_size));

        let mut elapsed_steps = common::ElapsedSteps::new();

        let mut mb = MultiBar::new();
        /// *********************************************************************************************************
        let mut start_time = time::precise_time_ns();
        'outer: loop {

            if elapsed_steps.get() > self.max_steps {
                break 'outer;
            }

            let elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;

            println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------------------------------------------"));
            println!("{} Completed Steps: {:.2} - Percentage of Completion: {:.2}% - Estimated \
                      time to Complete: {:.2} Hrs",
                     Green.paint("[TUNER]"),
                     elapsed_steps.get(),
                     elapsed_steps.get() as f64 / (self.max_steps as f64) * 100.0,
                     elapsed_time);
            println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------------------------------------------"));


            // Shuffle pointers of population elements
			population.shuffle();
			
            let mut th_handlers: Vec<JoinHandle<_>> = Vec::with_capacity(num_cores);

            println!("Pop: {:?}", population);

            // Divide the population in num_cores chunks
            let chunk_size = (self.population_size as f64 / num_cores as f64).floor() as usize;
            let mut chunks: Vec<Vec<State>> =
                (0..num_cores).map(|_| Vec::with_capacity(num_cores)).collect();
            for i in 0..num_cores {
                for j in 0..chunk_size {
                    match population.pop() {
                        Some(v) => {
                            chunks[i].push(v);
                        }
                        None => break,
                    };
                }
            }
			
			println!("SS {}",population.size());

            for core in 0..num_cores {

                let mut problem_c = problem.clone();
                let elapsed_steps_c = elapsed_steps.clone();

                let nrg_type = self.clone().energy_type;
                let max_steps = self.clone().max_steps;
                let cooling_sched = self.clone().cooling_schedule;
                let cooler_c = cooler.clone();
				let population_c=population.clone();
                let sub_population = chunks[core].clone();
                let mut temperature = self.max_temp;
                let sub_population_c = sub_population.clone();
                

                /// *********************************************************************************************************
                th_handlers.push(thread::spawn(move || {
                    let len_subpop = sub_population_c.len();
	                let mut pb = mb.create_bar((len_subpop/2) as u64);
                    pb.show_message = true;
                    
                    let mut new_sub_population: Vec<State> = Vec::with_capacity(len_subpop);

                    let mut rng = rand::thread_rng();
                    for step in 0..len_subpop / 2 {
                        pb.message(&format!("TID [{}] - Sub-Population Exploration Status - ",
                                            core));

                        let (parent_1, parent_2) = get_parents(&mut sub_population_c.to_vec());

                        let cost_parent_1 = problem_c.energy(&parent_1, nrg_type.clone(), core)
                            .unwrap();
                        let cost_parent_2 = problem_c.energy(&parent_2, nrg_type.clone(), core)
                            .unwrap();

                        let (mut child_1, mut child_2) =
                            generate_children(&mut problem_c, &parent_1, &parent_2);

                        let cost_child_1 = problem_c.energy(&child_1, nrg_type.clone(), core)
                            .unwrap();
                        let cost_child_2 = problem_c.energy(&child_2, nrg_type.clone(), core)
                            .unwrap();

                        // Compare cost of parent_1 with cost of child_2
                        let range = Range::new(0.0, 1.0);

                        let de_p1_c2 = match nrg_type {
                            EnergyType::throughput => cost_parent_1 - cost_child_2,
                            EnergyType::latency => -(cost_parent_1 - cost_child_2), 
                        };

                        if range.ind_sample(&mut rng) <
                           1.0 / (1.0 + (de_p1_c2 / temperature).exp()) {
                            new_sub_population.push(parent_1);
                        } else {
                            new_sub_population.push(child_2);
                        }



                        // Compare cost of parent_2 with cost of child_1
                        let de_p2_c1 = match nrg_type {
                            EnergyType::throughput => cost_parent_2 - cost_child_1,
                            EnergyType::latency => -(cost_parent_2 - cost_child_1), 
                        };

                        if range.ind_sample(&mut rng) <
                           1.0 / (1.0 + (de_p2_c1 / temperature).exp()) {
                            new_sub_population.push(parent_2);
                        } else {
                            new_sub_population.push(child_1);
                        }


                        pb.inc();
    					elapsed_steps_c.increment();
    					
                        temperature = match cooling_sched {
                            CoolingSchedule::linear => cooler_c.linear_cooling(step),
                            CoolingSchedule::exponential => cooler_c.exponential_cooling(step),
                            CoolingSchedule::basic_exp_cooling => cooler_c.basic_exp_cooling(temperature),
                        };
                        
                    }

                    pb.finish_print(&format!("Child Thread [{}] Terminated the Execution", core));

                    population_c.push_bulk(&mut new_sub_population);
                }));
 
            }

            mb.listen();

            for h in th_handlers {
                h.join().unwrap();
            }
        }


        MrResult {
            energy: 9.8,
            state: master_state,
        }
    }
}


fn get_parents(sub_population: &mut Vec<State>) -> (State, State) {
    let mut rng = rand::thread_rng();
    let len = sub_population.len();
    let parent_1 = sub_population.swap_remove(rng.gen_range(0, len - 1));
    let parent_2 = sub_population.swap_remove(rng.gen_range(0, len - 1));
    return (parent_1, parent_2);
}

fn generate_children(problem: &mut Problem, parent_1: &State, parent_2: &State) -> (State, State) {

    // Enforce Crossover between parent_1 and parent_2 configurations
    let cutting_point = ((0.4 * parent_1.len() as f64).floor()) as usize;

    let mut child_1 = HashMap::new();
    let mut child_2 = HashMap::new();

    let mut p1_iter = parent_1.iter();
    let mut p2_iter = parent_2.iter();
    let (iters_size, _) = p1_iter.size_hint();

    for i in 0..iters_size {
        let (mut key_p1, mut val_p1) = p1_iter.next().unwrap();
        let (mut key_p2, mut val_p2) = p2_iter.next().unwrap();

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
    let mut random_gene = rand::thread_rng().choose(&keys_c).unwrap();
    let mut gen_space_state = &problem.params_configurator.params_space_state.get(random_gene);

    let mut new_value = rand::thread_rng().choose(&*gen_space_state.unwrap()).unwrap();
    *(child_1).get_mut(random_gene).unwrap() = *new_value;


    // Enforce Mutation on Child_2
    keys = child_2.keys()
        .map(|arg| arg.clone())
        .collect();
    random_gene = rand::thread_rng().choose(&keys).unwrap();

    let mut gen_space_state_2 = &problem.params_configurator.params_space_state.get(random_gene);

    new_value = rand::thread_rng().choose(&*gen_space_state_2.unwrap()).unwrap();
    *(child_2).get_mut(random_gene).unwrap() = *new_value;



    return (child_1, child_2);

}
