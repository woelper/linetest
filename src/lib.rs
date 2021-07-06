use std::time::SystemTime;

use serde::{Deserialize, Serialize};
mod latency;


type MeasurementResult = Vec<Datapoint>;

pub trait Measurable {
    fn mean_latency(&self) -> f32 {
        unimplemented!()
    }
}

impl Measurable for MeasurementResult {
    fn mean_latency(&self) -> f32 {
        self.iter().fold(0.0, |acc, e| acc + e.latency) / self.len() as f32
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

}

/// A single network measurement point
#[derive(Serialize, Deserialize, Debug)]
pub struct Datapoint {
    pub latency: f32,
    pub throughput_up: f32,
    pub throughput_down: f32,
    pub timestamp: SystemTime,
}

impl Datapoint {
    fn new(latency: f32, throughput_up: f32, throughput_down: f32) -> Self {
        Self {
            latency,
            throughput_up,
            throughput_down,
            timestamp: SystemTime::now(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn write() {
        let mut log: MeasurementResult = vec![];

        for i in 1..10 {
            log.push(Datapoint::new(i as f32, 10., 5.));
        }

        dbg!(&log);
        dbg!(log.mean_latency());
    }
}
