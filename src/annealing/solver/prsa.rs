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

/******************************************************************************
*******************************************************************************
/// **
/// Parallel Recombinative Simulated Annealing (PRSA)
/// *  
*******************************************************************************
*******************************************************************************/
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

use rand::{Rng,thread_rng};
use rand::distributions::{Range, IndependentSample};
use ansi_term::Colour::Green;
use std::collections::HashMap;
use pbr::{ProgressBar,MultiBar};
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
    
	type State=State;


	fn solve(&mut self, problem: &mut Problem) -> MrResult {
      
    	let cooler=StepsCooler {
                      max_steps:self.max_steps,
                      min_temp: self.min_temp,
                      max_temp: self.max_temp,
                      };

        let mut start_time = time::precise_time_ns();

			
		//Get num_cores initial different populations 
	 	let num_cores = common::get_num_cores();
	 	
 	    let mut master_state = problem.initial_state();
 	    
 	    //Generate a Population of specified size with different configurations randomly selected
 	    //from the space state
 	    let mut population = problem.get_population(self.population_size);
 	    
		let mut elapsed_steps = common::ElapsedSteps::new();
 		
 		let mut mb = MultiBar::new();
 		/************************************************************************************************************/
        start_time = time::precise_time_ns();
        
        
        
		MrResult {
          energy: 9.8,
          state: master_state,
          }
	}
                        
                
}                     	


fn get_parents(sub_population: &mut Vec<State>) -> (State, State) {
		let mut rng = rand::thread_rng(); 
		let len=sub_population.len();
		let parent_1=sub_population.swap_remove(rng.gen_range(0, len-1));
		let parent_2=sub_population.swap_remove(rng.gen_range(0, len-1));
		return (parent_1, parent_2);
} 
	
fn generate_children(problem: &mut Problem, parent_1: &State, 
					parent_2: &State) -> (State, State) {
	
	//Enforce Crossover between parent_1 and parent_2 configurations
	let cutting_point = ((0.4*parent_1.len() as f64).floor()) as usize;
	
	let mut child_1 = HashMap::new();
	let mut child_2 = HashMap::new(); 

	let mut p1_iter=parent_1.iter();
	let mut p2_iter=parent_2.iter();
	let (iters_size, _)=p1_iter.size_hint();
	
	for i in 0..iters_size{
		let (mut key_p1,mut val_p1)=p1_iter.next().unwrap();
		let (mut key_p2,mut val_p2)=p2_iter.next().unwrap();
		
		if i < cutting_point {
        	child_1.insert(key_p1.clone(),val_p1.clone());
        	child_2.insert(key_p2.clone(),val_p2.clone());
        }else{
    		child_1.insert(key_p2.clone(),val_p2.clone());
        	child_2.insert(key_p1.clone(),val_p1.clone());
    	}
	}
			
	//Enforce Uniform Mutation on Child_1: This operator replaces the value of the chosen "gene" (configuration parameter) with a 
	//uniform random value selected between the upper and lower bounds for that gene (into the space state of the configuration parameter).
	let mut keys: Vec<_> = child_1.keys().map(|arg| {
	    arg.clone()
	}).collect();
	
	let keys_c=keys.clone();
	let mut random_gene=rand::thread_rng().choose(&keys_c).unwrap();
	let mut gen_space_state=&problem.params_configurator.params_space_state.get(random_gene);
	
    let mut new_value = rand::thread_rng().choose(&*gen_space_state.unwrap()).unwrap();
	*(child_1).get_mut(random_gene).unwrap() = *new_value;


	//Enforce Mutation on Child_2
	keys=child_2.keys().map(|arg| {
	    arg.clone()
	}).collect();
	random_gene=rand::thread_rng().choose(&keys).unwrap();
	
	let mut gen_space_state_2=&problem.params_configurator.params_space_state.get(random_gene);
	
    new_value = rand::thread_rng().choose(&*gen_space_state_2.unwrap()).unwrap();
	*(child_2).get_mut(random_gene).unwrap() = *new_value;
	
	
	
	return (child_1, child_2);

}	




