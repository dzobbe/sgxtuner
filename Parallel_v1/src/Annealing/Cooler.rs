/// ///////////////////////////////////////////////////////////////////////////
///  File: ParallelAnnealing/Cooler.rs
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

#[derive(Debug, Clone)]
pub struct TimeCooler {
    pub max_time: usize,
    pub min_temp: f64,
    pub max_temp: f64,
}

#[derive(Debug, Clone)]
pub struct StepsCooler {
    pub max_steps: usize,
    pub min_temp: f64,
    pub max_temp: f64,
}

pub trait Cooler {
    fn exponential_cooling(&self, metric: usize) -> f64;
    fn linear_cooling(&self, metric: usize) -> f64;
    fn basic_exp_cooling(&self, current_temp: f64) -> f64;
}



impl Cooler for TimeCooler {
    fn exponential_cooling(&self, elapsed_time: usize) -> f64 {
        return 0.0;
    }

    fn linear_cooling(&self, elapsed_time: usize) -> f64 {
        return 0.0;

    }

    fn basic_exp_cooling(&self, current_temp: f64) -> f64 {
        return 0.0;
    }
}


impl Cooler for StepsCooler {
    fn exponential_cooling(&self, step: usize) -> f64 {
        if self.min_temp <= 0.0 {
            panic!("Exponential cooling requires a minimum temperature greater than zero");
        }

        let reduction_factor = -(self.max_temp / self.min_temp).ln();

        return self.max_temp * (reduction_factor * (step as f64) / (self.max_steps as f64)).exp();
    }

    fn linear_cooling(&self, step: usize) -> f64 {
        if self.min_temp <= 0.0 {
            panic!("Linear cooling requires a minimum temperature greater than zero");
        }

        let reduction_factor = -(self.max_temp / self.min_temp).ln();
        return self.max_temp *
               (1.0 / 1.0 + (0.7 * (reduction_factor * (step as f64) / (self.max_steps as f64))));

    }

    fn basic_exp_cooling(&self, current_temp: f64) -> f64 {
        return current_temp*0.99;
    }
}
