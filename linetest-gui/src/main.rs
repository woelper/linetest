
mod app;
use std::time::Duration;

// When compiling natively:
fn main() {
    let mut measurement = linetest::Measurement::default();
    // measurement.ping_delay = Duration::from_secs(1);
    let app = app::TemplateApp{
        receiver: measurement.run_periodic().unwrap(),
        datapoints: vec![],
    };
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}