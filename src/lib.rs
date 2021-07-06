use std::time::SystemTime;

use serde::{Deserialize, Serialize};
mod latency;


type MeasurementResult = Vec<Datapoint>;

pub trait Measurable {
    fn mean(&self) -> f32 {
        unimplemented!()
    }
}

impl Measurable for MeasurementResult {
    fn mean(&self) -> f32 {
        self.iter().fold(0.0, |acc, e| {
            match e {
                Datapoint::Latency(l, _t) => acc + l,
                Datapoint::ThroughputUp(up, _t) => acc + up,
                Datapoint::ThroughputDown(dn, _t) => acc + dn,
            }
        }) / self.len() as f32
    }
}


pub struct Measurement {
    ping_ip: String,

}

impl Default for Measurement {
    fn default() -> Self {
        Self {
            ping_ip: "8.8.8.8".to_string()
        }
    }
}

impl Measurement {
    pub fn new() -> Self {
        Measurement::default()
    }
    pub fn run(&self) {
        
    }
}


/// A single data point, containing different possible measurements. All of them
/// are time stamped.
#[derive(Serialize, Deserialize, Debug)]
pub enum Datapoint {
    Latency(f32, SystemTime),
    ThroughputUp(f32, SystemTime),
    ThroughputDown(f32, SystemTime),
}


impl Datapoint {
    /// Add a latency `Datapoint`
    fn add_latency(latency: f32) -> Self {
        Datapoint::Latency(latency, SystemTime::now())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn write() {
        let mut log: MeasurementResult = vec![];

        for i in 1..10 {
            log.push(Datapoint::add_latency(i as f32));
        }

        dbg!(&log);
        dbg!(log.mean());
    }
}
