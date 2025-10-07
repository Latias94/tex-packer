//! Setup panel UI (left side)

use crate::state::AppState;
use dear_imgui_rs::*;

pub fn render(ui: &Ui, state: &mut AppState) {
    ui.window("Setup")
        .size([420.0, 700.0], Condition::FirstUseEver)
        .build(|| {
            render_io_section(ui, state);
            ui.separator();
            render_preset_section(ui, state);
            ui.separator();
            render_size_section(ui, state);
            ui.separator();
            render_advanced_section(ui, state);
            ui.separator();
            render_actions(ui, state);
        });
}

fn render_io_section(ui: &Ui, state: &mut AppState) {
    if ui.collapsing_header("📁 Input / Output", TreeNodeFlags::DEFAULT_OPEN) {
        ui.indent();

        // Input
        ui.text("Input:");
        if ui.button("Browse Folder...") {
            state.pick_input_dir();
        }
        ui.same_line();
        if ui.button("Browse Files...") {
            state.pick_files();
        }

        if let Some(dir) = &state.input_dir {
            ui.text_colored([0.7, 0.7, 0.7, 1.0], format!("  {}", dir.display()));
        } else {
            ui.text_colored([0.5, 0.5, 0.5, 1.0], "  <none>");
        }

        ui.spacing();

        // Output
        ui.text("Output:");
        if ui.button("Browse Output...") {
            state.pick_output_dir();
        }

        if let Some(dir) = &state.output_dir {
            ui.text_colored([0.7, 0.7, 0.7, 1.0], format!("  {}", dir.display()));
        } else {
            ui.text_colored([0.5, 0.5, 0.5, 1.0], "  <none>");
        }

        ui.spacing();

        // Stats
        ui.text(format!("Loaded: {} images", state.inputs.len()));

        if ui.button("Reload") {
            if let Err(e) = state.load_inputs() {
                state.set_error(e.to_string());
            }
        }

        ui.unindent();
    }
}

fn render_preset_section(ui: &Ui, state: &mut AppState) {
    if ui.collapsing_header("🎯 Preset", TreeNodeFlags::DEFAULT_OPEN) {
        ui.indent();

        let preset_names: Vec<String> = state
            .presets
            .iter()
            .map(|p| format!("{} {}", p.icon, p.name))
            .collect();

        let current = state.selected_preset_idx;
        let display_name = if state.is_custom_preset {
            format!("⚙️ Custom (based on {})", state.presets[current].name)
        } else {
            preset_names[current].clone()
        };

        ui.text("Preset:");
        ui.set_next_item_width(-1.0);
        if let Some(_token) = ui.begin_combo("##preset", &display_name) {
            for (idx, name) in preset_names.iter().enumerate() {
                let is_selected = idx == current && !state.is_custom_preset;
                if ui.selectable_config(name).selected(is_selected).build() {
                    state.apply_preset(idx);
                }
                if is_selected {
                    ui.set_item_default_focus();
                }
            }
        }

        ui.spacing();

        // Preset description
        let preset = state.current_preset();
        ui.text_colored([0.8, 0.9, 1.0, 1.0], preset.description);

        ui.spacing();

        // Preset details in a child window
        ui.child_window("preset_details")
            .size([0.0, 150.0])
            .border(true)
            .build(ui, || {
                ui.text_colored([0.7, 0.7, 0.7, 1.0], "Details:");
                for detail in &preset.details {
                    ui.text_wrapped(detail);
                }
            });

        ui.unindent();
    }
}

fn render_size_section(ui: &Ui, state: &mut AppState) {
    if ui.collapsing_header("📐 Atlas Size", TreeNodeFlags::DEFAULT_OPEN) {
        ui.indent();

        let sizes = state.recommended_sizes();
        let size_labels: Vec<String> = sizes
            .iter()
            .map(|(w, h)| format!("{}x{}", w, h))
            .collect();

        let current = state.selected_size_idx.min(size_labels.len().saturating_sub(1));

        ui.text("Size:");
        ui.set_next_item_width(-1.0);
        if let Some(_token) = ui.begin_combo("##size", &size_labels[current]) {
            for (idx, label) in size_labels.iter().enumerate() {
                let is_selected = idx == current;
                if ui.selectable_config(label).selected(is_selected).build() {
                    state.apply_size(idx);
                }
                if is_selected {
                    ui.set_item_default_focus();
                }
            }
        }

        ui.spacing();

        // Custom size inputs
        ui.text("Custom:");
        let mut w = state.cfg.max_width as i32;
        let mut h = state.cfg.max_height as i32;
        ui.set_next_item_width(120.0);
        if ui.input_int("Width", &mut w) {
            state.cfg.max_width = w.max(1) as u32;
            state.mark_custom();
        }
        ui.set_next_item_width(120.0);
        if ui.input_int("Height", &mut h) {
            state.cfg.max_height = h.max(1) as u32;
            state.mark_custom();
        }

        ui.unindent();
    }
}

fn render_advanced_section(ui: &Ui, state: &mut AppState) {
    let header_open = ui.collapsing_header("⚙️ Advanced Options", TreeNodeFlags::empty());

    if header_open {
        ui.indent();

        // Trim
        let mut trim = state.cfg.trim;
        if ui.checkbox("Trim Transparent Edges", &mut trim) {
            state.cfg.trim = trim;
            state.mark_custom();
        }
        if trim {
            ui.same_line();
            let mut threshold = state.cfg.trim_threshold as i32;
            ui.set_next_item_width(80.0);
            if ui.input_int("Threshold", &mut threshold) {
                state.cfg.trim_threshold = threshold.clamp(0, 255) as u8;
                state.mark_custom();
            }
        }

        // Rotation
        let mut rotation = state.cfg.allow_rotation;
        if ui.checkbox("Allow Rotation", &mut rotation) {
            state.cfg.allow_rotation = rotation;
            state.mark_custom();
        }

        // Padding
        let mut padding = state.cfg.texture_padding as i32;
        ui.set_next_item_width(120.0);
        if ui.input_int("Padding", &mut padding) {
            state.cfg.texture_padding = padding.max(0) as u32;
            state.mark_custom();
        }

        // Extrusion
        let mut extrusion = state.cfg.texture_extrusion as i32;
        ui.set_next_item_width(120.0);
        if ui.input_int("Extrusion", &mut extrusion) {
            state.cfg.texture_extrusion = extrusion.max(0) as u32;
            state.mark_custom();
        }

        // Border
        let mut border = state.cfg.border_padding as i32;
        ui.set_next_item_width(120.0);
        if ui.input_int("Border", &mut border) {
            state.cfg.border_padding = border.max(0) as u32;
            state.mark_custom();
        }

        ui.spacing();

        // Power of 2 / Square
        let mut pow2 = state.cfg.power_of_two;
        if ui.checkbox("Power of 2", &mut pow2) {
            state.cfg.power_of_two = pow2;
            state.mark_custom();
        }

        let mut square = state.cfg.square;
        if ui.checkbox("Square", &mut square) {
            state.cfg.square = square;
            state.mark_custom();
        }

        let mut force_max = state.cfg.force_max_dimensions;
        if ui.checkbox("Force Max Dimensions", &mut force_max) {
            state.cfg.force_max_dimensions = force_max;
            state.mark_custom();
        }

        ui.spacing();

        // Debug
        let mut outlines = state.cfg.texture_outlines;
        if ui.checkbox("Draw Debug Outlines", &mut outlines) {
            state.cfg.texture_outlines = outlines;
            state.mark_custom();
        }

        ui.unindent();
    }
}

fn render_actions(ui: &Ui, state: &mut AppState) {
    // Atlas name
    ui.text("Atlas Name:");
    let mut name_buf = state.atlas_name.clone();
    ui.set_next_item_width(-1.0);
    if ui.input_text("##atlas_name", &mut name_buf).build() {
        state.atlas_name = name_buf;
    }

    ui.spacing();

    // Action buttons
    let button_width = (ui.content_region_avail()[0] - 8.0) / 2.0;

    if ui.button_with_size("Pack", [button_width, 40.0]) {
        state.do_pack();
    }

    ui.same_line();

    if ui.button_with_size("Export", [button_width, 40.0]) {
        state.do_export();
    }

    // Error display
    if let Some(err) = &state.last_error {
        ui.spacing();
        ui.text_colored([1.0, 0.3, 0.3, 1.0], "Error:");
        ui.text_wrapped(err);
    }
}

