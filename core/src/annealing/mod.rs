pub mod problem;
pub mod solver;
pub mod cooler;

use annealing::problem::Problem;
use ansi_term::Colour::Green;
use shared::TunerParameter;


/// Check if the temperature is given by the user or if Tmin and Tmax need to be evaluated
pub fn eval_temperature(tuner_params: &mut TunerParameter, problem: &mut Problem) {
    let num_exec = 10;

    let t_min = tuner_params.min_temp;
    let t_max = tuner_params.max_temp;


    let min_temp = match t_min {
        Some(val) => val,
        None => 1.0,
    };


    let max_temp = match t_max {
        Some(val) => val,
        None => {
            let mut deltas: Vec<f64> = Vec::with_capacity(num_exec);
            /// Search for Tmax: a temperature that gives 98% acceptance
            /// Tmin: equal to 1.
            println!(
                "{} Temperature not provided. Starting its Evaluation",
                Green.paint("[TUNER]")
            );
            let mut state = problem.initial_state();
            let mut energy = match problem.energy(&state, 0) {
                Some(nrg) => nrg,
                None => panic!("The initial configuration does not allow to calculate the energy"),
            };

            for i in 0..num_exec {

                let next_state = problem.rand_state();
                let new_energy = match problem.energy(&next_state, 0) {
                    Some(new_nrg) => deltas.push((energy - new_nrg).abs()),
                    None => {
                        println!(
                            "{} The current configuration parameters cannot be evaluated. \
                                  Skip!",
                            Green.paint("[TUNER]")
                        );
                    }
                };

            }

            let desired_prob: f64 = 0.98;
            let sum_deltas: f64 = deltas.iter().cloned().sum();
            //(energies.iter().cloned().fold(0. / 0., f64::max) -energies.iter().cloned().fold(0. / 0., f64::min))
            (sum_deltas / deltas.len() as f64) / (-desired_prob.ln())
        }
    };


    tuner_params.min_temp = Some(min_temp);
    tuner_params.max_temp = Some(max_temp);

    // return ret_val;
}
