/// ///////////////////////////////////////////////////////////////////////////
///  File: neil/problem.rs
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

use states_gen;
use energy_eval;
use EnergyType;
use ProblemType;
use std::collections::HashMap;
use std::fmt::Debug;
use rustc_serialize::Encodable;
use State;
use rand;
use rand::{Rng, thread_rng};
use std::f64::consts;
use rand::distributions::Range;
use rand::distributions::IndependentSample;
/**
 * A problem represents something to be solved using simulated
 * annealing, and provides methods to calculate the energy of a
 * state and generate new states.
 */
#[derive(Debug, Clone)]
pub struct Problem {
    pub problem_type: ProblemType,
    pub params_configurator: states_gen::ParamsConfigurator,
    pub energy_evaluator: energy_eval::EnergyEval,
}


impl Problem {
    /**
	Return space of Neighborhoods of a specific state given in input
	**/
    pub fn neigh_space(&mut self, state: &State) -> Vec<State> {
        return self.params_configurator.get_neigh_one_varying(state);
    }


    /**
	Start Extraction of Initial State: it takes the Parameters Configuration 
    given in input
	**/
    pub fn initial_state(&mut self) -> State {
        return self.params_configurator.get_initial_param_conf();
    }


    /**
	Start Energy Evaluation: it starts the execution of the benchmark for the 
    specific parameter configuration and evaluate the performance result
	**/
    pub fn energy(&mut self,
                  state: &State,
                  energy_type: EnergyType,
                  id_thread: usize,
                  mut rng: rand::ThreadRng)
                  -> Option<f64> {

        /// Data Needed for the benchmark problems*
        // The dimension of the hypercube for the benchmark_problems
        let hypcube_dim = 30;
        let x: Vec<_> = (0..hypcube_dim)
            .map(|_| self.domain_dist().ind_sample(&mut rng))
            .collect();
        let p = x.len() as f64;

        let nrg = match self.problem_type {

            ProblemType::default => {
                self.energy_evaluator.execute_test_instance(state, energy_type, id_thread)
            }

            ProblemType::rastr => {
                Some(10_f64 * p +
                     x.iter().fold(0_f64, |sum, x| {
                    sum + x.powi(2) - 10_f64 * (2_f64 * consts::PI * x).cos()
                }))
            }

            ProblemType::griew => {
                Some(1_f64 + x.iter().fold(0_f64, |sum, x| sum + x.powi(2) / 4000_f64) -
                     x.iter().enumerate().fold(1_f64, |prod, (i, x)| {
                    prod * (x / ((i + 1) as f64).sqrt()).cos()
                }))
            }   
        };

        return nrg;

    }


    /**
	Start Extraction of New Neighborhood State 
	**/
    pub fn new_state(&mut self,
                     state: &State,
                     max_steps: usize,
                     current_step: usize)
                     -> Option<State> {
        return self.params_configurator.get_neighborhood(state, max_steps, current_step);
    }

    /**
	Return a random state
	**/
    pub fn rand_state(&mut self) -> State {
        return self.params_configurator.get_rand_param();
    }



    /**
	Return random population
	**/
    pub fn get_population(&mut self, size: usize) -> Vec<State> {
        return self.params_configurator.get_rand_population(size);
    }



    /// Domain for the benchmark problems.
    fn domain(&self) -> (f64, f64) {
        match self.problem_type {
            ProblemType::default => (0.0, 1.0),
            ProblemType::rastr => (-5.12_f64, 5.12_f64),
            ProblemType::griew => (-600_f64, 600_f64),
        }
    }

    /// Random distribution for benchmark problem's domain
    fn domain_dist(&self) -> Range<f64> {
        let (a, b) = self.domain();
        Range::new(a, b)
    }
}
