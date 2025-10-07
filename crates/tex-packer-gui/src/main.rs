//! tex-packer-gui using dear-app runner + docking layout

mod presets;
mod state;
mod stats;
mod ui;

use dear_app::{AddOnsConfig, AppBuilder, DockingConfig, RedrawMode, RunnerConfig, Theme};
use dear_imgui_rs::*;
use state::AppState;

fn main() {
    // Init tracing (RUST_LOG controls verbosity)
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    let mut state = AppState::default();

    let mut cfg = RunnerConfig::default();
    cfg.window_title = "tex-packer GUI - Texture Atlas Packer".into();
    cfg.window_size = (1400.0, 900.0);
    cfg.clear_color = [0.10, 0.10, 0.13, 1.0];
    cfg.theme = Some(Theme::Dark);
    cfg.redraw = RedrawMode::Poll;
    cfg.docking = DockingConfig {
        enable: true,
        auto_dockspace: false,
        dockspace_flags: DockFlags::PASSTHRU_CENTRAL_NODE,
        host_window_flags: WindowFlags::empty(),
        host_window_name: "DockSpaceHost",
    };

    // Enable dear-app add-ons (includes texture upload handling)
    let addons = AddOnsConfig::auto();

    AppBuilder::new()
        .with_config(cfg)
        .with_addons(addons)
        .on_frame(move |ui, _addons| {
            // Menu bar
            ui::menu_bar::render(ui, &mut state);

            // Dockspace and layout
            ui::build_dockspace_and_layout(ui, &mut state);

            // Panels
            ui::setup_panel::render(ui, &mut state);
            ui::preview_panel::render(ui, &mut state);
        })
        .run()
        .unwrap();
}
