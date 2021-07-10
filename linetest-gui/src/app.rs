use std::sync::mpsc::Receiver;
use linetest::{self, Datapoint, Measurable};
use eframe::{egui, epi};
use egui::plot::{Line, Plot, Value, Values};
use std::time::UNIX_EPOCH;
/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    // label: String,
    pub receiver: Receiver<linetest::Datapoint>,
    pub datapoints: Vec<Datapoint>,

    // // this how you opt-out of serialization of a member
    // #[cfg_attr(feature = "persistence", serde(skip))]
    // value: f32,
}

// impl Default for TemplateApp {
//     fn default() -> Self {
//         Self {
//             // Example stuff:
//             label: "Hello World!".to_owned(),
//             value: 2.7,
//         }
//     }
// }

impl epi::App for TemplateApp {
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
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        let Self {
            // label,
            // value ,
            receiver,
            datapoints,
        } = self;

      

        ctx.request_repaint();
        for dp in receiver.try_recv() {
            datapoints.push(dp);
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
            ui.label(format!("{:.1} Mbit/s down", datapoints.mean_dl()));
            ui.label(format!("{:.1} ms mean latency", datapoints.mean_latency()*1000.));
            ui.label(format!("{} timeouts", datapoints.timeouts()));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add(
                    egui::Hyperlink::new("https://github.com/woelper/linetest/").text("github"),
                );
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            ui.heading("Latency (ms)");
   
            let mut ping_values = vec![];
            let mut dl_values = vec![];

            for dp in datapoints {
                match dp {
                    Datapoint::Latency(ms, t) => ping_values.push(Value::new(t.duration_since(UNIX_EPOCH).unwrap().as_secs_f64(), ms.unwrap_or_default()*1000.)),
                    Datapoint::ThroughputUp(_, _) => todo!(),
                    Datapoint::ThroughputDown(d, t) => dl_values.push(Value::new(t.duration_since(UNIX_EPOCH).unwrap().as_secs_f64(), d.unwrap_or_default())),
                }
            }

 
            let latency_line = Line::new(Values::from_values(ping_values));
            ui.add(
                Plot::new("latency").line(latency_line).view_aspect(2.0)
            );

            ui.heading("Download speed (Mbit/s)");

            let latency_line = Line::new(Values::from_values(dl_values));
            ui.add(
                Plot::new("dl").line(latency_line).view_aspect(2.0)
            );

        });

 
    }
}