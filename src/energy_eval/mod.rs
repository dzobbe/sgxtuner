use time;
use libc;
use hwloc;
use pbr;
use pbr::{ProgressBar,MultiBar};
use std::str;
use std::cell::RefCell;
use std::rc::Rc;
use std::process::{Command, Child, Stdio};
use std::{thread, env};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use perf_counters::PerfMetrics;
use meter_proxy::MeterProxy;
use ansi_term::Colour::{Red, Yellow, Green};
use std::time::Duration;
use EnergyType;
use std::net::{TcpStream, Shutdown, IpAddr};
use std::io::{stdout,Stdout};
use hwloc::{Topology, CPUBIND_PROCESS, CPUBIND_THREAD,CpuSet, TopologyObject, ObjectType};
use libc::{kill, SIGTERM};
use State;

 
#[derive(Clone,Debug,RustcEncodable)]
pub struct EnergyEval {
    pub target_path: String,
    pub bench_path: String,
    pub target_args: String,
    pub bench_args: String,
    pub num_iter: u8,
}


static mut notified: bool = false;

struct BenchExecTime(Arc<Mutex<u32>>);
impl BenchExecTime {
    fn new() -> Self {
        BenchExecTime(Arc::new(Mutex::new(0)))
    }
	fn set(&self, val:u32){
        let mut exec_time = self.0.lock().unwrap();
        *exec_time=val;
	}
	fn get(&self) -> u32{
        let mut exec_time = self.0.lock().unwrap();
        *exec_time
	}
}

#[derive(Clone)]
struct SpawnedMeterProxy(Arc<Mutex<HashMap<String,MeterProxy>>>);
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
        let res=(*spawned_vec).get(&address).unwrap().clone();
        res
    }

}


lazy_static! {
    static ref spawned_proxies : SpawnedMeterProxy     = {SpawnedMeterProxy::new()};
    static ref bench_exec_time : BenchExecTime     = {BenchExecTime::new()};

    }
 

impl EnergyEval {
    pub fn new() -> EnergyEval { 
        Default::default()
    }



    /**
	Execute an an instance of the benchmark on the target application for the specific
	configuration of parameters. The function returns the cost result (in this case the response throughput)
	that will be used by the simulated annealing algorithm for the energy evaluation
	**/
    pub fn execute_test_instance(&mut self,
                                 params: &State,
                                 energy_type: EnergyType,
                                 core: usize)
                                 -> Option<f64> {

        let perf_metrics_handler = PerfMetrics::new();
        
        //Modify the target and benchmark arguments in order to start different instances
        //on different ports. The annealing core is given to them. This will be sum
        //to the port number
        let new_target_args=self.change_port_arg(self.clone().target_args, 12400, core);
        self.target_args=new_target_args;
        let new_bench_args =self.change_port_arg(self.clone().bench_args, 12600, core);
        self.bench_args=new_bench_args;

        
        let (target_addr, target_port) = self.parse_args(self.clone().target_args);
        let (bench_addr, bench_port)   = self.parse_args(self.clone().bench_args);

        
        let mut target_alive: bool = false;

        
        // Repeat the execution num_iter times for accurate results
        let mut nrg_vec = Vec::with_capacity(self.num_iter as usize);
        println!("{} TID [{}] - Evaluation of: {:?}", Green.paint("====>"), core, params);
        println!("{} Waiting for {} iterations to complete",
                 Green.paint("====>"), 
                 self.num_iter);

        let mut pb = ProgressBar::new(self.num_iter as u64);
        pb.format("╢▌▌░╟");
 		pb.show_message = true;
 		pb.message(&format!("Thread [{}] - ", core));

 
        for i in 0..self.num_iter {
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
            let mut command_2_launch=Command::new(self.target_path.clone());
            /// Set the environement variables that will configure the parameters
	        /// needed by the target application
	        ///
            for (param_name, param_value) in params.iter() {
           		command_2_launch.env(param_name.to_string(), param_value.to_string());
        	}
            
            let mut vec_args: Vec<&str> = self.target_args.split_whitespace().collect();
            let mut target_process = Some(command_2_launch
                .args(vec_args.as_ref()) 
                .stdout(Stdio::piped())
                .spawn()
                .expect("Failed to execute Target!"));
        	
            let pid_target = target_process.as_mut().unwrap().id();
			

            // Wait for target to startup
            thread::sleep(Duration::from_millis(1000));
            // Check if the target is alive
            target_alive = self.check_target_alive(target_addr.clone(), target_port as u16);
            if target_alive == false {
                target_process.as_mut().unwrap().kill().expect("Target Process wasn't running");
                break;
            }
 
            
			/***********************************************************************************************************
            /// **
            /// Launch BENCHMARK Application and measure execution time
            /// *
			************************************************************************************************************/            
            let start_time = time::precise_time_ns();
            
            let bench_args: Vec<&str>=self.bench_args.split_whitespace().collect();
            let mut bench_process = Command::new(self.bench_path.clone())
                .args(bench_args.as_ref())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to execute Benchmark!"); 
            
            let pid=bench_process.id();
            thread::spawn(move || { 
            		 if bench_exec_time.get() != 0{
 	            		thread::sleep(Duration::from_millis((bench_exec_time.get()*4) as u64));
            		 	unsafe{kill(pid as i32, SIGTERM);}
        		 	}
            	});
            
            bench_process.wait().expect("Failed to wait on Benchmark");
  
            
            let end_time = time::precise_time_ns();
			
            let elapsed_ns: f64 = (end_time - start_time) as f64;
            let elapsed_time = elapsed_ns / 1000000000.0f64;
             
			bench_exec_time.set((elapsed_ns / 1000000.0f64) as u32);
  
			
            

			/***********************************************************************************************************
            /// **
            /// ENERGY Evaluation
            /// *
			************************************************************************************************************/    
            let nrg: f64 = match energy_type {
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
            target_process.as_mut().unwrap().kill().expect("Target Process wasn't running");
			

        }
		
        pb.finish();

        if target_alive {
            let sum_nrg: f64 = nrg_vec.iter().sum();
            let avg_nrg = sum_nrg / self.num_iter as f64;
            match energy_type {
                EnergyType::throughput => {
                   println!("Thread [{}] {} {:.4} KB/s", core,
                             Red.paint("====> Evaluated Avg. Response Rate: "),
                             avg_nrg);
                }
                EnergyType::latency => {
                    println!("Thread [{}] {} {:.4} ms", core,
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



	/************************************************************************************************************/
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
	/************************************************************************************************************/

    fn parse_args(&self, args_str: String) -> (String,u16) {
        let mut args: Vec<&str> = args_str.split_whitespace().collect();
        
    	let addr=match (&mut args).into_iter()
        		.position(|&mut x| x == "-l" || x == "--address" || x == "-h" || x == "--host"  || x == "--server"){
        			Some(index) => args[index+1].parse().unwrap(),
            		None => {
            				unsafe{
            				if notified==false {
            					println!("In: {:?} - Address not found. Using 127.0.0.1",args_str);
            					notified=true;
            				}}
            				"127.0.0.1".to_string()
            			},
        		};
        
       let port=match (&mut args).into_iter()
        		.position(|&mut x| x == "-p" || x == "--port"){
        			Some(index) => args[index+1].parse().unwrap(),
            		None => panic!("ERROR in: {:?} - Please specify the Port in the arguments",args_str),
        		};
		
  		return (addr,port);
		
    }
	/************************************************************************************************************/
 
 
    fn change_port_arg(&self, args_str: String,base_value: usize, val_2_add: usize) -> String{
    	let args= args_str.clone();
    	let mut new_args_string="".to_string();
    	let mut gotit=false;
    	
    	let vec_args: Vec<&str>=args.split_whitespace().collect();
        for arg in vec_args{
        	if gotit{
				let mut new_port_val=(base_value+val_2_add).to_string();
				new_args_string=new_args_string+" "+new_port_val.as_str();
				gotit=false;
        	}else{
				new_args_string=new_args_string+" "+arg;
			}
        	if arg== "-p" || arg == "--port"{
        		gotit=true;
        	}
        }
        return new_args_string;
    }
	/************************************************************************************************************/
    

}

/// Load the CpuSet for the given core index.
fn cpuset_for_core(topology: &Topology, idx: usize) -> CpuSet {
    let cores = (*topology).objects_with_type(&ObjectType::Core).unwrap();
    match cores.get(idx) {
        Some(val) => val.cpuset().unwrap(),
        None => panic!("No Core found with id {}", idx)
    }
}	

impl Default for EnergyEval {
    fn default() -> EnergyEval {
        EnergyEval {
            target_path:  String::new(),
            bench_path:   String::new(),
            target_args:  String::new(),
            bench_args:   String::new(),
            num_iter: 1,
        }
    }
}
