/// ///////////////////////////////////////////////////////////////////////////
///  File: Annealing/Solver/Common.rs
/// ///////////////////////////////////////////////////////////////////////////
///  Copyright 2017 Giovanni Mazzeo
///
///  Licensed under the Apache License, Version 2.0 (the "License");
///  you may not use this file except in compliance with the License.
///  You may obtain a copy of the License at
///
///      http://www.apache.org/licenses/LICENSE-2.0
///
///  Unless required by applicable law or agreed to in writing, software
///  distributed under the License is distributed on an "AS IS" BASIS,
///  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
///  See the License for the specific language governing permissions and
///  limitations under the License.
/// ///////////////////////////////////////////////////////////////////////////


use rand;
use hwloc;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use CoolingSchedule;
use rand::{thread_rng, Rng};
use annealing::cooler::{Cooler, StepsCooler, TimeCooler};
use hwloc::{Topology, ObjectType, CPUBIND_THREAD, CpuSet};
use State;

#[derive(Debug, Clone)]
pub struct MrResult {
    pub energy: f64,
    pub state: State,
}

#[derive(Debug, Clone)]
pub struct IntermediateResults{
	pub last_nrg: f64,
	pub last_state: State,
	pub best_nrg: f64,
	pub best_state: State,
}

// Get the number of physical cpu cores
pub fn get_num_cores() -> usize {
    let cpu_topology = Arc::new(Mutex::new(Topology::new()));
    let topo_rc = cpu_topology.clone();
    let topo_locked = topo_rc.lock().unwrap();
    return (*topo_locked).objects_with_type(&ObjectType::Core).unwrap().len();
}

#[derive(Debug, Clone)]
pub struct StatesPool(Arc<Mutex<Vec<State>>>);

#[derive(Debug, Clone)]
pub struct NeighborhoodsPool(Arc<Mutex<Vec<State>>>);

#[derive(Debug, Clone)]
pub struct ElapsedSteps(Arc<Mutex<usize>>);

#[derive(Debug, Clone)]
pub struct AcceptedStates(Arc<Mutex<usize>>);

#[derive(Debug, Clone)]
pub struct SubsequentAccStates(Arc<Mutex<usize>>);

#[derive(Debug, Clone)]
pub struct Temperature {
    temp: Arc<Mutex<f64>>,
    cooler: StepsCooler,
    cooling_schedule: CoolingSchedule,
}

#[derive(Debug, Clone)]
pub struct ThreadsResults(Arc<Mutex<Vec<MrResult>>>);

/// *********************************************************************************************************

impl StatesPool {
    pub fn new() -> Self {
        StatesPool(Arc::new(Mutex::new(Vec::new())))
    }

    pub fn new_with_val(init_vec: Vec<State>) -> Self {
        StatesPool(Arc::new(Mutex::new(init_vec)))
    }

    pub fn push(&self, new_elem: State) {
        let mut pool = self.0.lock().unwrap();
        (*pool).push(new_elem);
    }

    pub fn pop(&self) -> Option<State> {
        let mut pool = self.0.lock().unwrap();
        (*pool).pop()
    }

	pub fn shuffle(&self){
        let mut pool = self.0.lock().unwrap();
        rand::thread_rng().shuffle(&mut pool);
	}

    pub fn remove_one(&self) -> Option<State> {
        let mut pool = self.0.lock().unwrap();
        if pool.len() == 0 {
            return None;
        } else {
            let len = pool.len();
            return Some(pool.swap_remove(rand::thread_rng().gen_range(0, len)));
        }
    }
    pub fn size(&self) -> u64 {
        let pool = self.0.lock().unwrap();
        pool.len() as u64
    }
    
    pub fn push_bulk(&self, mut v_1: &mut Vec<State>) {
        let mut pool = self.0.lock().unwrap();
        (*pool).append(&mut v_1)
    }
} 

/// *********************************************************************************************************

impl NeighborhoodsPool {
    pub fn new(neighs: Vec<State>) -> Self {
        NeighborhoodsPool(Arc::new(Mutex::new(neighs)))
    }

    pub fn remove_one(&self) -> Option<State> {
        let mut neighs = self.0.lock().unwrap();

        if neighs.len() == 0 {
            return None;
        } else {
            let len = neighs.len();
            return Some(neighs.swap_remove(rand::thread_rng().gen_range(0, len)));
        }
    }
    pub fn size(&self) -> u64 {
        let neighs = self.0.lock().unwrap();
        neighs.len() as u64
    }
}
/// *********************************************************************************************************

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
    pub fn add(&self, val: usize) {
        let mut steps = self.0.lock().unwrap();
        *steps=*steps + val;
    }
}
/// *********************************************************************************************************

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
/// *********************************************************************************************************

impl SubsequentAccStates {
    pub fn new() -> Self {
        SubsequentAccStates(Arc::new(Mutex::new(0)))
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
/// *********************************************************************************************************

impl Temperature {
    pub fn new(start_temp: f64, c: StepsCooler, cs: CoolingSchedule) -> Self {
        Temperature {
            temp: Arc::new(Mutex::new(start_temp)),
            cooler: c,
            cooling_schedule: cs,
        }
    }

    pub fn update(&self, elapsed_steps: usize) {
        let mut temperature = self.temp.lock().unwrap();

        *temperature = match self.cooling_schedule {
            CoolingSchedule::linear => self.cooler.linear_cooling(elapsed_steps),
            CoolingSchedule::exponential => self.cooler.exponential_cooling(elapsed_steps),
            CoolingSchedule::basic_exp_cooling => self.cooler.basic_exp_cooling(*temperature),
        };
    }

    pub fn get(&self) -> f64 {
        let temperature = self.temp.lock().unwrap();
        *temperature
    }
}

/// *********************************************************************************************************

impl ThreadsResults {
    pub fn new() -> Self {
        ThreadsResults(Arc::new(Mutex::new(Vec::new())))
    }

    pub fn push(&self, res: MrResult) {
        let mut res_coll = self.0.lock().unwrap();
        (*res_coll).push(res);
    }

    pub fn get_coll(&self) -> Vec<MrResult> {
        let mut res_coll = self.0.lock().unwrap();
        let r = (*res_coll).clone();
        r
    }
}
