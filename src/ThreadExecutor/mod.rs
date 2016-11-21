extern crate monitor;
extern crate time;
extern crate ansi_term;

use std::str;
use std::process::{Command, Stdio};
use std::{thread,env};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use super::PerfCounters;
use super::MeterProxy;  
use self::ansi_term::Colour::{Red,Yellow};
use std::time::Duration;

#[derive(Clone)]
pub struct ThreadExecutor {
      pub target_path: String,
      pub bench_path:  String,
      pub target_args: String,
      pub bench_args:  Vec<String>,
      pub meter_proxy: MeterProxy::Meter,
  }


impl ThreadExecutor {

	pub fn new() -> ThreadExecutor{
		Default::default()
	}
	
	
	
	/**
	Execute an an instance of the benchmark on the target application for the specific
	configuration of parameters. The function returns the cost result (in this case the response throughput)
	that will be used by the simulated annealing algorithm for the energy evaluation
	**/
	pub fn execute_test_instance(& mut self, params: &HashMap<String,u32>) -> f64 { 
		
		let perf_metrics_handler = PerfCounters::PerfMetrics::new();
			
	    
   		/**
		Set the environement variables that will configure the parameters
		needed by the target application
		**/
		for (param_name, param_value) in params.iter() {
			env::set_var(param_name.to_string(), param_value.to_string());
			println!("Environment Variable for {:?} set to: {:?}",param_name,param_value);
		}
		
	 	


		/**
		Launch Target Application
		**/
        let vec_args: Vec<&str>=self.target_args.split_whitespace().collect();
        let mut target_process = Some(Command::new(self.target_path.clone())
                    .args(vec_args.as_ref())
                    .stdout(Stdio::piped())
                    .spawn()
                    .expect("Failed to execute Target!"));    
        let pid_target = target_process.as_mut().unwrap().id();
        //thread::sleep(Duration::from_millis(1000));




		/**
		Start MeterProxy, which will interpose between the Target and the
		Benchmark apps to extract info on the Response Throughput
		**/
	    let meter_proxy=MeterProxy::Meter::new();
	    let meter_proxy_c=meter_proxy.clone();
	    let child_meter_proxy=thread::spawn(move || { 
			meter_proxy.start(12347,12349);
	    });
			
	    
	      
		/**
		Launch Benchmark Application and measure execution time
		**/
		let cloned_self=self.clone();
		let elapsed_s_mutex = Arc::new(Mutex::new(0.0));
		let (tx, rx) = channel();
    	let (elapsed_s_mutex_c,tx_c) = (elapsed_s_mutex.clone(), tx.clone());

        let bench_thread_handler=thread::spawn(move || {
					let mut elapsed_s_var = elapsed_s_mutex_c.lock().unwrap();

        			let start = time::precise_time_ns();
			    	let mut bench_process = Some(Command::new(cloned_self.bench_path.clone())
			                        	.args(cloned_self.bench_args.as_ref())
			                        	//.stdout(Stdio::piped())										                        	
				                        .spawn()
				                        .expect("Failed to execute Benchmark!"));
			    	bench_process.as_mut().unwrap().wait().expect("Failed to wait on Benchmark");
			    	
			    	let end = time::precise_time_ns();
			    	
			    	let elapsed_ns: f64=(end-start) as f64;
			    	*elapsed_s_var= elapsed_ns/1000000000.0f64;
			    	
 					tx_c.send(()).unwrap();
					});
		rx.recv().unwrap();


		
		
		/**
		The response throughput is calculated and returned
		**/	
		let elapsed_time=*(elapsed_s_mutex.lock().unwrap());
		
		let num_bytes=meter_proxy_c.get_num_bytes() as f64;
		let resp_rate=(num_bytes/elapsed_time)/1024.0;
		
		println!("{} {:.3} KB/s",Red.paint("Response Rate: "),resp_rate);	    	
        println!("[TARG-THREAD] Finished Waiting! Shutting down the target and cleaning resources...");
        
        
      	meter_proxy_c.stop_and_reset();
	    child_meter_proxy.join();
	    target_process.as_mut().unwrap().kill().expect("Target Process wasn't running");
	    
        println!("Test Instance Terminated!!");
        println!("{}",Yellow.paint("==============================================================================="));	    	
        
        //Wait that socket file descriptors are cleaned up to avoid OS exception ("Too many open files")
        thread::sleep(Duration::from_millis(3000));
		return resp_rate;
	}

}

impl Default for ThreadExecutor {
    fn default() -> ThreadExecutor {
       ThreadExecutor{
	          	target_path: "".to_string(),
	          	bench_path:  "".to_string(),
	          	target_args: "".to_string(),
	          	bench_args:  Vec::new(),
	          	meter_proxy: MeterProxy::Meter::new()

		}
    }
}
