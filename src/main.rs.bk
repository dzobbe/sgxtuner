extern crate rustc_serialize;
extern crate docopt;
extern crate rand;
extern crate libc;

mod PerfCounters;
mod Parameters;
mod SimulatedAnnealing;
mod ThreadExecutor;
mod MeterProxy;

use std::sync::{Arc, Mutex, Condvar};
use std::sync::RwLock;

use docopt::Docopt;
use std::process::Command;
use PerfCounters::PerfMetrics;
use std::thread;

//The Docopt usage string.
const USAGE: &'static str = "
Usage:   sgxmusl-autotuner [-t] --targ=<targetPath> [--args2targ=<args>] [-b] --bench=<benchmarkPath> [--args2bench=<args>] [-ms] --maxSteps=<maxSteps> [-tp] --temp=<initialTemp> [-rf] --redFact=<tempReductionFactor> [-at] --maxAtt=<maxAttempts> [-ac] --maxAcc=<maxAccepts> [-rj] --maxRej=<maxRejects>							  
Options:
    -t,    --targ=<args>     	someoption.
    --args2targ=<args>   		arguments for target.
    -b,    --bench=<args>     	someoption.
    --args2bench=<args>  		arguments for benchmark.
    -ms,   --maxSteps=<args>    someoption.
    -tp,   --temp=<args>     	someoption.
    -rf,   --redFact=<args>     someoption.
    -at,   --maxAtt=<args>     	someoption.
    -ac,   --maxAcc=<args>     	someoption.
    -rj,   --maxRej=<args>     	someoption.  
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_targ				: String,
    flag_bench				: String,
	flag_maxSteps			: u64,
	flag_temp				: f64,
	flag_redFact			: f64,
	flag_maxAtt				: u64,
	flag_maxAcc				: u64,
	flag_maxRej				: u64, 	   	
    flag_args2targ			: String,
    flag_args2bench 		: String,
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
	
	/**
	Collect command line arguments
	**/
	let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());
	println!("{:?}", args);
	
	

	
	/**
	Create ParamsConfigurator useful to manage the parameters (or states) 
	that the simulated annealing algorithm will explore. ParamsConfigurator set initial default parameters
	defined in the initial-params.txt input file
	**/
	let params_config=Parameters::ParamsConfigurator{
						param_file_path:"../initial-params.txt".to_string(),
						.. Parameters::ParamsConfigurator::default()
						};


	
	/**
	Instantiate the AppsManager struct needed for start/stop the Target and the Benchmark applications
	**/
	let executor= ThreadExecutor::ThreadExecutor{
						target_path: args.flag_targ,
						bench_path:  args.flag_bench,
						target_args: args.flag_args2targ,
						bench_args:  args.flag_args2bench.split_whitespace().map(String::from).collect(),
						.. ThreadExecutor::ThreadExecutor::default()
						};
	//executor.start_meter_proxy();
	
	
	/**
	Configure the Simulated Annealing problem with the ParamsConfigurator and AppsManager instances.
	Finally,the solver is started
	**/
	let mut problem=SimulatedAnnealing::Problem::ProblemInputs{params_configurator: params_config, thread_executor: executor};
	let annealing_solver=SimulatedAnnealing::Solver::Solver{
					    steps						: args.flag_maxSteps,
					    initial_temperature			: args.flag_temp,
					    temperature_reduction		: args.flag_redFact,
					    max_attempts				: args.flag_maxAtt,
					    max_accepts					: args.flag_maxAcc,
					    max_rejects					: args.flag_maxRej,
						};
	
	annealing_solver.solve(&mut problem);
	
 	
 	
}

