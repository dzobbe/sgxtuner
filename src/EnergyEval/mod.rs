use time;
use libc;
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
use std::net::{TcpStream, Shutdown};

#[derive(Clone)]
pub struct EnergyEval {
    pub target_path: String,
    pub bench_path: String,
    pub target_args: String,
    pub bench_args: Vec<String>,
}

// Number of iteration with which re-execute the same parameter configuration
const num_iterations: u8 = 1;

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
                                 params: &HashMap<String, u32>,
                                 energy_type: EnergyType)
                                 -> Option<f64> {

        let perf_metrics_handler = PerfCounters::PerfMetrics::new();
        let target_port = 12347;
        let mut target_alive: bool = false;

        /// Set the environement variables that will configure the parameters
        /// needed by the target application
        ///
        for (param_name, param_value) in params.iter() {
            env::set_var(param_name.to_string(), param_value.to_string());
        }


        // Repeat the execution 10 times for accurate results
        let mut nrg_vec = Vec::with_capacity(num_iterations as usize);
        for i in 0..num_iterations {

            println!("{} {:?}", Green.paint("====> Evaluation of: "), params);
            println!("{} {}",
                     Green.paint("====> Iteration Number for current Configuration: "),
                     i + 1);


            /// **
            /// Launch TARGET Application
            /// *
            let vec_args: Vec<&str> = self.target_args.split_whitespace().collect();
            let mut target_process = Some(Command::new(self.target_path.clone())
                .args(vec_args.as_ref())
                .stdout(Stdio::piped())
                .spawn()
                .expect("Failed to execute Target!"));
            let pid_target = target_process.as_mut().unwrap().id();

            // Wait for target to startup
            thread::sleep(Duration::from_millis(2000));

            // Realize one fake connection to check if the target is alive
            // It can happen that the configuration of parameters does not allow to start the target.
            // In that case the energy returned by this function is None
            target_alive = match TcpStream::connect(("127.0.0.1", target_port)) {
                Err(e) => {
                    println!("{} The Target Application seems down. Maybe a bad configuration: {}",
                             Red.paint("*****ERROR***** --> "),
                             e);
                    target_process.as_mut().unwrap().kill().expect("Target Process wasn't running");
                    false
                }
                Ok(s) => {
                    s.shutdown(Shutdown::Both);
                    drop(s);
                    true
                }
            };

            if target_alive == false {
                break;
            }


            /// **
            /// Start METER-PROXY, which will interpose between the Target and the
            /// Benchmark apps to extract metrics for the energy evaluation
            /// *
            let meter_proxy = MeterProxy::Meter::new(target_port);
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
                let mut bench_process = Some(Command::new(cloned_self.bench_path.clone())
                    .args(cloned_self.bench_args.as_ref())
                    .stdout(Stdio::piped())
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
                    let num_bytes = meter_proxy_c.get_num_bytes_rcvd() as f64;
                    let resp_rate = (num_bytes / elapsed_time) / 1024.0;
                    println!("{} {:.4} KB/s",
                             Red.paint("====> Response Rate: "),
                             resp_rate);
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


        if target_alive {
            let sum_nrg: f64 = nrg_vec.iter().sum();
            let avg_nrg = sum_nrg / num_iterations as f64;
            match energy_type {
                EnergyType::throughput => {
                    println!("{} {:.3} KB/s",
                             Red.paint("====> Avg. Response Rate: "),
                             avg_nrg);//Red.paint("Std. Dev.: "),std_dev);
                }
                EnergyType::latency => {
                    println!("{} {:.3} s", Red.paint("Avg. Latency: "), avg_nrg);//Red.paint("Std. Dev.: "),std_dev);
                }
            };
            println!("Execution Terminated!!");
            println!("{}",Yellow.paint("==================================================================================================================="));

            return Some(avg_nrg);
        } else {

            return None;
        }

        // println!("Latency {:?} ms", meter_proxy_c.get_latency_ms());
        // meter_proxy_c.print();
    }
}

impl Default for EnergyEval {
    fn default() -> EnergyEval {
        EnergyEval {
            target_path: "".to_string(),
            bench_path: "".to_string(),
            target_args: "".to_string(),
            bench_args: Vec::new(),
        }
    }
}
