 #![feature(stmt_expr_attributes)] 
extern crate x86;
extern crate perfcnt;
extern crate rustc_serialize;
extern crate docopt;
extern crate rand;
extern crate libc;
extern crate time;
extern crate ansi_term;
extern crate pbr;
extern crate csv;
extern crate hwloc;
extern crate num_cpus;
extern crate wait_timeout;
//extern crate papi;

#[macro_use]
extern crate futures;
#[macro_use]
extern crate tokio_core;
extern crate futures_cpupool;

#[macro_use]
extern crate lazy_static;
extern crate influent;

use ansi_term::Colour::{Green, Yellow};
use std::time::Duration;
use std::collections::HashMap;
use annealing::problem::Problem;
use annealing::solver::seqsea::Seqsea;
use annealing::solver::Solver;
use annealing::solver::common::MrResult;

use std::sync::{Arc, Mutex, Condvar};
use std::sync::RwLock;

use docopt::Docopt;
use std::process::Command;
use std::thread;
use rand::{Rng, thread_rng};

mod perf_counters;
mod states_gen;
mod annealing;
mod energy_eval;
mod meter_proxy;
mod results_emitter;

type State = HashMap<String, usize>;

#[derive(Debug, Clone,RustcDecodable)]
pub enum ProblemType {
    default,
    rastr,
    griew,
}

#[derive(Debug, Clone,RustcDecodable)]
pub enum CoolingSchedule {
    linear,
    exponential,
    basic_exp_cooling,
}

#[derive(Debug, Clone,RustcDecodable)]
pub enum SolverVersion {
    seqsea,
    spis,
    mips,
    prsa,
}

#[derive(Debug, Clone, Copy, RustcDecodable)]
pub enum EnergyType {
    throughput,
    latency,
}


#[derive(Debug, Clone,RustcDecodable)]
pub enum ExecutionType {
    sequential,
    parallel,
}



//The Docopt usage string.
const USAGE: &'static str = "
Usage:   annealing-tuner [-t] --targ=<targetPath> --args2targ=<args> [-b] --bench=<benchmarkPath> --args2bench=<args> [-ms] --maxSteps=<maxSteps> [-ni] --numIter=<numIter> [-tp] [--maxTemp=<maxTemperature>] [-mt] [--minTemp=<minTemperature>] [-e] --energy=<energy> [-c] --cooling=<cooling> --problem=<problem> --version=<version>

Options:
    -t,    --targ=<args>     	Target Path.
    --args2targ=<args>          Arguments for Target (Specify Host and Port!).
    -b,    --bench=<args>     	Benchmark Path.
    --args2bench=<args>         Arguments for Benchmark
    -ms,   --maxSteps=<args>    Max Steps of Annealing.
    -ni,   --numIter=<args>     Number of Iterations for each stage of exploration
    -tp,   --maxTemp=<args>     (Optional) Max Temperature.
    -mt,   --minTemp=<args>     (Optional) Min Temperature.
    -e,	   --energy=<args>      Energy to eval (latency or throughput)
    -c,    --cooling=<args>     Cooling Schedule (linear, exponential, basic_exp_cooling)
    -p,	   --problem=<args>     Type of problem to solve (default, rastr, griew)
    -v,	   --version=<args>     Type of solver to use (seqsea, spis, mips, prsa)
";





#[derive(Debug, RustcDecodable)]
struct Args {
    flag_targ: String,
    flag_bench: String,
    flag_maxSteps: usize,
    flag_numIter: u8,
    flag_maxTemp: Option<f64>,
    flag_minTemp: Option<f64>,
    flag_energy: EnergyType,
    flag_cooling: CoolingSchedule,
    flag_problem: ProblemType,
    flag_version: SolverVersion,
    flag_args2targ: String,
    flag_args2bench: String,
}


/**
The Annealing Tuner is a tool able to needs in input:

	- The target app
	- The benchmark app
	- An initial default parameter configuration
	- The iteration number r for random selection of initial parameter configuration
	- Fixed number s of random moves for perturbation
**/
/**
Annealing Tuner Entry Point
**/
fn main() {


    /// Collect command line arguments
    ///
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    println!("{:?}", args);




    /// Create ParamsConfigurator useful to manage the parameters (or states)
    /// that the simulated annealing algorithm will explore. ParamsConfigurator set initial default parameters
    /// defined in the initial-params.txt input file
    ///
    let params_config = states_gen::ParamsConfigurator::new("params.conf".to_string());




    /// Instantiate the EnergyEval struct needed for start/stop the Target and the Benchmark applications
    /// and then evaluate the energy selected by the user
    ///
    let energy_eval = energy_eval::EnergyEval {
        target_path: args.flag_targ,
        bench_path: args.flag_bench,
        target_args: args.flag_args2targ,
        bench_args: args.flag_args2bench,
        num_iter: args.flag_numIter,
    };




    /// Configure the Simulated Annealing problem with the ParamsConfigurator and EnergyEval instances.
    /// Finally,the solver is started
    ///
    let mut problem = Problem {
    	problem_type: args.flag_problem,
        params_configurator: params_config,
        energy_evaluator: energy_eval,
    };

    /// Based on the user choice define what type of energy evaluate. Based on this, the Solver will perform
    /// either a maximization problem or a minimization problem
    ///
    let energy_type = match args.flag_energy {
        EnergyType::latency => EnergyType::latency,
        EnergyType::throughput => EnergyType::throughput,
    };


    /// An important aspect of the simulated anneling is how the temperature decrease.
    /// Therefore, the user can choice three types of decreasing function (exp, lin, adapt)
    ///
    let cooling_schedule = match args.flag_cooling {
        CoolingSchedule::exponential => CoolingSchedule::exponential,
        CoolingSchedule::linear => CoolingSchedule::linear,
        CoolingSchedule::basic_exp_cooling => CoolingSchedule::basic_exp_cooling,
    };


    let (t_min, t_max) = eval_temperature(args.flag_minTemp,
                                          args.flag_maxTemp,
                                          energy_type.clone(),
                                          &mut problem);



    let mr_result = match args.flag_version {
        SolverVersion::seqsea => {
            let mut solver = annealing::solver::seqsea::Seqsea {
                min_temp: t_min,
                max_temp: t_max,
                max_steps: args.flag_maxSteps,
                energy_type: energy_type,
                cooling_schedule: cooling_schedule.clone(),
            };

            solver.solve(&mut problem)
        }
        SolverVersion::spis => {
            let mut solver = annealing::solver::spis::Spis {
                min_temp: t_min,
                max_temp: t_max,
                max_steps: args.flag_maxSteps,
                energy_type: energy_type,
                cooling_schedule: cooling_schedule.clone(),
            };

            solver.solve(&mut problem)
        }
        SolverVersion::mips => {
            let mut solver = annealing::solver::mips::Mips {
                min_temp: t_min,
                max_temp: t_max,
                max_steps: args.flag_maxSteps,
                energy_type: energy_type,
                cooling_schedule: cooling_schedule.clone(),
            };

            solver.solve(&mut problem)
        }
        SolverVersion::prsa => {
            let mut solver = annealing::solver::prsa::Prsa {
                min_temp: t_min,
                max_temp: t_max,
                max_steps: args.flag_maxSteps,
                population_size: 50,
                energy_type: energy_type,
                cooling_schedule: cooling_schedule.clone(),
            };

            solver.solve(&mut problem)
        }
    };

    println!("{}",Yellow.paint("\n-----------------------------------------------------------------------------------------------------------------------------------------------"));
    println!("{} {:?}",
             Yellow.paint("The Best Configuration found is: "),
             mr_result.state);
    println!("{} {:?}", Yellow.paint("Energy: "), mr_result.energy);
    println!("{}",Yellow.paint("-----------------------------------------------------------------------------------------------------------------------------------------------"));


}


/// Check if the temperature is given by the user or if Tmin and Tmax need to be evaluated
fn eval_temperature(t_min: Option<f64>,
                    t_max: Option<f64>,
                    nrg_type: EnergyType,
                    problem: &mut Problem)
                    -> (f64, f64) {
    let num_exec = 20;
    let ngr_type_c = nrg_type.clone();

    let min_temp = match t_min {
        Some(val) => val,
        None => 1.0,
    };

	let mut rng = thread_rng();
	
    let max_temp = match t_max {
        Some(val) => val,
        None => {
            let mut energies = Vec::with_capacity(num_exec);
            /// Search for Tmax: a temperature that gives 98% acceptance
            /// Tmin: equal to 1.
            println!("{} Temperature not provided. Starting its Evaluation",
                     Green.paint("[TUNER]"));
            let mut state = problem.initial_state();
            match problem.energy(&state, nrg_type.clone(), 0,rng.clone()) {
                Some(nrg) => energies.push(nrg),
                None => panic!("The initial configuration does not allow to calculate the energy"),
            };

            for i in 0..num_exec {

                let next_state = problem.rand_state();
                match problem.energy(&next_state, ngr_type_c, 0,rng.clone()) {
                    Some(new_energy) => {
                        energies.push(new_energy);
                    }
                    None => {
                        println!("{} The current configuration parameters cannot be evaluated. \
                                  Skip!",
                                 Green.paint("[TUNER]"));
                    }
                };
            }

            let desired_prob: f64 = 0.98;
            (energies.iter().cloned().fold(0. / 0., f64::max) -
             energies.iter().cloned().fold(0. / 0., f64::min)) / desired_prob.ln()
        }
    };

    return (min_temp, max_temp);
}
