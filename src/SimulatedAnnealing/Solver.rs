/// ///////////////////////////////////////////////////////////////////////////
///  File: neil/solver.rs
/// ///////////////////////////////////////////////////////////////////////////
///  Copyright 2016 Samuel Sleight
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
use TerminationCriteria;
use rand::thread_rng;
use rand::distributions::{Range, IndependentSample};
use ansi_term::Colour::{Green};
use super::Problem::Problem;
use super::Cooler::{Cooler, StepsCooler, TimeCooler};

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
     * The maximimum number of attempts to find a new state
     * before lowering the temperature.
     */
    pub max_attempts: u64,


    /**
     * The maximum number of accepted new states before lowering
     * the temperature.
     */
    pub max_accepts: u64,

    /**
     * The maximum number of rejected states before terminating the
     * process.
     */
    pub max_rejects: u64,
    
    /**
     * The Cooling Schedule procedure to select
     */
    pub cooling_schedule: CoolingSchedule
    
    
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
     * Run the solver for a maximization problem
     */
    pub fn solve<P>(&self, problem: &mut P) -> P::State
        where P: Problem
    {
		let best_configuration=match self.termination_criteria {
			TerminationCriteria::Max_Steps(value) 		 => {
				self.solve_step_based(problem, value,
					 StepsCooler{max_steps: value, min_temp: self.min_temperature, max_temp: self.max_temperature})
				},
			TerminationCriteria::Max_Time_Seconds(value) => {
				self.solve_time_based(problem, value, 
					TimeCooler{max_time: value, min_temp: self.min_temperature, max_temp: self.max_temperature})
				}
		};
	
		return best_configuration;
    }
    
    
    fn solve_step_based<P>(&self, problem: &mut P, max_steps: u64, cooler: StepsCooler) -> P::State
        where P: Problem
    {
        let mut rng = thread_rng();
        let range = Range::new(0.0, 1.0);

        let mut state = problem.initial_state();
        let mut energy = problem.energy(&state);
        let mut temperature = self.max_temperature;

        let mut attempted = 0;
        let mut accepted = 0;
        let mut rejected = 0;
		let mut total_improves=0;
		let mut subsequent_improves=0;
		let mut elapsed_time=0.0;

		
        let start_time = time::precise_time_ns();

        for elapsed_steps in 0..max_steps {
        	
        	elapsed_time =(time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;
        	
        	println!("{} Step Number: {:?} - Current Execution Time: {:?} s", Green.paint("[ANNEALING SOLVER]"), elapsed_steps,elapsed_time);
        	
            state = {
                let next_state = problem.new_state(&state, max_steps, elapsed_steps);
                let new_energy = problem.energy(&next_state);

                attempted += 1;

                let de = new_energy - energy;

                if de > 0.0 || range.ind_sample(&mut rng) <= (-de / temperature).exp() {
                    println!("{} New Accepted Solution: {:?}", Green.paint("[ANNEALING SOLVER]"), elapsed_steps);
                    accepted += 1;
                    energy = new_energy;
                    
                    if de > 0.0{
                    	total_improves=total_improves+1;
                    	subsequent_improves=subsequent_improves+1;
                    }
                    next_state
                    
                } else {
                	subsequent_improves=0;
                    state
                }
            };

            if attempted >= self.max_attempts || accepted >= self.max_accepts {
                if accepted == 0 {
                    rejected += 1;
                } else {
                    rejected = 0;
                }
                

                attempted = 0;
                accepted = 0;
                
                let temperature: f64 = match self.cooling_schedule{
        			CoolingSchedule::Linear 	 => cooler.linear_cooling(),
        			CoolingSchedule::Exponential => cooler.exponential_cooling(elapsed_steps),
        			CoolingSchedule::Adaptive 	 => cooler.adaptive_cooling(),
                };
                
                
                if rejected >= self.max_rejects {
                    break;
                }
            }
        }

        state
    }
    
    
    
    fn solve_time_based<P>(&self, problem: &mut P, max_time: u64, cooler: TimeCooler) -> P::State
        where P: Problem
    {
        let mut rng = thread_rng();
        let range = Range::new(0.0, 1.0);

        let mut state = problem.initial_state();
        
        return state;
    }
    

	 /**
     * Automatica Evaluation of Tmin and Tmax based on certain number 
     * of algorithm executions
     */
	 pub fn auto_param_evaluation(&self){
	 	
	 }
}

impl Default for Solver {
    fn default() -> Solver {
        Solver {
            termination_criteria: TerminationCriteria::Max_Steps(1000000),
            min_temperature: 2.5,
            max_temperature: 100.0,
            max_attempts: 50,
            max_accepts: 10,
            max_rejects: 4,
            cooling_schedule: CoolingSchedule::Exponential
        }
    }
}
