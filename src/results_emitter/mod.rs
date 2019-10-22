/*
use influent::create_client;
use influent::client::{Client, Credentials};
use influent::measurement::{Measurement, Value};*/

use csv;
use states_gen;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::io::{BufWriter, BufReader, BufRead};
use std::collections::HashMap;
use State;
use xml_reader;
use xml_reader::XMLReader;

pub struct Emitter2File {
    csv_writer: csv::Writer<File>,
    ordered_params: Vec<String>,
}

pub struct Emitter2Influx {
    host: String,
    port: usize, 
   // client: Client,
}




pub trait Emitter {
    fn new(id: String) -> Self;
    fn send_update(&mut self,
                   temperature: f64,
                   time: f64,
                   cputime: f64,
                   measured_val: f64,
                   measured_state: &State,
                   best_val: f64,
                   best_state: &State,
                   num_iter: usize);
}



// impl Emitter for Emitter2Influx {
// fn new(h: String, p: u16, user: String, pwd: String, db: String) -> Emitter2Influx {
//
// prepare client
// let credentials = Credentials {
// username: user,
// password: pwd,
// database: db
// };
//
// let addr="http://".to_string() + h + p;
// let hosts = vec![addr];
//
// let c = create_client(credentials, hosts);
//
// Emitter2Influx {
// client:   c,
// host:	  h,
// port:	  p,
// }
//
// }
//
// fn send_update(&mut self,
// measured_val: f64,
// measured_state: &State,
// best_val: f64,
// best_state: &State,
// num_iter: u64) {
//
// prepare measurement
// let mut measurement = Measurement::new("measured_nrg");
// measurement.add_field("some_field", Value::String("hello"));
// measurement.add_tag("some_region", "Moscow");
//
// self.client.write_one(measurement, Some(measured_val));
//
// prepare measurement
// let mut measurement = Measurement::new("best_nrg");
// self.client.write_one(measurement, Some(best_val));
//
// }
// }



impl Emitter for Emitter2File {
    fn new(id: String) -> Self {
        let mut temp_vec: Vec<String> = Vec::new();

		let filename=format!("{}{}{}",
                              "results-",
                              id.as_str(),
                              ".csv");
        let f = OpenOptions::new().write(true).create(true).truncate(true).open(filename);

        let mut writer = BufWriter::new(f.unwrap());
        let mut wtr = csv::Writer::from_buffer(writer);

		let xml_reader = xml_reader::XMLReader::new("conf.xml".to_string());
        // Create a path to the params file
        for param in xml_reader.get_musl_params() {
            temp_vec.push(param.name);
        }

        let mut vec_2_write: Vec<String> = Vec::new();
        vec_2_write.push("time_s".to_string());
        vec_2_write.push("cputime_s".to_string());
        vec_2_write.push("temperature".to_string());
        vec_2_write.push("best_nrg".to_string());
        for param_name in temp_vec.clone().iter().cloned() {
            vec_2_write.push("best_".to_string() + &*param_name);
        }

        vec_2_write.push("last_nrg".to_string());
        for param_name in temp_vec.clone().iter().cloned() {
            vec_2_write.push("last_".to_string() + &*param_name);
        }

        let res = wtr.encode(vec_2_write);
        assert!(res.is_ok());

        wtr.flush();

        Emitter2File {
            csv_writer: wtr,
            ordered_params: temp_vec,
        }
    }


    fn send_update(&mut self,
                   temperature: f64,
                   time: f64,
                   cputime: f64,
                   measured_val: f64,
                   measured_state: &State,
                   best_val: f64,
                   best_state: &State,
                   num_iter: usize) {

        let mut vec_2_write: Vec<String> = Vec::new();

        vec_2_write.push(time.to_string());
        vec_2_write.push(cputime.to_string());
        vec_2_write.push(temperature.to_string());

        vec_2_write.push(best_val.to_string());
        for param_name in self.ordered_params.clone().iter().cloned() {
            vec_2_write.push((best_state.get(&param_name).unwrap()).to_string());
        }


        vec_2_write.push(measured_val.to_string());

        for param_name in self.ordered_params.clone().iter().cloned() {
            vec_2_write.push((measured_state.get(&param_name).unwrap()).to_string());
        }

        let result = self.csv_writer.encode(vec_2_write);
        assert!(result.is_ok());

        self.csv_writer.flush();
    }
}