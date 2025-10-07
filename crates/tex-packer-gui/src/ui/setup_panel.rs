//! Setup panel UI (left side, egui)

use crate::state::AppState;
use eframe::egui;
use egui_extras::TableBuilder;
use image::GenericImageView;
use tex_packer_core::prelude::*;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            render_inputs_section(ui, state);
            ui.separator();
            render_io_section(ui, state);
            ui.separator();
            render_selection_section(ui, state);
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

fn render_inputs_section(ui: &mut egui::Ui, state: &mut AppState) {
    egui::CollapsingHeader::new("Inputs")
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.text_edit_singleline(&mut state.input_filter);
                if ui.button("Include All").clicked() {
                    state.excluded_keys.clear();
                    state.dirty_config = true;
                }
                if ui.button("Exclude All").clicked() {
                    state.excluded_keys = state.inputs.iter().map(|i| i.key.clone()).collect();
                    state.dirty_config = true;
                }
            });
            ui.add_space(4.0);

            let text_height = egui::TextStyle::Body.resolve(ui.style()).size.max(18.0);
            TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(egui_extras::Column::auto())
                .column(egui_extras::Column::remainder())
                .column(egui_extras::Column::auto())
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("✔");
                    });
                    header.col(|ui| {
                        ui.strong("Name");
                    });
                    header.col(|ui| {
                        ui.strong("Size");
                    });
                })
                .body(|mut body| {
                    let filter = state.input_filter.to_ascii_lowercase();
                    for inp in &state.inputs {
                        if !filter.is_empty() && !inp.key.to_ascii_lowercase().contains(&filter) {
                            continue;
                        }
                        body.row(text_height, |mut row| {
                            let key = &inp.key;
                            let mut included = !state.excluded_keys.contains(key);
                            row.col(|ui| {
                                if ui.checkbox(&mut included, "").changed() {
                                    if included {
                                        state.excluded_keys.remove(key);
                                    } else {
                                        state.excluded_keys.insert(key.clone());
                                    }
                                    state.dirty_config = true;
                                }
                            });
                            row.col(|ui| {
                                ui.label(key);
                            });
                            row.col(|ui| {
                                let (w, h) = inp.image.dimensions();
                                ui.label(format!("{}x{}", w, h));
                            });
                        });
                    }
                });
            ui.add_space(4.0);
            ui.weak(format!(
                "Total: {}  |  Included: {}",
                state.inputs.len(),
                state.inputs.len() - state.excluded_keys.len()
            ));
        });
}

fn render_selection_section(ui: &mut egui::Ui, state: &mut AppState) {
    egui::CollapsingHeader::new("Selection")
        .default_open(true)
        .show(ui, |ui| {
            if let (Some(sel), Some(result)) = (&state.selected, &state.result) {
                let sel_page = sel.page_index;
                if let Some(page) = result.pages.get(sel_page) {
                    if let Some(fr) = page.page.frames.iter().find(|f| f.key == sel.key) {
                        let name = fr.key.clone();
                        ui.horizontal(|ui| {
                            ui.strong("Name:");
                            ui.label(&name);
                        });
                        ui.horizontal(|ui| {
                            ui.strong("Page:");
                            ui.label(format!("{}", sel_page + 1));
                        });
                        ui.separator();
                        ui.label(format!(
                            "Frame: x={} y={} w={} h={}",
                            fr.frame.x, fr.frame.y, fr.frame.w, fr.frame.h
                        ));
                        ui.label(format!("Rotated: {} | Trimmed: {}", fr.rotated, fr.trimmed));
                        ui.label(format!(
                            "Source: x={} y={} w={} h={}",
                            fr.source.x, fr.source.y, fr.source.w, fr.source.h
                        ));
                        ui.label(format!(
                            "SourceSize: {}x{}",
                            fr.source_size.0, fr.source_size.1
                        ));
                        ui.add_space(4.0);
                        let excluded_now = state.excluded_keys.contains(&name);
                        ui.horizontal(|ui| {
                            if ui.button("Go to page").clicked() {
                                state.selected_page = sel_page;
                            }
                            let btn = if excluded_now {
                                egui::Button::new("Include")
                            } else {
                                egui::Button::new("Exclude")
                            };
                            if ui.add(btn).clicked() {
                                if excluded_now {
                                    state.excluded_keys.remove(&name);
                                } else {
                                    state.excluded_keys.insert(name.clone());
                                }
                                state.dirty_config = true;
                            }
                            if ui.button("Deselect").clicked() {
                                state.selected = None;
                            }
                        });
                    } else {
                        ui.weak("Selected sprite not found on current result.");
                    }
                } else {
                    ui.weak("Selected page index is out of range.");
                }
            } else {
                ui.weak("No selection.");
            }
        });
}

fn render_io_section(ui: &mut egui::Ui, state: &mut AppState) {
    egui::CollapsingHeader::new("Input / Output")
        .default_open(true)
        .show(ui, |ui| {
            ui.label("Input:");
            ui.horizontal(|ui| {
                if ui.button("Browse Folder...").clicked() {
                    state.pick_input_dir();
                }
                if ui.button("Browse Files...").clicked() {
                    state.pick_files();
                }
                if ui.button("Clear Inputs").clicked() {
                    state.inputs.clear();
                    state.clear_result();
                }
            });
            if let Some(dir) = &state.input_dir {
                ui.label(format!("  {}", dir.display()));
            } else {
                ui.weak("  <none>");
            }

            ui.add_space(6.0);
            ui.label("Output:");
            if ui.button("Choose Output Folder...").clicked() {
                state.pick_output_dir();
            }
            if let Some(dir) = &state.output_dir {
                ui.label(format!("  {}", dir.display()));
            } else {
                ui.weak("  <none>");
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Atlas Name:");
                ui.text_edit_singleline(&mut state.atlas_name);
            });

            ui.add_space(6.0);
            ui.label(format!("Loaded: {} images", state.inputs.len()));
            if ui.button("Reload").clicked() {
                if let Err(e) = state.load_inputs() {
                    state.set_error(e.to_string());
                }
            }
        });
}

fn render_preset_section(ui: &mut egui::Ui, state: &mut AppState) {
    egui::CollapsingHeader::new("Presets")
        .default_open(true)
        .show(ui, |ui| {
            let current = state.selected_preset_idx;
            let display_name = if state.is_custom_preset {
                format!("Custom (from {})", state.presets[current].name)
            } else {
                format!(
                    "{} {}",
                    state.presets[current].icon, state.presets[current].name
                )
            };

            ui.label("Preset:");
            egui::ComboBox::from_id_salt("preset_combo")
                .selected_text(display_name)
                .show_ui(ui, |ui| {
                    let items: Vec<(usize, String)> = state
                        .presets
                        .iter()
                        .enumerate()
                        .map(|(i, p)| (i, format!("{} {}", p.icon, p.name)))
                        .collect();
                    for (idx, txt) in items.into_iter() {
                        if ui
                            .selectable_label(idx == current && !state.is_custom_preset, txt)
                            .clicked()
                        {
                            state.apply_preset(idx);
                        }
                    }
                });
            ui.horizontal(|ui| {
                if ui.button("Reset to Preset").clicked() {
                    let idx = state.selected_preset_idx;
                    state.apply_preset(idx);
                }
                if state.is_custom_preset {
                    ui.weak("(customized)");
                }
            });

            ui.add_space(6.0);
            let preset = state.current_preset();
            let desc_color = if ui.visuals().dark_mode {
                egui::Color32::from_rgb(204, 230, 255)
            } else {
                egui::Color32::from_rgb(40, 80, 120)
            };
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(desc_color, preset.description);
                let resp = ui.small_button("?");
                resp.on_hover_ui(|ui| {
                    ui.strong("Preset Details");
                    ui.separator();
                    for d in preset.details.iter().copied() {
                        ui.label(d);
                    }
                });
            });
        });
}

fn render_size_section(ui: &mut egui::Ui, state: &mut AppState) {
    egui::CollapsingHeader::new("Size")
        .default_open(true)
        .show(ui, |ui| {
            ui.label("Recommended sizes:");
            let sizes: Vec<(u32, u32)> = state.recommended_sizes().to_vec();
            ui.horizontal_wrapped(|ui| {
                for (i, (w, h)) in sizes.iter().copied().enumerate() {
                    let label = format!("{}x{}", w, h);
                    let selected = i == state.selected_size_idx;
                    if ui.selectable_label(selected, label).clicked() {
                        state.apply_size(i);
                    }
                }
            });

            ui.add_space(6.0);
            ui.label("Atlas max size:");
            let mut w = state.cfg.max_width as i32;
            let mut h = state.cfg.max_height as i32;
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut w)
                        .speed(1)
                        .range(1..=16384)
                        .prefix("W:"),
                );
                ui.add(
                    egui::DragValue::new(&mut h)
                        .speed(1)
                        .range(1..=16384)
                        .prefix("H:"),
                );
            });
            let w = w.clamp(1, 16384) as u32;
            let h = h.clamp(1, 16384) as u32;
            if w != state.cfg.max_width || h != state.cfg.max_height {
                state.cfg.max_width = w;
                state.cfg.max_height = h;
                state.mark_custom();
            }
        });
}

fn render_advanced_section(ui: &mut egui::Ui, state: &mut AppState) {
    egui::CollapsingHeader::new("Advanced")
        .default_open(false)
        .show(ui, |ui| {
            state.show_advanced = true;
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(
                    &mut state.advanced_tab,
                    crate::state::AdvancedTab::General,
                    "General",
                );
                ui.selectable_value(
                    &mut state.advanced_tab,
                    crate::state::AdvancedTab::Algorithm,
                    "Algorithm",
                );
                ui.selectable_value(
                    &mut state.advanced_tab,
                    crate::state::AdvancedTab::Sorting,
                    "Sorting",
                );
            });
            ui.separator();
            match state.advanced_tab {
                crate::state::AdvancedTab::General => render_advanced_general(ui, state),
                crate::state::AdvancedTab::Algorithm => render_advanced_algorithm(ui, state),
                crate::state::AdvancedTab::Sorting => render_advanced_sorting(ui, state),
            }
        });
}

fn render_advanced_general(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("General");
    let mut any_changed = false;
    ui.horizontal_wrapped(|ui| {
        any_changed |= ui
            .toggle_value(&mut state.cfg.allow_rotation, "Allow rotation")
            .changed();
        any_changed |= ui
            .toggle_value(&mut state.cfg.trim, "Trim transparent")
            .changed();
        any_changed |= ui
            .toggle_value(&mut state.cfg.texture_outlines, "Debug outlines")
            .changed();
        any_changed |= ui
            .toggle_value(&mut state.cfg.power_of_two, "Power-of-two")
            .changed();
        any_changed |= ui.toggle_value(&mut state.cfg.square, "Square").changed();
        any_changed |= ui
            .toggle_value(&mut state.cfg.use_waste_map, "Skyline waste-map")
            .changed();
    });
    if state.cfg.trim {
        let mut thr = state.cfg.trim_threshold as i32;
        if ui
            .add(egui::Slider::new(&mut thr, 0..=255).text("Trim threshold"))
            .changed()
        {
            state.cfg.trim_threshold = thr as u8;
            any_changed = true;
        }
    }
    let mut tp = state.cfg.texture_padding as i32;
    let mut te = state.cfg.texture_extrusion as i32;
    let mut bp = state.cfg.border_padding as i32;
    any_changed |= ui
        .add(egui::Slider::new(&mut tp, 0..=64).text("Texture padding (px)"))
        .changed();
    any_changed |= ui
        .add(egui::Slider::new(&mut te, 0..=16).text("Edge extrusion (px)"))
        .changed();
    any_changed |= ui
        .add(egui::Slider::new(&mut bp, 0..=128).text("Border padding (px)"))
        .changed();
    state.cfg.texture_padding = tp as u32;
    state.cfg.texture_extrusion = te as u32;
    state.cfg.border_padding = bp as u32;

    if any_changed {
        state.mark_custom();
    }
}

fn render_advanced_algorithm(ui: &mut egui::Ui, state: &mut AppState) {
    let mut any_changed = false;
    ui.heading("Algorithm");
    ui.horizontal_wrapped(|ui| {
        let mut fam = state.cfg.family.clone();
        if ui
            .selectable_label(matches!(fam, AlgorithmFamily::Skyline), "Skyline")
            .clicked()
        {
            fam = AlgorithmFamily::Skyline;
        }
        if ui
            .selectable_label(matches!(fam, AlgorithmFamily::MaxRects), "MaxRects")
            .clicked()
        {
            fam = AlgorithmFamily::MaxRects;
        }
        if ui
            .selectable_label(matches!(fam, AlgorithmFamily::Guillotine), "Guillotine")
            .clicked()
        {
            fam = AlgorithmFamily::Guillotine;
        }
        if ui
            .selectable_label(matches!(fam, AlgorithmFamily::Auto), "Auto")
            .clicked()
        {
            fam = AlgorithmFamily::Auto;
        }
        if fam != state.cfg.family {
            state.cfg.family = fam;
            any_changed = true;
        }
    });

    match state.cfg.family {
        AlgorithmFamily::Skyline => {
            ui.label("Skyline heuristic:");
            let mut h = state.cfg.skyline_heuristic.clone();
            if ui
                .selectable_label(matches!(h, SkylineHeuristic::BottomLeft), "BottomLeft")
                .clicked()
            {
                h = SkylineHeuristic::BottomLeft;
            }
            if ui
                .selectable_label(matches!(h, SkylineHeuristic::MinWaste), "MinWaste")
                .clicked()
            {
                h = SkylineHeuristic::MinWaste;
            }
            if h != state.cfg.skyline_heuristic {
                state.cfg.skyline_heuristic = h;
                any_changed = true;
            }
        }
        AlgorithmFamily::MaxRects => {
            ui.label("MaxRects heuristic:");
            for (label, val) in [
                ("BestAreaFit", MaxRectsHeuristic::BestAreaFit),
                ("BestShortSideFit", MaxRectsHeuristic::BestShortSideFit),
                ("BestLongSideFit", MaxRectsHeuristic::BestLongSideFit),
                ("BottomLeft", MaxRectsHeuristic::BottomLeft),
                ("ContactPoint", MaxRectsHeuristic::ContactPoint),
            ] {
                let sel = state.cfg.mr_heuristic == val;
                if ui.selectable_label(sel, label).clicked() {
                    state.cfg.mr_heuristic = val;
                    any_changed = true;
                }
            }
        }
        AlgorithmFamily::Guillotine => {
            ui.label("Guillotine choice:");
            for (label, val) in [
                ("BestAreaFit", GuillotineChoice::BestAreaFit),
                ("BestShortSideFit", GuillotineChoice::BestShortSideFit),
                ("BestLongSideFit", GuillotineChoice::BestLongSideFit),
                ("WorstAreaFit", GuillotineChoice::WorstAreaFit),
                ("WorstShortSideFit", GuillotineChoice::WorstShortSideFit),
                ("WorstLongSideFit", GuillotineChoice::WorstLongSideFit),
            ] {
                let sel = state.cfg.g_choice == val;
                if ui.selectable_label(sel, label).clicked() {
                    state.cfg.g_choice = val;
                    any_changed = true;
                }
            }
            ui.separator();
            ui.label("Guillotine split:");
            for (label, val) in [
                (
                    "SplitShorterLeftoverAxis",
                    GuillotineSplit::SplitShorterLeftoverAxis,
                ),
                (
                    "SplitLongerLeftoverAxis",
                    GuillotineSplit::SplitLongerLeftoverAxis,
                ),
                ("SplitMinimizeArea", GuillotineSplit::SplitMinimizeArea),
                ("SplitMaximizeArea", GuillotineSplit::SplitMaximizeArea),
                ("SplitShorterAxis", GuillotineSplit::SplitShorterAxis),
                ("SplitLongerAxis", GuillotineSplit::SplitLongerAxis),
            ] {
                let sel = state.cfg.g_split == val;
                if ui.selectable_label(sel, label).clicked() {
                    state.cfg.g_split = val;
                    any_changed = true;
                }
            }
        }
        AlgorithmFamily::Auto => {
            ui.label("Auto mode:");
            for (label, val) in [("Fast", AutoMode::Fast), ("Quality", AutoMode::Quality)] {
                let sel = state.cfg.auto_mode == val;
                if ui.selectable_label(sel, label).clicked() {
                    state.cfg.auto_mode = val;
                    any_changed = true;
                }
            }
            ui.add_space(4.0);
            let mut tb: i64 = state.cfg.time_budget_ms.unwrap_or(0) as i64;
            ui.horizontal(|ui| {
                ui.label("Time budget (ms, optional):");
                let _ = ui.add(egui::DragValue::new(&mut tb).speed(50).range(0..=60_000));
            });
            let new_tb = if tb <= 0 { None } else { Some(tb as u64) };
            if new_tb != state.cfg.time_budget_ms {
                state.cfg.time_budget_ms = new_tb;
                any_changed = true;
            }
            any_changed |= ui
                .toggle_value(&mut state.cfg.parallel, "Parallel (if available)")
                .changed();
            any_changed |= ui
                .toggle_value(&mut state.cfg.mr_reference, "MaxRects reference mode")
                .changed();
            ui.label("MR reference auto thresholds (optional):");
            let mut t_ms: i64 = state.cfg.auto_mr_ref_time_ms_threshold.unwrap_or(0) as i64;
            let mut t_n: i64 = state.cfg.auto_mr_ref_input_threshold.unwrap_or(0) as i64;
            ui.horizontal(|ui| {
                let _ = ui.add(
                    egui::DragValue::new(&mut t_ms)
                        .speed(50)
                        .range(0..=60_000)
                        .prefix("Time:"),
                );
                let _ = ui.add(
                    egui::DragValue::new(&mut t_n)
                        .speed(1)
                        .range(0..=100000)
                        .prefix("Inputs:"),
                );
            });
            let new_tms = if t_ms <= 0 { None } else { Some(t_ms as u64) };
            let new_tn = if t_n <= 0 { None } else { Some(t_n as usize) };
            if new_tms != state.cfg.auto_mr_ref_time_ms_threshold {
                state.cfg.auto_mr_ref_time_ms_threshold = new_tms;
                any_changed = true;
            }
            if new_tn != state.cfg.auto_mr_ref_input_threshold {
                state.cfg.auto_mr_ref_input_threshold = new_tn;
                any_changed = true;
            }
        }
    }
    if any_changed {
        state.mark_custom();
    }
}

fn render_advanced_sorting(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Sorting");
    for (label, so) in [
        ("None", SortOrder::None),
        ("NameAsc", SortOrder::NameAsc),
        ("AreaDesc", SortOrder::AreaDesc),
        ("MaxSideDesc", SortOrder::MaxSideDesc),
    ] {
        let sel = state.cfg.sort_order == so;
        if ui.selectable_label(sel, label).clicked() {
            state.cfg.sort_order = so;
            state.mark_custom();
        }
    }
}

fn render_actions(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if state.pack_in_progress {
            ui.add(egui::Spinner::new());
            ui.weak("Packing...");
            if ui.button("Cancel").clicked() {
                state.cancel_requested = true;
            }
        } else if ui.button("Pack").clicked() {
            state.pack_requested = true;
        }
        ui.toggle_value(&mut state.autopack, "Auto Pack");
        ui.separator();
        ui.label("Export Format:");
        egui::ComboBox::from_id_salt("export_format")
            .selected_text(match state.export_format {
                crate::state::ExportFormat::Hash => "Hash",
                crate::state::ExportFormat::Array => "Array",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut state.export_format,
                    crate::state::ExportFormat::Hash,
                    "Hash",
                );
                ui.selectable_value(
                    &mut state.export_format,
                    crate::state::ExportFormat::Array,
                    "Array",
                );
            });
        let export_enabled =
            state.result.is_some() && state.output_dir.is_some() && !state.pack_in_progress;
        if ui
            .add_enabled(export_enabled, egui::Button::new("Export"))
            .clicked()
        {
            state.do_export();
        }
    });

    if let Some(err) = &state.last_error {
        ui.colored_label(
            egui::Color32::from_rgb(255, 120, 120),
            format!("Error: {err}"),
        );
    }
    if let Some(stats) = &state.stats {
        ui.colored_label(
            egui::Color32::from_rgb(150, 220, 150),
            stats.status_string(),
        );
    }
}
