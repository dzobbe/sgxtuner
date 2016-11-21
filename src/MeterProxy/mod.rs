extern crate libc;

use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use std::io::prelude::*;
use self::libc::setrlimit;
use std::sync::RwLock;


#[derive(Clone)]
pub struct ConcurrentCounter(Arc<Mutex<usize>>);

impl ConcurrentCounter {
    fn new(val: usize) -> Self {
        ConcurrentCounter(Arc::new(Mutex::new(val)))
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
pub struct Meter {
    pub num_target_responses: ConcurrentCounter,
    pub reset_lock_flag: Arc<RwLock<bool>>,
}

impl Meter {
    pub fn new() -> Meter {
        Meter {
            num_target_responses: ConcurrentCounter::new(0),
            reset_lock_flag: Arc::new(RwLock::new(false)),
        }
    }


    pub fn start(&self, port_target: u16, port_proxy: u16) {
        // Increase the limit of resources for sockets limits (this avoids exception: "Too many open files (os error 24)")
        let rlim = libc::rlimit {
            rlim_cur: 4096,
            rlim_max: 4096,
        };
        unsafe {
            libc::setrlimit(libc::RLIMIT_NOFILE, &rlim);
        }

        let acceptor = TcpListener::bind("127.0.0.1:12349").unwrap();
        let mut children = vec![];

        for stream in acceptor.incoming() {
            let num_target_responses_c = self.num_target_responses.clone();
            let reset_lock_flag_c = self.reset_lock_flag.clone();

            let flag_c = reset_lock_flag_c.clone();

            if *flag_c.read().unwrap() == true {
                println!("Reset Flag Raised");
                break;
            }

            match stream {
                Err(e) => println!("Strange connection broken: {}", e),
                Ok(stream) => {
                    children.push(thread::spawn(move || {
                        // connection succeeded
                        let mut stream_c = stream.try_clone().unwrap();

                        stream_c.set_read_timeout(Some(Duration::new(1, 0)));
                        let mut header = [0; 1];

                        match stream_c.read_exact(&mut header) {
                            Err(..) => None,
                            Ok(b) => Some(b),
                        };

                        Meter::start_pipe(stream_c,
                                          port_target,
                                          Some(header[0]),
                                          num_target_responses_c,
                                          reset_lock_flag_c);
                        drop(stream);

                    }));

                }
            }
        }
        drop(acceptor);
        return;
    }


    pub fn stop_and_reset(&self) {
        *self.reset_lock_flag.write().unwrap() = true;
        self.num_target_responses.reset();
        TcpStream::connect(("127.0.0.1", 12349));
    }


    pub fn get_num_bytes(&self) -> usize {
        return self.num_target_responses.get();
    }

    fn start_pipe(front: TcpStream,
                  port: u16,
                  header: Option<u8>,
                  counter: ConcurrentCounter,
                  reset_lock_flag: Arc<RwLock<bool>>) {
        let mut back = match TcpStream::connect(("127.0.0.1", 12347)) {
            Err(e) => {
                println!("Error connecting to target application: {}", e);
                drop(front);
                return;
            }
            Ok(b) => b,
        };
        if header.is_some() {
            let mut buf_header = [0; 1];
            buf_header[0] = header.unwrap();
            match back.write(&mut buf_header) {
                Err(e) => {
                    println!("Error writing first byte to target: {}", e);
                    drop(back);
                    drop(front);
                    return;
                }
                Ok(..) => (),
            };
        }

        let front_copy = front.try_clone().unwrap();
        let back_copy = back.try_clone().unwrap();

        let timedOut = Arc::new(AtomicBool::new(false));
        let timedOut_copy = timedOut.clone();

        let reset_lock_flag_c = reset_lock_flag.clone();
        thread::spawn(move || {
            Meter::keep_copying_bench_2_targ(front, back, timedOut, reset_lock_flag);
        });

        thread::spawn(move || {
            Meter::keep_copying_targ_2_bench(back_copy,
                                             front_copy,
                                             timedOut_copy,
                                             counter,
                                             reset_lock_flag_c);
        });


    }



    fn keep_copying_bench_2_targ(mut front: TcpStream,
                                 mut back: TcpStream,
                                 timedOut: Arc<AtomicBool>,
                                 reset_lock_flag: Arc<RwLock<bool>>) {
        front.set_read_timeout(Some(Duration::new(15 * 60, 0)));
        let mut buf = [0; 1024];

        loop {

            if *reset_lock_flag.read().unwrap() == true {
                drop(front);
                drop(back);
                return;
            }

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
                    drop(front);
                    drop(back);
                    return; // normal errors, stop
                }
                Ok(r) => r,
            };
            timedOut.store(false, Ordering::Release);
            match back.write(&buf[0..read]) {
                Err(..) => {
                    timedOut.store(true, Ordering::Release);
                    drop(front);
                    drop(back);
                    return;
                }
                Ok(..) => (),
            };

        }

    }



    fn keep_copying_targ_2_bench(mut back: TcpStream,
                                 mut front: TcpStream,
                                 timedOut: Arc<AtomicBool>,
                                 num_responses: ConcurrentCounter,
                                 reset_lock_flag: Arc<RwLock<bool>>) {

        back.set_read_timeout(Some(Duration::new(15 * 60, 0)));
        let mut buf = [0; 1024];

        loop {
            if *reset_lock_flag.read().unwrap() == true {
                drop(front);
                drop(back);
                return;
            }

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
                    drop(back);
                    drop(front);
                    return; // normal errors, stop
                }
                Ok(r) => r,
            };
            num_responses.increment(read);


            timedOut.store(false, Ordering::Release);
            match front.write(&buf[0..read]) {
                Err(..) => {
                    timedOut.store(true, Ordering::Release);
                    drop(back);
                    drop(front);
                    return;
                }
                Ok(..) => (),
            };
        }


    }
}
