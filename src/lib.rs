use anyhow::Error;
use chrono::{Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::{create_dir_all, File},
    path::{Path, PathBuf},
    sync::{
        mpsc::{channel, Receiver},
    },
    thread::{self, sleep},
    time::{Duration, SystemTime},
};

/// Latency measurement tools
mod latency;
/// Throughput measurement tools (Download speed)
mod throughput;
use log::{debug, info};
pub type MeasurementResult = Vec<Datapoint>;

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

    fn save<P: AsRef<Path>>(&self, _path: P) -> Result<(), Error> {
        unimplemented!()
    }

    fn load<P: AsRef<Path>>(&mut self, _path: P) -> Result<(), Error> {
        unimplemented!()
    }

    fn duration(&self) -> Duration {
        unimplemented!()
    }
}

impl Measurable for MeasurementResult {
    fn mean_dl(&self) -> f32 {
        let count = self
            .iter()
            .filter(|e| match e {
                Datapoint::ThroughputDown(_, _) => true,
                _ => false,
            })
            .count();

        self.iter().fold(0.0, |acc, e| match e {
            //TODO: using anything as mean calculation is not good, maybe skip these values?
            Datapoint::ThroughputDown(dn, _t) => acc + dn.unwrap_or_default(),
            _ => acc,
        }) / count as f32
    }

    fn mean_latency(&self) -> f32 {
        let count = self
            .iter()
            .filter(|e| match e {
                Datapoint::Latency(d, _) => d.is_some(),
                _ => false,
            })
            .count();

        self.iter().fold(0.0, |acc, e| match e {
            //TODO: using anything as mean calculation is not good, maybe skip these values?
            Datapoint::Latency(l, _t) => acc + l.unwrap_or(0.),
            _ => acc,
        }) / count as f32
    }

    fn timeouts(&self) -> usize {
        self.iter()
            .filter(|e| match e {
                Datapoint::Latency(l, _) => l.is_none(),
                _ => false,
            })
            .count()
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        // make sure parent dir exists
        if let Some(parent) = path.as_ref().parent() {
            if !parent.is_dir() {
                create_dir_all(parent)?;
            }
        }
        let f = File::create(path.as_ref())?;
        serde_json::to_writer(f, self)?;
        Ok(())
    }

    fn load<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Error> {
        *self = serde_json::from_reader(File::open(path.as_ref())?)?;
        Ok(())
    }

    fn duration(&self) -> Duration {
        if let Some(first) = self.first() {
            if let Some(last) = self.last() {
                match first {
                    Datapoint::Latency(_, t)
                    | Datapoint::ThroughputDown(_, t)
                    | Datapoint::ThroughputUp(_, t) => match last {
                        Datapoint::Latency(_, t2)
                        | Datapoint::ThroughputDown(_, t2)
                        | Datapoint::ThroughputUp(_, t2) => {
                            if let Ok(dur) = t2.duration_since(*t) {
                                return dur;
                            }
                        }
                    },
                }
            }
        }
        Duration::from_secs(0)
    }
}

// fn load<P: AsRef<Path>>(&mut self, path: P) -> Result<MeasurementResult, Error> {
//     Ok(serde_json::from_reader(File::open(path.as_ref())?)?)
// }

pub struct MeasurementController {
    /// The IP address to use for latency tests.
    pub ping_ips: Vec<String>,
    pub downloads: Vec<String>,
    pub result: MeasurementResult,
    pub ping_delay: Duration,
    pub logfile: Option<PathBuf>,
}

impl Default for MeasurementController {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            ping_ips: vec!["8.8.8.8".to_string()],
            downloads: vec![
                "https://github.com/aseprite/aseprite/releases/download/v1.2.27/Aseprite-v1.2.27-Source.zip".to_string(),
                "https://dl.google.com/drive-file-stream/GoogleDriveSetup.exe".to_string(),
                "https://awscli.amazonaws.com/AWSCLIV2.msi".to_string(),
                "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip".to_string(),
            ],
            result: vec![],
            ping_delay: Duration::from_secs(7),
            logfile: Some(MeasurementController::get_data_dir().join(format!("{}-{}-{}-{}h{}m.ltst", now.year(), now.month(), now.day(), now.hour(), now.minute())))
        }
    }
}

impl MeasurementController {
    /// Generate a default measurement
    pub fn new() -> Self {
        MeasurementController::default()
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

    pub fn get_data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or(PathBuf::from("."))
            .join("linetest")
    }

    /// Run periodic measurements
    pub fn run_periodic(&self) -> Result<Receiver<Datapoint>, Error> {
        //define how many latency tests to perform before running a download test
        let latency_download_ratio = 10;

        let (sender, receiver) = channel();

        let ping_delay = self.ping_delay;
        let ping_ip = self
            .ping_ips
            .get(0)
            .unwrap_or(&"8.8.8.8".to_string())
            .clone();
        let ping_sender = sender.clone();

        let download_urls = self.downloads.clone();

        thread::spawn(move || {
            let mut stop = false;
            loop {
                if stop {
                    break;
                }

                for _ in 0..latency_download_ratio {
                    if stop {
                        break;
                    }
                    latency::ping_callback(&ping_ip, |duration_result| {
                        match duration_result {
                            Some(duration) => {
                                stop = ping_sender
                                    .send(Datapoint::add_latency(Some(duration.as_secs_f32())))
                                    .is_err()
                            }
                            None => stop = ping_sender.send(Datapoint::add_latency(None)).is_err(),
                        };
                    })
                    .expect("Ping failed on this system");
                    debug!("Waiting {:?} to next speed ping", ping_delay);
                    sleep(ping_delay);
                }

                if stop {
                    break;
                }

                let download_result = throughput::combined_download(&download_urls)
                    .ok()
                    .map(|d| throughput::to_mbits(d));

                stop = sender
                    .send(Datapoint::add_tp_down(download_result))
                    .is_err();
            }
            info!("Stopping thread");
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

        let measurement = MeasurementController::default();

        for url in measurement.downloads {
            let res = throughput::measured_download(&url).unwrap();
            info!("DL {} => {:?}", url, &res);
        }
    }

    #[test]
    fn run() {
        std::env::set_var("RUST_LOG", "info");
        let _ = env_logger::try_init();

        let mut measurement = MeasurementController::default();
        measurement.run().unwrap();

        // info!("{:?}", measurement.result);
    }
}
