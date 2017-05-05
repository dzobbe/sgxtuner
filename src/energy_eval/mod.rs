use time;
use libc;
use hwloc;
use pbr;
use pbr::{ProgressBar, MultiBar};
use std::str;
use std::cell::RefCell;
use std::rc::Rc;
use std::process::{Command, Child, Stdio};
use std::{thread, env};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use ansi_term::Colour::{Red, Yellow, Green};
use std::time::Duration;
use EnergyType;
use ExecutionType;
use BenchmarkName;
use std::net::{TcpStream, Shutdown, IpAddr};
use std::io::{stdout, Stdout};
use hwloc::{Topology, CPUBIND_PROCESS, CPUBIND_THREAD, CpuSet, TopologyObject, ObjectType};
use libc::{kill, SIGTERM};
use State;
use energy_eval::command_executor::CommandExecutor;
use xml_reader::XMLReader;
use shared::Process2Spawn;
use shared::ProcessPool;
use ctrlc;
use std::process;
use energy_eval::output_parser::Parser;


pub mod command_executor;
pub mod output_parser;


#[derive(Clone,Debug)]
pub struct EnergyEval {
    pub xml_reader: XMLReader,
}


static mut notified: bool = false;
static mut counter: u16 = 0;



struct BenchExecTime(Arc<Mutex<u32>>);
impl BenchExecTime {
    fn new() -> Self {
        BenchExecTime(Arc::new(Mutex::new(0)))
    }
    fn set(&self, val: u32) {
        let mut exec_time = self.0.lock().unwrap();
        *exec_time = val;
    }
    fn get(&self) -> u32 {
        let mut exec_time = self.0.lock().unwrap();
        *exec_time
    }
}



lazy_static! {
	static ref bench_exec_time: BenchExecTime = {BenchExecTime::new()};
    }


impl EnergyEval {
	
    /**
	Execute an an instance of the benchmark on the target application for the specific
	configuration of parameters. The function returns the cost result (in this case the response throughput)
	that will be used by the simulated annealing algorithm for the energy evaluation
	**/
    pub fn execute_test_instance(&mut self, params: &State, core: usize) -> Option<f64> {

        //Extract the target pool
        let mut target_pool = self.xml_reader.get_targs_pool();
        //Extract the bench pool
        let mut bench_pool = self.xml_reader.get_bench_pool();

		
		let target_x =  target_pool.remove(core.to_string());
        let bench_x =  bench_pool.remove(core.to_string());


        let (target_addr, target_port) = (target_x.clone().address,target_x.clone().port.parse::<u16>().unwrap());
        let (bench_addr, bench_port) = (bench_x.clone().address, bench_x.clone().port.parse::<u16>().unwrap()); 
        
        
        let mut new_bench_args =
            bench_x.clone().args.replace(bench_x.port.to_string().as_str(),
                                         target_port.to_string().as_str());
        new_bench_args=new_bench_args.replace(bench_x.clone().address.as_str(),target_addr.as_str());
		
		
        let mut valid_result: bool = false;

        // Repeat the execution num_iter times for accurate results
        let mut nrg_vec = Vec::with_capacity(self.xml_reader.ann_num_iter() as usize);
        println!("{} TID [{}] - Evaluation of: {:?}",
                 Green.paint("====>"),
                 core,
                 params);
        println!("{} Waiting for {} iterations to complete",
                 Green.paint("====>"),
                 self.xml_reader.ann_num_iter());

        let mut pb = ProgressBar::new(self.xml_reader.ann_num_iter() as u64);
        pb.format("╢▌▌░╟");
        pb.show_message = true;
        pb.message(&format!("Thread [{}] - ", core));


        for i in 0..self.xml_reader.ann_num_iter() {
            pb.inc();


            /***********************************************************************************************************
            /// **
            /// Launch TARGET Application
            /// *
            	************************************************************************************************************/
            /// Set the environement variables that will configure the parameters
            /// needed by the target application
            ///
            let (stop_tx, stop_rx) = channel::<bool>();

            match target_x.clone().execution_type {
                ExecutionType::local => {
	                    let local_cmd_executor = command_executor::LocalCommandExecutor;	
                		local_cmd_executor.execute_target(target_x.path.clone(),
                                                  target_x.bin.clone(),
                                                  target_x.args.clone(),
                                                  &params,
                                                  stop_rx);
                }
                ExecutionType::remote => {
                    let remote_cmd_executor = command_executor::RemoteCommandExecutor {
                        host: target_x.clone().host,
                        user: target_x.clone().user,
                        pwd: target_x.clone().user,
                    };
                    remote_cmd_executor.execute_target(target_x.path.clone(),
                                                       target_x.bin.clone(),
                                                       target_x.args.clone(),
                                                       &params.clone(),
                                                       stop_rx);
                }
            }

			
            // Wait for target to startup
            thread::sleep(Duration::from_millis(1000));
            // Check if the target is alive
            valid_result = self.check_target_alive(target_addr.clone(), target_port as u16);
            if valid_result == false {
	            target_pool.push(target_x,core.to_string());
        		bench_pool.push(bench_x.clone(),core.to_string());
                stop_tx.send(true);
                break;
            }


			let stop_tx_c=stop_tx.clone();
			ctrlc::set_handler(move || {
	                stop_tx_c.send(true);
	                process::exit(0);
		    });
			
			
            let start_time = time::precise_time_ns();



            /***********************************************************************************************************
            /// **
            /// Launch BENCHMARK Application and measure execution time
            /// *
            	************************************************************************************************************/
			let parser=output_parser::Parser{
				benchmark_name: self.xml_reader.get_bench_name(),
			};
			

			let mut energy: Option<f64>=None;
			
            match bench_x.clone().execution_type {
                ExecutionType::local => {
                    let local_cmd_executor = command_executor::LocalCommandExecutor;
                    energy=local_cmd_executor.execute_bench(bench_x.path.clone(),
                                                     bench_x.bin.clone(),
                                                     new_bench_args.clone(),
                                                     parser);
                }
                ExecutionType::remote => {
                    let remote_cmd_executor = command_executor::RemoteCommandExecutor {
                        host: bench_x.clone().host,
                        user: bench_x.clone().user,
                        pwd: bench_x.clone().user,
                    };
                    energy=remote_cmd_executor.execute_bench(bench_x.path.clone(),
                                                      bench_x.bin.clone(),
                                                      new_bench_args.clone(),
                                                      parser);
                }
            }
            let end_time = time::precise_time_ns();
            let elapsed_ns: f64 = (end_time - start_time) as f64;
            let elapsed_time = elapsed_ns / 1000000000.0f64;
					
			
			match energy {
				Some(v) => nrg_vec.push(v),
				None => {
					valid_result=false;
		            target_pool.push(target_x,core.to_string());
        			bench_pool.push(bench_x.clone(),core.to_string());
                	stop_tx.send(true);
               		break;
            	}
			};
			
			

		
            /************************************************************************************************************
            /// **
            /// Clean Resources
            /// *
            	*************************************************************************************************************/

            //Send signal to target to exit
            target_pool.push(target_x.clone(),core.to_string());
            bench_pool.push(bench_x.clone(),core.to_string());
            stop_tx.send(true);


        }

        pb.finish();

        if valid_result {
            let sum_nrg: f64 = nrg_vec.iter().sum();
            let avg_nrg = sum_nrg / self.xml_reader.ann_num_iter() as f64;
            match self.xml_reader.ann_energy() {
                EnergyType::throughput => {
                    println!("Thread [{}] {} {:.4} Ops/s",
                             core,
                             Red.paint("====> Evaluated Avg. Response Rate: "),
                             avg_nrg);
                }
                EnergyType::latency => {
                    println!("Thread [{}] {} {:.4} ms",
                             core,
                             Red.paint("====> Evaluated Avg. Latency: "),
                             avg_nrg);
                }
            };
            println!("{}",Yellow.paint("==================================================================================================================="));

            return Some(avg_nrg);
        } else {

            return None;
        }

    }



    /// *********************************************************************************************************
    fn check_target_alive(&self, target_addr: String, target_port: u16) -> bool {
        // Realize one fake connection to check if the target is alive
        // It can happen that the configuration of parameters does not allow to start the target.
        // In that case the energy returned by this function is None
        let targ_addr: IpAddr = target_addr.parse()
            .expect("Unable to parse Target Address");
            
        let target_alive = match TcpStream::connect((target_addr.as_str(), target_port)) {
            Err(e) => {
                println!("{} The Target Application seems down. Maybe a bad configuration: {}",
                         Red.paint("*****ERROR***** --> "),
                         e);
                false
            }
            Ok(s) => {
                s.shutdown(Shutdown::Both);
                drop(s);
                true
            }
        };
        return target_alive;
    }


}

/// Load the CpuSet for the given core index.
fn cpuset_for_core(topology: &Topology, idx: usize) -> CpuSet {
    let cores = (*topology).objects_with_type(&ObjectType::Core).unwrap();
    match cores.get(idx) {
        Some(val) => val.cpuset().unwrap(),
        None => panic!("No Core found with id {}", idx),
    }
}
