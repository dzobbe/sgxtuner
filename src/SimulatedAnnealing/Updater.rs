use csv;
use Parameters;
use std::fs::{File,OpenOptions};
use std::io::Write;
use std::io::{BufWriter,BufReader,BufRead};
use std::collections::{HashMap};
use super::Problem::Problem;


pub struct UpdateFile {
    csv_writer: csv::Writer<File>,
    ordered_params: Vec<String>,
}

pub struct UpdateInflux {
    
}

pub trait Updater {
	fn new() -> Self;
    fn send_update(& mut self, measured_val: f64, measured_state: &HashMap<String, u32>, best_val :f64, best_state: &HashMap<String, u32>,num_iter: u64);
}


        
        
impl Updater for UpdateInflux {
	fn new() -> UpdateInflux{
		UpdateInflux {
        }
	}
	
    fn send_update(& mut self, measured_val: f64, measured_state: &HashMap<String, u32>, best_val :f64, best_state: &HashMap<String, u32>,num_iter: u64)  {
    	
    }
}



impl Updater for UpdateFile {
	fn new() -> Self {
		let mut temp_vec: Vec<String>=Vec::new();
		
		let f=OpenOptions::new().write(true).create(true).open("results.csv");
		
		let mut writer = BufWriter::new(f.unwrap());
		let mut wtr = csv::Writer::from_buffer(writer);
		
		
        // Create a path to the params file
        let file_reader = BufReader::new(File::open("params.conf").unwrap());
        for (_, line) in file_reader.lines().enumerate() {
				temp_vec.push(line.unwrap().split(":").next().unwrap().to_string());
		}		
		
		let mut vec_2_write: Vec<String>=Vec::new();
		vec_2_write.push("best_nrg".to_string());
		for param_name in temp_vec.clone().iter().cloned(){
			vec_2_write.push("best_".to_string()+&*param_name);
		}
		
		vec_2_write.push("last_nrg".to_string());
		for param_name in temp_vec.clone().iter().cloned(){
			vec_2_write.push("last_".to_string()+&*param_name);
		}
		
		let res=wtr.encode(vec_2_write);
		assert!(res.is_ok());
		
		wtr.flush();
		
		UpdateFile {
            csv_writer: wtr,
            ordered_params: temp_vec
        }
    } 
	
	
	fn send_update(&mut self, measured_val: f64, measured_state: &HashMap<String, u32>, best_val :f64, best_state: &HashMap<String, u32>,num_iter: u64) {

		let mut vec_2_write: Vec<String>=Vec::new();
		
		
		vec_2_write.push(best_val.to_string());
		for param_name in self.ordered_params.clone().iter().cloned(){
			vec_2_write.push((best_state.get(&param_name).unwrap()).to_string());
		}
		
		
		vec_2_write.push(measured_val.to_string());
		 
		for param_name in self.ordered_params.clone().iter().cloned(){
			vec_2_write.push((measured_state.get(&param_name).unwrap()).to_string());
		}       
		
	    let result = self.csv_writer.encode(vec_2_write);
	    assert!(result.is_ok());
	    
		self.csv_writer.flush();
    }  
}

