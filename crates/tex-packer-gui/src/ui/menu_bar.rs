//! Menu bar UI (egui)

use crate::state::AppState;
use eframe::egui;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    egui::MenuBar::new().ui(ui, |ui| {
        // File
        ui.menu_button("File", |ui| {
            if ui.button("Open Folder...").clicked() {
                state.pick_input_dir();
                ui.close();
            }
            if ui.button("Open Files...").clicked() {
                state.pick_files();
                ui.close();
            }
            ui.separator();
            if ui.button("Set Output Folder...").clicked() {
                state.pick_output_dir();
                ui.close();
            }
            ui.separator();
            let export_enabled = state.result.is_some();
            if ui
                .add_enabled(export_enabled, egui::Button::new("Export"))
                .clicked()
            {
                state.do_export();
                ui.close();
            }
            ui.separator();
            if ui.button("Exit").clicked() {
                std::process::exit(0);
            }
        });

        // Presets
        ui.menu_button("Presets", |ui| {
            let count = state.presets.len();
            for idx in 0..count {
                let label = {
                    let p = &state.presets[idx];
                    format!("{} {}", p.icon, p.name)
                };
                if ui.button(label).clicked() {
                    state.apply_preset(idx);
                    ui.close();
                }
            }
        });

        // View
        ui.menu_button("View", |ui| {
            ui.toggle_value(&mut state.fit_to_window, "Fit to Window");
            ui.separator();
            if ui.button("Clear Result").clicked() {
                state.clear_result();
                ui.close();
            }
            if ui.button("Clear Error").clicked() {
                state.clear_error();
                ui.close();
            }
        });

        // Help
        ui.menu_button("Help", |ui| {
            ui.label("tex-packer GUI with egui");
        });

        ui.separator();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let export_enabled =
                state.result.is_some() && state.output_dir.is_some() && !state.pack_in_progress;
            if ui
                .add_enabled(export_enabled, egui::Button::new("Export"))
                .clicked()
            {
                state.do_export();
            }
            ui.separator();
            if state.pack_in_progress {
                if ui.button("Cancel").clicked() {
                    state.cancel_requested = true;
                }
                ui.add(egui::Spinner::new());
            } else {
                if ui.button("Pack").clicked() {
                    state.pack_requested = true;
                }
            }
            ui.toggle_value(&mut state.autopack, "Auto Pack");
        });
    });
}
