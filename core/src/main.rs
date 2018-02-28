
extern crate rustc_serialize;
extern crate rand;
extern crate libc;
extern crate time;
extern crate ansi_term;
extern crate pbr;
extern crate csv;
extern crate num_cpus;
extern crate wait_timeout;
extern crate ssh2;
extern crate xml;
extern crate ctrlc;
extern crate zmq;
extern crate yaml_rust;
extern crate influent;

#[macro_use]
extern crate futures;
#[macro_use]
extern crate tokio_core;
extern crate futures_cpupool;

#[macro_use]
extern crate lazy_static;



use ansi_term::Colour::{Green, Yellow};
use std::collections::HashMap;

use annealing::problem::Problem;
use annealing::solver::seqsa;
use annealing::solver::Solver;
use annealing::solver::common::MrResult;

mod annealing;
mod res_emitters;
mod parsers;
mod states_gen;
mod energy_eval;

mod shared;

type State = HashMap<String, String>;


/***
TUNER ENTRY POINT
***/
fn main() {


    println!("Tuner Started!");

    /***
	Initialize the YML parser to read the docker-compose.yml file and get addresses of nodes
	***/
    let yml_reader = parsers::yml_parser::YMLReader::new("../docker-compose.yml".to_string());


    /***
	Initialize the XML parser to read the configuration file
	***/
    let xml_reader = parsers::xml_parser::XMLReader::new("../conf.xml".to_string());


    /***
	Create the State Configuration Generator useful to manage parameters (or states) 
	that the simulated annealing algorithm will explore.	
	***/
    let mut conf_generator = states_gen::ParamsConfigurator::new(
        xml_reader.get_target_int_params(),
        xml_reader.get_target_bool_params(),
    );


    /*** 
    Configure the Energy Evaluator needed to start/stop Target and Benchmark applications
    and evaluate the Energy selected by the user (e.g. latency, throughput)
    ***/
    let energy_eval = energy_eval::EnergyEval::new(
        yml_reader.get_target_addresses(),
        yml_reader.get_bench_addresses(),
        xml_reader.get_tuner_params(),
    );



    /// Configure the Simulated Annealing problem with the ParamsConfigurator and EnergyEval instances.
    /// Finally,the solver is started
    ///
    let mut problem = Problem {
        params_configurator: conf_generator.clone(),
        energy_evaluator: energy_eval,
    };



    let mut tuner_params = xml_reader.get_tuner_params();


    let res_emitter = res_emitters::Emitter {
        influx_res_emitter: res_emitters::influx_emitter::InfluxEmitter::new(
            format!("http://{}:{}", yml_reader.get_influx_address(), "8086"),
            "".to_string(),
            "".to_string(),
            "tuner_db".to_string(),
        ),
        csv_res_emitter: res_emitters::csv_emitter::CSVEmitter::new(
            yml_reader.get_num_targets(),
            conf_generator.get_params_name(),
        ),
    };


    annealing::eval_temperature(&mut tuner_params, &mut problem);

    println!("temp: {:?}", tuner_params.max_temp);

    let mr_result = match tuner_params.version {
        SolverVersion::seqsa => {
            let mut solver = annealing::solver::seqsa::Seqsa {
                tuner_params: tuner_params,
                res_emitter: res_emitter,
            };

            solver.solve(&mut problem, 1)
        }
        SolverVersion::spisa => {
            let mut solver = annealing::solver::spisa::Spisa {
                tuner_params: tuner_params,
                res_emitter: res_emitter,
            };

            solver.solve(&mut problem, yml_reader.get_num_targets())
        }
        SolverVersion::mir => {
            let mut solver = annealing::solver::mir::Mir {
                tuner_params: tuner_params,
                res_emitter: res_emitter,
            };

            solver.solve(&mut problem, yml_reader.get_num_targets())
        }
        SolverVersion::prsa => {
            let mut solver = annealing::solver::mir::Mir {
                //TOADJUST
                tuner_params: tuner_params,
                res_emitter: res_emitter,
            };

            solver.solve(&mut problem, yml_reader.get_num_targets())
        }/*{
        let mut solver = annealing::solver::prsa::Prsa {
                min_temp: t_min,
                max_temp: t_max,
                max_steps: tuner_params.max_step,
                population_size: 32,
                energy_type: tuner_params.energy,
                cooling_schedule: tuner_params.cooling,
            };

            solver.solve(&mut problem, yml_reader.get_num_targets()) //TODO
        }*/
    };

    println!("{}",Yellow.paint("\n-----------------------------------------------------------------------------------------------------------------------------------------------"));
    println!(
        "{} {:?}",
        Yellow.paint("The Best Configuration found is: "),
        mr_result.state
    );
    println!("{} {:?}", Yellow.paint("Energy: "), mr_result.energy);
    println!("{}",Yellow.paint("-----------------------------------------------------------------------------------------------------------------------------------------------"));


}





#[derive(Debug, Clone, RustcDecodable)]
pub enum CoolingSchedule {
    linear,
    exponential,
    basic_exp_cooling,
}

#[derive(Debug, Clone, RustcDecodable)]
pub enum SolverVersion {
    seqsa,
    spisa,
    mir,
    prsa,
}

#[derive(Debug, Clone, Copy, RustcDecodable)]
pub enum EnergyType {
    throughput,
    latency,
}


#[derive(Debug, Clone, RustcDecodable)]
pub enum ExecutionType {
    local,
    remote,
}

#[derive(Debug, Clone, RustcDecodable)]
pub enum ParameterLevel {
    runtime,
    service_config,
    compile,
}




impl std::str::FromStr for CoolingSchedule {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "linear" => Ok(CoolingSchedule::linear),
            "exponential" => Ok(CoolingSchedule::exponential),
            "basic_exp_cooling" => Ok(CoolingSchedule::basic_exp_cooling),
            _ => Err("Cooling Schedule - not a valid value"),
        }
    }
}

impl std::str::FromStr for SolverVersion {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "seqsa" => Ok(SolverVersion::seqsa),
            "spisa" => Ok(SolverVersion::spisa),
            "mir" => Ok(SolverVersion::mir),
            "prsa" => Ok(SolverVersion::prsa),
            _ => Err("Solver Version - not a valid value"),
        }
    }
}

impl std::str::FromStr for EnergyType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "throughput" => Ok(EnergyType::throughput),
            "latency" => Ok(EnergyType::latency),
            _ => Err("Energy Type - not a valid value"),
        }
    }
}

impl std::str::FromStr for ExecutionType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local" => Ok(ExecutionType::local),
            "remote" => Ok(ExecutionType::remote),
            _ => Err("Execution Type - not a valid value"),
        }
    }
}

impl std::str::FromStr for ParameterLevel {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "runtime" => Ok(ParameterLevel::runtime),
            "service-config" => Ok(ParameterLevel::service_config),
            "compile" => Ok(ParameterLevel::compile),
            _ => Err("Parameter Level - not a valid value"),
        }
    }
}
