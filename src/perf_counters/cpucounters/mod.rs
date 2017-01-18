use std::sync::{Arc, Mutex};


pub mod producer;
pub mod consumer;

#[derive(Debug, Clone)]
pub struct PCM_Counters {
    inst_retired_any: Arc<Mutex<u64>>,
    cpu_clk_unhalted_thread: Arc<Mutex<u64>>,
    cpu_clk_unhalted_ref: Arc<Mutex<u64>>,
}

impl PCM_Counters {
    pub fn new() -> Self {
        PCM_Counters {
            inst_retired_any: Arc::new(Mutex::new(0)),
            cpu_clk_unhalted_thread: Arc::new(Mutex::new(0)),
            cpu_clk_unhalted_ref: Arc::new(Mutex::new(0)),
        }
    }

    pub fn set_inst_ret(&self, val: u64) {
        let mut cnt = self.inst_retired_any.lock().unwrap();
        *cnt = val;
    }

    pub fn set_clk_un_th(&self, val: u64) {
        let mut cnt = self.cpu_clk_unhalted_thread.lock().unwrap();
        *cnt = val;
    }

    pub fn set_clk_ref(&self, val: u64) {
        let mut cnt = self.cpu_clk_unhalted_ref.lock().unwrap();
        *cnt = val;
    }


    pub fn get_inst_ret(&self) -> u64 {
        let mut cnt = self.inst_retired_any.lock().unwrap();
        *cnt
    }

    pub fn get_clk_un_th(&self) -> u64 {
        let mut cnt = self.cpu_clk_unhalted_thread.lock().unwrap();
        *cnt
    }

    pub fn get_clk_ref(&self) -> u64 {
        let mut cnt = self.cpu_clk_unhalted_ref.lock().unwrap();
        *cnt
    }
}
