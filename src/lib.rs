use anyhow::Error;
use chrono::{Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt, fs::File, path::Path, sync::mpsc::{channel, Receiver}, thread::{self, sleep}, time::{Duration, SystemTime}};

/// Latency measurement tools
mod latency;
/// Throughput measurement tools (Download speed)
mod throughput;
use log::{debug, info};
type MeasurementResult = Vec<Datapoint>;

pub trait Measurable {
    fn mean_dl(&self) -> f32 {
        unimplemented!()
    }

    fn mean_latency(&self) -> f32 {
        unimplemented!()
    }

    fn timeouts(&self) -> usize {
        unimplemented!()
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        unimplemented!()

    }
}

impl Measurable for MeasurementResult {
    fn mean_dl(&self) -> f32 {
        let count = self.iter().filter(|e| match e {
            Datapoint::ThroughputDown(_,_) => true,
            _ => false
        }).count();

        self.iter().fold(0.0, |acc, e| match e {
            //TODO: using anything as mean calculation is not good, maybe skip these values?
            Datapoint::ThroughputDown(dn, _t) => acc + dn.unwrap_or_default(),
            _ => acc
        }) / count as f32
    }

    fn mean_latency(&self) -> f32 {
        let count = self.iter().filter(|e| match e {
            Datapoint::Latency(d,_) => d.is_some(),
            _ => false
        }).count();

        self.iter().fold(0.0, |acc, e| match e {
            //TODO: using anything as mean calculation is not good, maybe skip these values?
            Datapoint::Latency(l, _t) => acc + l.unwrap_or(0.),
            _ => acc
        }) / count as f32
    }

    fn timeouts(&self) -> usize {
        self.iter().filter(|e| match e {
            Datapoint::Latency(l,_) => l.is_none(),
            _ => false
        }).count()
    }
    
    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let f= File::create(path.as_ref())?;
        serde_json::to_writer(f, self)?;
        Ok(())
    }
}

pub struct Measurement {
    /// The IP address to use for latency tests.
    pub ping_ips: Vec<String>,
    pub downloads: Vec<String>,
    pub result: MeasurementResult,
    pub ping_delay: Duration,
    pub throughput_delay: Duration,
    pub logfile: Option<String>,
}

impl Default for Measurement {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            ping_ips: vec!["8.8.8.8".to_string()],
            downloads: vec![
                "https://github.com/aseprite/aseprite/releases/download/v1.2.27/Aseprite-v1.2.27-Source.zip".to_string(),
                // "http://87.76.21.20/test.zip".to_string(),
                "https://dl.google.com/drive-file-stream/GoogleDriveSetup.exe".to_string(),
                "https://awscli.amazonaws.com/AWSCLIV2.msi".to_string(),
                "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip".to_string(),
            ],
            result: vec![],
            ping_delay: Duration::from_secs(7),
            throughput_delay: Duration::from_secs(30),
            logfile: Some(format!("{}-{}-{}-{}", now.year(),now.month(), now.day(), now.minute()))
        }
    }
}

impl Measurement {
    /// Generate a default measurement
    pub fn new() -> Self {
        Measurement::default()
    }

    /// Execute a measurement once
    pub fn run(&mut self) -> Result<(), Error> {
        latency::ping_callback(
            &self
                .ping_ips
                .get(0)
                .unwrap_or(&"8.8.8.8".to_string())
                .clone(),
            |duration_result| {
                match duration_result {
                    Some(duration) => self
                        .result
                        .push(Datapoint::add_latency(Some(duration.as_secs_f32()))),
                    None => self.result.push(Datapoint::add_latency(None)),
                };
            },
        )?;

        info!("Seq: {:?}", self.result);

        let d = throughput::combined_download(&self.downloads).unwrap();
        info!("Combined rayon: {:?}", d);
        info!("Combined rayon: {:?}", throughput::to_mbits(d));
        // Ok(Datapoint::add_latency(Some(duration.as_secs_f32())))
        Ok(())
    }

    pub fn run_periodic(&self) -> Result<Receiver<Datapoint>, Error> {
        let (sender, receiver) = channel();

        let ping_delay = self.ping_delay;
        let ping_ip = self
            .ping_ips
            .get(0)
            .unwrap_or(&"8.8.8.8".to_string())
            .clone();
        let ping_sender = sender.clone();
        thread::spawn(move || loop {
            latency::ping_callback(&ping_ip, |duration_result| {
                match duration_result {
                    Some(duration) => ping_sender
                        .send(Datapoint::add_latency(Some(duration.as_secs_f32())))
                        .unwrap(),
                    None => ping_sender.send(Datapoint::add_latency(None)).unwrap(),
                };
            })
            .expect("Ping failed on this system");
            debug!("Waiting {:?} to next speed ping", ping_delay);
            sleep(ping_delay);
        });

        let throughput_delay = self.throughput_delay;
        let urls = self.downloads.clone();
        thread::spawn(move || loop {
            let download_result = throughput::combined_download(&urls)
                .ok()
                .map(|d| throughput::to_mbits(d));

            let _ = sender.send(Datapoint::add_tp_down(download_result));
            debug!("Waiting {:?} to next speed test", throughput_delay);
            sleep(throughput_delay);
        });

        Ok(receiver)
    }
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

impl fmt::Display for Datapoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Datapoint::Latency(l, _t) => write!(
                f,
                "Ping: {} ms",
                l.map(|d| (d * 1000.).to_string())
                    .unwrap_or("Timeout".to_string())
            ),
            Datapoint::ThroughputUp(up, _t) => write!(
                f,
                "Upload speed: {} Mbit/s",
                up.map(|d| d.to_string()).unwrap_or("Timeout".to_string())
            ),
            Datapoint::ThroughputDown(dn, _t) => write!(
                f,
                "Download speed: {} Mbit/s",
                dn.map(|d| d.to_string()).unwrap_or("Timeout".to_string())
            ),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    // use log::*;

    #[test]
    fn latency() {
        std::env::set_var("RUST_LOG", "debug");
        let _ = env_logger::try_init();

        let mut log: MeasurementResult = vec![];

        for i in 1..10 {
            info!("Ping {}", i);
            latency::ping_callback("8.8.8.8", |duration_result| {
                match duration_result {
                    Some(duration) => {
                        log.push(Datapoint::add_latency(Some(duration.as_secs_f32())))
                    }
                    None => log.push(Datapoint::add_latency(None)),
                };
            })
            .unwrap();
        }

        info!("{:?}", &log);
        info!("{:?}", &log.mean_dl());
    }

    #[test]
    fn throughput_all_urls() {
        std::env::set_var("RUST_LOG", "debug");
        let _ = env_logger::try_init();

        let measurement = Measurement::default();

        for url in measurement.downloads {
            let res = throughput::measured_download(&url).unwrap();
            info!("DL {} => {:?}", url, &res);
        }
    }

    #[test]
    fn run() {
        std::env::set_var("RUST_LOG", "info");
        let _ = env_logger::try_init();

        let mut measurement = Measurement::default();
        measurement.run().unwrap();

        // info!("{:?}", measurement.result);
    }
}
