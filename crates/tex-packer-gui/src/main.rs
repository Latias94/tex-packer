//! tex-packer-gui using egui/eframe with left/right layout

mod presets;
mod state;
mod stats;
mod ui;

use crate::stats::PackStats as GuiPackStats;
use eframe::{egui, egui::Context};
use state::AppState;
use std::time::{Duration, Instant};
use tex_packer_core::prelude::*;

struct GuiApp {
    state: AppState,
    // Cache of egui textures for pages, recreated after packing
    page_textures: Vec<Option<egui::TextureHandle>>,
    // Async pack job
    pack_job: Option<std::thread::JoinHandle<Result<(PackOutput, GuiPackStats), String>>>,
    cancel_requested: bool,
    autopack_deadline: Option<Instant>,
}

impl Default for GuiApp {
    fn default() -> Self {
        Self {
            state: AppState::default(),
            page_textures: Vec::new(),
            pack_job: None,
            cancel_requested: false,
            autopack_deadline: None,
        }
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Handle deferred packing via background thread
        if self.state.pack_requested && self.pack_job.is_none() {
            self.spawn_pack_job();
            self.state.pack_requested = false;
        }

        // Auto pack when config or inputs changed
        if self.state.autopack && self.state.dirty_config {
            // debounce: 300ms after last change, and no running job
            if self.pack_job.is_none() {
                self.autopack_deadline = Some(Instant::now() + Duration::from_millis(300));
            }
        }

        // Start autopack when deadline hits
        if let Some(deadline) = self.autopack_deadline {
            if self.pack_job.is_none() && Instant::now() >= deadline {
                self.spawn_pack_job();
                self.autopack_deadline = None;
            }
        }

        // Handle cancel requests (soft cancel)
        if self.state.cancel_requested {
            self.cancel_requested = true;
            // UI stays in-progress until job finishes; we just ignore results.
            self.state.cancel_requested = false;
        }

        // Poll pack job completion
        if let Some(handle) = &self.pack_job {
            if handle.is_finished() {
                let handle = self.pack_job.take().unwrap();
                match handle.join().expect("pack job panicked") {
                    Ok((out, stats)) => {
                        if !self.cancel_requested {
                            self.state.result = Some(out);
                            self.state.stats = Some(stats);
                            self.page_textures.clear();
                        }
                    }
                    Err(err) => {
                        if !self.cancel_requested {
                            self.state.set_error(err);
                        }
                    }
                }
                self.state.pack_in_progress = false;
                self.cancel_requested = false;
                self.state.dirty_config = false;
                // If autopack is on and further changes queued during job, rearm debounce
                if self.state.autopack && self.state.dirty_config {
                    self.autopack_deadline = Some(Instant::now() + Duration::from_millis(300));
                }
            }
        }

        // Handle drag & drop files into window
        let dropped: Vec<std::path::PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if !dropped.is_empty() {
            if let Err(e) = self.state.handle_dropped_paths(&dropped) {
                self.state.set_error(e.to_string());
            }
        }

        // Menu bar at top
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui::menu_bar::render(ui, &mut self.state);
        });

        // Left: setup
        egui::SidePanel::left("left_setup")
            .resizable(true)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui::setup_panel::render(ui, &mut self.state);
            });

        // Right: preview
        egui::CentralPanel::default().show(ctx, |ui| {
            ui::preview_panel::render(ctx, ui, &mut self.state, &mut self.page_textures);
        });
    }
}

impl GuiApp {
    fn spawn_pack_job(&mut self) {
        if self.state.inputs.is_empty() {
            self.state.set_error("No inputs loaded");
            return;
        }
        let inputs: Vec<InputImage> = self
            .state
            .inputs
            .iter()
            .filter(|i| !self.state.excluded_keys.contains(&i.key))
            .map(|i| InputImage {
                key: i.key.clone(),
                image: i.image.clone(),
            })
            .collect();
        let cfg = self.state.cfg.clone();
        let num_images = inputs.len();
        self.state.pack_in_progress = true;
        self.page_textures.clear();
        self.cancel_requested = false;
        let handle = std::thread::spawn(move || {
            let start = std::time::Instant::now();
            match pack_images(inputs, cfg.clone()) {
                Ok(out) => {
                    let pack_time_ms = start.elapsed().as_millis() as u64;
                    let stats = GuiPackStats::from_output(&out, num_images, pack_time_ms);
                    Ok((out, stats))
                }
                Err(e) => Err(format!("Pack error: {e:?}")),
            }
        });
        self.pack_job = Some(handle);
    }
}

fn main() {
    // Init tracing (RUST_LOG controls verbosity)
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("tex-packer GUI - Texture Atlas Packer")
            .with_inner_size([1400.0, 900.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    let app = GuiApp::default();
    eframe::run_native("tex-packer GUI", options, Box::new(|_cc| Ok(Box::new(app)))).unwrap();
}
