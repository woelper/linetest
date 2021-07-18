#![windows_subsystem = "windows"]
mod app;
use anyhow::{Error, Result};
use linetest::MeasurementBuilder;
use std::time::SystemTime;

fn main() -> Result<(), Error> {
    // Start tool with warnings enabled
    std::env::set_var("RUST_LOG", "info");
    let _ = env_logger::try_init();

    let measurement = MeasurementBuilder::default();

    let mut app = app::LinetestApp::default();
    app.log_file = measurement.logfile.unwrap_or_default();
    
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
