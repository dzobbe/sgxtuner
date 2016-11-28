
use influent::create_client;
use influent::client::{Client, Credentials};
use influent::measurement::{Measurement, Value};

pub struct InfluxProxy {
	host:	  String,
	port:	  u32,
	username: String,
    password: String,
    database: String
}

impl InfluxProxy {
		///TODO: Proxy to transfer metrics from the Simulated Annealing to InfluxDB	
}

