pub mod csv_emitter;
pub mod influx_emitter;

use State;

#[derive(Debug, Clone)]
pub struct Emitter {
    pub influx_res_emitter: influx_emitter::InfluxEmitter,
    pub csv_res_emitter: csv_emitter::CSVEmitter,
}


impl Emitter {
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

        self.influx_res_emitter.send_update(
            temperature,
            time,
            cputime,
            measured_val,
            measured_state,
            best_val,
            best_state,
            num_iter,
            tid,
        );
        self.csv_res_emitter.send_update(
            temperature,
            time,
            cputime,
            measured_val,
            measured_state,
            best_val,
            best_state,
            num_iter,
            tid,
        );


    }
}
