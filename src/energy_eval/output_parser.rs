use std::process::Output;
use std::str;
use BenchmarkName;
use std::error::Error;

#[derive(Clone)]
pub struct Parser {
	pub benchmark_name: BenchmarkName,
}


impl Parser {
	 
	
	pub fn parse(&self, output: Output) -> Option<f64> {
			
			let energy=match self.benchmark_name {
				BenchmarkName::ycsb => self.parse_ycsb(output),
				BenchmarkName::wrk  => self.parse_wrk(output),
				BenchmarkName::memaslap => self.parse_memaslap(output),
			};
			
			return energy;
		
	}


	    /***********************************************************************************************************
	/// **
	/// YCSB
	/// *
		************************************************************************************************************/
	fn parse_ycsb(&self, output: Output) -> Option<f64> {
		
			let chars_2_search="Throughput(ops/sec),";
			let offset_wo_space=0;
			let char_2_delete="";
			let multiplier_constant=1.0; 
			
		
			let buf=output.stdout;
			
			let output_str = match str::from_utf8(buf.as_slice()) {
		        Ok(v) => v,
		        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
		    };
			
			let mut iter=output_str.split_whitespace();
			
			let (mut base, mut index)=(0,0);		
			for elem in iter {
				if elem==chars_2_search{
					base=index;
				}
				index+=1;
			}
			
			
			//println!("REEEE: {:?}",output_str.split_whitespace().nth(base+offset_wo_space));
		
			let raw_value=match output_str.split_whitespace().nth(base+offset_wo_space) {
				Some(v) => v,
				None => return None,
			};
			
			
			let value=raw_value.replace(char_2_delete,"");
			
			let fvalue=match value.parse::<f64>(){
				Ok(v) => v,
				Err(e) => return None,
			};
			
			return Some(fvalue*multiplier_constant);
	}
	
	
	
	    /***********************************************************************************************************
	/// **
	/// WRK
	/// *
		************************************************************************************************************/
	fn parse_wrk(&self, output: Output) -> Option<f64> {
		
			let chars_2_search="Requests/sec:";
			let offset_wo_space=1;
			let char_2_delete="";
			let multiplier_constant=1.0; 
			
		
			let buf=output.stdout;
			
			let output_str = match str::from_utf8(buf.as_slice()) {
		        Ok(v) => v,
		        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
		    };
			
			let mut iter=output_str.split_whitespace();
			
			let (mut base, mut index)=(0,0);		
			for elem in iter {
				if elem==chars_2_search{
					base=index;
				}
				index+=1;
			}
			
			
		
			let raw_value=match output_str.split_whitespace().nth(base+offset_wo_space) {
				Some(v) => v,
				None => {
					println!("Error during parsing - None value");
					return None;
					},
			};
			
			
			let value=raw_value.replace(char_2_delete,"");
			
			let fvalue=match value.parse::<f64>(){
				Ok(v) => v,
				Err(e) => {
					println!("Error during parsing - {:?}",e.description());
					return None;
					},
			};
			
			return Some(fvalue*multiplier_constant);
	}



	    /***********************************************************************************************************
	/// **
	/// MEMASLAP
	/// *
		************************************************************************************************************/
	fn parse_memaslap(&self, output: Output) -> Option<f64> {
		
			let chars_2_search="TPS:"; 
			let offset_wo_space=1;
			let char_2_delete="";
			let multiplier_constant=1.0; 
			
		
			let buf=output.stdout;
			
			let output_str = match str::from_utf8(buf.as_slice()) {
		        Ok(v) => v,
		        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
		    };
			
			let mut iter=output_str.split_whitespace();
			
			let (mut base, mut index)=(0,0);		
			for elem in iter {
				if elem==chars_2_search{
					base=index;
				}
				index+=1;
			}
			
			

			let raw_value=match output_str.split_whitespace().nth(base+offset_wo_space) {
				Some(v) => v,
				None => {
					println!("Error during parsing - None value");
					return None;
					},
			};
			
			
			let value=raw_value.replace(char_2_delete,"");
			
			let fvalue=match value.parse::<f64>(){
				Ok(v) => v,
				Err(e) => {
					println!("Error during parsing - {:?}",e.description());
					return None;
					},
			};
			
			return Some(fvalue*multiplier_constant);
	}
		
}
