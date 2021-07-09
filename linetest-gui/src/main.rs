
mod app;

// When compiling natively:
fn main() {
    let measurement = linetest::Measurement::default();
    let app = app::TemplateApp{
        receiver: measurement.run_periodic().unwrap(),
        datapoints: vec![],
    };
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}