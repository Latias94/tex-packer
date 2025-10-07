//! UI modules

pub mod menu_bar;
pub mod preview_panel;
pub mod setup_panel;

use crate::state::AppState;
use dear_imgui_rs::*;

/// Build dockspace and layout (following dear_app_docking.rs pattern)
pub fn build_dockspace_and_layout(ui: &Ui, state: &mut AppState) {
    // Create fullscreen host window for dockspace (like dear_app_docking.rs)
    let viewport = ui.main_viewport();
    ui.set_next_window_viewport(Id::from(viewport.id()));
    let pos = viewport.pos();
    let size = viewport.size();

    let mut host_flags = WindowFlags::NO_TITLE_BAR
        | WindowFlags::NO_RESIZE
        | WindowFlags::NO_MOVE
        | WindowFlags::NO_COLLAPSE
        | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS
        | WindowFlags::NO_NAV_FOCUS
        | WindowFlags::NO_DOCKING;
    // If using passthrough central node, avoid drawing host background
    host_flags |= WindowFlags::NO_BACKGROUND;

    // Zero rounding/border and remove padding for a clean host window
    let rounding = ui.push_style_var(StyleVar::WindowRounding(0.0));
    let border = ui.push_style_var(StyleVar::WindowBorderSize(0.0));
    let padding = ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0]));

    ui.window("DockSpaceHost")
        .flags(host_flags)
        .position([pos[0], pos[1]], Condition::Always)
        .size([size[0], size[1]], Condition::Always)
        .build(|| {
            // Pop padding/border/rounding to restore defaults inside
            padding.pop();
            border.pop();
            rounding.pop();

            let dockspace_id = ui.get_id("MainDockSpace");

            // Build/restore layout BEFORE creating DockSpace so our splits apply
            if !state.layout_built {
                // Clear any previous layout (ini or runtime) and rebuild
                DockBuilder::remove_node(dockspace_id);
                let root = DockBuilder::add_node(dockspace_id, DockNodeFlags::NONE);
                DockBuilder::set_node_size(root, size);

                // Split: left panel (30%) and right preview (70%)
                let (dock_id_left, dock_main_id) =
                    DockBuilder::split_node(root, SplitDirection::Left, 0.30);

                DockBuilder::dock_window("Setup", dock_id_left);
                DockBuilder::dock_window("Preview", dock_main_id);
                DockBuilder::finish(root);

                state.layout_built = true;
            }

            // Render DockSpace inside the host window
            let avail = ui.content_region_avail();
            let _ = ui.dock_space(dockspace_id, [avail[0], avail[1]]);
        });
}
