extern crate rand;


use std::io::BufReader;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::env;
use std::process::Command;
use std::collections::HashMap;
use self::rand::Rng;

#[derive(Clone)]
pub struct ParamInfo {
	pub default_value:  u32,
    pub lbound: 		u32,
    pub ubound: 		u32,
    pub step: 			u32,
}

#[derive(Clone)]
pub struct ParamsConfigurator{
		  pub param_file_path:	  String,
          pub params_info_list:   HashMap<String,ParamInfo>,
          pub params_space_state: HashMap<String,Vec<u32>>,
          pub neighborhoods:	  Vec<HashMap<String,u32>>,	
          pub current_state:	  HashMap<String,u32>
          }

impl ParamsConfigurator{
	
		pub fn new() -> ParamsConfigurator {
			Default::default()
		}

        pub fn get_initial_param_conf(& mut self) -> HashMap<String,u32>{
		    
		    let f=self.param_file_path.clone();
			// Create a path to the desired file
		    let path = Path::new(&f);
			let display = path.display();
	
	    		// Open the path in read-only mode, returns `io::Result<File>`
			let file = match File::open(&path) {
	    		Err(why) => panic!("couldn't open {}: {}", display,why.description()),
	    		Ok(file) => file,
			};
	
	 		let file_reader = BufReader::new(&file);
			println!("Reading SGX-MUSL Parameters from file: ");
			for (num, line) in file_reader.lines().enumerate() {
		    		let mut topline= line.unwrap();
					let mut topsplit=topline.split(":");
					
		            let (mut var_name, mut var_value,
		            	 mut var_lbound, mut var_ubound, mut var_step);
						
					match topsplit.next() {
		    			Some(x) => var_name=x,
						None => break,
		   			}
					
					match topsplit.next() {
		                Some(subline) => {
		                	let mut subsplit=subline.split(",");
		                	match subsplit.next() {
		                		Some(x) => var_lbound=str::replace(x,"[",""),
		                		None => break,
		                	}
		                	match subsplit.next() {
		                		Some(x) => var_ubound=x,
		                		None => break,
		                	}
		                	match subsplit.next() {
		                		Some(x) => var_step=str::replace(x,"]",""),
		                		None => break,		                		
		                	}
		                },
		                None => break,
		            }
					
					match topsplit.next() {
		    			Some(x) => var_value=x,
						None => break,
		   			}
					
					let params_info_2_add = ParamInfo{
								default_value:var_value.parse::<u32>().unwrap(),
								lbound:var_lbound.parse::<u32>().unwrap(),
								ubound:var_ubound.parse::<u32>().unwrap(),
								step:var_step.parse::<u32>().unwrap()
								};
					
					self.params_info_list.insert(var_name.to_string(),params_info_2_add);
					self.params_space_state.insert(
						var_name.to_string(),
						ParamsConfigurator::get_space_state(var_lbound.parse::<u32>().unwrap(),
								var_ubound.parse::<u32>().unwrap(),var_step.parse::<u32>().unwrap()));
					self.current_state.insert(var_name.to_string(),var_value.parse::<u32>().unwrap());
					
					
		            println!("Parameter {:?} - Default value: {:?} {:5} Space State: [{:?},{:?},{:?}]",
		            	var_name,var_value,"-",var_lbound,var_ubound,var_step);

				}				
				//ParamsConfigurator::fill_neighborhoods_vec(self);
				
				return self.current_state.clone();			
		
		} 
       
       
        fn get_space_state(lbound: u32,ubound: u32,step: u32) -> Vec<u32>{
        	 let mut res_vec=Vec::new();
        	 let num_it=(ubound-lbound)/step;
        	 for x in 0..num_it {
        	 	res_vec.push(lbound+(step*x));
        	 	if x==num_it-1 {
        	 		res_vec.push(lbound+(step*(x+1)));
        	 		}	
			 }	
        	 //Randomize the order of the vector elements
        	 rand::thread_rng().shuffle(&mut res_vec);
        	 println!("Space State Elements: {:?}",res_vec);
			 return res_vec;		
        }
       
       
        fn fill_neighborhoods_vec(& mut self,max_steps: u64, current_step: u64) {
        	
        	for (param_name, space_state_vec) in self.params_space_state.iter() {
				for param_values in space_state_vec.iter(){
					let mut temp = self.current_state.clone();
					*(temp).get_mut(param_name).unwrap() = *param_values;
					self.neighborhoods.push(temp);
				}
        	}
        	println!("Created the vector of Neighborhoods, composed by {:?} parameter configurations",self.neighborhoods.len());
        }
        
        
        pub fn get_neighborhood_params(& mut self,max_steps: u64, current_step: u64) -> Option<HashMap<String,u32>> {
        	
        	ParamsConfigurator::fill_neighborhoods_vec(self,max_steps,current_step);
        	
        	if self.neighborhoods.len()==0{
        		return None;
        	}else{	
        		let num_configurations=self.neighborhoods.len();
        		let mut curr_state=self.current_state.clone();
        		
        		curr_state.clone_from(&
        			self.neighborhoods.remove(rand::thread_rng().gen_range(0,num_configurations-1)));
    			println!("New Parameter Configuration set from Neighborhood Set: {:?}",curr_state);
        		return Some(curr_state);
       		}
        }
        
        
        pub fn get_random_state(&mut self) -> HashMap<String,u32>{
        	
			for (param_name, space_state) in self.params_space_state.iter_mut(){
				let random_value=rand::thread_rng().choose(&space_state).unwrap();
				*(self.current_state).get_mut(param_name).unwrap() = *random_value;
			}
			println!("New Random Parameter Configuration set to:\n {:?}",self.current_state);
			//ParamsConfigurator::fill_neighborhoods_vec(self);
			return self.current_state.clone();			
		}
        
}


impl Default for ParamsConfigurator {
    fn default() -> ParamsConfigurator {
       ParamsConfigurator{
       			param_file_path:	"".to_string(),
				params_info_list:   HashMap::new(),
          		params_space_state: HashMap::new(),
          		neighborhoods:		Vec::new(),
          		current_state:		HashMap::new()
		}
		
    }
}
