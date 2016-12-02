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
use EnergyType;
use TerminationCriteria;
use rand::thread_rng;
use rand::distributions::{Range, IndependentSample};
use ansi_term::Colour::Green;
use super::Problem::Problem;
use super::Updater::{Updater, UpdateFile};
use super::Cooler::{Cooler, StepsCooler, TimeCooler};
use std::fs::{File, OpenOptions};
use std::collections::HashMap;

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

    /**
     * Run the solver for a maximization problem
     */
    pub fn solve(&self, problem: &mut Problem) -> HashMap<String, u32> {
        let best_configuration = match self.termination_criteria {
            TerminationCriteria::Max_Steps(value) => {
                self.solve_step_based(problem,
                                      value,
                                      StepsCooler {
                                          max_steps: value,
                                          min_temp: self.min_temperature,
                                          max_temp: self.max_temperature,
                                      })
            }
            TerminationCriteria::Max_Time_Seconds(value) => {
                self.solve_time_based(problem,
                                      value,
                                      TimeCooler {
                                          max_time: value,
                                          min_temp: self.min_temperature,
                                          max_temp: self.max_temperature,
                                      })
            }
        };

        return best_configuration;
    }


    fn solve_step_based(&self,
                        problem: &mut Problem,
                        max_steps: u64,
                        cooler: StepsCooler)
                        -> HashMap<String, u32> {
        let mut updater = UpdateFile::new();
        let mut rng = thread_rng();
        let range = Range::new(0.0, 1.0);

        println!("{}",Green.paint("\n-------------------------------------------------------------------------------------------------------------------"));
        println!("{} Initialization Phase: Evaluation of Energy for Default Paramters",
                 Green.paint("[TUNER]"));
        println!("{}",Green.paint("-------------------------------------------------------------------------------------------------------------------"));

        let mut start_time = time::precise_time_ns();

        let mut state = problem.initial_state();
        let mut energy = match problem.energy(&state, self.energy_type.clone()) {
            Some(nrg) => nrg,
            None => panic!("The initial configuration does not allow to calculate the energy"),
        };

        let mut elapsed_time = (time::precise_time_ns() - start_time) as f64 / 1000000000.0f64;
        let estimated_time_2_complete = ((elapsed_time as f64) * max_steps as f64) / 3600.0;

        let mut temperature: f64 = self.max_temperature;
        let mut attempted = 0;
        let mut accepted = 0;
        let mut rejected = 0;
        let mut total_improves = 0;
        let mut subsequent_improves = 0;
        let mut last_nrg = energy;


        start_time = time::precise_time_ns();

        for elapsed_steps in 0..max_steps {

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
                let next_state = match problem.new_state(&state, max_steps, elapsed_steps) {
                    // There is a neighborhood available
                    Some(n_s) => n_s,
                    // No neighborhood available, all states have been visited
                    None => {
                        println!("{} Any Neighborhood Available - Terminate the Annealing",
                                 Green.paint("[TUNER]"));
                        break;
                    }
                };

                let accepted_state = match problem.energy(&next_state, self.clone().energy_type) {
                    Some(new_energy) => {
                        last_nrg = new_energy;

                        let de = match self.energy_type {
                            EnergyType::throughput => new_energy - energy,
                            EnergyType::latency => -(new_energy - energy), 
                        };

                        if de > 0.0 || range.ind_sample(&mut rng) <= (-de / temperature).exp() {
                            accepted += 1;
                            energy = new_energy;

                            if de > 0.0 {
                                total_improves = total_improves + 1;
                                subsequent_improves = subsequent_improves + 1;
                            }

                            updater.send_update(new_energy,
                                                &next_state,
                                                energy,
                                                &next_state,
                                                elapsed_steps);
                            next_state

                        } else {
                            subsequent_improves = 0;
                            updater.send_update(new_energy,
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
                CoolingSchedule::adaptive => cooler.adaptive_cooling(),
            };
        }

        state
    }



    fn solve_time_based(&self,
                        problem: &mut Problem,
                        max_time: u64,
                        cooler: TimeCooler)
                        -> HashMap<String, u32> {
        let mut rng = thread_rng();
        let range = Range::new(0.0, 1.0);

        let mut state = problem.initial_state();

        return state;
    }



    /**
     * Automatic Evaluation of Tmin and Tmax based on certain number 
     * of algorithm executions
     */
    pub fn auto_param_evaluation(&self) {}
}

impl Default for Solver {
    fn default() -> Solver {
        Solver {
            termination_criteria: TerminationCriteria::Max_Steps(10000),
            min_temperature: 2.5,
            max_temperature: 1000.0,
            cooling_schedule: CoolingSchedule::exponential,
            energy_type: EnergyType::throughput,
        }
    }
}
