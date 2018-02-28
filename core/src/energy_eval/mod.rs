use time;
use pbr;
use pbr::{ProgressBar, MultiBar};
use std::str;
use std::process::{Command, Child, Stdio};
use std::{thread, env};
use std::collections::HashMap;
use ansi_term::Colour::{Red, Yellow, Green};
use EnergyType;
use ExecutionType;
use State;
use shared::{TunerParameter, ProcessPool, ScriptInfo};
use ctrlc;
use zmq;
use std::process;
use std::time::{Duration, Instant};
use std::sync::mpsc::{Receiver, channel};


#[derive(Clone, Debug)]
pub struct EnergyEval {
    targets_pool: ProcessPool,
    benchs_pool: ProcessPool,
    tuner_params: TunerParameter,
}


static mut notified: bool = false;
static mut counter: u16 = 0;



impl EnergyEval {
    pub fn new(targets: Vec<String>, benchs: Vec<String>, tuner_params: TunerParameter) -> Self {
        let t_pool: ProcessPool = ProcessPool::new();
        for targ in targets.iter() {
            t_pool.push(targ.to_string());
        }

        let b_pool: ProcessPool = ProcessPool::new();
        for bench in benchs.iter() {
            b_pool.push(bench.to_string());
        }

        EnergyEval {
            targets_pool: t_pool,
            benchs_pool: b_pool,
            tuner_params: tuner_params,
        }
    }

    /***
	Execute an an instance of the benchmark on the target application for the specific
	configuration of parameters. The function returns the cost result (in this case the response throughput)
	that will be used by the simulated annealing algorithm for the energy evaluation
	***/

    pub fn execute_test_instance(&mut self, params: &State, tid: usize) -> Option<f64> {

        //Init ZMQ context
        let mut msg = zmq::Message::new();
        let mut zmq_ctx = zmq::Context::new();


        //Extract a target from the pool and connect to it
        let req_socket_targ = zmq_ctx.socket(zmq::REQ).unwrap();
        let targ = self.targets_pool.pop();
        req_socket_targ
            .connect(format!("tcp://{}", targ).as_str())
            .unwrap();

        //Extract a bench from the pool and connect to it
        let req_socket_bench = zmq_ctx.socket(zmq::REQ).unwrap();
        let bench = self.benchs_pool.pop();
        req_socket_bench
            .connect(format!("tcp://{}", bench).as_str())
            .unwrap();



        let mut valid_result: bool = false;

        // Repeat the execution num_iter times for accurate results
        let mut nrg_vec = Vec::with_capacity(self.tuner_params.num_iter as usize);
        println!(
            "{} TID [{}] - Evaluation of: {:?}",
            Green.paint("====>"),
            tid,
            params
        );
        println!(
            "{} Waiting for {} iterations to complete",
            Green.paint("====>"),
            self.tuner_params.num_iter
        );

        let mut pb = ProgressBar::new(self.tuner_params.num_iter as u64);
        pb.format("╢▌▌░╟");
        pb.show_message = true;
        pb.message(&format!("Thread [{}] - ", tid));


        let mut measured_nrg: f64 = 0.0;

        for i in 0..self.tuner_params.num_iter {
            pb.inc();

            let (stop_tx, stop_rx) = channel::<bool>();
            let stop_tx_c = stop_tx.clone();

            self.set_stop_handler(targ.clone(), bench.clone(), stop_rx);

            ctrlc::set_handler(move || {
                stop_tx_c.send(true);
                thread::sleep(Duration::from_secs(1));
                process::exit(0);
            });

            /***********************************************************************************************************
            /// **
            /// Launch Target and Benchmark Applications by sending messages to related agents
            /// *
            	************************************************************************************************************/
            let mut params_sequence = String::new();
            for (name, value) in params.iter() {
                params_sequence += format!("{}={}|", name, value).as_str();
            }


            req_socket_targ
                .send(format!("start_target|{}", params_sequence).as_str(), 0)
                .unwrap();

            req_socket_targ.recv(&mut msg, 0).unwrap();

            let start_time = time::precise_time_ns();

            if msg.as_str().unwrap() == "target_ok" {
                //Target correctly started, we can start the Benchmark now
                req_socket_bench.send("start_bench", 0).unwrap();

                req_socket_bench.recv(&mut msg, 0).unwrap();
                let result_meas = msg.as_str().unwrap();

                if result_meas == "None" {
                    //The benchmark was not able to get a result
                    valid_result = false;
                    stop_tx.clone().send(true);
                    println!("Not a valid target configuration");
                } else {
                    valid_result = true;
                    measured_nrg = result_meas.parse::<f64>().unwrap();
                    nrg_vec.push(measured_nrg);
                    println!("Received from bench {:?}", measured_nrg);
                }

            } else {
                valid_result = false;
                break;
            }


            let end_time = time::precise_time_ns();
            let elapsed_ns: f64 = (end_time - start_time) as f64;
            let elapsed_time = elapsed_ns / 1000000000.0f64;


            if measured_nrg == 0.0 {
                valid_result = false;
            }




            /************************************************************************************************************
            /// **
            /// Clean Resources
            /// *
             *************************************************************************************************************/

            self.targets_pool.push(targ.clone().to_string());
            self.benchs_pool.push(bench.clone().to_string());
            stop_tx.clone().send(true);

        }

        pb.finish();

        if valid_result {
            let sum_nrg: f64 = nrg_vec.iter().sum();
            let avg_nrg = sum_nrg / self.tuner_params.num_iter as f64;
            match self.tuner_params.energy {
                EnergyType::throughput => {
                    println!(
                        "Thread [{}] {} {:.4} Ops/s",
                        tid,
                        Red.paint("====> Evaluated Avg. Response Rate: "),
                        avg_nrg
                    );
                }
                EnergyType::latency => {
                    println!(
                        "Thread [{}] {} {:.4} ms",
                        tid,
                        Red.paint("====> Evaluated Avg. Latency: "),
                        avg_nrg
                    );
                }
            };
            println!("{}",Yellow.paint("==================================================================================================================="));

            return Some(avg_nrg);
        } else {
            return None;
        }

    }

    fn set_stop_handler(&mut self, target: String, bench: String, stop_rx: Receiver<bool>) {


        let self_c = self.clone();
        thread::spawn(move || {

            stop_rx.recv();

            println!("Stopping the Tuner!");
            let mut zmq_ctx = zmq::Context::new();

            let req_socket_targ = zmq_ctx.socket(zmq::REQ).unwrap();
            req_socket_targ
                .connect(format!("tcp://{}", target).as_str())
                .unwrap();

            let req_socket_bench = zmq_ctx.socket(zmq::REQ).unwrap();
            req_socket_bench
                .connect(format!("tcp://{}", bench).as_str())
                .unwrap();
            //req_socket_bench.send("stop_bench", 0).unwrap();
            req_socket_targ.send("stop_target", 0).unwrap();

        });



    }
}
