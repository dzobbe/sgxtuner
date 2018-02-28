extern crate zmq;
extern crate lazy_static;

pub mod output_parser;

use std::env;
use output_parser::Parser;
use std::process::{Command, Stdio};



fn main() {

    let mut str_address = String::new();
    match env::var("OWN_ADDRESS") {
        Ok(v) => str_address = v,
        Err(e) => println!("Couldn't read OWN_ADDRESS ({})", e),
    };

    let mut str_bin_path = String::new();
    match env::var("BIN_PATH") {
        Ok(v) => str_bin_path = v,
        Err(e) => println!("Couldn't read BIN_PATH ({})", e),
    };

    let mut str_bin_args = String::new();
    match env::var("BIN_ARGS") {
        Ok(v) => str_bin_args = v,
        Err(e) => println!("Couldn't read BIN_ARGS ({})", e),
    };


    let mut str_bench_type = String::new();
    match env::var("BENCH_TYPE") {
        Ok(v) => str_bench_type = v,
        Err(e) => println!("Couldn't read BENCH_TYPE ({})", e),
    };

    let bench_type: BenchmarkName = str_bench_type.parse().unwrap();

    let parser = output_parser::Parser { benchmark_name: bench_type };



    println!("Starting Bench Agent!");

    let ctx = zmq::Context::new();

    let rep_socket = ctx.socket(zmq::REP).unwrap();

    let ip_address = format!("tcp://{}", str_address);

    rep_socket.bind(ip_address.as_str()).unwrap();


    loop {

        let msg = rep_socket.recv_string(0).unwrap().unwrap();



        if msg == "start_bench" {
            //Start the benchmark if the master asked so
            println!("Received START for Bench!");

            rep_socket.send("45.6", 0).unwrap();

            /*match execute_bench(str_bin_path.clone(), str_bin_args.clone(), parser.clone()) {
                Some(r) => {
                    rep_socket.send(r.to_string().as_str(), 0).unwrap();
                }
                None => rep_socket.send("None", 0).unwrap(),
            }*/

        }

    }
}

fn execute_bench(bench_bin_path: String, bench_args: String, parser: Parser) -> Option<f64> {

    let bench_args: Vec<&str> = bench_args.split_whitespace().collect();

    let bench_process = Command::new(bench_bin_path)
        .args(bench_args)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute Benchmark!");

    let pid = bench_process.id();


    let output = bench_process.wait_with_output().expect(
        "Failed to wait on Benchmark",
    );


    //Extract result of energy
    let meas_nrg = parser.parse(output);


    return meas_nrg;

}

#[derive(Debug, Clone)]
pub enum BenchmarkName {
    Wrk,
    Ycsb,
    Memaslap,
}

impl std::str::FromStr for BenchmarkName {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "wrk" => Ok(BenchmarkName::Wrk),
            "ycsb" => Ok(BenchmarkName::Ycsb),
            "memaslap" => Ok(BenchmarkName::Memaslap),
            _ => Err("Benchmark Name - not a valid value"),
        }
    }
}
