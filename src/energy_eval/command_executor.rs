use std::io::prelude::*;
use std::net::{TcpStream};
use ssh2::Session;
use std::process::{Command, Child, Stdio};

use State;



pub trait CommandExecutor{
	fn execute_target(&self, target_path: String, target_args: String, params: &State);
}


pub struct RemoteCommandExecutor{
	pub host: &'static str,
	pub user_4_agent: &'static str,
}

pub struct LocalCommandExecutor;


impl CommandExecutor for RemoteCommandExecutor{
	
	fn execute_target(&self, target_path: String, target_args: String, params: &State){
		
		let cmd = format!("{} {}", target_path.as_str(), target_args.as_str());

		// Connect to the Remote SSH server
		let tcp = TcpStream::connect(self.host).unwrap();
		let mut sess = Session::new().unwrap();
		sess.handshake(&tcp).unwrap();
		sess.userauth_agent(self.user_4_agent).unwrap();
	
		
		let mut channel = sess.channel_session().unwrap();
		
	    for (param_name, param_value) in params.iter() {
	   		channel.setenv(param_name, param_value.to_string().as_str());
		}
	    
		channel.exec(cmd.as_str()).unwrap();
		//let mut s = String::new();
		//channel.read_to_string(&mut s).unwrap();
		//println!("{}", s);
		//println!("{}", channel.exit_status().unwrap());
	}
	

}



impl CommandExecutor for LocalCommandExecutor{
	
	fn execute_target(&self, target_path: String, target_args: String, params: &State) {
		let mut command_2_launch=Command::new(target_path);
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
        	
       // return target_process;
	}
	
		
}

