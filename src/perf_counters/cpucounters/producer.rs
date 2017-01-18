use x86;
use perfcnt::{PerfCounter, AbstractPerfCounter};
use perfcnt::linux::PerfCounterBuilderLinux;
use perfcnt::linux;
use time;
use raw_cpuid;
use raw_cpuid::CpuId;
use libc;
use std::thread;
use perf_counters::cpucounters::PCM_Counters;
use std::time::Duration;

static UNHALTED_RTSC: &'static str = "CPU_CLK_UNHALTED.REF_TSC";
static UNHALTED_THREAD: &'static str = "CPU_CLK_UNHALTED.THREAD_P";
static INST_RETIRED: &'static str = "INST_RETIRED.ANY_P";
static TOT_UOPS: &'static str = "UOPS_RETIRED.TOTAL_CYCLES";
static STALL_UOPS: &'static str = "UOPS_RETIRED.STALL_CYCLES";


pub struct CountersProducer {
    pub counters: PCM_Counters,
}



impl CountersProducer {
    pub fn start() -> CountersProducer {
        let mut cnts = PCM_Counters::new();

        let mut cnts_c = cnts.clone();
        let h = thread::spawn(move || {
            let mut inst_ret_cnt =
                match x86::shared::perfcnt::intel::core_counters().unwrap().get(INST_RETIRED) {
                    Some(i_ret) => {
                        let counter = PerfCounterBuilderLinux::from_intel_event_description(i_ret)
                            .for_pid(get_pid())
                            .inherit()
                            .on_all_cpus()
                            .exclude_idle()
                            .finish()
                            .expect("Could not create counter Instruction Retired Counter");
                        counter
                    } 
                    None => panic!("Cannot Find {:?}", INST_RETIRED),
                };

            inst_ret_cnt.start()
                .expect("Could not start Instruction Retired Counter");


            let mut un_thr_cnt =
                match x86::shared::perfcnt::intel::core_counters().unwrap().get(UNHALTED_THREAD) {
                    Some(u_thr) => {
                        PerfCounterBuilderLinux::from_intel_event_description(u_thr)
                            .for_pid(get_pid())
                            .inherit()
                            .on_all_cpus()
                            .exclude_idle()
                            .finish()
                            .expect("Could not create counter Unhalted Thread Core Counter")
                    } 
                    None => panic!("Cannot Find {:?}", UNHALTED_THREAD),
                };

            un_thr_cnt.start()
                .expect("Could not start Unhalted Thread Core Counter");

            let mut rtsc_cnt =
                match x86::shared::perfcnt::intel::core_counters().unwrap().get(UNHALTED_RTSC) {
                    Some(u_rtsc) => {
                        let counter = PerfCounterBuilderLinux::from_intel_event_description(u_rtsc)
                            .for_pid(get_pid())
                            .inherit()
                            .on_all_cpus()
                            .exclude_idle()
                            .finish()
                            .expect("Could not create counter Unhalted Core Counter");
                        counter
                    }
                    None => panic!("Cannot Find {:?}", UNHALTED_RTSC),
                };

            rtsc_cnt.start()
                .expect("Could not start RTSC Counter");

            loop {

                let un_thr_val = un_thr_cnt.read()
                    .expect("Could not read Unhalted Thread Core Counter");
                cnts.set_clk_un_th(un_thr_val);


                let rtsc_val = rtsc_cnt.read().expect("Could not read RTSC Counter");
                cnts.set_clk_ref(rtsc_val);


                let inst_ret_val = inst_ret_cnt.read()
                    .expect("Could not read Instruction Retired Counter");
                cnts.set_inst_ret(inst_ret_val);

                thread::sleep(Duration::from_millis(500));
            }

        });

        CountersProducer { counters: cnts_c }

    }
}


fn get_pid() -> libc::pid_t {
    unsafe { libc::getpid() }
}
