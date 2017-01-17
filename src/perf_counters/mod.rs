use x86;
use perfcnt::{PerfCounter, AbstractPerfCounter};
use perfcnt::linux::PerfCounterBuilderLinux;
use time;
use raw_cpuid;
use raw_cpuid::CpuId;

pub struct MeasuredCounters {
    pub val_unhalted_rtsc: u64,
    pub val_unhalted_thr: u64,
    pub val_inst_ret: u64,
}

pub struct PerfMeter {
    cnt_unhalted_rtsc: PerfCounter,
    cnt_unhalted_thr: PerfCounter,
    cnt_inst_ret: PerfCounter,
    initial_values: MeasuredCounters,
    start_time: u64,
}

static UNHALTED_RTSC: &'static str = "CPU_CLK_UNHALTED.REF_TSC";
static UNHALTED_THREAD: &'static str = "CPU_CLK_UNHALTED.THREAD_P";
static INST_RETIRED: &'static str = "INST_RETIRED.ANY_P";
static TOT_UOPS: &'static str = "UOPS_RETIRED.TOTAL_CYCLES";
static STALL_UOPS: &'static str = "UOPS_RETIRED.STALL_CYCLES";

impl PerfMeter {
    pub fn new() -> PerfMeter {
    
        let mut rtsc=match x86::shared::perfcnt::intel::core_counters().unwrap().get(UNHALTED_RTSC) {
            Some(u_rtsc) => {
                let counter=PerfCounterBuilderLinux::from_intel_event_description(u_rtsc)
			        //.for_pid(process_pid as i32)
			        .inherit() 
			        .on_all_cpus() 
			        .exclude_idle()
			        .finish() 
			        .expect("Could not create counter Unhalted Core Counter");
                counter
                    .start()
                    .expect("Could not start Unhalted Core Counter");
                counter
            }
            None => panic!("Cannot Find {:?}", UNHALTED_RTSC)
        };
        

        let mut un_thr=match x86::shared::perfcnt::intel::core_counters().unwrap().get(UNHALTED_THREAD) {
            Some(u_thr) => {
                let counter = PerfCounterBuilderLinux::from_intel_event_description(u_thr)
			        //.for_pid(process_pid as i32) 
			        .inherit()
			        .on_all_cpus()
			        .exclude_idle()
			        .finish()
			        .expect("Could not create counter Unhalted Core Counter Thread Ref");
                counter
                    .start()
                    .expect("Could not start Instruction Retired Counter");
                counter
            } 
            None => panic!("Cannot Find {:?}", UNHALTED_THREAD),
        };
        
 
	     let mut inst_ret=match x86::shared::perfcnt::intel::core_counters().unwrap().get(INST_RETIRED) {
            Some(i_ret) => {
                let counter = PerfCounterBuilderLinux::from_intel_event_description(i_ret)
			        //.for_pid(process_pid as i32) 
			        .inherit()
			        .on_all_cpus()
			        .exclude_idle()
			        .finish()
			        .expect("Could not create counter Instruction Retired Counter");
                counter
                    .start()
                    .expect("Could not start Instruction Retired Counter");
                counter
            } 
            None => panic!("Cannot Find {:?}", UNHALTED_THREAD),
        };
      

	   let initial_meas=MeasuredCounters{
	   	    val_unhalted_rtsc:rtsc.read().expect("Can not read counter"),
		    val_unhalted_thr: un_thr.read().expect("Can not read counter"),
		    val_inst_ret: 	  inst_ret.read().expect("Can not read counter"),
	   };
	        
       PerfMeter{
       	    cnt_unhalted_rtsc: rtsc,
		    cnt_unhalted_thr: un_thr,
		    cnt_inst_ret: inst_ret,
		    initial_values: initial_meas,
		    start_time: time::precise_time_ns(),
       }

    }

   
   
    pub fn get_cpu_time(&mut self) -> f64 {
		
		let cpuid = CpuId::new();

		/*let cpu_freq=match cpuid.get_processor_frequency_info() {
	        Some(frequency) => {
	        	println!("CPU speed: {} MHz", frequency);
	        	frequency
	        	},
	        None => {
	        	println!("Couldn't get CPU speed.");
	        	0
	        	}
	    };*/
                

		//CPU Time = CPU cycles executed ∗ Cycle times
		//where Cycle Times=1000/CPU Freq=1000/3599.61MHz=0.277ns
		return 0.0;//((current_un_rtsc-self.initial_values.val_unhalted_rtsc) as f64 * 0.277) /1000000000.0f64 as f64;

    }
     
    pub fn get_cpi(&mut self) -> f64 { 
        let current_un_thr=self.cnt_unhalted_thr
                .read() 
                .expect("Can not read counter");
                
        let current_inst_ret=self.cnt_inst_ret
                .read() 
                .expect("Can not read counter");
		
        let cpi=(current_un_thr-self.initial_values.val_unhalted_thr) as f64
        		/(current_inst_ret-self.initial_values.val_inst_ret) as f64;
        		
		return cpi;
    }
     
    pub fn get_cpu_usage(&mut self) -> f64 {
	    
	    let mut cnt_uops_total = match x86::shared::perfcnt::intel::core_counters().unwrap().get(TOT_UOPS) {
            Some(t_uops) => {
                let counter = PerfCounterBuilderLinux::from_intel_event_description(t_uops)
			        //.for_pid(process_pid as i32) 
			        .inherit()
			        .on_all_cpus()
			        .exclude_idle()
			        .finish()
			        .expect("Could not create counter Instruction Retired Counter");
                counter
                    .start()
                    .expect("Could not start Instruction Retired Counter");
                counter
            } 
            None => panic!("Cannot Find {:?}", TOT_UOPS),
        };
    	let mut cnt_uops_stall = match x86::shared::perfcnt::intel::core_counters().unwrap().get(STALL_UOPS) {
            Some(s_uops) => {
                let counter = PerfCounterBuilderLinux::from_intel_event_description(s_uops)
			        //.for_pid(process_pid as i32) 
			        .inherit()
			        .on_all_cpus()
			        .exclude_idle()
			        .finish()
			        .expect("Could not create counter Instruction Retired Counter");
                counter
                    .start()
                    .expect("Could not start Instruction Retired Counter");
                counter
            } 
            None => panic!("Cannot Find {:?}", TOT_UOPS),
        };
    	
    	let total_read=cnt_uops_total.read().expect("Can not read counter");
    	let stalls=cnt_uops_total.read().expect("Can not read counter");

    	
		return ((total_read-stalls) as f64 /total_read as f64)*100.0 ;
    }
     
    pub fn get_cpu_exec_time(&mut self) -> f64 {
    	let cpuid = CpuId::new();
	
		let cpu_freq=3599.61; //MHz 
		/*match cpuid.get_processor_frequency_info() {
	        Some(frequency) => {
	        	println!("CPU speed: {} MHz", frequency);
	        	frequency
	        	},
	        None => {
	        	println!("Couldn't get CPU speed.");
	        	0
	        	}
	    };*/
    	let current_inst_ret=self.cnt_inst_ret
                .read() 
                .expect("Can not read counter");
                    	let current_inst_ret=self.cnt_inst_ret
                .read() 
                .expect("Can not read counter");
    	
    	//Get CPI=ClkCycles/Instructions
    	let cpi=self.get_cpi();
    	 
    	//Get CPU ClockCycle = 1/ClockRate
    	let cpu_clock_cycle=1.0/(cpu_freq*1000000.0f64);
    	
    	//Get instruction count
    	let instruction_count=(current_inst_ret-self.initial_values.val_inst_ret) as f64;
    	

    	//Evaluate CPU execution time=CPI * CPU ClkCycle * Instruction count
    	let cpu_exec_time=cpi*cpu_clock_cycle*instruction_count;
    	 
	 	return cpu_exec_time;
    }
}

