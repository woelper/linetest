use anyhow::{Error, Result};
use log::info;
use rayon::prelude::*;
use std::io::Read;
use std::time::{Duration, SystemTime};
use ureq;

type Bytes = usize;
type Mbit = f32;
type DownloadResult = (Duration, Bytes);

pub fn to_mbits(dr: DownloadResult) -> Mbit {
    let (duration, bytes) = dr;
    let mbit: f32 = (bytes as f32 * 8.) / 1000. / 1000.;
    // let mb: f32 = bytes as f32 / 1024. /1024.;
    // info!("secs {}", duration.as_secs_f32());
    info!("Mbit:{}  B:{}", mbit, bytes);
    mbit / duration.as_secs_f32()
}

/// Retrieve a file
pub fn measured_download(url: &str) -> Result<DownloadResult, Error> {
    let t = SystemTime::now();
    let res = ureq::get(url).call()?;
    let mut reader = res.into_reader();
    let mut bytes = vec![];
    reader.read_to_end(&mut bytes)?;
    // let payload = res.into_reader();
    let d = t.elapsed()?;
    // info!("{:?}", d);
    let byte_count = bytes.len();
    Ok((d, byte_count))
}

/// Retrieve multiple files, return the combined size and the time until the last one finishes
pub fn combined_download(urls: &Vec<String>) -> Result<DownloadResult, Error> {
    let t = SystemTime::now();

    let d = urls
        .par_iter()
        .map(|url| measured_download(&url))
        .collect::<Vec<_>>();
    let completion_time = t.elapsed()?;
    let res = d.iter().fold((Duration::ZERO, 0), |mut acc, maybe_res| {
        match maybe_res {
            Ok(res) => {
                acc.1 += res.1;
                // // Check if this duration is longer
                // // since we want to keep the longest duration
                // if res.0 > acc.0 {
                //     acc.0 = res.0;
                // }
                acc
            }
            Err(_e) => acc,
        }
    });
    Ok((completion_time, res.1))
}
