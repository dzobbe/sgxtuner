use time;
use libc;
use std::io::prelude::*;
use std::net::{TcpStream};
use ssh2::Session;
use std::process::{Command, Child, Stdio};
use std::time::Duration;
use std::thread;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use libc::{kill, SIGTERM};

use State;

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



pub trait CommandExecutor{
	fn execute_target(&self, target_path: String, target_bin:String, target_args: String, params: &State, signal_ch: mpsc::Receiver<bool>);
	fn execute_bench(&self, bench_path: String, bench_bin:String, bench_args: String);
}


pub struct RemoteCommandExecutor{
	pub host: String,
	pub user_4_agent: String,
}

pub struct LocalCommandExecutor;


impl CommandExecutor for RemoteCommandExecutor{
	
	fn execute_target(&self, target_path: String, target_bin:String, target_args: String, params: &State, signal_ch: mpsc::Receiver<bool>){
		
		let host=self.host.clone();
		let user=self.user_4_agent.clone();
		let params_c=params.clone();
        thread::spawn(move || { 
				
				// Connect to the Remote SSH server
				let tcp = TcpStream::connect(host.as_str()).unwrap();
				let mut sess = Session::new().unwrap();
				sess.set_allow_sigpipe(true);
				sess.handshake(&tcp).unwrap();
				sess.userauth_agent(user.as_str()).unwrap();
			
				 
				let mut channel = sess.channel_session().unwrap();
				
				let mut env_cmd=String::new();
				for (param_name, param_value) in params_c.iter() {
					env_cmd = format!("{}export {}={};",env_cmd.as_str(), param_name, param_value);
				}
								
			    let cmd = format!("{}{} {}",env_cmd.as_str(), (target_path+target_bin.as_str()).as_str(), target_args.as_str());
	    		channel.exec(cmd.as_str()).unwrap();
				
				let mut channel_2 = sess.channel_session().unwrap();
					    		
	    		signal_ch.recv();
	    		
	    		let kill_cmd=format!("pkill {}",target_bin);
	    		channel_2.exec(kill_cmd.as_str()).unwrap();
		});
        
	}
	
	fn execute_bench(&self, bench_path: String, bench_bin:String, bench_args: String) {
		let host=self.host.clone();
		let user=self.user_4_agent.clone();
        thread::spawn(move || { 
				// Connect to the Remote SSH server
				let tcp = TcpStream::connect(host.as_str()).unwrap();
				let mut sess = Session::new().unwrap();
				sess.set_allow_sigpipe(true);
				sess.handshake(&tcp).unwrap();
				sess.userauth_agent(user.as_str()).unwrap();
			
				 
				let mut channel = sess.channel_session().unwrap();
				
						
			    let cmd = format!("{} {}",(bench_path+bench_bin.as_str()).as_str(), bench_args.as_str());
	    		channel.exec(cmd.as_str()).unwrap();
		});
	}
	
}



impl CommandExecutor for LocalCommandExecutor{
	
	fn execute_target(&self, target_path: String, target_bin:String, target_args: String, params: &State, signal_ch:  mpsc::Receiver<bool>) {
		
		let mut command_2_launch=Command::new(target_path+target_bin.as_str());
	    /// Set the environement variables that will configure the parameters
	    /// needed by the target application
	    ///
	    for (param_name, param_value) in params.iter() {
	   		command_2_launch.env(param_name.to_string(), param_value.to_string());
		}
	    
	    
		let mut vec_args: Vec<&str> = target_args.split_whitespace().collect();
        let mut target_process = command_2_launch
                .args(vec_args.as_ref()) 
                .stdout(Stdio::piped())
                .spawn()
                .expect("Failed to execute Target!");
        	
        	
		signal_ch.recv();
        target_process.kill().expect("Target Process wasn't running");

	}

	fn execute_bench(&self, bench_path: String, bench_bin:String, bench_args: String) {
        let start_time = time::precise_time_ns();
		let bench_args: Vec<&str>=bench_args.split_whitespace().collect();
        let mut bench_process = Command::new(bench_path+bench_bin.as_str())
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
    	let end_time = time::precise_time_ns();
	    let elapsed_ns: f64 = (end_time - start_time) as f64;
	    let elapsed_time = elapsed_ns / 1000000000.0f64;
	     
		bench_exec_time.set((elapsed_ns / 1000000.0f64) as u32);
        
        bench_process.wait().expect("Failed to wait on Benchmark");
	}
	
}

