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
#[derive(Clone)]
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

#[derive(Clone)]
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
    
    fn get_size(&self) -> usize{
    	let mut time_vec = self.0.lock().unwrap();
        time_vec.len()
    }
}

lazy_static! {
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
    pub a_proxyserv: String,
    pub p_proxy_serv: u16,
    time_table: SharedTimeVec,
    byte_counter: SharedCounter,
    pub reset_lock_flag: Arc<RwLock<bool>>,
}


impl Meter {
    pub fn new(addr_target: String, port_target: u16, add_proxy_server: String, port_proxy_server: u16) -> Meter {
        Meter {
            a_target: addr_target,
            p_target: port_target,
            a_proxyserv: add_proxy_server,
    		p_proxy_serv: port_proxy_server,
	    	time_table: SharedTimeVec::new(),
    		byte_counter: SharedCounter::new(),
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
		
		let proxy_server_addr=self.a_proxyserv.clone();
        let server_addr_str = proxy_server_addr+":"+&self.p_proxy_serv.to_string();
        let server_addr: SocketAddr = server_addr_str.parse()
            .expect("Unable to parse socket address");
        let acceptor = TcpListener::bind(server_addr).unwrap();
        let mut children = vec![];

		let shared_time_info: SharedTimeVec = SharedTimeVec::new();
    	let shared_num_bytes: SharedCounter = SharedCounter::new();
		
        for stream in acceptor.incoming() {

            let reset_lock_flag_c = self.reset_lock_flag.clone();

            if *reset_lock_flag_c.read().unwrap() == true {
                // Reset Flag raised: Exit the Server loop to clean resources
                break;
            }

            match stream {
                Err(e) => println!("Strange connection broken: {}", e),
                Ok(stream) => {
                	let cloned_self=self.clone();
                    children.push(thread::spawn(move || {
                        // connection succeeded
                        let mut stream_c = stream.try_clone().unwrap();
                        let stream_c2 = stream.try_clone().unwrap();
                        stream_c2.set_read_timeout(Some(Duration::new(4, 0)));

                        cloned_self.start_pipe(stream_c);
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
        self.byte_counter.reset();
        self.time_table.reset();
        // Spurious connection needed to break the proxy server loop
        let proxy_addr: IpAddr = self.a_proxyserv.parse()
            .expect("Unable to parse Target Address");
        TcpStream::connect((proxy_addr, self.p_proxy_serv));
    }


    pub fn get_num_bytes_rcvd(&self) -> usize {
        return self.byte_counter.get();
    }

    pub fn get_latency_ms(&self) -> f64 {
        return self.time_table.get_avg_value() / 1000000.0f64;
    }

    fn start_pipe(&self, front: TcpStream) {

        let targ_addr: IpAddr = self.a_target.parse()
            .expect("Unable to parse Target Address");
        let mut back = match TcpStream::connect((targ_addr, self.p_target)) {
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


		let cloned_self=self.clone();
		let cloned_self_2=self.clone();
        thread::spawn(move || {
            cloned_self.keep_copying_bench_2_targ(front, back, timedOut, latency_mutex, tx);
        });

        thread::spawn(move || {
            cloned_self_2.keep_copying_targ_2_bench(back_c, front_c, timedOut_c, latency_mutex_c, rx);
        });


    }

    /**
	Pipe BACK(Targ)<======FRONT(Bench)
	**/
    fn keep_copying_bench_2_targ(&self,mut front: TcpStream,
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
    fn keep_copying_targ_2_bench(&self, mut back: TcpStream,
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
                    self.time_table.insert(time::precise_time_ns() - res);
                }
                RecvError => {}
            };

            // Increment the number of bytes read counter
            self.byte_counter.increment(read);
			
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
