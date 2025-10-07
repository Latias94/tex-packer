//! Menu bar UI

use crate::state::AppState;
use dear_imgui_rs::*;

pub fn render(ui: &Ui, state: &mut AppState) {
    if let Some(_mb) = ui.begin_main_menu_bar() {
        render_file_menu(ui, state);
        render_presets_menu(ui, state);
        render_view_menu(ui, state);
        render_help_menu(ui);
        _mb.end();
    }
}

fn render_file_menu(ui: &Ui, state: &mut AppState) {
    if let Some(_m) = ui.begin_menu("File") {
        if ui.menu_item("Open Folder...") {
            state.pick_input_dir();
        }
        if ui.menu_item("Open Files...") {
            state.pick_files();
        }
        ui.separator();
        if ui.menu_item("Set Output Folder...") {
            state.pick_output_dir();
        }
        ui.separator();
        let export_enabled = state.result.is_some();
        if export_enabled && ui.menu_item("Export") {
            state.do_export();
        }
        ui.separator();
        if ui.menu_item("Exit") {
            std::process::exit(0);
        }
        _m.end();
    }
}

fn render_presets_menu(ui: &Ui, state: &mut AppState) {
    if let Some(_m) = ui.begin_menu("Presets") {
        let preset_count = state.presets.len();
        for idx in 0..preset_count {
            let label = format!("{} {}", state.presets[idx].icon, state.presets[idx].name);
            if ui.menu_item(&label) {
                state.apply_preset(idx);
            }
        }
        _m.end();
    }
}

fn render_view_menu(ui: &Ui, state: &mut AppState) {
    if let Some(_m) = ui.begin_menu("View") {
        ui.menu_item_toggle("Fit to Window", None::<&str>, &mut state.fit_to_window, true);
        ui.separator();
        if ui.menu_item("Clear Result") {
            state.clear_result();
        }
        if ui.menu_item("Clear Error") {
            state.clear_error();
        }
        _m.end();
    }
}

fn render_help_menu(ui: &Ui) {
    if let Some(_m) = ui.begin_menu("Help") {
        if ui.menu_item("About") {
            // Could open a modal here
        }
        _m.end();
    }
}

