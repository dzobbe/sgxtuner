use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use ExecutionType;


#[derive(Debug, Clone)]
pub struct Process2Spawn {
    pub execution_type: ExecutionType,
    pub host: String,
    pub user: String,
    pub bin: String,
    pub path: String,
    pub args: String,
    pub address: String,
    pub port: String,
}


#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub min: usize,
    pub max: usize,
    pub step: usize,
    pub default: usize,
}


#[derive(Debug, Clone)]
pub struct ProcessPool(Arc<Mutex<Vec<Process2Spawn>>>);
impl ProcessPool {
    pub fn new() -> Self {
        ProcessPool(Arc::new(Mutex::new(Vec::new())))
    }

    pub fn push(&self, elem: Process2Spawn) {
        let mut collection = self.0.lock().unwrap();
        (*collection).push(elem);
    }

    pub fn remove(&mut self) -> Process2Spawn {
        let mut collection = self.0.lock().unwrap();
        let res = (*collection).pop().unwrap().clone();
        res
    }
}
