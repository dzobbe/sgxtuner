use std::fs::File;
use std::io::BufReader;
use xml::reader::{EventReader, XmlEvent};
use std::collections::HashMap;
use EnergyType;
use ProblemType;
use ExecutionType;
use CoolingSchedule;
use SolverVersion;
use shared::{Process2Spawn, Parameter, ProcessPool};

#[derive(Debug, Clone)]
pub struct XMLReader {
    targets_collection: ProcessPool,
    benchs_collection: ProcessPool,
    ann_params: HashMap<String, String>,
    musl_params: Vec<Parameter>,
}

impl XMLReader {
    pub fn new(file: String) -> Self {
        let file = File::open(file).unwrap();
        let file = BufReader::new(file);

        let parser = EventReader::new(file);
        let mut found_targ = false;
        let mut found_bench = false;
        let mut found_ann = false;
        let mut found_muslp = false;

        let mut tag = String::new();

        let mut ann_p = HashMap::new();

        let mut targ_p = ProcessPool::new();
        let mut targ_p_x: HashMap<String, String> = HashMap::new();

        let mut bench_p = ProcessPool::new();
        let mut bench_p_x: HashMap<String, String> = HashMap::new();


        let mut musl_p: Vec<Parameter> = Vec::new();
        let mut musl_p_x: HashMap<String, String> = HashMap::new();

		let mut index_targs=0;
		let mut index_bench=0;
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                    tag = name.to_string();
                    if name.to_string() == "bench" {
                        found_ann = false;
                        found_bench = true;
                        found_targ = false;
                        found_muslp = false;
                    } else if name.to_string() == "target" {
                        found_ann = false;
                        found_bench = false;
                        found_targ = true;
                        found_muslp = false;
                    } else if name.to_string() == "annealing" {
                        found_ann = true;
                        found_bench = false;
                        found_targ = false;
                        found_muslp = false;
                    } else if name.to_string() == "musl-params" {
                        found_ann = false;
                        found_bench = false;
                        found_targ = false;
                        found_muslp = true;
                    }

                }
                Ok(XmlEvent::Characters(val)) => {
                    if found_bench == true {
                        bench_p_x.insert(tag.clone(), val.clone());
                    } else if found_targ == true {
                        targ_p_x.insert(tag.clone(), val.clone());
                    } else if found_ann == true {
                        ann_p.insert(tag.clone(), val.clone());
                    } else if found_muslp == true {
                        musl_p_x.insert(tag.clone(), val.clone());
                    }
                }

                Ok(XmlEvent::EndElement { name }) => {
                	
            	 	if name.to_string() == "bench" {
                        found_bench=false;
                    } else if name.to_string() == "target" {
                        found_targ=false;
                    } else if name.to_string() == "musl-params" {
                        found_muslp=false;
                    }
                    
                    if found_bench == true && name.to_string() != tag.to_string() {
                        let exec_type_enum: ExecutionType =
                            bench_p_x.get("execution").unwrap().to_string().parse().unwrap();
                            
                        let (host, user)=match exec_type_enum{
                        	ExecutionType::local  =>("".to_string(),"".to_string()),
                        	ExecutionType::remote =>(bench_p_x.get("host").unwrap().to_string(), bench_p_x.get("user").unwrap().to_string()),
                        };
                        let mut bench_2_spawn = Process2Spawn {
                            execution_type: exec_type_enum,
                            host: host,
                            user: user,
                            bin: bench_p_x.get("bin").unwrap().to_string(),
                            path: bench_p_x.get("path").unwrap().to_string(),
                            args: bench_p_x.get("args").unwrap().to_string(),
                            address: bench_p_x.get("address").unwrap().to_string(),
                            port: bench_p_x.get("port").unwrap().to_string(),
                        };
                        bench_p.push(bench_2_spawn,index_bench.to_string());
                        index_bench+=1;
                    }

                    if found_targ == true && name.to_string() != tag.to_string() {
                        let exec_type_enum: ExecutionType =
                            targ_p_x.get("execution").unwrap().to_string().parse().unwrap();
                        let (host, user)=match exec_type_enum{
                        	ExecutionType::local  =>("".to_string(),"".to_string()),
                        	ExecutionType::remote =>(targ_p_x.get("host").unwrap().to_string(), targ_p_x.get("user").unwrap().to_string()),
                        };
                        
                        let mut targ_2_spawn = Process2Spawn {
                            execution_type: exec_type_enum,
                            host: host,
                            user: user,
                            bin: targ_p_x.get("bin").unwrap().to_string(),
                            path: targ_p_x.get("path").unwrap().to_string(),
                            args: targ_p_x.get("args").unwrap().to_string(),
                            address: targ_p_x.get("address").unwrap().to_string(),
                            port: targ_p_x.get("port").unwrap().to_string(),
                        };
						
                        targ_p.push(targ_2_spawn,index_targs.to_string());
                        index_targs+=1;
                    }

                    if found_muslp == true && name.to_string() != tag.to_string() {
                        let mut musl_parameter = Parameter {
                            name: musl_p_x.get("name").unwrap().to_string(),
                            min: musl_p_x.get("minimum")
                                .unwrap()
                                .to_string()
                                .parse::<usize>()
                                .unwrap(),
                            max: musl_p_x.get("maximum")
                                .unwrap()
                                .to_string()
                                .parse::<usize>()
                                .unwrap(),
                            step: musl_p_x.get("step")
                                .unwrap()
                                .to_string()
                                .parse::<usize>()
                                .unwrap(),
                            default: musl_p_x.get("default")
                                .unwrap()
                                .to_string()
                                .parse::<usize>()
                                .unwrap(),
                        };
                        musl_p.push(musl_parameter);
                    }

                }

                Err(e) => {
                    println!("Error: {}", e);
                    break;
                }
                _ => {}
            }
        }
         

		if  ann_p.get("version").unwrap().to_string() != "seqsea" {
			
			while targ_p.size() != ann_p.get("workers").unwrap().to_string().parse::<usize>().unwrap(){
				println!("S {}",targ_p.size());
				let copy_elem=targ_p.get("0".to_string());
				targ_p.push(copy_elem,targ_p.size().to_string());
			}
			
			while bench_p.size() != ann_p.get("workers").unwrap().to_string().parse::<usize>().unwrap(){
				let copy_elem=bench_p.get("0".to_string());
				bench_p.push(copy_elem,bench_p.size().to_string());
			}
			
		}
         
        XMLReader {
            targets_collection: targ_p,
            benchs_collection: bench_p,
            ann_params: ann_p,
            musl_params: musl_p,
        }
    }

    /***********************************************************************************************************
    /// **
    /// MUSL Params
    /// *
    	************************************************************************************************************/

    pub fn get_musl_params(&self) -> Vec<Parameter> {
        return self.musl_params.clone();
    }



    /***********************************************************************************************************
    /// **
    /// Target
    /// *
    	************************************************************************************************************/

    pub fn get_targs_pool(&self) -> ProcessPool {
        return self.targets_collection.clone();
    }


    /***********************************************************************************************************
    /// **
    /// Benchmark
    /// *
    	************************************************************************************************************/

    pub fn get_bench_pool(&self) -> ProcessPool {
        return self.benchs_collection.clone();
    }



    /***********************************************************************************************************
    /// **
    /// Annealing Parameters
    /// *
    	************************************************************************************************************/

    pub fn ann_max_steps(&self) -> usize {
        return self.ann_params.get("max_step").unwrap().to_string().parse().unwrap();
    }
    pub fn ann_num_iter(&self) -> usize {
        return self.ann_params.get("num_iter").unwrap().to_string().parse().unwrap();
    }
    pub fn ann_min_temp(&self) -> Option<f64> {
        return Some(self.ann_params.get("min_temp").unwrap().to_string().parse::<f64>().unwrap());
    }
    pub fn ann_max_temp(&self) -> Option<f64> {
        return Some(self.ann_params.get("max_temp").unwrap().to_string().parse::<f64>().unwrap());
    }
    
    pub fn ann_workers(&self) -> usize  {
        return self.ann_params.get("workers").unwrap().to_string().parse::<usize>().unwrap();
    }
    
    pub fn ann_energy(&self) -> EnergyType {
        let energy_type: EnergyType =
            self.ann_params.get("energy").unwrap().to_string().parse().unwrap();
        return energy_type;
    }
    pub fn ann_cooling(&self) -> CoolingSchedule {
        let cooling_schedule: CoolingSchedule =
            self.ann_params.get("cooling").unwrap().to_string().parse().unwrap();
        return cooling_schedule;
    }
    pub fn ann_problem(&self) -> ProblemType {
        let problem_type: ProblemType =
            self.ann_params.get("problem").unwrap().to_string().parse().unwrap();
        return problem_type;
    }
    pub fn ann_version(&self) -> SolverVersion {
        let solver_version: SolverVersion =
            self.ann_params.get("version").unwrap().to_string().parse().unwrap();
        return solver_version;
    }
}
