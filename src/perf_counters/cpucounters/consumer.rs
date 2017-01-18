use perf_counters::cpucounters::PCM_Counters;
use super::producer::CountersProducer;

// Maximum Instruction per cycles of Intel Skylake CPU architecture
static max_ipc: u8 = 4;

#[derive(Debug, Clone)]
pub struct CntMeasurement {
    inst_retired_any: u64,
    cpu_clk_unhalted_thread: u64,
    cpu_clk_unhalted_ref: u64,
}

#[derive(Debug, Clone)]
pub struct CountersConsumer {
    counters: PCM_Counters,
}


impl CountersConsumer {
    pub fn new() -> CountersConsumer {
        let counters_producer = CountersProducer::start();
        CountersConsumer { counters: counters_producer.counters }
    }

    pub fn get_current_counters(&self) -> CntMeasurement {
        CntMeasurement {
            inst_retired_any: self.counters.get_inst_ret(),
            cpu_clk_unhalted_thread: self.counters.get_clk_un_th(),
            cpu_clk_unhalted_ref: self.counters.get_clk_ref(),
        }
    }

    pub fn get_ipc(&self, before: CntMeasurement, after: CntMeasurement) -> f64 {
        let ipc = (after.inst_retired_any - before.inst_retired_any) as f64 /
                  (after.cpu_clk_unhalted_thread - before.cpu_clk_unhalted_thread) as f64;
        return ipc;
    }

    pub fn get_core_ipc(&self, before: CntMeasurement, after: CntMeasurement) -> f64 {

        return self.get_ipc(before, after) as f64 * get_max_th_core() as f64;
    }


    pub fn get_ipc_utilization(&self, before: CntMeasurement, after: CntMeasurement) -> f64 {

        return (self.get_core_ipc(before, after) as f64 / max_ipc as f64) * 100.0;
    }


    pub fn get_cpi(&mut self, before: CntMeasurement, after: CntMeasurement) -> f64 {

        let cpi = (after.cpu_clk_unhalted_thread - before.cpu_clk_unhalted_thread) as f64 /
                  (after.inst_retired_any - before.inst_retired_any) as f64;
        return cpi;
    }



    pub fn get_core_utilization(&mut self, before: CntMeasurement, after: CntMeasurement) -> f64 {
        let exec_usage = (after.inst_retired_any - before.inst_retired_any) as f64 /
                         (after.cpu_clk_unhalted_ref - before.cpu_clk_unhalted_ref) as f64;

        return ((exec_usage * get_max_th_core() as f64) / max_ipc as f64) * 100.0;
    }

    pub fn get_cpu_exec_time(&mut self, before: CntMeasurement, after: CntMeasurement) -> f64 {


        let cpu_freq = get_cpu_freq();

        // Get CPI=ClkCycles/Instructions
        let cpi = self.get_cpi(before.clone(), after.clone());

        // Get CPU ClockCycle = 1/ClockRate
        let cpu_clock_cycle = 1.0 / (cpu_freq * 1000000.0f64);

        // Get instruction count
        let instruction_count = (after.inst_retired_any - before.inst_retired_any) as f64;

        // println!("ee {} {}",after.get_inst_ret(),before.get_inst_ret());

        // Evaluate CPU execution time=CPI * CPU ClkCycle * Instruction count
        return cpi * cpu_clock_cycle * instruction_count;

    }
}

fn get_cpu_freq() -> f64 {
    // TODO
    // let cpuid = CpuId::new();
    return 3599.61; //MHz
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
}

fn get_max_th_core() -> u8 {
    // TODO
    return 2;
}
