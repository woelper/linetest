use eframe::egui::plot::{Legend, Points};
use eframe::egui::{Color32, FontData, FontDefinitions, FontFamily, TextStyle, Visuals};
use eframe::{egui, epi};
use egui::plot::{HLine, Line, Plot, Value, Values};
use linetest::{self, Datapoint, Evaluation, MeasurementBuilder};
use log::info;
use std::ffi::OsStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{path::PathBuf, sync::mpsc::Receiver};
/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct LinetestApp {
    pub receiver: Option<Receiver<Datapoint>>,
    pub datapoints: Vec<Datapoint>,
    pub logs: Vec<PathBuf>,
    pub log_index: usize,
    pub dark_mode: bool,
    pub measurement: MeasurementBuilder,
}

impl Default for LinetestApp {
    fn default() -> Self {
        Self {
            receiver: None,
            datapoints: vec![],
            logs: MeasurementBuilder::get_logs().unwrap_or_default(),
            log_index: 0,
            dark_mode: false,
            measurement: MeasurementBuilder::new()
                .with_aws_payload()
                .with_ping_delay(1),
        }
    }
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

    fn setup(
        &mut self,
        ctx: &egui::Context,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        let mut fonts = FontDefinitions::default();

        // fonts.font_data.insert(key, value)

        fonts.font_data.insert(
            "plex".to_owned(),
            FontData::from_static(include_bytes!("IBMPlexSans-Regular.ttf")), // std::borrow::Cow::Borrowed(include_bytes!("IBMPlexSans-Regular.ttf")),
        );

        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "plex".into());

        let mut style: egui::Style = (*ctx.style()).clone();


        style.text_styles.get_mut(&TextStyle::Body).unwrap().size = 20.;
        style.text_styles.get_mut(&TextStyle::Button).unwrap().size = 20.;
        style.text_styles.get_mut(&TextStyle::Small).unwrap().size = 15.;
        ctx.set_style(style);
        ctx.set_fonts(fonts);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        let Self {
            receiver,
            datapoints,
            logs,
            log_index,
            dark_mode,
            measurement,
        } = self;

        let line_color = Color32::from_rgb(255, 208, 0);

        ctx.request_repaint();
        if let Some(valid_receiver) = receiver {
            for dp in valid_receiver.try_iter() {
                datapoints.push(dp);
                if let Some(log) = &measurement.logfile {
                    let _ = datapoints.save(&log);
                }
            }
        }

        if *dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                // egui::menu::menu(ui, "File", |ui| {
                //     if ui.button("Quit").clicked() {
                //         frame.quit();
                //     }
                //     if ui.button("Toggle light/dark").clicked() {
                //         *dark_mode = !*dark_mode;
                //     }
                // });
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.label("Info");

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

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add(egui::Hyperlink::new("https://github.com/woelper/linetest/"));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            let mut ping_values = vec![];
            let mut dl_values = vec![];
            let mut timeout_values = vec![];

            let first_instant: SystemTime = match datapoints.first() {
                Some(dp) => match dp {
                    Datapoint::Latency(_, ms)
                    | Datapoint::ThroughputDown(_, ms)
                    | Datapoint::ThroughputUp(_, ms) => *ms,
                },
                None => UNIX_EPOCH,
            };

            for dp in &mut *datapoints {
                match dp {
                    Datapoint::Latency(maybe_ms, t) =>
                    // check if this is a timeout
                    {
                        match maybe_ms {
                            Some(ms) => ping_values.push(Value::new(
                                t.duration_since(first_instant)
                                    .expect("can't set duration")
                                    .as_secs_f64(),
                                ms.as_secs_f64() * 1000.,
                            )),
                            None => {
                                // mark as timeout
                                timeout_values.push(Value::new(
                                    t.duration_since(first_instant)
                                        .expect("can't set duration")
                                        .as_secs_f64(),
                                    4.0,
                                ));
                                // also set to a value
                                ping_values.push(Value::new(
                                    t.duration_since(first_instant)
                                        .expect("can't set duration")
                                        .as_secs_f64(),
                                    0.01,
                                ))
                            }
                        }
                    }
                    Datapoint::ThroughputUp(_, _) => todo!(),
                    Datapoint::ThroughputDown(d, t) => dl_values.push(Value::new(
                        t.duration_since(first_instant)
                            .expect("can't set duration")
                            .as_secs_f64(),
                        d.unwrap_or_default(),
                    )),
                }
            }

            // let line_color = ui.style().visuals.hyperlink_color;
            ui.label("Latency (ms)");
            let latency_line = Line::new(Values::from_values(ping_values.clone()))
                .color(line_color)
                .name("Ping (ms)")
                .fill(0.0);
            let latency_points = Points::new(Values::from_values(ping_values))
                .stems(0.0)
                .color(line_color);
            let timeouts = Points::new(Values::from_values(timeout_values))
                .filled(true)
                .radius(8.)
                .name("timeout")
                .shape(egui::plot::MarkerShape::Down);

            Plot::new("latency")
                .view_aspect(5.0)
                .legend(Legend::default().text_style(TextStyle::Small))
                .show(ui, |plot_ui| {
                    plot_ui.points(latency_points);

                    // add a line to the plot if it is not dense
                    if datapoints.len() < 100 {
                        plot_ui.line(latency_line);
                    }
                    plot_ui.points(timeouts);
                    plot_ui.hline(
                        HLine::new(datapoints.mean_latency().as_millis() as f64)
                            .name(format!(
                                "Mean latency ({}ms)",
                                datapoints.mean_latency().as_millis()
                            ))
                            .color(line_color.linear_multiply(0.1)),
                    );
                });

            ui.label("Download speed (Mbit/s)");
            let download_line = Line::new(Values::from_values(dl_values))
                .color(line_color)
                .fill(0.0);
            Plot::new("dl").view_aspect(4.0).show(ui, |plot_ui| {
                plot_ui.line(download_line);
            });

            if receiver.is_none() {
                if ui.button("⏺ Start recording").clicked() {
                    //measurement.logfile = MeasurementBuilder::default().logfile;

                    *datapoints = vec![];
                    if let Ok(new_rec) = measurement.run_until_receiver_drops() {
                        *receiver = Some(new_rec);
                    }
                }
            } else if ui.button("⏹ Stop").clicked() {
                *receiver = None;

                //refresh logs on disk after last session finishes
                if let Ok(new_logs) = MeasurementBuilder::get_logs() {
                    *logs = new_logs;
                }
                // generate new log name so we don't overwrite the last
                measurement.logfile = MeasurementBuilder::default().logfile;
            }

            egui::CollapsingHeader::new("Settings").show(ui, |ui| {
                if let Some(log) = measurement.logfile.as_mut() {
                    let mut log_file_string = log
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    if ui.text_edit_singleline(&mut log_file_string).changed() {
                        if let Some(parent) = log.parent() {
                            *log = parent.join(log_file_string).with_extension("ltest");
                        }
                    }
                }

                let mut delay = measurement.ping_delay.as_secs();

                ui.horizontal(|ui| {
                    if ui.add(egui::DragValue::new(&mut delay)).changed() {
                        measurement.ping_delay = Duration::from_secs(delay);
                    }
                    ui.label("ping delay (s)");
                });

                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut measurement.throughput_ping_ratio));
                    ui.label("Perform speedtest after these many pings");
                });
            });

            egui::CollapsingHeader::new("Log archive").show(ui, |ui| {
                if egui::ComboBox::from_label(if receiver.is_some() {
                    "Stop and load selected log"
                } else {
                    "Load selected log"
                })
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
                    datapoints.clear();
                    if let Some(log) = logs.get(*log_index) {
                        datapoints.load(log).unwrap();
                        info!("Loaded {} data points", datapoints.len());
                    }
                }

                if let Some(log) = logs.get(*log_index) {
                    if ui
                        .button(format!(
                            "Delete {}",
                            log.file_name()
                                .unwrap_or(OsStr::new("no_file_name"))
                                .to_string_lossy()
                        ))
                        .clicked()
                    {
                        let _ = std::fs::remove_file(&log);
                        if let Ok(new_logs) = MeasurementBuilder::get_logs() {
                            *logs = new_logs;
                        }
                    }
                }
            });
        });
    }
}
