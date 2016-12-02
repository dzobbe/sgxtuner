use lazy_static;
use libc;
use time;
use ansi_term;
use ansi_term::Colour::{Red, Yellow};
use std::net::{TcpListener, TcpStream, Shutdown, SocketAddr, IpAddr};
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{thread, str};
use std::time::Duration;
use std::io::prelude::*;
use libc::setrlimit;
use std::collections::HashMap;
use EnergyType;
use std::sync::mpsc::{channel, Sender, Receiver};

/// /////////////////////////////////////////////////////////////////////
/// /////////////////////////////////////////////////////////////////////
/**
Definition of Shared Counter for the THROUGHPUT evaluation and 
Time Table for the LATENCY evaluation 
**/
pub struct SharedCounter(Arc<Mutex<usize>>);
impl SharedCounter {
    fn new() -> Self {
        SharedCounter(Arc::new(Mutex::new(0)))
    }
    fn increment(&self, quantity: usize) {
        let mut counter = self.0.lock().unwrap();
        *counter = *counter + quantity;
    }
    fn get(&self) -> usize {
        let counter = self.0.lock().unwrap();
        *counter
    }

    fn reset(&self) {
        let mut counter = self.0.lock().unwrap();
        *counter = 0;
    }
}

pub struct SharedTimeVec(Arc<Mutex<Vec<u64>>>);
impl SharedTimeVec {
    fn new() -> Self {
        SharedTimeVec(Arc::new(Mutex::new(Vec::new())))
    }

    fn insert(&self, value: u64) {
        let mut time_vec = self.0.lock().unwrap();
        time_vec.push(value);
    }

    fn get_avg_value(&self) -> f64 {
        let mut time_vec = self.0.lock().unwrap();
        let sum: u64 = time_vec.iter().sum();
        return sum as f64 / time_vec.len() as f64;
    }

    fn reset(&self) {
        let mut time_vec = self.0.lock().unwrap();
        time_vec.clear();
    }
}

lazy_static! {
    static ref TIME_TABLE: SharedTimeVec   = {SharedTimeVec::new()};
    static ref NUM_BYTES : SharedCounter   = {SharedCounter::new()};
    static ref ERROR: Arc<Mutex<bool>>	   = Arc::new(Mutex::new(false));
}

/// /////////////////////////////////////////////////////////////////////
/// /////////////////////////////////////////////////////////////////////
/**
The MeterProxy is a proxy which interposes between the TARGET and the BENCHMARK application to measure 
performance metrics and use them as energy for the simulated annealing algorithm.
It measures both Throughput and Latency of the TARGET application under test.
**/
#[derive(Clone)]
pub struct Meter {
    pub a_target: String,
    pub p_target: u16,
    pub reset_lock_flag: Arc<RwLock<bool>>,
}


impl Meter {
    pub fn new(addr_target: String, port_target: u16) -> Meter {
        Meter {
            a_target: addr_target,
            p_target: port_target,
            reset_lock_flag: Arc::new(RwLock::new(false)),
        }
    }


    pub fn start(&self) {
        // Increase file descriptor resources limits (this avoids  the risk of exception: "Too many open files (os error 24)")
        let rlim = libc::rlimit {
            rlim_cur: 4096,
            rlim_max: 4096,
        };
        unsafe {
            libc::setrlimit(libc::RLIMIT_NOFILE, &rlim);
        }

        let server_addr_str = "127.0.0.1:12349";//.to_string()+&self.p_proxy.to_string();
        let server_addr: SocketAddr = server_addr_str.parse()
            .expect("Unable to parse socket address");
        let acceptor = TcpListener::bind(server_addr).unwrap();
        let mut children = vec![];


        for stream in acceptor.incoming() {

            let reset_lock_flag_c = self.reset_lock_flag.clone();
            let a_target_c = self.clone().a_target;
            let p_target_c = self.p_target;

            if *reset_lock_flag_c.read().unwrap() == true {
                // Reset Flag raised: Exit the Server loop to clean resources
                break;
            }

            match stream {
                Err(e) => println!("Strange connection broken: {}", e),
                Ok(stream) => {
                    children.push(thread::spawn(move || {
                        // connection succeeded
                        let mut stream_c = stream.try_clone().unwrap();
                        let stream_c2 = stream.try_clone().unwrap();
                        stream_c2.set_read_timeout(Some(Duration::new(3, 0)));

                        Meter::start_pipe(stream_c, a_target_c, p_target_c);
                        drop(stream);

                    }));

                }
            }
        }
        for child in children {
            // Wait for the thread to finish. Returns a result.
            let _ = child.join();
        }
        drop(acceptor);
        return;
    }


    /**
	Stop the proxy server and clean resources
	**/
    pub fn stop_and_reset(&self) {
        *self.reset_lock_flag.write().unwrap() = true;
        NUM_BYTES.reset();
        TIME_TABLE.reset();
        // Spurious connection needed to break the proxy server loop
        TcpStream::connect(("127.0.0.1", 12349));
    }


    pub fn get_num_bytes_rcvd(&self) -> usize {
        return NUM_BYTES.get();
    }

    pub fn get_latency_ms(&self) -> f64 {
        return TIME_TABLE.get_avg_value() / 1000000.0f64;
    }

    fn start_pipe(front: TcpStream, target_addr: String, target_port: u16) {

        let targ_addr: IpAddr = target_addr.parse()
            .expect("Unable to parse Target Address");
        let mut back = match TcpStream::connect((targ_addr, target_port)) {
            Err(e) => {
                let mut err = ERROR.lock().unwrap();
                if *err == false {
                    println!("{} Unable to connect to the Target Application. Maybe a bad \
                              configuration: {}",
                             Red.paint("*****ERROR***** --> "),
                             e);
                };
                *err = true;
                front.shutdown(Shutdown::Both);
                drop(front);
                return;
            }
            Ok(b) => b,
        };



        let front_c = front.try_clone().unwrap();
        let back_c = back.try_clone().unwrap();

        let timedOut = Arc::new(AtomicBool::new(false));
        let timedOut_c = timedOut.clone();


        let latency_mutex: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
        let (tx, rx) = channel();
        let latency_mutex_c = latency_mutex.clone();



        thread::spawn(move || {
            Meter::keep_copying_bench_2_targ(front, back, timedOut, latency_mutex, tx);
        });

        thread::spawn(move || {
            Meter::keep_copying_targ_2_bench(back_c, front_c, timedOut_c, latency_mutex_c, rx);
        });


    }

    /**
	Pipe BACK(Targ)<======FRONT(Bench)
	**/
    fn keep_copying_bench_2_targ(mut front: TcpStream,
                                 mut back: TcpStream,
                                 timedOut: Arc<AtomicBool>,
                                 time_mutex: Arc<Mutex<u64>>,
                                 tx: Sender<u8>) {

        front.set_read_timeout(Some(Duration::new(1000, 0)));
        let mut buf = [0; 1024];


        loop {

            let read = match front.read(&mut buf) {
                Err(ref err) => {
                    let other = timedOut.swap(true, Ordering::AcqRel);
                    if other {
                        // the other side also timed-out / errored, so lets go
                        drop(front);
                        drop(back);
                        return;
                    }
                    // normal errors, just stop
                    front.shutdown(Shutdown::Both);
                    back.shutdown(Shutdown::Both);
                    // normal errors, just stop
                    drop(front);
                    drop(back);
                    return; // normal errors, stop
                }
                Ok(r) => r,
            };


            let mut start_time = time_mutex.lock().unwrap();
            *start_time = time::precise_time_ns();

            timedOut.store(false, Ordering::Release);
            match back.write(&buf[0..read]) {
                Err(..) => {
                    timedOut.store(true, Ordering::Release);
                    // normal errors, just stop
                    front.shutdown(Shutdown::Both);
                    back.shutdown(Shutdown::Both);
                    drop(front);
                    drop(back);
                    return;
                }
                Ok(..) => (),
            };

            tx.send(1).unwrap();
        }

    }

    /**
	Pipe BACK(Targ)======>FRONT(Bench)
	**/
    fn keep_copying_targ_2_bench(mut back: TcpStream,
                                 mut front: TcpStream,
                                 timedOut: Arc<AtomicBool>,
                                 time_mutex: Arc<Mutex<u64>>,
                                 rx: Receiver<u8>) {

        back.set_read_timeout(Some(Duration::new(1000, 0)));
        let mut buf = [0; 1024];


        loop {

            let read = match back.read(&mut buf) {
                Err(ref err) => {
                    let other = timedOut.swap(true, Ordering::AcqRel);
                    if other {
                        // the other side also timed-out / errored, so lets go
                        drop(back);
                        drop(front);
                        return;
                    }
                    // normal errors, just stop
                    front.shutdown(Shutdown::Both);
                    back.shutdown(Shutdown::Both);
                    drop(back);
                    drop(front);

                    return; // normal errors, stop
                }
                Ok(r) => r,
            };

            match rx.try_recv() {
                Ok(r) => {
                    let res = *(time_mutex.lock().unwrap());
                    TIME_TABLE.insert(time::precise_time_ns() - res);
                }
                RecvError => {}
            };

            // Increment the number of bytes read counter
            NUM_BYTES.increment(read);


            timedOut.store(false, Ordering::Release);
            match front.write(&buf[0..read]) {
                Err(..) => {
                    timedOut.store(true, Ordering::Release);
                    // normal errors, just stop
                    front.shutdown(Shutdown::Both);
                    back.shutdown(Shutdown::Both);
                    drop(back);
                    drop(front);
                    return;
                }
                Ok(..) => (),
            };


        }

        drop(back);
        drop(front);


    }
}
