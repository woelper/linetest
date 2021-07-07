use std::time::SystemTime;
use serde::{Deserialize, Serialize};
/// Latency measurement tools
mod latency;
/// Throughput measurement tools (Download speed)
mod throughput;

type MeasurementResult = Vec<Datapoint>;

pub trait Measurable {
    fn mean(&self) -> f32 {
        unimplemented!()
    }
}

impl Measurable for MeasurementResult {
    fn mean(&self) -> f32 {
        self.iter().fold(0.0, |acc, e| match e {
            //TODO: using anything as mean calculation is not good, maybe skip these values? 
            Datapoint::Latency(l, _t) => acc + l.unwrap_or(1000.),
            Datapoint::ThroughputUp(up, _t) => acc + up.unwrap_or_default(),
            Datapoint::ThroughputDown(dn, _t) => acc + dn.unwrap_or_default(),
        }) / self.len() as f32
    }
}

pub struct Measurement {
    /// The IP address to use for latency tests.
    pub ping_ip: String,
    pub downloads: Vec<String>
}

impl Default for Measurement {
    fn default() -> Self {
        Self {
            ping_ip: "8.8.8.8".to_string(),
            downloads: vec!["https://github.com/aseprite/aseprite/releases/download/v1.2.27/Aseprite-v1.2.27-Source.zip".to_string()]
        }
    }
}

impl Measurement {
    pub fn new() -> Self {
        Measurement::default()
    }
    pub fn run(&self) {}
}

/// A single data point, containing different possible measurements. All of them
/// are time stamped.
#[derive(Serialize, Deserialize, Debug)]
pub enum Datapoint {
    Latency(Option<f32>, SystemTime),
    ThroughputUp(Option<f32>, SystemTime),
    ThroughputDown(Option<f32>, SystemTime),
}

impl Datapoint {
    /// Add a latency `Datapoint`
    pub fn add_latency(latency: Option<f32>) -> Self {
        Datapoint::Latency(latency, SystemTime::now())
    }

    /// Add a throughput upload `Datapoint`
    pub fn add_tp_up(tp: Option<f32>) -> Self {
        Datapoint::ThroughputUp(tp, SystemTime::now())
    }

    /// Add a throughput download `Datapoint`
    pub fn add_tp_down(tp: Option<f32>) -> Self {
        Datapoint::ThroughputDown(tp, SystemTime::now())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use log::*;


    #[test]
    fn write() {
        std::env::set_var("RUST_LOG", "debug");
        let _ = env_logger::try_init();

        let mut log: MeasurementResult = vec![];

        for i in 1..10 {
            info!("Ping {}", i);
            latency::ping_callback("8.8.8.8", |duration_result| {
                match duration_result {
                    Some(duration) => log.push(Datapoint::add_latency(Some(duration.as_secs_f32()))),
                    None => log.push(Datapoint::add_latency(None)),
                };
            }).unwrap();
        }

        info!("{:?}", &log);
        info!("{:?}", &log.mean());
    }
}
