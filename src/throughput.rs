use std::time::Duration;

use anyhow::{Result, Error};


type Bytes = usize;
type DownloadResult = (Duration, Bytes);

/// Retrieve a file
pub fn measured_download(url: &str) -> Result<DownloadResult, Error> {

    Ok((Duration::from_millis(2000), 2000))
}


