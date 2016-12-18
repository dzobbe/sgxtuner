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

use Parameters;
use EnergyType;
use EnergyEval;
use std::collections::HashMap;
use std::fmt::Debug;
use rustc_serialize::Encodable;

/**
 * A problem represents something to be solved using simulated
 * annealing, and provides methods to calculate the energy of a
 * state and generate new states.
 */

#[derive(Debug, Clone)]
pub struct Problem {
    pub params_configurator: Parameters::ParamsConfigurator,
    pub energy_evaluator: EnergyEval::EnergyEval,
}


impl Problem {
	
	
	pub fn neigh_space(&mut self, state: &HashMap<String, u32>) -> Vec<HashMap<String, u32>> {
        return self.params_configurator.get_neigh_one_varying(state);
    }
	
	
    /**
	Start Extraction of Initial State: it takes the Parameters Configuration 
    given in input
	**/
    pub fn initial_state(&mut self) -> HashMap<String, u32> {
        return self.params_configurator.get_initial_param_conf();
    }


    /**
	Start Energy Evaluation: it starts the execution of the benchmark for the 
    specific parameter configuration and evaluate the performance result
	**/
    pub fn energy(&mut self, state: HashMap<String, u32>, energy_type: EnergyType) -> Option<f64> {
        return self.energy_evaluator.execute_test_instance(state, energy_type);
    }


    /**
	Start Extraction of New State from Neighborhood Set
	**/
    pub fn new_state(&mut self,
                 state: &HashMap<String, u32>,
                 max_steps: u64,
                 current_step: u64)
                 -> Option<HashMap<String, u32>> {
        return self.params_configurator.get_rand_neighborhood(state, max_steps, current_step);
    }
}
