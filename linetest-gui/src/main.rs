#![windows_subsystem = "windows"]
mod app;
use anyhow::{Error, Result};
use linetest::MeasurementController;
use std::fs::read_dir;
use std::path::PathBuf;

/// discover all log files present on this system
fn get_logs() -> Result<Vec<PathBuf>, Error> {
    Ok(read_dir(MeasurementController::get_data_dir())?
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect::<Vec<_>>())
}

fn main() -> Result<(), Error> {
    // Start tool with warnings enabled
    std::env::set_var("RUST_LOG", "warning");
    let _ = env_logger::try_init();

    let measurement = MeasurementController::default();

    // get a list of log files from this system
    let logs = get_logs().unwrap_or_default();

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
