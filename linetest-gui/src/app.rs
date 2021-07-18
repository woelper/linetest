use super::get_logs;
use eframe::egui::plot::Points;
use eframe::egui::Color32;
use eframe::{egui, epi};
use egui::plot::{Line, Plot, Value, Values};
use linetest::{self, Datapoint, Evaluation, MeasurementBuilder};
use std::ffi::OsStr;
use std::time::SystemTime;
use std::{path::PathBuf, sync::mpsc::Receiver};
/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct LinetestApp {
    // Example stuff:
    // label: String,
    pub receiver: Option<Receiver<Datapoint>>,
    pub datapoints: Vec<Datapoint>,
    pub logs: Vec<PathBuf>,
    pub log_index: usize,
    pub log_file: PathBuf,
    pub startup_time: SystemTime,
}

impl epi::App for LinetestApp {
    fn name(&self) -> &str {
        "linetest"
    }

    /// Called by the framework to load old app state (if any).
    #[cfg(feature = "persistence")]
    fn load(&mut self, storage: &dyn epi::Storage) {
        *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
    }

    /// Called by the frame work to save state before shutdown.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        let Self {
            receiver,
            datapoints,
            logs,
            log_index,
            log_file,
            startup_time,
        } = self;

        ctx.request_repaint();
        if let Some(valid_receiver) = receiver {
            for dp in valid_receiver.try_iter() {
                datapoints.push(dp);
                let _ = datapoints.save(&log_file);
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Info");

            ui.label(format!("{} samples", datapoints.len()));
            ui.label(format!("Time: {:.1}s", datapoints.duration().as_secs_f64()));
            ui.label(format!("{:.1} Mbit/s down", datapoints.mean_dl()));
            ui.label(format!(
                "{:.1} ms mean latency",
                datapoints.mean_latency().as_millis()
            ));
            ui.label(format!("{} timeouts", datapoints.timeouts()));
            ui.label(format!(
                "{:.1} % timeout ",
                datapoints.timeouts_for_session() * 100.
            ));

            if egui::ComboBox::from_label("Log")
                .show_index(ui, log_index, logs.len(), |i| {
                    logs.get(i)
                        .unwrap_or(&PathBuf::from("None"))
                        .file_name()
                        .unwrap_or(OsStr::new("no_file_name"))
                        .to_string_lossy()
                        .to_string()
                })
                .changed()
            {
                *receiver = None;
                if let Some(log) = logs.get(*log_index) {
                    datapoints.load(log).unwrap();
                }
            }

            if receiver.is_none() {
                if ui.button("New session").clicked() {
                    let measurement = MeasurementBuilder::default();
                    if let Some(log) = &measurement.logfile {
                        *log_file = log.clone();
                    }
                    *datapoints = vec![];
                    if let Ok(new_rec) = measurement.run_periodic() {
                        *receiver = Some(new_rec);
                    }
                }
            } else if ui.button("Stop").clicked() {
                *receiver = None;
                if let Ok(new_logs) = get_logs() {
                    *logs = new_logs;
                }
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add(egui::Hyperlink::new("https://github.com/woelper/linetest/").text("github"));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            let mut ping_values = vec![];
            let mut dl_values = vec![];
            let mut timeout_values = vec![];

            for dp in datapoints {
                match dp {
                    Datapoint::Latency(maybe_ms, t) =>
                    // check if this is a timeout
                    {
                        match maybe_ms {
                            Some(ms) => ping_values.push(Value::new(
                                t.duration_since(*startup_time)
                                    .unwrap_or_default()
                                    .as_secs_f64(),
                                ms.as_secs_f64() * 1000.,
                            )),
                            None => {
                                // mark as timeout
                                timeout_values.push(Value::new(
                                    t.duration_since(*startup_time)
                                        .unwrap_or_default()
                                        .as_secs_f64(),
                                    4.0,
                                ));
                                // also set to a value ()
                                ping_values.push(Value::new(
                                    t.duration_since(*startup_time)
                                        .unwrap_or_default()
                                        .as_secs_f64(),
                                    0.,
                                ))
                            }
                        }
                    }
                    Datapoint::ThroughputUp(_, _) => todo!(),
                    Datapoint::ThroughputDown(d, t) => dl_values.push(Value::new(
                        t.duration_since(*startup_time)
                            .unwrap_or_default()
                            .as_secs_f64(),
                        d.unwrap_or_default(),
                    )),
                }
            }

            ui.heading("Latency (ms)");
            let latency_line = Line::new(Values::from_values(ping_values)).color(Color32::GOLD);
            let timeouts = Points::new(Values::from_values(timeout_values))
                .filled(true)
                .radius(8.)
                .highlight()
                .name("timeout")
                .shape(egui::plot::MarkerShape::Down);
            ui.add(
                Plot::new("latency")
                    .points(timeouts)
                    .line(latency_line)
                    .view_aspect(4.0),
            );

            ui.heading("Download speed (Mbit/s)");

            let latency_line = Line::new(Values::from_values(dl_values)).color(Color32::GOLD);
            ui.add(Plot::new("dl").line(latency_line).view_aspect(4.0));
        });
    }
}
