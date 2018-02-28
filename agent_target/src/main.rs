extern crate zmq;
extern crate xml;


use std::env;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::process::{Command, Stdio};


mod xml_parser;
mod conf_handler;
mod shared;

fn main() {

    let mut str_address = String::new();
    match env::var("OWN_ADDRESS") {
        Ok(v) => str_address = v,
        Err(e) => println!("Couldn't read OWN_ADDRESS ({})", e),
    };

    let mut service_conf_dir = String::new();
    match env::var("SERVICE_CONF_DIR") {
        Ok(v) => service_conf_dir = v,
        Err(e) => println!("Couldn't read SERVICE_CONF_DIR ({})", e),
    };

    println!("Target Agent Started!");

    let xml_reader = xml_parser::XMLReader::new("../conf.xml".to_string());
    let hm_params_level = xml_reader.get_target_hm_service_level();
    let script_info = xml_reader.get_script_info();

    let mut envfile_writer = conf_handler::ConfWriter::new(script_info.clone().envfile);


    let ctx = zmq::Context::new();
    let rep_socket = ctx.socket(zmq::REP).unwrap();


    rep_socket
        .bind(format!("tcp://{}", str_address).as_str())
        .unwrap();


    loop {

        let msg = rep_socket.recv_string(0).unwrap().unwrap();
        let (stop_tx, stop_rx) = channel::<bool>();

        //Split the message received: START|PARAM_NAME_1=VALUE|PARAM_NAME_2=VALUE|...
        let mut msg_sequence: Vec<&str> = msg.split('|').collect();


        if msg_sequence.remove(0) == "start_target" {
            //Start the benchmark if the master asked so
            println!("Received START for Target!");

            for param in msg_sequence.iter() {
                let splitted_param: Vec<&str> = param.split('=').collect();
                match hm_params_level.get(splitted_param[0]) {
                    Some(service_level) => {
                        match *service_level {
                            ParameterLevel::Runtime => {
                                envfile_writer.push_line(format!("{}{}", param, "\n"));
                            }
                            ParameterLevel::ServiceConfig => {
                                conf_handler::search_and_write(
                                    service_conf_dir.clone(),
                                    splitted_param[0].to_string(),
                                    splitted_param[1].to_string(),
                                );
                            }
                            ParameterLevel::Compile => {
                                envfile_writer.push_line(format!("{}{}", param, "\n"));
                            }
                        };
                    }
                    None => {}
                }


            }

            envfile_writer.flush_write();

            rep_socket.send("target_ok", 0).unwrap();
            //Start Target Process
            /*match execute_target(
                script_info.clone().name,
                format!("{} {}", script_info.fulltag, script_info.envfile),
                stop_rx,
            ) {
                Ok(_) => rep_socket.send("target_ok", 0).unwrap(),
                Err(_) => rep_socket.send("error", 0).unwrap(),
            }*/


        }

        if msg.as_str() == "stop_target" {
            //Start the benchmark if the master asked so
            println!("Received STOP for Target!");
            stop_tx.send(true);
            rep_socket.send("stop_target_ok", 0).unwrap();
        }
    }
}


fn execute_target(
    target_bin_path: String,
    target_args: String,
    signal_ch: Receiver<bool>,
) -> Result<&'static str, &'static str> {

    let (status_tx, status_rx) = channel::<bool>();

    let status_tx_c = status_tx.clone();

    thread::spawn(move || {

        let mut command_2_launch = Command::new(target_bin_path);


        let vec_args: Vec<&str> = target_args.split_whitespace().collect();
        let mut target_process = match command_2_launch
            .args(vec_args)
            .stdout(Stdio::piped())
            .spawn() {
            Ok(v) => {
                status_tx_c.send(true);
                v
            }
            Err(_) => {
                status_tx_c.send(false);
                return;
            }
        };


        signal_ch.recv();

        println!("Killing target process");

        target_process.kill().expect(
            "Target Process wasn't running",
        );
    });

    let res = status_rx.recv().unwrap();
    if res == true {
        return Ok("ok");
    } else {
        return Err("failed");
    }

}



#[derive(Debug, Clone)]
pub enum ParameterLevel {
    Runtime,
    ServiceConfig,
    Compile,
}


impl std::str::FromStr for ParameterLevel {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "runtime" => Ok(ParameterLevel::Runtime),
            "service-config" => Ok(ParameterLevel::ServiceConfig),
            "compile" => Ok(ParameterLevel::Compile),
            _ => Err("Parameter Level - not a valid value"),
        }
    }
}
