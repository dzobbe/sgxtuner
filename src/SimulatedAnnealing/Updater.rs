use std::fs::{File,OpenOptions};
use std::io::Write;
use std::collections::HashMap;

pub struct UpdateFile {
    file: File,
}

pub struct UpdateInflux {
    
}

pub trait Updater {
	fn new() -> Self;
    fn send_update(&self, measured_val: f64, measured_state: HashMap<String, u32>, best_val :f64, best_state: HashMap<String, u32>,num_iter: u64);
}




impl Updater for UpdateInflux {
	fn new() -> UpdateInflux{
		UpdateInflux {
        }
	}
	
    fn send_update(&self,measured_val: f64, measured_state: HashMap<String, u32>, best_val :f64, best_state: HashMap<String, u32>,num_iter: u64) {
    	
    }
}


impl Updater for UpdateFile {
	fn new() -> Self {
		let f=OpenOptions::new().write(true).create(true).open("results.log");
		UpdateFile {
            file: f.unwrap(),
        }
    } 
	
	fn send_update(&self,measured_val: f64, measured_state: HashMap<String, u32>, best_val :f64, best_state: HashMap<String, u32>,num_iter: u64) {
		//self.file.write(&(line_2_write).into_bytes());
    }  
}

