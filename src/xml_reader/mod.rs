use std::fs::File;
use std::io::BufReader;
use xml::reader::{EventReader, XmlEvent};
use std::collections::HashMap;
use EnergyType;
use ProblemType;
use ExecutionType;
use CoolingSchedule;
use SolverVersion;

#[derive(Debug, Clone)]
pub struct XMLReader{
	target_params: HashMap<String,String>,
	bench_params: HashMap<String,String>,
	ann_params: HashMap<String,String>,
}

impl XMLReader{
	
	pub fn new(file: String) -> Self{
	 	let file = File::open(file).unwrap();
	    let file = BufReader::new(file);
	
	    let parser = EventReader::new(file);
	    let mut found_targ=false;
	    let mut found_bench=false;
	    let mut found_ann=false;

	    let mut tag=String::new();
	    
		let mut targ_p=HashMap::new();
		let mut bench_p=HashMap::new();
		let mut ann_p=HashMap::new();

	    for e in parser {
	        match e{
	            Ok(XmlEvent::StartElement { name, .. }) => {
	               tag=name.to_string();
	               if name.to_string()=="bench"{
                    	found_ann=false;
                		found_bench=true;
                		found_targ=false;
                	}else if name.to_string()=="target"{
						found_ann=false;
                		found_bench=false;
                		found_targ=true;
                	}else if name.to_string()=="annealing"{
                		found_ann=true;
                		found_bench=false;
                		found_targ=false;
                	}
	
	            }
	            Ok(XmlEvent::Characters(val))=>{
	                if found_bench==true{
						bench_p.insert(tag.clone(),val.clone());
	                }else if found_targ==true{
	                	targ_p.insert(tag.clone(),val.clone());
	                }else if found_ann==true{
	                	ann_p.insert(tag.clone(),val.clone());
	                }
	            }
	            
	            Err(e) => {
	                println!("Error: {}", e);
	                break;
	            }
	            _ => {}
	        }
    	}
	    
		XMLReader{
			target_params: targ_p,
			bench_params: bench_p,
			ann_params: ann_p,
		}
	}
	
	
	/***********************************************************************************************************
	/// **
    /// Target Parameters
    /// * 
	************************************************************************************************************/            
    pub fn targ_exec_type(&self) -> ExecutionType{
	    let exec_type_enum: ExecutionType = self.target_params.get("execution").unwrap().to_string().parse().unwrap();
		return exec_type_enum;
	}
	pub fn targ_host(&self) -> String{
    	return self.target_params.get("host").unwrap().to_string();
	}
	pub fn targ_host_user(&self) -> String{
		return self.target_params.get("user").unwrap().to_string()
	}
	pub fn targ_bin(&self)  -> String{
		return self.target_params.get("bin").unwrap().to_string();
	}
	pub fn targ_path(&self) -> String{
		return self.target_params.get("path").unwrap().to_string();
	}
	pub fn targ_args(&self) -> String{
		return self.target_params.get("args").unwrap().to_string();
	}
	pub fn targ_address(&self) -> String{
		return self.target_params.get("address").unwrap().to_string();
	}
	pub fn targ_port(&self) -> String{
		return self.target_params.get("port").unwrap().to_string();
	}	
	
	
	/***********************************************************************************************************
	/// **
    /// Benchmark Parameters
    /// * 
	************************************************************************************************************/            
	pub fn bench_exec_type(&self) -> ExecutionType{
	    let exec_type_enum: ExecutionType = self.bench_params.get("execution").unwrap().to_string().parse().unwrap();
		return exec_type_enum;
	}
	pub fn bench_host(&self) -> String{
    	return self.bench_params.get("host").unwrap().to_string();
	}
	pub fn bench_host_user(&self) -> String{
		return self.bench_params.get("user").unwrap().to_string()
	}
	pub fn bench_bin(&self)  -> String{
		return self.bench_params.get("bin").unwrap().to_string();
	}
	pub fn bench_path(&self) -> String{
		return self.bench_params.get("path").unwrap().to_string();
	}
	pub fn bench_args(&self) -> String{
		return self.bench_params.get("args").unwrap().to_string();
	}
	pub fn bench_address(&self) -> String{
		return self.bench_params.get("address").unwrap().to_string();
	}
	pub fn bench_port(&self) -> String{
		return self.bench_params.get("port").unwrap().to_string();
		
	}
	
	
	
	/***********************************************************************************************************
	/// **
    /// Annealing Parameters
    /// * 
	************************************************************************************************************/            
	pub fn ann_max_steps(&self) -> usize{
		return self.ann_params.get("max_step").unwrap().to_string().parse().unwrap();
	}
	pub fn ann_num_iter(&self)  -> usize{
		return self.ann_params.get("num_iter").unwrap().to_string().parse().unwrap();
	}
	pub fn ann_min_temp(&self) -> Option<f64>{
		return Some(self.ann_params.get("min_temp").unwrap().to_string().parse::<f64>().unwrap());
	}
	pub fn ann_max_temp(&self) -> Option<f64>{
		return Some(self.ann_params.get("max_temp").unwrap().to_string().parse::<f64>().unwrap());
	}
	pub fn ann_energy(&self)  -> EnergyType{
	    let energy_type: EnergyType = self.ann_params.get("energy").unwrap().to_string().parse().unwrap();
		return energy_type;
	}
	pub fn ann_cooling(&self) -> CoolingSchedule{
   		let cooling_schedule: CoolingSchedule = self.ann_params.get("cooling").unwrap().to_string().parse().unwrap();
		return cooling_schedule;
	}
	pub fn ann_problem(&self) -> ProblemType{
   		let problem_type: ProblemType = self.ann_params.get("problem").unwrap().to_string().parse().unwrap();
		return problem_type;
	}
	pub fn ann_version(&self) -> SolverVersion{
   		let solver_version: SolverVersion = self.ann_params.get("version").unwrap().to_string().parse().unwrap();
		return solver_version;
	}
    
	
}