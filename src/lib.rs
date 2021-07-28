use anyhow::Error;
use chrono::{Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt, fs::read_dir, path::{PathBuf}, sync::mpsc::{channel, Receiver}, thread::{self, sleep}, time::{Duration, SystemTime}};
use log::{debug, info};

/// Latency measurement tools
mod latency;
/// Throughput measurement tools (Download speed)
mod throughput;

/// Evaluation tools
mod eval;
pub use eval::Evaluation;

/// The result of a measurement, just a Vec of [Datapoint]s.
pub type MeasurementResult = Vec<Datapoint>;


/// A structure to set up and start a network measurement
#[derive(Debug, Clone)]
pub struct MeasurementBuilder {
    /// The IP address to use for latency tests. Currently, only  the first one is used.
    pub ping_ips: Vec<String>,
    /// the urls of files to download. The speedtest will be evaluated by downloading all of them
    /// in parallel and measuring the time.
    pub downloads_urls: Vec<String>,
    /// The delay between pings
    pub ping_delay: Duration,
    /// The path to a logfile. Will be used if not `None`.
    pub logfile: Option<PathBuf>,
}

impl Default for MeasurementBuilder {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            ping_ips: vec!["8.8.8.8".to_string()],
            downloads_urls: vec![
                "https://github.com/aseprite/aseprite/releases/download/v1.2.27/Aseprite-v1.2.27-Source.zip".to_string(),
                "https://dl.google.com/drive-file-stream/GoogleDriveSetup.exe".to_string(),
                "https://awscli.amazonaws.com/AWSCLIV2.msi".to_string(),
                "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip".to_string(),
            ],
            ping_delay: Duration::from_secs(7),
            logfile: Some(MeasurementBuilder::get_data_dir().join(format!("{}-{}-{}-{}h{}m.ltst", now.year(), now.month(), now.day(), now.hour(), now.minute())))
        }
    }
}

impl MeasurementBuilder {
    /// Generate a default measurement
    pub fn new() -> Self {
        MeasurementBuilder::default()
    }

    pub fn with_aws_payload(&self) -> Self {
        Self {
            downloads_urls: vec![
                "https://d1dgjrknbc1uuw.cloudfront.net/2M".to_string(),
                "https://d1dgjrknbc1uuw.cloudfront.net/1M".to_string(),
                "https://d1dgjrknbc1uuw.cloudfront.net/4M".to_string(),
            ],
            ..self.to_owned()
        }
    }

    /// Return the directory containing measurement results
    pub fn get_data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or(PathBuf::from("."))
            .join("linetest")
    }


    // discover all log files present on this system
    pub fn get_logs() -> Result<Vec<PathBuf>, Error> {
        Ok(read_dir(Self::get_data_dir())?
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect::<Vec<_>>())
    }

    /// Execute a measurement once
    pub fn run_once(&self) -> Result<MeasurementResult, Error> {
        let mut result: MeasurementResult = vec![];
        latency::ping_callback(
            &self
                .ping_ips
                .get(0)
                .unwrap_or(&"8.8.8.8".to_string())
                .clone(),
            |duration_result| {
                match duration_result {
                    Some(duration) => result.push(Datapoint::add_latency(Some(duration))),
                    None => result.push(Datapoint::add_latency(None)),
                };
            },
        )?;

        debug!("Seq: {:?}", result);

        let mbits = throughput::combined_download(&self.downloads_urls)
            .ok()
            .map(|dl| throughput::to_mbits(dl));
        result.push(Datapoint::add_tp_down(mbits));
        Ok(result)
    }
    pub fn run_until_receiver_drops(&self) -> Result<Receiver<Datapoint>, Error> {
        self.run_advanced(None)
    }

    pub fn run_until_duration(&self, duration: Duration) -> Result<Receiver<Datapoint>, Error> {
        self.run_advanced(Some(duration))
    }

    /// Run periodic measurements to a Receiver containing [Datapoint]s
    pub fn run_advanced(&self, duration: Option<Duration>) -> Result<Receiver<Datapoint>, Error> {
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

        let download_urls = self.downloads_urls.clone();

        thread::spawn(move || {
            info!("Start thread");

            let start = SystemTime::now();
            let mut stop = false;
            loop {

                if let Some(d)= duration {
                    if start.elapsed().unwrap_or_default() > d {
                        info!("Test concluded after specified duration ({:?})", d);
                        break;
                    }
                }

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
                                    .send(Datapoint::add_latency(Some(duration)))
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
/// are time stamped. If a measurement failed, the `Option` is `None`.
#[derive(Serialize, Deserialize, Debug)]
pub enum Datapoint {
    Latency(Option<Duration>, SystemTime),
    ThroughputUp(Option<f32>, SystemTime),
    ThroughputDown(Option<f32>, SystemTime),
}

impl Datapoint {
    /// Add a latency `Datapoint`
    pub fn add_latency(latency: Option<Duration>) -> Self {
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
                "Ping:\t{:.2} ms",
                l.map(|d| (d.as_secs_f32() * 1000.).to_string())
                    .unwrap_or("Timeout".to_string())
            ),
            Datapoint::ThroughputUp(up, _t) => write!(
                f,
                "Upload speed: {:.1} Mbit/s",
                up.map(|d| d.to_string()).unwrap_or("Timeout".to_string())
            ),
            Datapoint::ThroughputDown(dn, _t) => write!(
                f,
                "Speed:\t{} Mbit/s",
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
                info!("res {:?}", duration_result);

                match duration_result {
                    Some(duration) => log.push(Datapoint::add_latency(Some(duration))),
                    None => log.push(Datapoint::add_latency(None)),
                };
            })
            .expect("Can't ping on this system");
        }

        info!("{:?}", &log);
        info!("{:?}", &log.mean_dl());
    }

    #[test]
    fn throughput_all_urls() {
        std::env::set_var("RUST_LOG", "info");
        let _ = env_logger::try_init();
        let measurement = MeasurementBuilder::default();
        for url in measurement.downloads_urls {
            let res = throughput::measured_download(&url).unwrap();
            info!("DL {} => {:?}", url, &res);
        }
        let measurement = MeasurementBuilder::default().with_aws_payload();
        for url in measurement.downloads_urls {
            let res = throughput::measured_download(&url).unwrap();
            info!("DL {} => {:?}", url, &res);
        }
    }



    #[test]
    fn run() {
        std::env::set_var("RUST_LOG", "info");
        let _ = env_logger::try_init();
        let measurement = MeasurementBuilder::default();
        measurement.run_once().unwrap();
    }

    #[test]
    fn run_aws() {
        std::env::set_var("RUST_LOG", "info");
        let _ = env_logger::try_init();
        let measurement = MeasurementBuilder::default().with_aws_payload();
        let res = measurement.run_once().unwrap();
        info!("{:#?}", res);
    }

    #[test]
    fn gen_save() {
        std::env::set_var("RUST_LOG", "debug");
        let _ = env_logger::try_init();

        let mut measurement = MeasurementBuilder::default();
        measurement.logfile = Some(MeasurementBuilder::get_data_dir().join("example_manual.ltst"));
        measurement.ping_delay = Duration::from_secs(2);
        
        let mut log: MeasurementResult = vec![];

        for i in 1..10 {
            info!("Ping {}", i);
            latency::ping_callback("8.8.8.8", |duration_result| {
                info!("res {:?}", duration_result);

                thread::sleep(measurement.ping_delay);

                match duration_result {
                    Some(duration) => log.push(Datapoint::add_latency(Some(duration))),
                    None => log.push(Datapoint::add_latency(None)),
                };

                // thread::sleep(measurement.ping_delay);

                // log.push(Datapoint::add_latency(None));

            })
            .expect("Can't ping on this system");
        }

        log.save(measurement.logfile.unwrap()).unwrap();

        let mut auto_log = vec![];
        measurement.logfile = Some(MeasurementBuilder::get_data_dir().join("example_auto.ltst"));
        let receiver = measurement.run_until_duration(Duration::from_secs(20)).unwrap();

        info!("sleeping");
        thread::sleep(Duration::from_secs(25));
        info!("sleeping done");

        for dp in receiver.try_iter() {
            auto_log.push(dp);

        }
        auto_log.save(measurement.logfile.unwrap()).unwrap();


    }


}
