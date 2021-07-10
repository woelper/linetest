mod app;
use anyhow::{Error, Result};
use linetest::Measurement;
use std::fs::read_dir;
use std::path::PathBuf;
use std::time::Duration;

fn get_logs() -> Result<Vec<PathBuf>, Error> {
    Ok(read_dir(Measurement::get_data_dir())?
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect::<Vec<_>>())
}

// When compiling natively:
fn main() -> Result<(), Error> {
    let measurement = Measurement::default();
    // measurement.ping_delay = Duration::from_secs(1);

    let logs = get_logs()?;

    let app = app::LinetestApp {
        receiver: Some(measurement.run_periodic()?),
        datapoints: vec![],
        logs,
        log_index: 0,
        log_file: measurement.logfile.unwrap_or_default(),
    };
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
