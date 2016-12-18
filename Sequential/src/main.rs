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

#[macro_use]
extern crate lazy_static;
extern crate influent;

use ansi_term::Colour::Yellow;


mod PerfCounters;
mod Parameters;
mod Annealing;
mod EnergyEval;
mod MeterProxy;
mod ResultsEmitter;

#[derive(Debug, Clone,RustcDecodable)]
pub enum CoolingSchedule {
    linear,
    exponential,
    adaptive,
}

#[derive(Debug, Clone,RustcDecodable)]
pub enum EnergyType {
    throughput,
    latency,
}

#[derive(Debug, Clone)]
pub enum TerminationCriteria {
    Max_Steps(u64),
    Max_Time_Seconds(u64),
}


use std::sync::{Arc, Mutex, Condvar};
use std::sync::RwLock;

use docopt::Docopt;
use std::process::Command;
use PerfCounters::PerfMetrics;
use std::thread;


//The Docopt usage string.
const USAGE: &'static str = "
Usage:   annealing-tuner [-t] --targ=<targetPath> --args2targ=<args> [-b] --bench=<benchmarkPath> --args2bench=<args> [-ms] --maxSteps=<maxSteps> [-ni] --numIter=<numIter> [-tp] --maxTemp=<maxTemperature> [-mt] --minTemp=<minTemperature> [-e] --energy=<energy> [-c] --cooling=<cooling> [-infl] [--host=<args>] [--port] [--user] [--pwd]
Options:
    -t,    --targ=<args>     	Target Path.
    --args2targ=<args>          Arguments for Target (Specify Host and Port!).
    -b,    --bench=<args>     	Benchmark Path.
    --args2bench=<args>         Arguments for Benchmark (start it on localhost:12349!).
    -ms,   --maxSteps=<args>    Max Steps of Annealing.
    -ni,   --numIter=<args>     Number of Iterations for each stage of exploration
    -tp,   --maxTemp=<args>     Max Temperature.
    -mt,   --minTemp=<args>     Min Temperature. 
    -e,	   --energy=<args>      Energy to eval (latency or throughput)
    -c,    --cooling=<args>     Cooling Schedule (linear, exponential, adaptive)
";


#[derive(Debug, RustcDecodable)]
struct Args {
    flag_targ: String,
    flag_bench: String,
    flag_maxSteps: u64,
    flag_numIter: u8,
    flag_maxTemp: f64,
    flag_minTemp: f64,
    flag_energy: EnergyType,
    flag_cooling: CoolingSchedule,
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
    let params_config = Parameters::ParamsConfigurator {
        param_file_path: "params.conf".to_string(),
        ..Parameters::ParamsConfigurator::default()
    };




    /// Instantiate the EnergyEval struct needed for start/stop the Target and the Benchmark applications
    /// and then evaluate the energy selected by the user
    ///
    let energy_eval = EnergyEval::EnergyEval {
        target_path: args.flag_targ,
        bench_path: args.flag_bench,
        target_args: args.flag_args2targ,
        bench_args: args.flag_args2bench.split_whitespace().map(String::from).collect(),
        num_iter: args.flag_numIter,
    };




    /// Configure the Simulated Annealing problem with the ParamsConfigurator and EnergyEval instances.
    /// Finally,the solver is started
    ///
    let mut problem = Annealing::Problem::ProblemInputs {
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
        CoolingSchedule::adaptive => CoolingSchedule::adaptive,
    };


    let annealing_solver = Annealing::Solver::Solver {
        termination_criteria: TerminationCriteria::Max_Steps(args.flag_maxSteps),
        min_temperature: args.flag_minTemp,
        max_temperature: args.flag_maxTemp,
        energy_type: energy_type,
        cooling_schedule: cooling_schedule,
    };

    /// Start the solver
    let best_state = annealing_solver.solve(&mut problem);
    println!("{}",Yellow.paint("\n-------------------------------------------------------------------------------------------------------------------"));
    println!("{} {:?}",
             Yellow.paint("The Best State found is: "),
             best_state);
    println!("{}",Yellow.paint("-------------------------------------------------------------------------------------------------------------------"));


}
