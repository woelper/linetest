#![windows_subsystem = "windows"]
mod app;
use anyhow::{Error, Result};

fn main() -> Result<(), Error> {
    // Start tool with warnings enabled
    std::env::set_var("RUST_LOG", "info");
    let _ = env_logger::try_init();

    let app = app::LinetestApp::default();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
