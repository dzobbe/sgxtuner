//////////////////////////////////////////////////////////////////////////////
//  File: neil/solver.rs
//////////////////////////////////////////////////////////////////////////////
//  Copyright 2016 Samuel Sleight
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//////////////////////////////////////////////////////////////////////////////
extern crate rand;

use self::rand::thread_rng;
use self::rand::distributions::{Range, IndependentSample};

use super::Problem::Problem;
use std::time;

/**
 * A solver will take a problem and use simulated annealing
 * to try and find an optimal state.
 */
#[derive(Debug, Clone)]
pub struct Solver {
    /**
     * The maximum number of steps to run the algorithm
     * for.
     */
    pub steps: u64,

    
    /**
     * The initial temperature of the process.
     */
    pub initial_temperature: f64,

    /**
     * The factor to multiply the temperature by each time it
     * is lowered - this should be a number between 0.0 and 1.0.
     */
    pub temperature_reduction: f64,

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
    pub fn build_new<F>(builder: F) -> Solver where F: FnOnce(&mut Solver) {
        let mut solver = Solver::new();
        builder(&mut solver);
        solver
    }

    /**
     * Run the solver on the given problem.
     */
    pub fn solve<P>(&self, problem: &mut P) -> P::State where P: Problem {
        let mut rng = thread_rng();
        let range = Range::new(0.0, 1.0);

        let mut state = problem.initial_state();
        let mut energy = problem.energy(&state);
        let mut temperature = self.initial_temperature;

        let mut attempted = 0;
        let mut accepted = 0;
        let mut rejected = 0;
		
		//let timespec = time::get_time();
    	//let start_time_ms: f64 = timespec.sec as f64 + (timespec.nsec as f64 / 1000.0 / 1000.0 / 1000.0 );
    	
   
        for elapsed_steps in 0 .. self.steps {
            state = {
                let next_state = problem.new_state(&state,self.steps,elapsed_steps);
                let new_energy = problem.energy(&next_state);

                attempted += 1;

                let de = new_energy - energy;

                if de < 0.0 || range.ind_sample(&mut rng) <= -de / temperature {
                    accepted += 1;
                    energy = new_energy;

                    next_state
                } else {
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
                temperature *= self.temperature_reduction;

                if rejected >= self.max_rejects {
                    break
                }
            } 
        }
	
        state
    }
    
    fn compute_T_factor(& self){
    	
    }
    
    fn reduce_temperature(& self){
    	
    }
}

impl Default for Solver {
    fn default() -> Solver {
        Solver {
            steps: 1000000,
            initial_temperature: 100.0,
            temperature_reduction: 0.95,
            max_attempts: 50,
            max_accepts: 10,
            max_rejects: 4,
        }
    }
}

