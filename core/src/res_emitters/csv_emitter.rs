use csv;
use states_gen;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::io::{BufWriter, BufReader, BufRead};
use std::collections::HashMap;

use State;

#[derive(Debug, Clone)]
pub struct CSVEmitter {
    pub ordered_params: Vec<String>,
}


impl CSVEmitter {
    pub fn new(num_targets: usize, params_name: Vec<String>) -> Self {
        let mut temp_vec: Vec<String> = Vec::new();

        for i in 0..num_targets {
            let filename = format!("{}{}{}", "results-", i, ".csv");
            let f = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(filename);

            let mut writer = BufWriter::new(f.unwrap());
            let mut wtr = csv::Writer::from_buffer(writer);

            // Create a path to the params file
            for param_name in params_name.iter() {
                temp_vec.push(param_name.to_string());
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
        }

        CSVEmitter { ordered_params: temp_vec }
    }


    pub fn send_update(
        &mut self,
        temperature: f64,
        time: f64,
        cputime: f64,
        measured_val: f64,
        measured_state: &State,
        best_val: f64,
        best_state: &State,
        num_iter: usize,
        tid: usize,
    ) {

        let filename = format!("{}{}{}", "results-", tid, ".csv");
        let f = OpenOptions::new().write(true).open(filename);

        let mut writer = BufWriter::new(f.unwrap());
        let mut wtr = csv::Writer::from_buffer(writer);


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

        let result = wtr.encode(vec_2_write);
        assert!(result.is_ok());

        wtr.flush();
    }
}
