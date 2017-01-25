
use rand;
use std::io::BufReader;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::collections::{HashMap, HashSet};
use rand::Rng;
use ansi_term::Colour::{Yellow, Red};
use std::boxed::Box;
use State;

#[derive(Clone,Debug,RustcEncodable)]
pub struct ParamsConfigurator {
    pub default_param: State,
    // Path of the file where the parameters configuration is
    pub param_file_path: String,
    // HashMap that stores the space state of each parameter
    pub params_space_state: HashMap<String, Vec<usize>>,
    // Indexes of parameters. It is needed to have an order of the parameters
    // for the insertion of new states into the visited_params_state.
    pub params_indexes: HashMap<String, u8>,
    // Visited parameters list. Saved in heap for memory space reasons
    pub visited_params_states: Box<HashSet<String>>,
}

static initial_decreasing_factor: f64 = 0.6;

impl ParamsConfigurator {
    pub fn new(file_path: String) -> ParamsConfigurator {
        let mut params_configurator = ParamsConfigurator {
            default_param: HashMap::new(),
            param_file_path: file_path,
            params_space_state: HashMap::new(),
            params_indexes: HashMap::new(),
            visited_params_states: Box::new(HashSet::new()),
        };
        params_configurator.init();

        params_configurator
    }

    pub fn get_initial_param_conf(&mut self) -> State {
        return self.clone().default_param;
    }


    /**
	Access the initial-params.conf file and extract the info on parameters to tune
	It returns the initial params state given in input by the user
	**/
    pub fn init(&mut self) {

        let f = self.param_file_path.clone();
        // Create a path to the desired file
        let path = Path::new(&f);
        let display = path.display();

        // Open the path in read-only mode, returns `io::Result<File>`
        let file = match File::open(&path) {
            Err(why) => panic!("couldn't open {}: {}", display, why.description()),
            Ok(file) => file,
        };


        let mut initial_params_state: State = HashMap::new();
        let file_reader = BufReader::new(&file);
        let mut index = 0;
        for (_, line) in file_reader.lines().enumerate() {
            let topline = line.unwrap();
            let mut topsplit = topline.split(":");

            let (var_name, var_value, var_lbound, var_ubound, var_step);

            match topsplit.next() {
                Some(x) => var_name = x,
                None => break,
            }

            match topsplit.next() {
                Some(subline) => {
                    let mut subsplit = subline.split(",");
                    match subsplit.next() {
                        Some(x) => var_lbound = str::replace(x, "[", ""),
                        None => break,
                    }
                    match subsplit.next() {
                        Some(x) => var_ubound = x,
                        None => break,
                    }
                    match subsplit.next() {
                        Some(x) => var_step = str::replace(x, "]", ""),
                        None => break,		                		
                    }
                } 
                None => break,
            }

            match topsplit.next() {
                Some(x) => var_value = x,
                None => break,
            }



            let space_state_elems =
                ParamsConfigurator::get_space_state(var_lbound.parse::<usize>().unwrap(),
                                                    var_ubound.parse::<usize>().unwrap(),
                                                    var_step.parse::<usize>().unwrap());
            let space_state_elems_c = space_state_elems.clone();

            self.params_space_state
                .insert(var_name.to_string(), space_state_elems);
            self.params_indexes.insert(var_name.to_string(), index);
            index = index + 1;

            initial_params_state.insert(var_name.to_string(), var_value.parse::<usize>().unwrap());


            println!("{} {:?}", Yellow.paint("Input Parameter ==> "), var_name);

            println!("{} [{:?},{:?},{:?}] - {} {:?} ",Yellow.paint("Space State ==> "),
                     var_lbound,
                     var_ubound,
                     var_step,
                     Yellow.paint("Default Value ==> "),
                     var_value,
                     );
            println!("{} {:?}",
                     Yellow.paint("Elements ==> "),
                     space_state_elems_c);

            println!("{}",Red.paint("*******************************************************************************************************************"));

        }

        self.default_param = initial_params_state.clone();

    }


    /**
	Private function useful to generate the whole space state for each parameter based on the [min:max:step] values
	given in input by the user.
	**/
    fn get_space_state(lbound: usize, ubound: usize, step: usize) -> Vec<usize> {
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

        for (param_name, space_state_vec) in self.params_space_state.iter() {

            // let num_varying_params=(space_state_vec.len() as f64 *0.7) as usize;
            for param_values in space_state_vec.iter() {
                let mut temp = current_state.clone();
                *(temp).get_mut(param_name).unwrap() = *param_values;
                neighborhoods.push(temp);
            }
        }
        println!("Created the vector of Neighborhoods, composed by {:?} parameter configurations",
                 neighborhoods.len());
        return neighborhoods;
    }




    /**
	Function that returns a neighborhood of the state given in input. The Neighborhood evaluation is performed in 
	an adaptive way. At the beginning of the Annealing the space of Neighborhoods will be large (60% of the parameters will vary).
	Then, the more the number of steps executed increase, the more the Neighborhood space gets smaller.   
	**/
    pub fn get_neighborhood(&mut self,
                            params_state: &State,
                            max_anneal_steps: usize,
                            current_anneal_step: usize)
                            -> Option<State> {


        // Evaluate the coefficient with which decrease the size of neighborhood selection (the number of parameters to vary). The factor will
        // decrease every period_of_variation. The initial value of the factor has been set to 0.6. Therefore,
        // 60% of the parameters will vary at the beginning and then such a value will decrease of 10% every period
        let period_of_variation: f64 = max_anneal_steps as f64 /
                                       ((initial_decreasing_factor as f64) * 10.0);
        let decreasing_factor: f64 = initial_decreasing_factor -
                                     ((current_anneal_step as f64 / period_of_variation).floor()) /
                                     10.0;
        // Evaluate the number of varying parameters based on factor evaluated before
        let mut num_params_2_vary = (params_state.len() as f64 * decreasing_factor) as usize;


        let mut new_params_state: State = HashMap::new();
        // Temp vector for the history
        let mut state_4_history: Vec<u8> = vec!(0;params_state.len());

        // The HashMap iterator provides (key,value) pair in a random order
        for (param_name, param_current_value) in params_state.iter() {
            let param_space_state = self.params_space_state.get(param_name).unwrap();
            if num_params_2_vary > 0 {
                // If there are values that can be changed take
                let new_value = rand::thread_rng().choose(&param_space_state).unwrap();
                new_params_state.insert(param_name.clone().to_string(), *new_value);
                num_params_2_vary -= 1;
            } else {
                new_params_state.insert(param_name.clone().to_string(), *param_current_value);
            }

            // Put at the index extracted from the params_indexes the new state evaluated.
            // Note that it won't put the values of the state but its index into the space state vector.
            // This is for occupying less memory as possible.
          /*  let index_in_space_state = param_space_state.iter()
                .position(|&r| r == *new_params_state.get(param_name).unwrap());
                
            match index_in_space_state {
                Some(i) => {
                    state_4_history[*self.params_indexes.get(param_name).unwrap() as usize] =
                        i as u8
                }
                None => panic!("I did not find the parameter into the space state!"),
            }*/

        }


        // Extract the string sequence of the new state
        //   let mut byte_state_str = String::new();
        // for x in 0..state_4_history.len() {
        // byte_state_str.push_str(&*state_4_history.get(x).unwrap().to_string());
        // }
        //
        // state_4_history.clear();


        // Insert the new state into the visited hashmap. For memory efficiency the visited states parameters
        // values are coded through their index into the space_state vector.
        // let there_wasnt = self.visited_params_states.insert(byte_state_str.clone());

        // If the neighborhood selected has been already visited recursively re-call the function
        // In case all states have been visited returns None to the Annealing Solver which will interrupt
        // the evaluation. Otherwise, the new state is added to the visited ones and the function return it.


        return Some(new_params_state);
    }


    /**
	Function that returns a random state
	**/
    pub fn get_rand_param(&mut self) -> State {

        let mut new_params_state: State = HashMap::new();

        // The HashMap iterator provides (key,value) pair in a random order
        for param_name in self.params_space_state.keys() {

            let param_space_state = self.params_space_state.get(param_name).unwrap();
            let new_value = rand::thread_rng().choose(&param_space_state).unwrap();
            new_params_state.insert(param_name.clone().to_string(), *new_value);
        }

        return new_params_state;
    }



    /**
	Functions useful for the hybrid annealing-genetic algorithm
	**/
    pub fn get_rand_population(&mut self, size: usize) -> Vec<State> {
        let mut res_vec = Vec::with_capacity(size);
        for i in 0..size {
            res_vec.push(self.get_rand_param());
        }
        return res_vec;
    }
}
