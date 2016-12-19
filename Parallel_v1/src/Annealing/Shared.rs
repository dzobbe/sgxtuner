use rand;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use CoolingSchedule;
use rand::{thread_rng,Rng};
use super::Cooler::{Cooler, StepsCooler, TimeCooler};

#[derive(Debug, Clone)]
pub struct MrResult{
	pub energy: f64,
	pub state: HashMap<String,u32>,
}

#[derive(Debug, Clone)]
pub struct NeighborhoodsPool(Arc<Mutex<Vec<HashMap<String, u32>>>>);

#[derive(Debug, Clone)]
pub struct ElapsedSteps(Arc<Mutex<usize>>);

#[derive(Debug, Clone)]
pub struct AcceptedStates(Arc<Mutex<usize>>);

#[derive(Debug, Clone)]
pub struct SubsequentRejStates(Arc<Mutex<usize>>);

#[derive(Debug, Clone)]
pub struct Temperature{
	temp:   Arc<Mutex<f64>>,
	cooler: StepsCooler, 
	cooling_schedule: CoolingSchedule,
	}

#[derive(Debug, Clone)]
pub struct ThreadsResults(Arc<Mutex<Vec<MrResult>>>);


impl NeighborhoodsPool {
    pub fn new(neighs: Vec<HashMap<String, u32>>) -> Self {
        NeighborhoodsPool(Arc::new(Mutex::new(neighs)))
    }
    
    pub fn remove_one(&self) -> Option<HashMap<String, u32>> {
        let mut neighs = self.0.lock().unwrap();

        if neighs.len() == 0 {
            return None;
        } else {
        	let len=neighs.len();       
            return Some(neighs.swap_remove(rand::thread_rng().gen_range(0, len)));
        }
    }

}

impl ElapsedSteps {
    pub fn new() -> Self {
        ElapsedSteps(Arc::new(Mutex::new(0)))
    }
    pub fn increment(&self) {
        let mut steps = self.0.lock().unwrap();
        *steps = *steps + 1;
    }
    pub fn get(&self) -> usize {
        let steps = self.0.lock().unwrap();
        *steps
    }

}

impl AcceptedStates {
    pub fn new() -> Self {
        AcceptedStates(Arc::new(Mutex::new(0)))
    }
    pub fn increment(&self) {
        let mut accepted = self.0.lock().unwrap();
        *accepted = *accepted + 1;
    }
    pub fn get(&self) -> usize {
        let accepted = self.0.lock().unwrap();
        *accepted
    }

}

impl SubsequentRejStates {
    pub fn new() -> Self {
        SubsequentRejStates(Arc::new(Mutex::new(0)))
    }
    pub fn increment(&self) {
        let mut rejected = self.0.lock().unwrap();
        *rejected = *rejected + 1;
    }
    pub fn get(&self) -> usize {
        let rejected = self.0.lock().unwrap();
        *rejected
    }
    
    pub fn reset(&self) {
        let mut rejected = self.0.lock().unwrap();
        *rejected = 0;
    }

}

impl Temperature {
    pub fn new(start_temp: f64, c: StepsCooler, cs: CoolingSchedule) -> Self {
        Temperature{
        	temp: Arc::new(Mutex::new(start_temp)),
        	cooler: c,
        	cooling_schedule: cs
        	}
    }
    
    pub fn update(&self, elapsed_steps: usize) {
        let mut temperature = self.temp.lock().unwrap();
        
        *temperature=match self.cooling_schedule {
			                CoolingSchedule::linear => self.cooler.linear_cooling(elapsed_steps),
			                CoolingSchedule::exponential => self.cooler.exponential_cooling(elapsed_steps),
			                CoolingSchedule::adaptive => self.cooler.basic_exp_cooling(*temperature),
		            };        
    }
    
    pub fn get(&self) -> f64 {
        let temperature = self.temp.lock().unwrap();
        *temperature
    }

} 

impl ThreadsResults {
    pub fn new() -> Self {
        ThreadsResults(Arc::new(Mutex::new(Vec::new())))
    }
    
    pub fn push(&self, res: MrResult) {
        let mut res_coll = self.0.lock().unwrap();
		(*res_coll).push(res);
    }
    
    pub fn get_coll(&self) -> Vec<MrResult>{
        let mut res_coll = self.0.lock().unwrap();
        let r=(*res_coll).clone();
		r
    }
    

}
