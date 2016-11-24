extern crate x86;
extern crate perfcnt;
extern crate rustc_serialize;
extern crate docopt;
extern crate rand;
extern crate libc;
extern crate time;
extern crate ansi_term;
extern crate mio;
//extern crate influent;



mod PerfCounters;
mod Parameters;
mod SimulatedAnnealing;
mod ThreadExecutor;
mod MeterProxy;
//mod InfluxProxy;

#[derive(Debug, Clone)]
pub enum CoolingSchedule {
    Linear,
    Exponential,
    Adaptive
}

#[derive(Debug, Clone)]
pub enum TerminationCriteria {
	Max_Steps (u64),
	Max_Time_Seconds (u64)
}


use std::sync::{Arc, Mutex, Condvar};
use std::sync::RwLock;

use docopt::Docopt;
use std::process::Command;
use PerfCounters::PerfMetrics;
use std::thread;


//The Docopt usage string.
const USAGE: &'static str = "
Usage:   sgxmusl-autotuner [-t] --targ=<targetPath> [--args2targ=<args>] [-b] --bench=<benchmarkPath> [--args2bench=<args>] [-ms] --maxSteps=<maxSteps> [-t] --maxTemp=<maxTemperature> [-mt] --minTemp=<minTemperature> [-at] --maxAtt=<maxAttempts> [-ac] --maxAcc=<maxAccepts> [-rj] --maxRej=<maxRejects>							  
Options:
    -t,    --targ=<args>     	Target Path.
    --args2targ=<args>   		Arguments for Target.
    -b,    --bench=<args>     	Benchmark Path.
    --args2bench=<args>  		Arguments for Benchmark.
    -ms,   --maxSteps=<args>    Max Steps.
    -tp,   --maxTemp=<args>     Max Temperature.
    -mt,   --minTemp=<args>     Min Temperature.
    -at,   --maxAtt=<args>     	Max Attemtps.
    -ac,   --maxAcc=<args>     	Max Accepts.
    -rj,   --maxRej=<args>     	Max Rejects.  
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_targ: String,
    flag_bench: String,
    flag_maxSteps: u64,
    flag_maxTemp: f64,
    flag_minTemp: f64,
    flag_maxAtt: u64,
    flag_maxAcc: u64,
    flag_maxRej: u64,
    flag_args2targ: String,
    flag_args2bench: String,
}


/**
The Sgx-Musl Auto Tuner is a tool able to needs in input:

	- The target app
	- The benchmark app
	- An initial default parameter configuration
	- The possible categorical levels that each parameter can assume
	- The cutoff time k after which terminate the target algorithm execution
	- The iteration number r for random selection of initial parameter configuration
	- Probability p_restart with which re-initialize the search at random
	- Fixed number s of random moves for perturbation
**/
/**
Sgx-Musl Auto Tuner Entry Point
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
        param_file_path: "initial-params.txt".to_string(),
        ..Parameters::ParamsConfigurator::default()
    };



    /// Instantiate the AppsManager struct needed for start/stop the Target and the Benchmark applications
    ///
    let executor = ThreadExecutor::ThreadExecutor {
        target_path: args.flag_targ,
        bench_path: args.flag_bench,
        target_args: args.flag_args2targ,
        bench_args: args.flag_args2bench.split_whitespace().map(String::from).collect(),
        ..ThreadExecutor::ThreadExecutor::default()
    };


    /// Configure the Simulated Annealing problem with the ParamsConfigurator and AppsManager instances.
    /// Finally,the solver is started
    ///
    let mut problem = SimulatedAnnealing::Problem::ProblemInputs {
        params_configurator: params_config,
        thread_executor: executor,
    };
    
    
    let annealing_solver = SimulatedAnnealing::Solver::Solver {
        termination_criteria: TerminationCriteria::Max_Steps(args.flag_maxSteps),
        min_temperature: args.flag_minTemp,
        max_temperature: args.flag_maxTemp,
        max_attempts: args.flag_maxAtt,
        max_accepts: args.flag_maxAcc,
        max_rejects: args.flag_maxRej,
        cooling_schedule: CoolingSchedule::Exponential
        
    };

    annealing_solver.solve(&mut problem);



}
