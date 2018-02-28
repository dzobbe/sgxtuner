use influent::create_client;
use influent::client::{Client, Credentials};
use influent::measurement::{Measurement, Value};

use std::time::{SystemTime, UNIX_EPOCH};
use State;

#[derive(Debug, Clone)]
pub struct InfluxEmitter {
    pub address: String,
    pub username: String,
    pub password: String,
    pub database: String,
}


impl InfluxEmitter {
    pub fn new(address: String, user: String, pwd: String, db: String) -> Self {
        InfluxEmitter {
            address: address,
            username: user,
            password: pwd,
            database: db,
        }
    }

    pub fn send_update(
        &mut self,
        temperature: f64,
        time: f64,
        cputime: f64,
        measured_val: f64,
        measured_state: &State,
        best_val: f64,
        best_state: &State,
        num_iter: usize,
        tid: usize,
    ) {
        let credentials = Credentials {
            username: self.username.as_str(),
            password: self.password.as_str(),
            database: self.database.as_str(),
        };

        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH).expect(
            "Time went backwards",
        );
        let timestamp: i64 = (since_the_epoch.as_secs() as i64) * 1000000000i64;

        let client = create_client(credentials, vec![self.address.as_str()]);
        let mut measurement_metrics = Measurement::new("Tuner Metrics");
        println!("{:?}", timestamp);
        measurement_metrics.set_timestamp(timestamp);
        measurement_metrics.add_field("Step", Value::Integer(num_iter as i64));
        measurement_metrics.add_field("Temperature", Value::Float(temperature));
        measurement_metrics.add_field("Measured NRG", Value::Float(measured_val));
        measurement_metrics.add_field("Best NRG", Value::Float(best_val));

        client.write_one(measurement_metrics, None);

        let mut measurement_current_states = Measurement::new("Tuner Measured States");
        measurement_current_states.set_timestamp(timestamp);
        for (param, value) in measured_state.iter() {
            measurement_current_states.add_field(param, Value::String(value));
        }

        client.write_one(measurement_current_states, None);

        let mut measurement_best_states = Measurement::new("Tuner Best States");
        measurement_best_states.set_timestamp(timestamp);
        for (param, value) in best_state.iter() {
            measurement_best_states.add_field(param, Value::String(value));
        }

        client.write_one(measurement_best_states, None);


    }
}
