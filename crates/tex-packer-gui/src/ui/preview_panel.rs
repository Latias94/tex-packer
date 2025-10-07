//! Preview panel UI (right side)

use crate::state::AppState;
use dear_imgui_rs::*;

pub fn render(ui: &Ui, state: &mut AppState) {
    ui.window("Preview")
        .size([800.0, 700.0], Condition::FirstUseEver)
        .build(|| {
            if state.previews.is_empty() {
                render_empty_state(ui);
            } else {
                render_preview(ui, state);
            }
        });
}

fn render_empty_state(ui: &Ui) {
    let avail = ui.content_region_avail();
    let text = "No preview available.";
    let text2 = "Load images and click Pack to generate atlas.";

    // Simple centered layout
    let pos_y = avail[1] * 0.4;
    ui.set_cursor_pos_y(pos_y);

    ui.text_colored([0.6, 0.6, 0.6, 1.0], text);
    ui.text_colored([0.5, 0.5, 0.5, 1.0], text2);
}

fn render_preview(ui: &Ui, state: &mut AppState) {
    // Stats display
    if let Some(stats) = &state.stats {
        ui.text_colored([0.5, 1.0, 0.5, 1.0], "✓ Packing Complete");
        ui.same_line();
        ui.text(format!(
            "| {} images → {} pages | {:.1}% occupancy | {} ms",
            stats.num_images, stats.num_pages, stats.occupancy, stats.pack_time_ms
        ));
    }

    ui.separator();

    // Page selector
    let pages = state.previews.len();
    if pages > 1 {
        ui.text("Page:");
        ui.same_line();
        let mut page = state.selected_page as i32;
        ui.set_next_item_width(200.0);
        if ui.slider("##page", 0, (pages as i32 - 1).max(0), &mut page) {
            state.selected_page = page.clamp(0, (pages as i32 - 1).max(0)) as usize;
        }
        ui.same_line();
        ui.text(format!("({}/{})", state.selected_page + 1, pages));
    } else {
        ui.text("Page: 1/1");
    }

    // View controls
    ui.checkbox("Fit to Window", &mut state.fit_to_window);
    if !state.fit_to_window {
        ui.same_line();
        ui.set_next_item_width(150.0);
        let _ = ui.slider("Zoom", 0.1f32, 4.0f32, &mut state.zoom);
    }

    ui.separator();

    // Preview image
    let pp = &mut state.previews[state.selected_page];
    let avail = ui.content_region_avail();
    let (img_w, img_h) = (pp.width as f32, pp.height as f32);

    let size = if state.fit_to_window {
        if img_w <= 0.0 || img_h <= 0.0 {
            [1.0, 1.0]
        } else {
            let scale = (avail[0] / img_w).min(avail[1] / img_h).max(0.01);
            [img_w * scale, img_h * scale]
        }
    } else {
        [img_w * state.zoom, img_h * state.zoom]
    };

    // Center the image if it's smaller than available space
    if size[0] < avail[0] {
        let offset = (avail[0] - size[0]) * 0.5;
        ui.set_cursor_pos_x(ui.cursor_pos()[0] + offset);
    }

    dear_imgui_rs::Image::new(ui, &mut *pp.tex, size).build();

    // Display image info below
    ui.spacing();
    ui.text(format!(
        "Atlas Size: {}x{} | Display: {:.0}x{:.0}",
        pp.width, pp.height, size[0], size[1]
    ));
}

