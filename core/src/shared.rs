use {EnergyType, CoolingSchedule, SolverVersion, ParameterLevel};
use std::sync::{Arc, Mutex};



#[derive(Debug, Clone)]
pub struct TunerParameter {
    pub max_step: usize,
    pub num_iter: u8,
    pub min_temp: Option<f64>,
    pub max_temp: Option<f64>,
    pub energy: EnergyType,
    pub cooling: CoolingSchedule,
    pub version: SolverVersion,
}

#[derive(Debug, Clone)]
pub struct ScriptInfo {
    pub name: String,
    pub fulltag: String,
    pub envfile: String,
}

#[derive(Debug, Clone)]
pub struct IntParameter {
    pub name: String,
    pub min: usize,
    pub max: usize,
    pub step: usize,
    pub default: usize,
    pub level: ParameterLevel,
}

#[derive(Debug, Clone)]
pub struct BoolParameter {
    pub name: String,
    pub true_val: String,
    pub false_val: String,
    pub default: bool,
    pub level: ParameterLevel,
}


#[derive(Debug, Clone)]
pub struct ProcessPool(Arc<Mutex<Vec<String>>>);
impl ProcessPool {
    pub fn new() -> Self {
        ProcessPool(Arc::new(Mutex::new(Vec::new())))
    }

    pub fn push(&self, elem: String) {
        let mut collection = self.0.lock().unwrap();
        (*collection).push(elem);
    }

    pub fn pop(&mut self) -> String {
        let mut collection = self.0.lock().unwrap();
        let res = (*collection).pop().unwrap().clone();
        res
    }

    pub fn size(&self) -> usize {
        let mut collection = self.0.lock().unwrap();
        (*collection).len()
    }
}
