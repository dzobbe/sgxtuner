
use rand;
use std::error::Error;
use std::io::prelude::*;
use std::path::Path;
use std::collections::{HashMap, HashSet};
use rand::Rng;
use ansi_term::Colour::{Yellow, Red};
use parsers::xml_parser::XMLReader;
use State;
use shared::{TunerParameter, IntParameter, BoolParameter};


#[derive(Clone, Debug)]
pub struct ParamsConfigurator {
    pub targ_int_params: Vec<IntParameter>,
    pub targ_bool_params: Vec<BoolParameter>,
    int_params_space_state: HashMap<String, Vec<usize>>,
    bool_params_space_state: HashMap<String, Vec<String>>,
}

static initial_decreasing_factor: f64 = 0.6;

impl ParamsConfigurator {
    pub fn new(ip: Vec<IntParameter>, bp: Vec<BoolParameter>) -> ParamsConfigurator {

        //Define the integer parameters space state
        let mut int_params_ss: HashMap<String, Vec<usize>> = HashMap::new();
        for int_param in ip.iter() {
            int_params_ss.insert(
                int_param.clone().name,
                ParamsConfigurator::generate_int_space_state(
                    int_param.min,
                    int_param.max,
                    int_param.step,
                ),
            );
        }

        //Define the bool parameters space state
        let mut bool_params_ss: HashMap<String, Vec<String>> = HashMap::new();
        for bool_param in bp.iter() {
            let mut temp_vec: Vec<String> = Vec::new();
            temp_vec.push(bool_param.clone().true_val);
            temp_vec.push(bool_param.clone().false_val);
            bool_params_ss.insert(bool_param.clone().name, temp_vec);
        }

        ParamsConfigurator {
            targ_int_params: ip,
            targ_bool_params: bp,
            int_params_space_state: int_params_ss,
            bool_params_space_state: bool_params_ss,
        }
    }

    pub fn get_initial_param_conf(&mut self) -> State {
        let mut initial_params_state: State = HashMap::new();

        for int_param in self.targ_int_params.iter() {
            initial_params_state.insert(int_param.clone().name, int_param.default.to_string());

            println!(
                "{} {:?} - {} {:?}",
                Yellow.paint("Parameter ==> "),
                int_param.name,
                Yellow.paint("Default Value ==> "),
                int_param.default
            );

        }

        for bool_param in self.targ_bool_params.iter() {

            if bool_param.default == true {
                initial_params_state.insert(bool_param.clone().name, bool_param.clone().true_val);
            } else {
                initial_params_state.insert(bool_param.clone().name, bool_param.clone().false_val);
            }

            println!(
                "{} {:?} - {} {:?}",
                Yellow.paint("Parameter ==> "),
                bool_param.name,
                Yellow.paint("Default Value ==> "),
                bool_param.default
            );
        }


        println!("{}",Red.paint("*******************************************************************************************************************"));

        return initial_params_state;
    }


    pub fn get_params_name(&self) -> Vec<String> {
        let mut params_name: Vec<String> = Vec::new();

        for int_param in self.targ_int_params.iter() {
            params_name.push(int_param.clone().name);
        }

        for bool_param in self.targ_bool_params.iter() {
            params_name.push(bool_param.clone().name);
        }
        return params_name;
    }

    /***
	Private function useful to generate the whole space state for each integer parameter based on the [min:max:step] values
	given in input by the user.
	***/
    fn generate_int_space_state(lbound: usize, ubound: usize, step: usize) -> Vec<usize> {
        let mut res_vec = Vec::new();
        let num_it = (ubound - lbound) / step;
        for x in 0..num_it {
            res_vec.push(lbound + (step * x));
            if x == num_it - 1 {
                res_vec.push(lbound + (step * (x + 1)));
            }
        }
        // Randomize the order of vector elements
        rand::thread_rng().shuffle(&mut res_vec);
        return res_vec;
    }


    pub fn get_neigh_one_varying(&mut self, current_state: &State) -> Vec<State> {

        let mut neighborhoods: Vec<State> = Vec::new();
        neighborhoods.clear();


        for (param_name, space_state_vec) in self.int_params_space_state.iter() {

            for param_values in space_state_vec.iter() {
                let mut temp = current_state.clone();
                *(temp).get_mut(param_name).unwrap() = (*param_values).to_string();
                neighborhoods.push(temp);
            }
        }

        for (param_name, space_state_vec) in self.bool_params_space_state.iter() {

            for param_values in space_state_vec.iter() {
                let mut temp = current_state.clone();
                *(temp).get_mut(param_name).unwrap() = (*param_values.clone()).to_string();
                neighborhoods.push(temp);
            }
        }

        println!(
            "Created the vector of Neighborhoods, composed by {:?} parameter configurations",
            neighborhoods.len()
        );

        return neighborhoods;
    }




    /***
	Function that returns a neighborhood of the state given in input. The Neighborhood evaluation is performed in 
	an adaptive way. At the beginning of the Annealing the space of Neighborhoods will be large (60% of the parameters will vary).
	Then, the more the number of steps executed increase, the more the Neighborhood space gets smaller.   
	***/
    pub fn get_neighborhood(
        &mut self,
        params_state: &State,
        max_anneal_steps: usize,
        current_anneal_step: usize,
    ) -> State {


        // Evaluate the coefficient with which decrease the size of neighborhood selection (the number of parameters to vary). The factor will
        // decrease every period_of_variation. The initial value of the factor has been set to 0.6. Therefore,
        // 60% of the parameters will vary at the beginning and then such a value will decrease of 10% every period
        let period_of_variation: f64 = max_anneal_steps as f64 /
            ((initial_decreasing_factor as f64) * 10.0);
        let decreasing_factor: f64 = initial_decreasing_factor -
            ((current_anneal_step as f64 / period_of_variation).floor()) / 10.0;
        // Evaluate the number of varying parameters based on factor evaluated before
        let mut num_params_2_vary = (params_state.len() as f64 * decreasing_factor) as usize;


        let mut new_params_state: State = HashMap::new();


        //Iterates over the parameters of the current state
        for (param_name, param_current_value) in params_state.iter() {

            //The current parameter is of integer type. Select a random value from the parameter space state
            if self.int_params_space_state.contains_key(param_name) {
                if num_params_2_vary > 0 {
                    let param_space_state = self.int_params_space_state.get(param_name).unwrap();

                    // If there are values that can be changed take
                    let new_value = rand::thread_rng().choose(&param_space_state).unwrap();
                    new_params_state.insert(
                        param_name.clone().to_string(),
                        (*new_value).to_string(),
                    );
                    num_params_2_vary -= 1;
                } else {
                    new_params_state.insert(
                        param_name.clone().to_string(),
                        (*param_current_value.clone()).to_string(),
                    );
                }
            //The current parameter is of bool type. Select a random value from the parameter space state
            } else {
                if num_params_2_vary > 0 {
                    let param_space_state = self.bool_params_space_state.get(param_name).unwrap();

                    // If there are values that can be changed take
                    let new_value = rand::thread_rng().choose(&param_space_state).unwrap();
                    new_params_state.insert(
                        param_name.clone().to_string(),
                        (*new_value.clone()).to_string(),
                    );
                    num_params_2_vary -= 1;
                } else {
                    new_params_state.insert(
                        param_name.clone().to_string(),
                        (*param_current_value.clone()).to_string(),
                    );
                }
            }


        }

        return new_params_state;
    }


    /***
	Function that returns a random state
	***/
    pub fn get_rand_param(&mut self) -> State {

        let mut new_params_state: State = HashMap::new();

        // The HashMap iterator provides (key,value) pair in a random order
        for int_param_name in self.int_params_space_state.keys() {

            let param_space_state = self.int_params_space_state.get(int_param_name).unwrap();
            let new_value = rand::thread_rng().choose(&param_space_state).unwrap();
            new_params_state.insert(int_param_name.clone().to_string(), (*new_value).to_string());
        }

        // The HashMap iterator provides (key,value) pair in a random order
        for bool_param_name in self.bool_params_space_state.keys() {

            let param_space_state = self.bool_params_space_state.get(bool_param_name).unwrap();
            let new_value = rand::thread_rng().choose(&param_space_state).unwrap();
            new_params_state.insert(
                bool_param_name.clone().to_string(),
                (*new_value.clone()).to_string(),
            );
        }


        return new_params_state;
    }



    /***
	Functions useful for the hybrid annealing-genetic algorithm
	***/
    pub fn get_rand_population(&mut self, size: usize) -> Vec<State> {
        let mut res_vec = Vec::with_capacity(size);
        for i in 0..size {
            res_vec.push(self.get_rand_param());
        }
        return res_vec;
    }
}
