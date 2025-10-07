//! tex-packer-gui using dear-app runner + docking layout
use ::image::ImageReader;
use dear_app::{AppBuilder, DockingConfig, RedrawMode, RunnerConfig, Theme};
use dear_imgui_rs as imgui;
use imgui::*;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info};

use tex_packer_core::prelude::*;

struct PreviewPage {
    tex: Box<dear_imgui_rs::texture::TextureData>,
    width: u32,
    height: u32,
}

struct AppState {
    // IO
    input_dir: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    inputs: Vec<InputImage>,
    // Config
    cfg: PackerConfig,
    // Result
    result: Option<PackOutput>,
    previews: Vec<PreviewPage>,
    // UI
    selected_page: usize,
    atlas_name: String,
    // Preview behaviour
    fit_to_window: bool,
    zoom: f32,
    // Errors
    last_error: Option<String>,
    // Dock layout
    layout_built: bool,
}

impl Default for PreviewPage {
    fn default() -> Self {
        let mut tex = dear_imgui_rs::texture::TextureData::new();
        tex.create(dear_imgui_rs::texture::TextureFormat::RGBA32, 1, 1);
        tex.set_data(&[0x00, 0x00, 0x00, 0xFF]);
        Self {
            tex,
            width: 1,
            height: 1,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            input_dir: None,
            output_dir: None,
            inputs: Vec::new(),
            cfg: PackerConfig::default(),
            result: None,
            previews: Vec::new(),
            selected_page: 0,
            atlas_name: "atlas".into(),
            fit_to_window: true,
            zoom: 1.0,
            last_error: None,
            layout_built: false,
        }
    }
}

impl AppState {
    fn set_error(&mut self, msg: impl ToString) {
        let s = msg.to_string();
        error!("{s}");
        self.last_error = Some(s);
    }

    fn pick_input_dir(&mut self) {
        if let Some(d) = rfd::FileDialog::new().set_directory(".").pick_folder() {
            self.input_dir = Some(d);
            if let Err(e) = self.load_inputs() {
                self.set_error(e.to_string());
            }
        }
    }

    fn pick_files(&mut self) {
        if let Some(files) = rfd::FileDialog::new().set_directory(".").pick_files() {
            if let Err(e) = self.load_inputs_from_paths(&files) {
                self.set_error(e.to_string());
            }
        }
    }

    fn pick_output_dir(&mut self) {
        if let Some(d) = rfd::FileDialog::new().set_directory(".").pick_folder() {
            self.output_dir = Some(d);
        }
    }

    fn load_inputs_from_paths(&mut self, paths: &[PathBuf]) -> anyhow::Result<()> {
        self.inputs.clear();
        for path in paths {
            if path.is_file() && is_image_path(path) {
                let key = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let img = ImageReader::open(path)?.decode()?;
                self.inputs.push(InputImage { key, image: img });
            }
        }
        info!("Loaded {} images (files)", self.inputs.len());
        Ok(())
    }

    fn load_inputs(&mut self) -> anyhow::Result<()> {
        self.inputs.clear();
        let Some(dir) = &self.input_dir else {
            return Ok(());
        };
        let mut count = 0usize;
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let path = entry.path();
                if is_image_path(&path) {
                    let key = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    let img = ImageReader::open(&path)?.decode()?;
                    self.inputs.push(InputImage { key, image: img });
                    count += 1;
                }
            }
        }
        info!("Loaded {} images", count);
        Ok(())
    }

    fn clear_result(&mut self) {
        self.result = None;
        self.previews.clear();
        self.selected_page = 0;
    }

    fn do_pack(&mut self) {
        self.clear_result();
        if self.inputs.is_empty() {
            self.set_error("No inputs loaded");
            return;
        }
        let inputs: Vec<InputImage> = self
            .inputs
            .iter()
            .map(|i| InputImage {
                key: i.key.clone(),
                image: i.image.clone(),
            })
            .collect();
        match pack_images(inputs, self.cfg.clone()) {
            Ok(out) => {
                let mut previews = Vec::with_capacity(out.pages.len());
                for p in &out.pages {
                    let mut tex = dear_imgui_rs::texture::TextureData::new();
                    tex.create(
                        dear_imgui_rs::texture::TextureFormat::RGBA32,
                        p.rgba.width() as i32,
                        p.rgba.height() as i32,
                    );
                    tex.set_data(p.rgba.as_raw());
                    previews.push(PreviewPage {
                        tex,
                        width: p.rgba.width(),
                        height: p.rgba.height(),
                    });
                }
                self.previews = previews;
                self.result = Some(out);
            }
            Err(e) => {
                self.set_error(format!("Pack error: {e:?}"));
            }
        }
    }

    fn do_export(&mut self) {
        let Some(outdir) = &self.output_dir else {
            self.set_error("Pick an output folder first");
            return;
        };
        let Some(result) = &self.result else {
            self.set_error("No result to export");
            return;
        };
        let name = self.atlas_name.as_str();
        // Write pages
        for p in &result.pages {
            let file = outdir.join(format!("{name}_{}.png", p.page.id));
            if let Err(e) = p.rgba.save(&file) {
                self.set_error(format!("Failed writing {:?}: {e}", file));
                return;
            }
        }
        // Write json (hash)
        let json = tex_packer_core::to_json_hash(&result.atlas);
        let json_path = outdir.join(format!("{name}.json"));
        if let Err(e) = fs::write(&json_path, serde_json::to_string_pretty(&json).unwrap()) {
            self.set_error(format!("Failed writing {:?}: {e}", json_path));
            return;
        }
        info!("Exported atlas to {:?}", outdir);
    }
}

fn build_dockspace_and_layout(ui: &Ui, _state: &mut AppState) {
    use dear_imgui_rs::{DockBuilder, DockNodeFlags, SplitDirection};

    // Create a fullscreen dockspace over the main viewport (passthru central)
    let dockspace_id = ui.dockspace_over_main_viewport();

    // Only configure layout if node doesn't exist yet (first time)
    if unsafe { dear_imgui_rs::sys::igDockBuilderGetNode(dockspace_id) }.is_null() {
        let size = ui.main_viewport().size();

        DockBuilder::remove_node(dockspace_id);
        DockBuilder::add_node(dockspace_id, DockNodeFlags::NONE);
        DockBuilder::set_node_size(dockspace_id, size);

        let mut dock_main_id = dockspace_id;
        let (new_main, left) = split_node(dockspace_id, SplitDirection::Left, 0.28);
        dock_main_id = new_main;

        DockBuilder::dock_window("Inputs & Config", left);
        DockBuilder::dock_window("Preview", dock_main_id);
        DockBuilder::finish(dockspace_id);
    }
}

// patch for dear-imgui-rs v0.3.0
fn split_node(
    node_id: u32,
    split_dir: SplitDirection,
    size_ratio_for_node_at_dir: f32,
) -> (u32, u32) {
    unsafe {
        let mut id_at_dir: sys::ImGuiID = 0;
        let mut id_at_opposite: sys::ImGuiID = 0;
        let _ = sys::igDockBuilderSplitNode(
            node_id.into(),
            split_dir.into(),
            size_ratio_for_node_at_dir,
            &mut id_at_dir,
            &mut id_at_opposite,
        );
        (id_at_dir as u32, id_at_opposite as u32)
    }
}

fn ui_left_panel(ui: &Ui, state: &mut AppState) {
    ui.window("Inputs & Config")
        .size([420.0, 520.0], Condition::FirstUseEver)
        .build(|| {
            // IO section
            if ui.button("Pick Input Folder…") {
                state.pick_input_dir();
            }
            ui.same_line();
            if ui.button("Pick Files…") {
                state.pick_files();
            }
            if let Some(dir) = &state.input_dir {
                ui.text(format!("Input: {dir:?}"));
            } else {
                ui.text("Input: <none>");
            }
            if ui.button("Pick Output Folder…") {
                state.pick_output_dir();
            }
            ui.same_line();
            if let Some(dir) = &state.output_dir {
                ui.text(format!("Output: {dir:?}"));
            } else {
                ui.text("Output: <none>");
            }
            if ui.button("Reload") {
                if let Err(e) = state.load_inputs() {
                    state.set_error(e.to_string());
                }
            }
            ui.separator();

            ui.text(format!("Inputs loaded: {}", state.inputs.len()));
            ui.separator();

            // Config editing
            {
                let mut w = state.cfg.max_width as i32;
                let mut h = state.cfg.max_height as i32;
                let _ = ui.input_int("Max Width", &mut w);
                let _ = ui.input_int("Max Height", &mut h);
                state.cfg.max_width = w.max(1) as u32;
                state.cfg.max_height = h.max(1) as u32;
            }

            // booleans
            {
                let mut allow_rot = state.cfg.allow_rotation;
                let mut force_max = state.cfg.force_max_dimensions;
                let mut pow2 = state.cfg.power_of_two;
                let mut square = state.cfg.square;
                let mut outlines = state.cfg.texture_outlines;
                let mut trim = state.cfg.trim;
                ui.checkbox("Allow Rotation", &mut allow_rot);
                ui.checkbox("Force Max Dimensions", &mut force_max);
                ui.checkbox("Power of Two", &mut pow2);
                ui.checkbox("Square", &mut square);
                ui.checkbox("Draw Outlines", &mut outlines);
                ui.checkbox("Trim Transparent Edges", &mut trim);
                state.cfg.allow_rotation = allow_rot;
                state.cfg.force_max_dimensions = force_max;
                state.cfg.power_of_two = pow2;
                state.cfg.square = square;
                state.cfg.texture_outlines = outlines;
                state.cfg.trim = trim;
            }

            // paddings/extrude/trim threshold
            {
                let mut border = state.cfg.border_padding as i32;
                let mut texpad = state.cfg.texture_padding as i32;
                let mut extrude = state.cfg.texture_extrusion as i32;
                let mut trim_th = state.cfg.trim_threshold as i32;
                let _ = ui.input_int("Border Padding", &mut border);
                let _ = ui.input_int("Texture Padding", &mut texpad);
                let _ = ui.input_int("Extrude", &mut extrude);
                let _ = ui.input_int("Trim Threshold", &mut trim_th);
                state.cfg.border_padding = border.max(0) as u32;
                state.cfg.texture_padding = texpad.max(0) as u32;
                state.cfg.texture_extrusion = extrude.max(0) as u32;
                state.cfg.trim_threshold = trim_th.clamp(0, 255) as u8;
            }

            // Algorithm family + heuristics
            {
                // Family combo
                let families = ["Skyline", "MaxRects", "Guillotine", "Auto"];
                let mut current: usize = match state.cfg.family {
                    AlgorithmFamily::Skyline => 0,
                    AlgorithmFamily::MaxRects => 1,
                    AlgorithmFamily::Guillotine => 2,
                    AlgorithmFamily::Auto => 3,
                };
                if ui.combo("Algorithm", &mut current, &families, |v: &&str| {
                    std::borrow::Cow::from(*v)
                }) {
                    state.cfg.family = match current {
                        0 => AlgorithmFamily::Skyline,
                        1 => AlgorithmFamily::MaxRects,
                        2 => AlgorithmFamily::Guillotine,
                        _ => AlgorithmFamily::Auto,
                    };
                }

                match state.cfg.family {
                    AlgorithmFamily::Skyline => {
                        let opts = ["BottomLeft", "MinWaste"];
                        let mut idx: usize = match state.cfg.skyline_heuristic {
                            SkylineHeuristic::BottomLeft => 0,
                            SkylineHeuristic::MinWaste => 1,
                        };
                        if ui.combo("Skyline Heuristic", &mut idx, &opts, |v: &&str| {
                            std::borrow::Cow::from(*v)
                        }) {
                            state.cfg.skyline_heuristic = match idx {
                                0 => SkylineHeuristic::BottomLeft,
                                _ => SkylineHeuristic::MinWaste,
                            };
                        }
                    }
                    AlgorithmFamily::MaxRects => {
                        let opts = [
                            "BestAreaFit",
                            "BestShortSideFit",
                            "BestLongSideFit",
                            "BottomLeft",
                            "ContactPoint",
                        ];
                        let mut idx: usize = match state.cfg.mr_heuristic {
                            MaxRectsHeuristic::BestAreaFit => 0,
                            MaxRectsHeuristic::BestShortSideFit => 1,
                            MaxRectsHeuristic::BestLongSideFit => 2,
                            MaxRectsHeuristic::BottomLeft => 3,
                            MaxRectsHeuristic::ContactPoint => 4,
                        };
                        if ui.combo("MaxRects Heuristic", &mut idx, &opts, |v: &&str| {
                            std::borrow::Cow::from(*v)
                        }) {
                            state.cfg.mr_heuristic = match idx {
                                0 => MaxRectsHeuristic::BestAreaFit,
                                1 => MaxRectsHeuristic::BestShortSideFit,
                                2 => MaxRectsHeuristic::BestLongSideFit,
                                3 => MaxRectsHeuristic::BottomLeft,
                                _ => MaxRectsHeuristic::ContactPoint,
                            };
                        }
                        let mut mr_ref = state.cfg.mr_reference;
                        ui.checkbox("Use MaxRects Reference Split/Prune", &mut mr_ref);
                        state.cfg.mr_reference = mr_ref;
                    }
                    AlgorithmFamily::Guillotine => {
                        let choices = ["BAF", "BSSF", "BLSF", "WAF", "WSSF", "WLSF"]; // labels only
                        let splits = [
                            "ShorterLeftoverAxis",
                            "LongerLeftoverAxis",
                            "MinArea",
                            "MaxArea",
                            "ShorterAxis",
                            "LongerAxis",
                        ];
                        let mut cidx: usize = match state.cfg.g_choice {
                            GuillotineChoice::BestAreaFit => 0,
                            GuillotineChoice::BestShortSideFit => 1,
                            GuillotineChoice::BestLongSideFit => 2,
                            GuillotineChoice::WorstAreaFit => 3,
                            GuillotineChoice::WorstShortSideFit => 4,
                            GuillotineChoice::WorstLongSideFit => 5,
                        };
                        if ui.combo("G Choice", &mut cidx, &choices, |v: &&str| {
                            std::borrow::Cow::from(*v)
                        }) {
                            state.cfg.g_choice = match cidx {
                                0 => GuillotineChoice::BestAreaFit,
                                1 => GuillotineChoice::BestShortSideFit,
                                2 => GuillotineChoice::BestLongSideFit,
                                3 => GuillotineChoice::WorstAreaFit,
                                4 => GuillotineChoice::WorstShortSideFit,
                                _ => GuillotineChoice::WorstLongSideFit,
                            };
                        }
                        let mut sidx: usize = match state.cfg.g_split {
                            GuillotineSplit::SplitShorterLeftoverAxis => 0,
                            GuillotineSplit::SplitLongerLeftoverAxis => 1,
                            GuillotineSplit::SplitMinimizeArea => 2,
                            GuillotineSplit::SplitMaximizeArea => 3,
                            GuillotineSplit::SplitShorterAxis => 4,
                            GuillotineSplit::SplitLongerAxis => 5,
                        };
                        if ui.combo("G Split", &mut sidx, &splits, |v: &&str| {
                            std::borrow::Cow::from(*v)
                        }) {
                            state.cfg.g_split = match sidx {
                                0 => GuillotineSplit::SplitShorterLeftoverAxis,
                                1 => GuillotineSplit::SplitLongerLeftoverAxis,
                                2 => GuillotineSplit::SplitMinimizeArea,
                                3 => GuillotineSplit::SplitMaximizeArea,
                                4 => GuillotineSplit::SplitShorterAxis,
                                _ => GuillotineSplit::SplitLongerAxis,
                            };
                        }
                    }
                    AlgorithmFamily::Auto => {
                        let opts = ["Fast", "Quality"];
                        let mut idx: usize = match state.cfg.auto_mode {
                            AutoMode::Fast => 0,
                            AutoMode::Quality => 1,
                        };
                        if ui.combo("Auto Mode", &mut idx, &opts, |v: &&str| {
                            std::borrow::Cow::from(*v)
                        }) {
                            state.cfg.auto_mode = if idx == 0 {
                                AutoMode::Fast
                            } else {
                                AutoMode::Quality
                            };
                        }
                        let mut ms = state.cfg.time_budget_ms.unwrap_or(0) as i32;
                        let _ = ui.input_int("Time Budget (ms)", &mut ms);
                        state.cfg.time_budget_ms = Some(ms.max(0) as u64);
                    }
                }
            }

            ui.separator();
            if ui.button("Pack") {
                state.do_pack();
            }
            ui.same_line();
            if ui.button("Export") {
                state.do_export();
            }
            if let Some(err) = &state.last_error {
                ui.text_colored([1.0, 0.2, 0.2, 1.0], err);
            }
        });
}

fn ui_right_preview(ui: &Ui, state: &mut AppState) {
    ui.window("Preview")
        .size([720.0, 700.0], Condition::FirstUseEver)
        .build(|| {
            if state.previews.is_empty() {
                ui.text("No preview. Pack to generate.");
                return;
            }
            // Controls
            let pages = state.previews.len();
            let mut page = state.selected_page as i32;
            let _ = ui.slider("Page", 0, (pages as i32 - 1).max(0), &mut page);
            state.selected_page = page.clamp(0, (pages as i32 - 1).max(0)) as usize;

            // Atlas name
            let mut name_buf = state.atlas_name.clone();
            if ui.input_text("Atlas Name", &mut name_buf).build() {
                state.atlas_name = name_buf;
            }

            ui.checkbox("Fit to Window", &mut state.fit_to_window);
            if !state.fit_to_window {
                let mut zoom = state.zoom;
                let _ = ui.slider("Zoom", 0.1f32, 4.0f32, &mut zoom);
                state.zoom = zoom;
            }
            ui.separator();

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

            dear_imgui_rs::Image::new(ui, &mut *pp.tex, size).build();
        });
}

fn is_image_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase()),
        Some(ext) if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "gif" | "tif" | "tiff")
    )
}

fn main() {
    // Init tracing (RUST_LOG controls verbosity)
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    let mut state = AppState::default();

    let mut cfg = RunnerConfig::default();
    cfg.window_title = "tex-packer-gui".into();
    cfg.window_size = (1280.0, 800.0);
    cfg.clear_color = [0.10, 0.10, 0.13, 1.0];
    cfg.theme = Some(Theme::Dark);
    cfg.redraw = RedrawMode::Poll;
    cfg.docking = DockingConfig {
        enable: true,
        auto_dockspace: false, // we'll create host window ourselves to control layout
        dockspace_flags: DockFlags::PASSTHRU_CENTRAL_NODE,
        host_window_flags: WindowFlags::empty(),
        host_window_name: "DockSpaceHost",
    };

    AppBuilder::new()
        .with_config(cfg)
        .on_frame(move |ui, _addons| {
            build_dockspace_and_layout(ui, &mut state);
            ui_left_panel(ui, &mut state);
            ui_right_preview(ui, &mut state);
        })
        .run()
        .unwrap();
}
