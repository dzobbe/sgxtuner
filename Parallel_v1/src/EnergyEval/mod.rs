use time;
use libc;
use pbr;
use pbr::{ProgressBar,MultiBar};
use std::str;
use std::process::{Command, Stdio};
use std::{thread, env};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use super::PerfCounters;
use super::MeterProxy;
use ansi_term::Colour::{Red, Yellow, Green};
use std::time::Duration;
use EnergyType;
use std::net::{TcpStream, Shutdown, IpAddr};
use std::io::Stdout;

#[derive(Clone,Debug,RustcEncodable)]
pub struct EnergyEval {
    pub target_path: String,
    pub bench_path: String,
    pub target_args: String,
    pub bench_args: String,
    pub num_iter: u8,
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
                                 params: HashMap<String, u32>,
                                 energy_type: EnergyType,
                                 id_thread: usize)
                                 -> Option<f64> {

        let perf_metrics_handler = PerfCounters::PerfMetrics::new();
        
        //Modify the target and benchmark arguments in order to start different instances
        //on different ports. The annealing id_thread is given to them. This will be sum
        //to the port number
        let new_target_args=self.change_port_arg(self.clone().target_args, 12300, id_thread);
        self.target_args=new_target_args;
        let new_bench_args =self.change_port_arg(self.clone().bench_args, 12400, id_thread);
        self.bench_args=new_bench_args;

        
        let (target_addr, target_port) = self.parse_args(self.clone().target_args);
        let (bench_addr, bench_port)   = self.parse_args(self.clone().bench_args);
        
        
        let mut target_alive: bool = false;

        
        // Repeat the execution 10 times for accurate results
        let mut nrg_vec = Vec::with_capacity(self.num_iter as usize);
        println!("{} Thread {} - Evaluation of: {:?}", Green.paint("====>"), id_thread, params);
        println!("{} Waiting for {} iterations to complete",
                 Green.paint("====>"),
                 self.num_iter);

		
        let mut pb = ProgressBar::new(self.num_iter as u64);
        pb.format("╢▌▌░╟");

 
        for i in 0..self.num_iter {
		println!("Thread: {} ",id_thread);

            pb.inc();
            /// **
            /// Launch TARGET Application
            /// *            
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
            thread::sleep(Duration::from_millis(1500));


            // Check if the target is alive
            target_alive = self.check_target_alive(target_addr.clone(), target_port as u16);
            if target_alive == false {
                target_process.as_mut().unwrap().kill().expect("Target Process wasn't running");
                break;
            }



            /// **
            /// Start METER-PROXY, which will interpose between the Target and the
            /// Benchmark apps to extract metrics for the energy evaluation
            /// *
            let meter_proxy = MeterProxy::Meter::new(target_addr.clone(), target_port, bench_addr.clone(),bench_port);
            let meter_proxy_c = meter_proxy.clone();
            let child_meter_proxy = thread::spawn(move || {
                meter_proxy.start();
            });




            /// **
            /// Launch BENCHMARK Application and measure execution time
            /// *
            let cloned_self = self.clone();
            let elapsed_s_mutex = Arc::new(Mutex::new(0.0));
            let (tx, rx) = channel();
            let (elapsed_s_mutex_c, tx_c) = (elapsed_s_mutex.clone(), tx.clone());

            let bench_thread_handler = thread::spawn(move || {
                let mut elapsed_s_var = elapsed_s_mutex_c.lock().unwrap();

                let start = time::precise_time_ns();
                let bench_args: Vec<&str>=cloned_self.bench_args.split_whitespace().collect();
                let mut bench_process = Some(Command::new(cloned_self.bench_path.clone())
                    .args(bench_args.as_ref())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("Failed to execute Benchmark!"));
                bench_process.as_mut().unwrap().wait().expect("Failed to wait on Benchmark");

                let end = time::precise_time_ns();

                let elapsed_ns: f64 = (end - start) as f64;
                *elapsed_s_var = elapsed_ns / 1000000000.0f64;

                tx_c.send(()).unwrap();
            });


            rx.recv().unwrap();


	
            /// **
            /// ENERGY Evaluation
            /// *
            let nrg: f64 = match energy_type {
                EnergyType::throughput => {
                    // Throughput Evaluation
                    let elapsed_time = *(elapsed_s_mutex.lock().unwrap());
        			
        			println!("Thread: {} - Time: {}",id_thread,elapsed_time);

                    let num_bytes = meter_proxy_c.get_num_bytes_rcvd() as f64;
                    let resp_rate = (num_bytes / elapsed_time) / 1024.0;
                    // println!("{} {:.4} KB/s",
                    // Red.paint("====> Response Rate: "),
                    // resp_rate);
                    resp_rate
                }
                EnergyType::latency => {
                    // Latency Evaluation
                    meter_proxy_c.get_latency_ms()
                }
            };
            nrg_vec.push(nrg);



            /// **
            /// Clean Resources
            /// *
            meter_proxy_c.stop_and_reset();
            child_meter_proxy.join();
            drop(meter_proxy_c);
            target_process.as_mut().unwrap().kill().expect("Target Process wasn't running");

        }

        pb.finish_print("");

        if target_alive {
            let sum_nrg: f64 = nrg_vec.iter().sum();
            let avg_nrg = sum_nrg / self.num_iter as f64;
            match energy_type {
                EnergyType::throughput => {
                    println!("Thread [{}] {} {:.4} KB/s", id_thread,
                             Red.paint("====> Evaluated Avg. Response Rate: "),
                             avg_nrg);//Red.paint("Std. Dev.: "),std_dev);
                }
                EnergyType::latency => {
                    println!("Thread [{}] {} {:.4} ms", id_thread,
                             Red.paint("====> Evaluated Avg. Latency: "),
                             avg_nrg);
                }
            };
            println!("{}",Yellow.paint("==================================================================================================================="));

            return Some(avg_nrg);
        } else {

            return None;
        }


        // println!("Latency {:?} ms", meter_proxy_c.get_latency_ms());
        // meter_proxy_c.print();
    }

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

    fn parse_args(&self, args_str: String) -> (String,u16) {
        let mut args: Vec<&str> = args_str.split_whitespace().collect();
        
    	let addr=match (&mut args).into_iter()
        		.position(|&mut x| x == "-l" || x == "--address" || x == "-h" || x == "--host"  || x == "--server"){
        			Some(index) => args[index+1].parse().unwrap(),
            		None => {
            				println!("In: {:?} - Address not found. Using 127.0.0.1",args_str);
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
 
    fn change_port_arg(&self, args_str: String,base_value: usize, val_2_add: usize) -> String{
    	let args= args_str.clone();
    	let mut new_args_string="".to_string();
    	let mut gotit=false;
    	
    	let vec_args: Vec<&str>=args.split_whitespace().collect();
        for arg in vec_args{
        	if gotit{
				let mut new_port_val=(base_value+val_2_add).to_string();//(arg.parse::<usize>().unwrap()+val_2_add).to_string();
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
