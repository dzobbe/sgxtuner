
use rand;
use std::error::Error;
use std::io::prelude::*;
use std::path::Path;
use std::collections::{HashMap, HashSet};
use rand::Rng;
use ansi_term::Colour::{Yellow, Red};
use std::boxed::Box;
use xml_reader::XMLReader;
use State;
use shared::Parameter;

#[derive(Clone,Debug)]
pub struct ParamsConfigurator {
    pub xml_reader: XMLReader,

    pub default_param: State,
    // HashMap that stores the space state of each parameter
    pub params_space_state: HashMap<String, Vec<usize>>,
    // Visited parameters list. Saved in heap for memory space reasons
    pub visited_params_states: Box<HashSet<String>>,
}

static initial_decreasing_factor: f64 = 0.6;

impl ParamsConfigurator {
    pub fn new(reader: XMLReader) -> ParamsConfigurator {
        let mut params_configurator = ParamsConfigurator {
            xml_reader: reader,
            default_param: HashMap::new(),
            params_space_state: HashMap::new(),
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


        let musl_params_collection = self.xml_reader.get_musl_params();
        let mut initial_params_state: State = HashMap::new();


        for param in musl_params_collection.iter() {


            let space_state_elems =
                ParamsConfigurator::get_space_state(param.min, param.max, param.step);
            let space_state_elems_c = space_state_elems.clone();

            self.params_space_state
                .insert(param.clone().name, space_state_elems);


            initial_params_state.insert(param.clone().name, param.default);


            println!("{} {:?}", Yellow.paint("Input Parameter ==> "), param.name);

            println!("{} [{:?},{:?},{:?}] - {} {:?} ",Yellow.paint("Space State ==> "),
                     param.min,
                     param.max,
                     param.step,
                     Yellow.paint("Default Value ==> "),
                     param.default,
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
                                     ((current_anneal_step as f64 / period_of_variation)
            .floor()) / 10.0;
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
