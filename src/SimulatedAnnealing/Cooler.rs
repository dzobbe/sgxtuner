
pub struct TimeCooler{
    pub max_time: u64,
    pub min_temp: f64,
    pub max_temp: f64
}

pub struct StepsCooler{
    pub max_steps: u64,
    pub min_temp: f64,
    pub max_temp: f64
}

pub trait Cooler {
	fn exponential_cooling(&self, metric :u64) -> f64; 
	fn linear_cooling(&self) -> f64;
	fn adaptive_cooling(&self) -> f64;
}



impl Cooler for TimeCooler {
    fn exponential_cooling(&self, elapsed_time :u64) -> f64 {
       	return 	0.0;
	}
    
    fn linear_cooling(&self) -> f64 {
    	return 0.0;
    	
	}
    
    fn adaptive_cooling(&self) -> f64 {
    	return 0.0;
	}
}


impl Cooler for StepsCooler {
	
    fn exponential_cooling(&self, step :u64) -> f64 {
    	if self.min_temp <= 0.0 {
    		panic!("Exponential cooling requires a minimum temperature greater than zero");
    	}
       
       	let reduction_factor = -(self.max_temp / self.min_temp).ln();
       	
       	return 	self.max_temp * (reduction_factor * (step as f64) / (self.max_steps as f64)).exp();
	}
    
    fn linear_cooling(&self) -> f64 {
    	return 0.0;
    	
	}
    
    fn adaptive_cooling(&self) -> f64 {
    	
    	return 0.0;
	}
}
