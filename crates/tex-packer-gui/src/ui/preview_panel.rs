//! Preview panel (right side, egui)

use crate::state::AppState;
use eframe::egui;
use eframe::egui::epaint::StrokeKind;
use eframe::egui::CornerRadius;

pub fn render(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    state: &mut AppState,
    page_textures: &mut Vec<Option<egui::TextureHandle>>,
) {
    ui.heading("Preview");

    if let Some(result) = &state.result {
        // Ensure textures vector size
        if page_textures.len() != result.pages.len() {
            page_textures.clear();
            page_textures.resize(result.pages.len(), None);
        }

        let pages = result.pages.len();
        // Page selector
        ui.horizontal(|ui| {
            ui.label("Page:");
            if pages > 1 {
                let mut page = state.selected_page as i32;
                ui.add(egui::Slider::new(&mut page, 0..=(pages as i32 - 1)).text(""));
                state.selected_page = page.clamp(0, pages as i32 - 1) as usize;
                ui.label(format!("({}/{})", state.selected_page + 1, pages));
            } else {
                ui.label("1/1");
            }

            ui.separator();
            ui.toggle_value(&mut state.fit_to_window, "Fit to window");
            if !state.fit_to_window {
                ui.add(egui::Slider::new(&mut state.zoom, 0.1..=4.0).text("Zoom"));
            }

            ui.separator();
            ui.toggle_value(&mut state.overlay_show_bounds, "Show bounds");
            ui.toggle_value(&mut state.overlay_show_names, "Show names");

            ui.separator();
            ui.toggle_value(&mut state.bg_checkerboard, "Checker BG");
            if state.bg_checkerboard {
                let mut s = state.bg_checker_size;
                if ui
                    .add(egui::Slider::new(&mut s, 4.0..=64.0).text("Size"))
                    .changed()
                {
                    state.bg_checker_size = s;
                }
            }

            ui.separator();
            let mut pixelated = matches!(state.pixel_filter, crate::state::PixelFilter::Nearest);
            if ui.toggle_value(&mut pixelated, "Pixelated").changed() {
                state.pixel_filter = if pixelated {
                    crate::state::PixelFilter::Nearest
                } else {
                    crate::state::PixelFilter::Linear
                };
                // Clear and defer rebuild; ensure we resize before indexing below
                page_textures.clear();
            }
        });

        // Ensure selected_page is in range and textures vector sized after any toolbar changes
        if state.selected_page >= pages {
            state.selected_page = pages.saturating_sub(1);
        }
        if page_textures.len() != pages {
            page_textures.resize(pages, None);
        }

        // Build texture for current page if needed
        let p = &result.pages[state.selected_page];
        let (w, h) = (p.rgba.width() as usize, p.rgba.height() as usize);
        if page_textures[state.selected_page].is_none() {
            let img = egui::ColorImage::from_rgba_unmultiplied([w, h], p.rgba.as_raw());
            let opts = match state.pixel_filter {
                crate::state::PixelFilter::Linear => egui::TextureOptions::LINEAR,
                crate::state::PixelFilter::Nearest => egui::TextureOptions::NEAREST,
            };
            let tex = ctx.load_texture(format!("page_tex_{}", state.selected_page), img, opts);
            page_textures[state.selected_page] = Some(tex);
        }
        let tex = page_textures[state.selected_page].as_ref().unwrap();

        // Compute display size and rect (with optional panning/zoom)
        let avail = ui.available_size();
        let img_size = egui::vec2(w as f32, h as f32);
        let zoom = if state.fit_to_window {
            if img_size.x <= 0.0 || img_size.y <= 0.0 {
                0.01
            } else {
                (avail.x / img_size.x).min(avail.y / img_size.y).max(0.01)
            }
        } else {
            state.zoom.max(0.01)
        };
        let disp = img_size * zoom;

        let rect = ui.available_rect_before_wrap();
        let center = rect.center();
        let mut origin = center - disp * 0.5;
        if !state.fit_to_window {
            origin += egui::vec2(state.pan.0, state.pan.1);
        }
        let mut desired = egui::Rect::from_min_size(origin, disp);
        let response = ui.allocate_rect(desired, egui::Sense::click_and_drag());

        // Mouse wheel zoom to cursor (manual mode only)
        if !state.fit_to_window && response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.0 {
                let pre_zoom = state.zoom;
                let zoom_factor = (1.0 + scroll * 0.001).clamp(0.25, 4.0);
                let mouse = ui.input(|i| i.pointer.hover_pos()).unwrap_or(center);
                let img_pos_before = (mouse - desired.min) / pre_zoom;
                state.zoom = (state.zoom * zoom_factor).clamp(0.1, 8.0);
                let new_min = mouse - img_pos_before * state.zoom;
                let delta = new_min - desired.min;
                state.pan.0 += delta.x;
                state.pan.1 += delta.y;
                // Recompute desired with new pan/zoom
                let disp2 = img_size * state.zoom;
                let origin2 = center - disp2 * 0.5 + egui::vec2(state.pan.0, state.pan.1);
                desired = egui::Rect::from_min_size(origin2, disp2);
            }
        }

        // Drag to pan (manual mode only)
        if !state.fit_to_window && (response.dragged() || response.is_pointer_button_down_on()) {
            let delta = ui.input(|i| i.pointer.delta());
            state.pan.0 += delta.x;
            state.pan.1 += delta.y;
            let origin2 = desired.min + delta;
            desired = egui::Rect::from_min_size(origin2, desired.size());
        }

        // Checkerboard background
        if state.bg_checkerboard {
            draw_checker(
                &ui.painter(),
                desired,
                state.bg_checker_size,
                ui.visuals().dark_mode,
            );
        }

        // Draw image
        ui.painter().image(
            tex.id(),
            desired,
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

        // Overlay: draw frame bounds & names
        let scale = disp.x / img_size.x.max(1.0);
        let page = &p.page;
        let mut hovered: Option<(String, (u32, u32))> = None;
        if state.overlay_show_bounds || state.overlay_show_names {
            for fr in &page.frames {
                let min =
                    desired.min + egui::vec2(fr.frame.x as f32 * scale, fr.frame.y as f32 * scale);
                let max = min + egui::vec2(fr.frame.w as f32 * scale, fr.frame.h as f32 * scale);
                let rect = egui::Rect::from_min_max(min, max);
                if state.overlay_show_bounds {
                    ui.painter().rect_stroke(
                        rect,
                        CornerRadius::ZERO,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 200, 255)),
                        StrokeKind::Outside,
                    );
                }
                if state.overlay_show_names {
                    ui.painter().text(
                        min + egui::vec2(2.0, 2.0),
                        egui::Align2::LEFT_TOP,
                        &fr.key,
                        egui::TextStyle::Small.resolve(ui.style()),
                        egui::Color32::from_rgb(255, 255, 0),
                    );
                }
            }
            // Hover highlight
            if let Some(mouse) = ui.ctx().pointer_hover_pos() {
                if response.rect.contains(mouse) {
                    let local = mouse - desired.min;
                    let atlas = egui::vec2(local.x / scale, local.y / scale);
                    for fr in &page.frames {
                        if atlas.x >= fr.frame.x as f32
                            && atlas.y >= fr.frame.y as f32
                            && atlas.x < (fr.frame.x + fr.frame.w) as f32
                            && atlas.y < (fr.frame.y + fr.frame.h) as f32
                        {
                            hovered = Some((fr.key.clone(), (atlas.x as u32, atlas.y as u32)));
                            let min = desired.min
                                + egui::vec2(fr.frame.x as f32 * scale, fr.frame.y as f32 * scale);
                            let max = min
                                + egui::vec2(fr.frame.w as f32 * scale, fr.frame.h as f32 * scale);
                            let rect = egui::Rect::from_min_max(min, max);
                            ui.painter().rect_stroke(
                                rect,
                                CornerRadius::ZERO,
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 120, 0)),
                                StrokeKind::Outside,
                            );
                            break;
                        }
                    }
                }
            }
        }

        ui.add_space(6.0);
        ui.label(format!(
            "Atlas size: {}x{} | Display: {:.0}x{:.0}",
            w, h, disp.x, disp.y
        ));

        if let Some(stats) = &state.stats {
            ui.weak(stats.status_string());
        }
        if let Some(err) = &state.last_error {
            ui.colored_label(
                egui::Color32::from_rgb(255, 120, 120),
                format!("Error: {err}"),
            );
        }
        if let Some((key, (x, y))) = hovered {
            ui.weak(format!("Hover: {} ({},{})", key, x, y));
        }

        // Click to select & draw selected highlight
        if response.clicked() {
            if let Some(mouse) = ui.ctx().pointer_hover_pos() {
                if response.rect.contains(mouse) {
                    let local = mouse - desired.min;
                    let atlas = egui::vec2(local.x / scale, local.y / scale);
                    for fr in &page.frames {
                        if atlas.x >= fr.frame.x as f32
                            && atlas.y >= fr.frame.y as f32
                            && atlas.x < (fr.frame.x + fr.frame.w) as f32
                            && atlas.y < (fr.frame.y + fr.frame.h) as f32
                        {
                            state.selected = Some(crate::state::SelectedSprite {
                                key: fr.key.clone(),
                                page_index: state.selected_page,
                            });
                            break;
                        }
                    }
                }
            }
        }

        if let Some(sel) = &state.selected {
            if sel.page_index == state.selected_page {
                for fr in &page.frames {
                    if fr.key == sel.key {
                        let min = desired.min
                            + egui::vec2(fr.frame.x as f32 * scale, fr.frame.y as f32 * scale);
                        let max =
                            min + egui::vec2(fr.frame.w as f32 * scale, fr.frame.h as f32 * scale);
                        let rect = egui::Rect::from_min_max(min, max);
                        ui.painter().rect_stroke(
                            rect,
                            CornerRadius::ZERO,
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 100)),
                            StrokeKind::Outside,
                        );
                        break;
                    }
                }
            }
        }
    } else {
        ui.centered_and_justified(|ui| {
            ui.weak("No result to preview. Select inputs and click Pack.")
        });
    }
}

fn draw_checker(p: &egui::Painter, rect: egui::Rect, size: f32, dark: bool) {
    let c1 = if dark {
        egui::Color32::from_gray(60)
    } else {
        egui::Color32::from_gray(220)
    };
    let c2 = if dark {
        egui::Color32::from_gray(35)
    } else {
        egui::Color32::from_gray(245)
    };
    let cols = ((rect.width() / size).ceil() as i32).max(1);
    let rows = ((rect.height() / size).ceil() as i32).max(1);
    for y in 0..rows {
        for x in 0..cols {
            let color = if (x + y) % 2 == 0 { c1 } else { c2 };
            let min = rect.min + egui::vec2(x as f32 * size, y as f32 * size);
            let max = (min + egui::vec2(size, size)).min(rect.max);
            p.rect_filled(egui::Rect::from_min_max(min, max), 0.0, color);
        }
    }
}
