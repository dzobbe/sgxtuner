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
use meter_proxy::MeterProxy;
use ansi_term::Colour::{Red, Yellow, Green};
use std::time::Duration;
use EnergyType;
use ExecutionType;
use std::net::{TcpStream, Shutdown, IpAddr};
use std::io::{stdout, Stdout};
use hwloc::{Topology, CPUBIND_PROCESS, CPUBIND_THREAD, CpuSet, TopologyObject, ObjectType};
use libc::{kill, SIGTERM};
use State;
use energy_eval::command_executor::CommandExecutor;
use xml_reader::XMLReader;

pub mod command_executor;

#[derive(Clone,Debug)]
pub struct EnergyEval {
    pub xml_reader: XMLReader, 
}


static mut notified: bool = false;



#[derive(Clone)]
struct SpawnedMeterProxy(Arc<Mutex<HashMap<String, MeterProxy>>>);
impl SpawnedMeterProxy {
    fn new() -> Self {
        SpawnedMeterProxy(Arc::new(Mutex::new(HashMap::new())))
    }

    fn insert(&self, address: String, m_proxy_obj: MeterProxy) {
        let mut spawned_vec = self.0.lock().unwrap();
        (*spawned_vec).insert(address, m_proxy_obj);
    }

    fn spawned(&self, address: String) -> bool {
        let spawned_vec = self.0.lock().unwrap();
        (*spawned_vec).contains_key(&address)
    }

    fn get(&mut self, address: String) -> MeterProxy {
        let spawned_vec = self.0.lock().unwrap();
        let res = (*spawned_vec).get(&address).unwrap().clone();
        res
    }
}


lazy_static! {
    static ref spawned_proxies : SpawnedMeterProxy     = {SpawnedMeterProxy::new()};
    }


impl EnergyEval {


    /**
	Execute an an instance of the benchmark on the target application for the specific
	configuration of parameters. The function returns the cost result (in this case the response throughput)
	that will be used by the simulated annealing algorithm for the energy evaluation
	**/
    pub fn execute_test_instance(&mut self,
                                 params: &State,
                                 core: usize)
                                 -> Option<f64> {

        // let perf_metrics_handler = PerfMeter::new();

        // Modify the target and benchmark arguments in order to start different instances
        // on different ports. The annealing core is given to them. This will be sum
        // to the port number
        let new_target_args = self.change_port_arg(self.xml_reader.targ_args(), 12400, core);
        let new_bench_args = self.change_port_arg(self.xml_reader.bench_args(), 12600, core);


        let (target_addr, target_port) = self.parse_args(new_target_args.clone());
        let (bench_addr, bench_port) = self.parse_args(new_bench_args.clone());
		

        let mut target_alive: bool = false;

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
            /// Start METER-PROXY, which will interpose between the Target and the
            /// Benchmark apps to extract metrics for the energy evaluation
            /// * 
			************************************************************************************************************/            
			
   			let mut meter_proxy = MeterProxy::new(target_addr.clone(), target_port, bench_addr.clone(),bench_port);
          	let mut meter_proxy_c = meter_proxy.clone();
        	
            if !spawned_proxies.spawned(bench_port.to_string()){

            	spawned_proxies.insert(bench_port.to_string(),meter_proxy.clone());
            	
	            thread::spawn(move || {
	                	meter_proxy.start();
	            });
            }else{
            	let mut sp=spawned_proxies.clone();
            	meter_proxy=sp.get(bench_port.to_string());
            	meter_proxy_c=meter_proxy.clone();
            }
          		
          
  
            
			/***********************************************************************************************************
            /// **
            /// Launch TARGET Application
            /// *  
			************************************************************************************************************/
            /// Set the environement variables that will configure the parameters
	        /// needed by the target application
	        ///
	        let host_targ= self.xml_reader.targ_host();
	        let user_targ=self.xml_reader.targ_host_user();
    		let (tx, rx) = channel::<bool>();
	        match self.xml_reader.targ_exec_type(){
	        	ExecutionType::local  => {
	        		let local_cmd_executor=command_executor::LocalCommandExecutor;     
		         	local_cmd_executor.execute_target(self.xml_reader.targ_path().clone(), self.xml_reader.targ_bin().clone(), new_target_args.clone(), &params.clone(),rx);
	        	}
	        	ExecutionType::remote => {
			        let remote_cmd_executor=command_executor::RemoteCommandExecutor{
	        					host:host_targ,
								user_4_agent: user_targ,
							};
		        	remote_cmd_executor.execute_target(self.xml_reader.targ_path().clone(), self.xml_reader.targ_bin().clone(), new_target_args.clone(), &params.clone(),rx);
	        	}
	        }
			
            // Wait for target to startup
            thread::sleep(Duration::from_millis(1000));
            // Check if the target is alive
            target_alive = self.check_target_alive(target_addr.clone(), target_port as u16);
            if target_alive == false {
				//Send signal to target to exit			
				tx.send(true);
                break;
            }
 
 
 			let start_time = time::precise_time_ns();
	  
			/***********************************************************************************************************
            /// **
            /// Launch BENCHMARK Application and measure execution time
            /// *
			************************************************************************************************************/ 
			let host_bench= self.xml_reader.bench_host();
			let user_bench=self.xml_reader.bench_host_user();
			match self.xml_reader.bench_exec_type(){
	        	ExecutionType::local  => {
	        		let local_cmd_executor=command_executor::LocalCommandExecutor;    
		         	local_cmd_executor.execute_bench(self.xml_reader.bench_path().clone(), self.xml_reader.bench_bin().clone(), new_bench_args.clone());
	        	}
	        	ExecutionType::remote => {
			        let remote_cmd_executor=command_executor::RemoteCommandExecutor{
	        					host: host_bench,
								user_4_agent: user_bench,
							};
		        	remote_cmd_executor.execute_bench(self.xml_reader.bench_path().clone(), self.xml_reader.bench_bin().clone(), new_bench_args.clone());
	        	}
	        }
	    	let end_time = time::precise_time_ns();
		    let elapsed_ns: f64 = (end_time - start_time) as f64;
		    let elapsed_time = elapsed_ns / 1000000000.0f64;

  
			
            

			/***********************************************************************************************************
            /// **
            /// ENERGY Evaluation
            /// *
			************************************************************************************************************/    
            let nrg: f64 = match self.xml_reader.ann_energy() {
                EnergyType::throughput => {
                    // Throughput Evaluation
                    let num_bytes = meter_proxy_c.get_num_kbytes_rcvd() as f64;
                    let resp_rate = num_bytes / elapsed_time;
                 	
                    resp_rate
                }
                EnergyType::latency => {
                    // Latency Evaluation
                    meter_proxy_c.get_num_resp() 
                  
                }
            };
            nrg_vec.push(nrg);
            
            
            
			/************************************************************************************************************
            /// **
            /// Clean Resources
            /// *
			*************************************************************************************************************/
            meter_proxy_c.reset();
			//Send signal to target to exit			
			tx.send(true);

        }

        pb.finish();

        if target_alive {
            let sum_nrg: f64 = nrg_vec.iter().sum();
            let avg_nrg = sum_nrg / self.xml_reader.ann_num_iter() as f64;
            match self.xml_reader.ann_energy() {
                EnergyType::throughput => {
                    println!("Thread [{}] {} {:.4} KB/s",
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
        let target_alive = match TcpStream::connect((targ_addr, target_port)) {
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
    /// *********************************************************************************************************

    fn parse_args(&self, args_str: String) -> (String, u16) {
        let mut args: Vec<&str> = args_str.split_whitespace().collect();

        let addr = match (&mut args)
            .into_iter()
            .position(|&mut x| {
                x == "-l" || x == "--address" || x == "-h" || x == "--host" || x == "--server"
            }) {
            Some(index) => args[index + 1].parse().unwrap(),
            None => {
                unsafe {
                    if notified == false {
                        println!("In: {:?} - Address not found. Using 127.0.0.1", args_str);
                        notified = true;
                    }
                }
                "127.0.0.1".to_string()
            }
        };

        let port = match (&mut args)
            .into_iter()
            .position(|&mut x| x == "-p" || x == "--port") {
            Some(index) => args[index + 1].parse().unwrap(),
            None => {12600}
        }; 

        return (addr, port);

    }
    /// *********************************************************************************************************


    fn change_port_arg(&self, args_str: String, base_value: usize, val_2_add: usize) -> String {
        let args = args_str.clone();
        let mut new_args_string = "".to_string();
        let mut gotit = false;

        let vec_args: Vec<&str> = args.split_whitespace().collect();
        for arg in vec_args {
            if gotit {
                let mut new_port_val = (base_value + val_2_add).to_string();
                new_args_string = new_args_string + " " + new_port_val.as_str();
                gotit = false;
            } else {
                new_args_string = new_args_string + " " + arg;
            }
            if arg == "-p" || arg == "--port" {
                gotit = true;
            }
        }
        return new_args_string;
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

