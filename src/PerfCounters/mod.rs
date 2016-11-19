extern crate perfcnt;
extern crate x86;

use self::perfcnt::{PerfCounter, AbstractPerfCounter};
use self::perfcnt::linux::{PerfCounterBuilderLinux};
use self::x86::shared::perfcnt::intel::{core_counters,uncore_counters};

pub struct MeasuredCounters {
   pub value_unhalted_core: u64,
   pub value_ret_instructions: u64,
}

pub struct PerfMetrics {
   pc_unhalted_core		: Option<PerfCounter>,
   pc_ret_instructions	: Option<PerfCounter>,
   initial_values		: Option<MeasuredCounters>,
   final_values			: Option<MeasuredCounters>
}

static UNHALTED_CNT: &'static str = "CPU_CLK_UNHALTED.THREAD";
static INST_RETIRED: &'static str = "INST_RETIRED.ANY_P";

impl PerfMetrics{
	
	pub fn new() -> PerfMetrics {
			Default::default()
	}
	
	pub fn StartCounters_4_CPI(&mut self, process_pid:u32) {
		
		match x86::shared::perfcnt::intel::core_counters().unwrap().get(UNHALTED_CNT) {
    		Some(x) => {
    			let clock_unhalted_core=x;
    			self.pc_unhalted_core = Some(PerfCounterBuilderLinux::from_intel_event_description(clock_unhalted_core)
			        .for_pid(process_pid as i32)
			        .inherit()
			        .on_all_cpus()
			        .exclude_idle()
			        .finish()
			        .expect("Could not create counter Unhalted Core Counter"));
    			self.pc_unhalted_core.as_mut().unwrap().start().expect("Could not start Unhalted Core Counter");
    		},
    		None    => panic!("Cannot Find {:?}",UNHALTED_CNT),
		}
		
		match x86::shared::perfcnt::intel::core_counters().unwrap().get(INST_RETIRED) {
    		Some(x) => { 
    			let retired_instruction=x;  
			    self.pc_ret_instructions = Some(PerfCounterBuilderLinux::from_intel_event_description(retired_instruction)
			        .for_pid(process_pid as i32)
			        .inherit()
			        .on_all_cpus()
			        .exclude_idle()
			        .finish()
			        .expect("Could not create counter Instruction Retired Counter"));
			    self.pc_ret_instructions.as_mut().unwrap().start().expect("Could not start Instruction Retired Counter");
		    }, 
    		None    => panic!("Cannot Find {:?}",INST_RETIRED),
		}   
		
		//Take the initial counter value
		let temp_4_meas = MeasuredCounters{
					value_unhalted_core:    self.pc_unhalted_core.as_mut().unwrap().read().expect("Can not read counter"),
					value_ret_instructions: self.pc_ret_instructions.as_mut().unwrap().read().expect("Can not read counter")
					};	
		self.initial_values=Some(temp_4_meas);	
	          
	}
	
	pub fn StopCounters_4_CPI(&mut self){
		
		//Take the final counter value
		let mut temp_4_meas = MeasuredCounters{
					value_unhalted_core:    self.pc_unhalted_core.as_mut().unwrap().read().expect("Can not read counter"),
					value_ret_instructions: self.pc_ret_instructions.as_mut().unwrap().read().expect("Can not read counter")
					};
					
		self.pc_unhalted_core.as_mut().unwrap().stop().expect("Can not stop the counter");
		self.pc_ret_instructions.as_mut().unwrap().stop().expect("Can not stop the counter");
		
		
		self.final_values=Some(temp_4_meas);		

	}  
	

	pub fn get_CPI(&mut self) -> f64 {
		
		let cpi: f64 =((self.final_values.as_mut().unwrap().value_unhalted_core-self.initial_values.as_mut().unwrap().value_unhalted_core)
			   		 /(self.final_values.as_mut().unwrap().value_ret_instructions-self.initial_values.as_mut().unwrap().value_ret_instructions)) as f64;
		return cpi;
	}
	
}

impl Default for PerfMetrics {
    fn default() -> PerfMetrics {
       PerfMetrics{
       	   pc_unhalted_core		: None,
		   pc_ret_instructions	: None,
		   initial_values		: None,
		   final_values			: None
		}
		
    }
}



    