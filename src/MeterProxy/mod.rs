extern crate libc;

use std::net::{TcpListener, TcpStream};
use std::sync::{Arc,Mutex,Condvar};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{thread, time};
use std::time::Duration;
use std::io::prelude::*;
use std::process::{Command, Child};
use std::sync::atomic::AtomicUsize;
use self::libc::setrlimit;
use std::sync::mpsc::{channel,Receiver,Sender};
use std::sync::mpsc;
use std::sync::RwLock;


#[derive(Clone)]
pub struct ConcurrentCounter(Arc<Mutex<usize>>);

impl ConcurrentCounter {
    pub fn new(val: usize) -> Self {
        ConcurrentCounter(Arc::new(Mutex::new(val)))
    }

    pub fn increment(&self) {
		 let mut counter = self.0.lock().unwrap();
		 *counter = *counter + 1;
    }
    
    pub fn get(&self) -> usize {
       let counter = self.0.lock().unwrap();
       *counter
    }
    
     pub fn reset(&self) {
		 let mut counter = self.0.lock().unwrap();
		 *counter = 0;
    }
}

#[derive(Clone)]
pub struct Meter {
	pub num_target_responses: ConcurrentCounter
}

impl  Meter {	
	
	pub fn new() -> Meter {
		       Meter{
		   			num_target_responses: ConcurrentCounter::new(0),
		   			}
			} 
	
	
	pub fn start(&self, port_target: u16, port_proxy: u16, reset_lock_flag: Arc<RwLock<bool>>){
		//Increase the limit of resources for sockets limits (this avoids exception: "Too many open files (os error 24)")
		let rlim=libc::rlimit{rlim_cur: 4096, rlim_max: 4096};
		unsafe{
			libc::setrlimit(libc::RLIMIT_NOFILE,&rlim);
		}
	    
	    let mut acceptor = TcpListener::bind("127.0.0.1:12349").unwrap();
	    let mut children = vec![];

	    for stream in acceptor.incoming() {
	    	
			let counter=self.num_target_responses.clone();
			let reset_lock_flag_c=reset_lock_flag.clone();
			
			match stream {
				Err(e) => println!("Strange connection broken: {}", e),
				Ok(stream) => {
						children.push(thread::spawn(move|| {
							// connection succeeded
							let mut stream_c=stream.try_clone().unwrap();
		 					
							stream_c.set_read_timeout(Some(Duration::new(3,0)));
							let mut header=[0;1];
						
							match stream_c.read_exact(&mut header) {
								Err(..) => None,
								Ok(b) => Some(b)
							};
						    Meter::start_pipe(stream_c, port_target, Some(header[0]), counter, reset_lock_flag_c);
						}));
						
					}
			}
	    } 
	    

	    for child in children {
	        // Wait for the thread to finish. Returns a result.
	        let _ = child.join();
	        println!("Joining!!");
    	}
	    
	    drop(acceptor);
	}

	pub fn reset_resources(&self){
		self.num_target_responses.reset();
		let (tx, _): (Sender<i32>, Receiver<i32>) = mpsc::channel();
            tx.send(5).unwrap();

		//let mut needed_reset = self.reset_mutex.lock().unwrap();
   		//*needed_reset = true;
    	//self.reset_flag.notify_all();
	}
	
	
	fn start_pipe(front: TcpStream, port: u16, header: Option<u8>, counter: ConcurrentCounter, reset_lock_flag: Arc<RwLock<bool>>) {
		let mut back = match TcpStream::connect(("127.0.0.1", 12347)) {
			Err(e) => { 
				println!("Error connecting to target application: {}", e); 
				drop(front);
				return;
			},
			Ok(b) => b
		};
		if header.is_some() {
			let mut buf_header=[0;1];
			buf_header[0]=header.unwrap();
			match back.write(&mut buf_header) {
				Err(e) => { 
					println!("Error writing first byte to target: {}", e); 
					drop(back); 
					drop(front); 
					return;
				},
				Ok(..) => ()
			};
		}
		
		let front_copy = front.try_clone().unwrap();
		let back_copy = back.try_clone().unwrap();
	
		let timedOut = Arc::new(AtomicBool::new(false));
		let timedOut_copy = timedOut.clone();

		let reset_lock_flag_c=reset_lock_flag.clone();
		let reset_lock_flag_c2=reset_lock_flag.clone();
		let child_f_b=thread::spawn(move|| {
			Meter::keep_copying_bench_2_targ(front, back, timedOut,reset_lock_flag);
		});
		let child_b_f=thread::spawn(move|| {
			Meter::keep_copying_targ_2_bench(back_copy, front_copy, timedOut_copy, counter,reset_lock_flag_c);
		});
		
		
		loop {
			if *reset_lock_flag_c2.read().unwrap() == true{
				println!("RAISED RESET FLAG");
				
			}
		}
		
		child_f_b.join();		
		child_b_f.join();
	}
	
	
	
	fn keep_copying_bench_2_targ(mut front: TcpStream, mut back: TcpStream, timedOut: Arc<AtomicBool>, reset_lock_flag: Arc<RwLock<bool>>) {
		front.set_read_timeout(Some(Duration::new(15*60,0)));
		let mut buf = [0; 1024];
		let mut index=0;
		
		while *reset_lock_flag.try_read().unwrap() != true {	
			
			if *reset_lock_flag.read().unwrap() == true{
				println!("RAISED RESET FLAG");
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
				},
				Ok(r) => r
			};
			timedOut.store(false, Ordering::Release);
			match back.write(&buf[0 .. read]) {
				Err(..) => {  
					timedOut.store(true, Ordering::Release);
					drop(front); 
					drop(back);
					return;
				},
				Ok(..) => ()
			};
			
		}
	}
	
	
	
	fn keep_copying_targ_2_bench(mut back: TcpStream, mut front: TcpStream, timedOut: Arc<AtomicBool>, 
										num_responses: ConcurrentCounter, reset_lock_flag: Arc<RwLock<bool>>) {
											
		back.set_read_timeout(Some(Duration::new(15*60,0)));
		let mut buf = [0; 1024];
		let mut index=0;

		while *reset_lock_flag.try_read().unwrap() != true {	
			//println!("FLAG: {:?}",*reset_lock_flag.try_read().unwrap());
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
				},
				Ok(r) => {
					num_responses.increment();
	            	
					r
				}
			};
			
			timedOut.store(false, Ordering::Release);
			match front.write(&buf[0 .. read]) {
				Err(..) => {  
					timedOut.store(true, Ordering::Release);
					drop(back); 
					drop(front);
					return;
				},
				Ok(..) => ()
			};
		}
		
		println!("RAISED RESET FLAG");
		drop(front); 
		drop(back);
	}
}
