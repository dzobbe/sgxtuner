use time;
use libc;
use hwloc;
use pbr;
use pbr::{ProgressBar, MultiBar};
use std::str;
use std::cell::RefCell;
use std::rc::Rc;
use std::process::{Command, Child, Stdio};
use std::{thread, env};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use meter_proxy::MeterProxy;
use ansi_term::Colour::{Red, Yellow, Green};
use std::time::Duration;
use EnergyType;
use ExecutionType;
use std::net::{TcpStream, Shutdown, IpAddr};
use std::io::{stdout, Stdout};
use hwloc::{Topology, CPUBIND_PROCESS, CPUBIND_THREAD, CpuSet, TopologyObject, ObjectType};
use libc::{kill, SIGTERM};
use State;
use energy_eval::command_executor::CommandExecutor;
use xml_reader::XMLReader;
use shared::Process2Spawn;
use shared::ProcessPool;

pub mod command_executor;

#[derive(Clone,Debug)]
pub struct EnergyEval {
    pub xml_reader: XMLReader,
}


static mut notified: bool = false;
static base_target_port: usize = 12400;
static base_bench_port: usize = 12600;
static mut counter: u16 = 0;




#[derive(Clone)]
struct SpawnedMeterProxy(Arc<Mutex<HashMap<String, MeterProxy>>>);
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
        let res = (*spawned_vec).get(&address).unwrap().clone();
        res
    }
}


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
    static ref spawned_proxies : SpawnedMeterProxy     = {SpawnedMeterProxy::new()};
	static ref bench_exec_time: BenchExecTime = {BenchExecTime::new()};
    }


impl EnergyEval {
    /**
	Execute an an instance of the benchmark on the target application for the specific
	configuration of parameters. The function returns the cost result (in this case the response throughput)
	that will be used by the simulated annealing algorithm for the energy evaluation
	**/
    pub fn execute_test_instance(&mut self, params: &State, core: usize) -> Option<f64> {

        //Extract the target pool
        let mut target_pool = self.xml_reader.get_targs_pool();
        //Extract the bench pool
        let mut bench_pool = self.xml_reader.get_bench_pool();


		/*let target_x = match target_pool.remove(){
        	Some(x) => x,
        	None => "",
        };       
		
        let bench_x = match bench_pool.remove(){
        	Some(x) => x,
        	None => "",
        };*/
		
		let target_x =  target_pool.remove(core.to_string());
		
        let bench_x =  bench_pool.remove(core.to_string());

        // let perf_metrics_handler = PerfMeter::new();

        // Modify the target and benchmark arguments in order to start different instances
        // on different ports. The annealing core is given to them. This will be sum
        // to the port number
        let new_target_args =
            target_x.clone().args.replace(target_x.port.as_str(),
                                          (base_target_port + core).to_string().as_str());
        let new_bench_args =
            bench_x.clone().args.replace(bench_x.port.as_str(),
                                         (base_bench_port + core).to_string().as_str());

		

        let (target_addr, target_port) = (target_x.clone().address,
                                          (base_target_port + core) as u16);
        let (bench_addr, bench_port) = (bench_x.clone().address, (base_bench_port + core) as u16);
		
		println!("Core: {} - {} - {} {} {} ", core, target_addr, target_port, bench_addr,bench_port);
		
        let mut target_alive: bool = false;

        // Repeat the execution num_iter times for accurate results
        let mut nrg_vec = Vec::with_capacity(self.xml_reader.ann_num_iter() as usize);
        println!("{} TID [{}] - Evaluation of: {:?}",
                 Green.paint("====>"),
                 core,
                 params);
        println!("{} Waiting for {} iterations to complete",
                 Green.paint("====>"),
                 self.xml_reader.ann_num_iter());

        let mut pb = ProgressBar::new(self.xml_reader.ann_num_iter() as u64);
        pb.format("╢▌▌░╟");
        pb.show_message = true;
        pb.message(&format!("Thread [{}] - ", core));


        for i in 0..self.xml_reader.ann_num_iter() {
            pb.inc();



            /***********************************************************************************************************
            /// **
            /// Start METER-PROXY, which will interpose between the Target and the
            /// Benchmark apps to extract metrics for the energy evaluation
            /// *
            	************************************************************************************************************/


            let mut meter_proxy = MeterProxy::new(target_addr.clone(),
                                                  target_port,
                                                  bench_addr.clone(),
                                                  bench_port);
            let mut meter_proxy_c = meter_proxy.clone();

			let mut key=target_addr.clone()+target_port.to_string().as_str()+bench_addr.clone().as_str()+bench_port.to_string().as_str();
            
            if !spawned_proxies.spawned(key.clone()) {
                spawned_proxies.insert(key.clone(), meter_proxy.clone());

                thread::spawn(move || { meter_proxy.start(); });
            } else {
                let mut sp = spawned_proxies.clone();
                meter_proxy = sp.get(key);
                meter_proxy_c = meter_proxy.clone();
            }




            /***********************************************************************************************************
            /// **
            /// Launch TARGET Application
            /// *
            	************************************************************************************************************/
            /// Set the environement variables that will configure the parameters
            /// needed by the target application
            ///
            let (tx, rx) = channel::<bool>();

            match target_x.clone().execution_type {
                ExecutionType::local => {
	                    let local_cmd_executor = command_executor::LocalCommandExecutor;	
                		local_cmd_executor.execute_target(target_x.path.clone(),
                                                  target_x.bin.clone(),
                                                  new_target_args.clone(),
                                                  &params,
                                                  rx);
                }
                ExecutionType::remote => {
                    let remote_cmd_executor = command_executor::RemoteCommandExecutor {
                        host: target_x.clone().host,
                        user_4_agent: target_x.clone().user,
                    };
                    remote_cmd_executor.execute_target(target_x.path.clone(),
                                                       target_x.bin.clone(),
                                                       new_target_args.clone(),
                                                       &params.clone(),
                                                       rx);
                }
            }

			
            // Wait for target to startup
            thread::sleep(Duration::from_millis(1000));
            // Check if the target is alive
            target_alive = self.check_target_alive(target_addr.clone(), target_port as u16);
            if target_alive == false {
	            target_pool.push(target_x,core.to_string());
        		bench_pool.push(bench_x.clone(),core.to_string());
                tx.send(true);
                break;
            }


            let start_time = time::precise_time_ns();

            /***********************************************************************************************************
            /// **
            /// Launch BENCHMARK Application and measure execution time
            /// *
            	************************************************************************************************************/


            match bench_x.clone().execution_type {
                ExecutionType::local => {
                    let local_cmd_executor = command_executor::LocalCommandExecutor;
                    local_cmd_executor.execute_bench(bench_x.path.clone(),
                                                     bench_x.bin.clone(),
                                                     new_bench_args.clone());
                }
                ExecutionType::remote => {
                    let remote_cmd_executor = command_executor::RemoteCommandExecutor {
                        host: bench_x.clone().host,
                        user_4_agent: bench_x.clone().user,
                    };
                    remote_cmd_executor.execute_bench(bench_x.path.clone(),
                                                      bench_x.bin.clone(),
                                                      new_bench_args.clone());
                }
            }
            let end_time = time::precise_time_ns();
            let elapsed_ns: f64 = (end_time - start_time) as f64;
            let elapsed_time = elapsed_ns / 1000000000.0f64;





            /***********************************************************************************************************
            /// **
            /// ENERGY Evaluation
            /// *
            	************************************************************************************************************/

            let nrg: f64 = match self.xml_reader.ann_energy() {
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
            //Send signal to target to exit
            target_pool.push(target_x.clone(),core.to_string());
            bench_pool.push(bench_x.clone(),core.to_string());
            tx.send(true);


        }

        pb.finish();

        if target_alive {
            let sum_nrg: f64 = nrg_vec.iter().sum();
            let avg_nrg = sum_nrg / self.xml_reader.ann_num_iter() as f64;
            match self.xml_reader.ann_energy() {
                EnergyType::throughput => {
                    println!("Thread [{}] {} {:.4} KB/s",
                             core,
                             Red.paint("====> Evaluated Avg. Response Rate: "),
                             avg_nrg);
                }
                EnergyType::latency => {
                    println!("Thread [{}] {} {:.4} ms",
                             core,
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



    /// *********************************************************************************************************
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


    /*fn create_ssh_tunnel(&self,core: usize){
    	
    	let host=self.xml_reader.targ_host();
    	let temp_host_vec: Vec<&str>=host.split(":").collect();
    	let remote_host_addr=temp_host_vec[0];
    	let remote_host_port=temp_host_vec[1];
				
		let host_4_cmd=format!("{}@{}",self.xml_reader.targ_host_user(),remote_host_addr);
		let addr_2_tunnel=format!("{}:localhost:{}",(base_target_port+core).to_string().as_str(),(base_target_port+core).to_string().as_str());
		
		let mut cmd_sshtunnel=Command::new("ssh")
							    .arg("-f")
							    .arg(host_4_cmd)
							    .arg("-p")
							    .arg(remote_host_port)
							    .arg("-L")
							    .arg(addr_2_tunnel)
							    .arg("-N")
							    .spawn()
							    .expect("ls command failed to start");
	            	
		    	
    }*/
}

/// Load the CpuSet for the given core index.
fn cpuset_for_core(topology: &Topology, idx: usize) -> CpuSet {
    let cores = (*topology).objects_with_type(&ObjectType::Core).unwrap();
    match cores.get(idx) {
        Some(val) => val.cpuset().unwrap(),
        None => panic!("No Core found with id {}", idx),
    }
}
