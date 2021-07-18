use anyhow::Error;

use std::{
    fs::{create_dir_all, File},
    path::{Path},
    time::{Duration},
};

use super::{MeasurementResult, Datapoint};

/// A couple of analyis methods on a [MeasurementResult]
pub trait Evaluation {
    /// Mean download speed for a measurement
    fn mean_dl(&self) -> f32 {
        unimplemented!()
    }

    /// Mean latency for a measurement
    fn mean_latency(&self) -> Duration {
        unimplemented!()
    }

    /// Sum of all timeouts in a measurement
    fn timeouts(&self) -> usize {
        unimplemented!()
    }

    /// Fraction of timeouts fot the measurements, 0-1, where
    /// 0 is perfect availability and 1 is complete data loss.
    fn timeouts_for_session(&self) -> f32 {
        unimplemented!()
    }

    /// Save the measurement to a file
    #[allow(unused_variables)]
    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        unimplemented!()
    }

    /// Load a file into a measurement
    #[allow(unused_variables)]
    fn load<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Error> {
        unimplemented!()
    }

    /// Total duration of a measurement, from first sample to last
    fn duration(&self) -> Duration {
        unimplemented!()
    }
}

impl Evaluation for MeasurementResult {
    fn mean_dl(&self) -> f32 {
        let count = self
            .iter()
            .filter(|e| match e {
                Datapoint::ThroughputDown(_, _) => true,
                _ => false,
            })
            .count();

        self.iter().fold(0.0, |acc, e| match e {
            Datapoint::ThroughputDown(dn, _t) => acc + dn.unwrap_or_default(),
            _ => acc,
        }) / count as f32
    }

    fn mean_latency(&self) -> Duration {
        let count = self
            .iter()
            .filter(|e| match e {
                Datapoint::Latency(d, _) => d.is_some(),
                _ => false,
            })
            .count();

        self.iter()
            .filter(|e| match e {
                Datapoint::Latency(d, _) => d.is_some(),
                _ => false,
            })
            .fold(Duration::from_secs(0), |acc, e| match e {
                //TODO: using anything as mean calculation is not good, maybe skip these values?
                Datapoint::Latency(l, _t) => acc + l.unwrap_or(Duration::from_secs(0)),
                _ => acc,
            })
            / count as u32
    }

    fn timeouts(&self) -> usize {
        self.iter()
            .filter(|e| match e {
                Datapoint::Latency(l, _) => l.is_none(),
                _ => false,
            })
            .count()
    }

    fn timeouts_for_session(&self) -> f32 {
        self.timeouts() as f32 / self.len() as f32
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