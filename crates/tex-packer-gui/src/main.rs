//! tex-packer-gui: dear-imgui-rs + winit + wgpu frontend for tex-packer-core
use ::image::ImageReader;
use dear_imgui_rs::*;
use dear_imgui_wgpu::WgpuRenderer;
use dear_imgui_winit::WinitPlatform;
use pollster::block_on;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use tracing::{error, info};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use tex_packer_core::prelude::*;

struct ImguiState {
    context: Context,
    platform: WinitPlatform,
    renderer: WgpuRenderer,
    last_frame: Instant,
}

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
    zoom: f32,
    atlas_name: String,
    // Stats
    last_error: Option<String>,
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
            zoom: 1.0,
            atlas_name: "atlas".into(),
            last_error: None,
        }
    }
}

struct AppWindow {
    device: wgpu::Device,
    queue: wgpu::Queue,
    window: Arc<Window>,
    surface_desc: wgpu::SurfaceConfiguration,
    surface: wgpu::Surface<'static>,
    imgui: ImguiState,
    state: AppState,
}

#[derive(Default)]
struct App {
    window: Option<AppWindow>,
}

impl AppWindow {
    fn new(event_loop: &ActiveEventLoop) -> Result<Self, Box<dyn std::error::Error>> {
        // GPU instance/surface
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let window = {
            let size = LogicalSize::new(1280.0, 800.0);
            Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("tex-packer-gui")
                        .with_inner_size(size),
                )?,
            )
        };
        let surface = instance.create_surface(window.clone())?;
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("No suitable GPU adapter");
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
        let caps = surface.get_capabilities(&adapter);
        let preferred_srgb = [
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ];
        let format = preferred_srgb
            .iter()
            .cloned()
            .find(|f| caps.formats.contains(f))
            .unwrap_or(caps.formats[0]);
        let physical_size = window.inner_size();
        let surface_desc = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: physical_size.width,
            height: physical_size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_desc);

        // ImGui context + platform + renderer
        let mut context = Context::create();
        context.set_ini_filename(None::<String>).unwrap();
        // Enable docking in ImGui config
        {
            use dear_imgui_rs::ConfigFlags;
            let io = context.io_mut();
            let mut flags = io.config_flags();
            flags.insert(ConfigFlags::DOCKING_ENABLE);
            io.set_config_flags(flags);
        }
        let mut platform = WinitPlatform::new(&mut context);
        platform.attach_window(&window, dear_imgui_winit::HiDpiMode::Default, &mut context);
        let init_info =
            dear_imgui_wgpu::WgpuInitInfo::new(device.clone(), queue.clone(), surface_desc.format);
        let mut renderer = WgpuRenderer::new(init_info, &mut context)?;
        renderer.set_gamma_mode(dear_imgui_wgpu::GammaMode::Auto);

        Ok(Self {
            device,
            queue,
            window,
            surface_desc,
            surface,
            imgui: ImguiState {
                context,
                platform,
                renderer,
                last_frame: Instant::now(),
            },
            state: AppState::default(),
        })
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_desc.width = new_size.width;
            self.surface_desc.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_desc);
        }
    }

    fn set_error(&mut self, msg: impl ToString) {
        let s = msg.to_string();
        error!("{s}");
        self.state.last_error = Some(s);
    }

    fn pick_input_dir(&mut self) {
        if let Some(d) = rfd::FileDialog::new().set_directory(".").pick_folder() {
            self.state.input_dir = Some(d);
            if let Err(e) = self.load_inputs() {
                self.set_error(e.to_string());
            }
        }
    }

    fn pick_output_dir(&mut self) {
        if let Some(d) = rfd::FileDialog::new().set_directory(".").pick_folder() {
            self.state.output_dir = Some(d);
        }
    }

    fn load_inputs_from_paths(&mut self, paths: &[PathBuf]) -> anyhow::Result<()> {
        self.state.inputs.clear();
        for path in paths {
            if path.is_file() && is_image_path(path) {
                let key = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let img = ImageReader::open(path)?.decode()?;
                self.state.inputs.push(InputImage { key, image: img });
            }
        }
        info!("Loaded {} images (files)", self.state.inputs.len());
        Ok(())
    }

    fn load_inputs(&mut self) -> anyhow::Result<()> {
        self.state.inputs.clear();
        let Some(dir) = &self.state.input_dir else {
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
                    self.state.inputs.push(InputImage { key, image: img });
                    count += 1;
                }
            }
        }
        info!("Loaded {} images", count);
        Ok(())
    }

    fn clear_result(&mut self) {
        self.state.result = None;
        self.state.previews.clear();
        self.state.selected_page = 0;
    }

    fn do_pack(&mut self) {
        self.clear_result();
        if self.state.inputs.is_empty() {
            self.set_error("No inputs loaded");
            return;
        }
        let inputs: Vec<InputImage> = self
            .state
            .inputs
            .iter()
            .map(|i| InputImage {
                key: i.key.clone(),
                image: i.image.clone(),
            })
            .collect();
        match pack_images(inputs, self.state.cfg.clone()) {
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
                self.state.previews = previews;
                self.state.result = Some(out);
            }
            Err(e) => {
                self.set_error(format!("Pack error: {e:?}"));
            }
        }
    }

    fn do_export(&mut self) {
        let Some(outdir) = &self.state.output_dir else {
            self.set_error("Pick an output folder first");
            return;
        };
        let Some(result) = &self.state.result else {
            self.set_error("No result to export");
            return;
        };
        let name = self.state.atlas_name.as_str();
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

    #[cfg(any())]
    fn render_ui(&mut self, ui: &Ui) {
        // Inputs + Config
        ui.window("Inputs & Config")
            .size([460.0, 520.0], Condition::FirstUseEver)
            .build(|| {
                if ui.button("Pick Input Folder…") {
                    self.pick_input_dir();
                }
                ui.same_line();
                if let Some(dir) = &self.state.input_dir {
                    ui.text(format!("{dir:?}"));
                } else {
                    ui.text("<none>");
                }
                if ui.button("Pick Output Folder…") {
                    self.pick_output_dir();
                }
                ui.same_line();
                if let Some(dir) = &self.state.output_dir {
                    ui.text(format!("{dir:?}"));
                } else {
                    ui.text("<none>");
                }
                if ui.button("Reload") {
                    if let Err(e) = self.load_inputs() {
                        self.set_error(e.to_string());
                    }
                }
                ui.separator();

                ui.text(format!("Inputs loaded: {}", self.state.inputs.len()));
                ui.separator();

                // Config editing
                // Dimensions
                {
                    let mut w = self.state.cfg.max_width as i32;
                    let mut h = self.state.cfg.max_height as i32;
                    let _ = ui.input_int("Max Width", &mut w);
                    let _ = ui.input_int("Max Height", &mut h);
                    self.state.cfg.max_width = w.max(1) as u32;
                    self.state.cfg.max_height = h.max(1) as u32;
                }

                // booleans
                {
                    let mut allow_rot = self.state.cfg.allow_rotation;
                    let mut force_max = self.state.cfg.force_max_dimensions;
                    let mut pow2 = self.state.cfg.power_of_two;
                    let mut square = self.state.cfg.square;
                    let mut trim = self.state.cfg.trim;
                    let mut outlines = self.state.cfg.texture_outlines;
                    ui.checkbox("Allow Rotation", &mut allow_rot);
                    ui.checkbox("Force Max Dimensions", &mut force_max);
                    ui.checkbox("Power of Two", &mut pow2);
                    ui.checkbox("Square", &mut square);
                    ui.checkbox("Trim", &mut trim);
                    ui.checkbox("Debug Outlines", &mut outlines);
                    self.state.cfg.allow_rotation = allow_rot;
                    self.state.cfg.force_max_dimensions = force_max;
                    self.state.cfg.power_of_two = pow2;
                    self.state.cfg.square = square;
                    self.state.cfg.trim = trim;
                    self.state.cfg.texture_outlines = outlines;
                }

                // paddings/extrude/trim threshold
                {
                    let mut border = self.state.cfg.border_padding as i32;
                    let mut texpad = self.state.cfg.texture_padding as i32;
                    let mut extrude = self.state.cfg.texture_extrusion as i32;
                    let mut trim_th = self.state.cfg.trim_threshold as i32;
                    let _ = ui.input_int("Border Padding", &mut border);
                    let _ = ui.input_int("Texture Padding", &mut texpad);
                    let _ = ui.input_int("Extrude", &mut extrude);
                    let _ = ui.input_int("Trim Threshold", &mut trim_th);
                    self.state.cfg.border_padding = border.max(0) as u32;
                    self.state.cfg.texture_padding = texpad.max(0) as u32;
                    self.state.cfg.texture_extrusion = extrude.max(0) as u32;
                    self.state.cfg.trim_threshold = trim_th.clamp(0, 255) as u8;
                }

                // Algorithm family + heuristics
                {
                    // Family combo
                    let families = ["Skyline", "MaxRects", "Guillotine", "Auto"];
                    let mut current: usize = match self.state.cfg.family {
                        AlgorithmFamily::Skyline => 0,
                        AlgorithmFamily::MaxRects => 1,
                        AlgorithmFamily::Guillotine => 2,
                        AlgorithmFamily::Auto => 3,
                    };
                    if ui.combo("Algorithm", &mut current, &families, |v: &&str| {
                        std::borrow::Cow::from(*v)
                    }) {
                        self.state.cfg.family = match current {
                            0 => AlgorithmFamily::Skyline,
                            1 => AlgorithmFamily::MaxRects,
                            2 => AlgorithmFamily::Guillotine,
                            _ => AlgorithmFamily::Auto,
                        };
                    }

                    match self.state.cfg.family {
                        AlgorithmFamily::Skyline => {
                            let opts = ["BottomLeft", "MinWaste"];
                            let mut idx: usize = match self.state.cfg.skyline_heuristic {
                                SkylineHeuristic::BottomLeft => 0,
                                SkylineHeuristic::MinWaste => 1,
                            };
                            if ui.combo("Skyline Heuristic", &mut idx, &opts, |v: &&str| {
                                std::borrow::Cow::from(*v)
                            }) {
                                self.state.cfg.skyline_heuristic = match idx {
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
                            let mut idx: usize = match self.state.cfg.mr_heuristic {
                                MaxRectsHeuristic::BestAreaFit => 0,
                                MaxRectsHeuristic::BestShortSideFit => 1,
                                MaxRectsHeuristic::BestLongSideFit => 2,
                                MaxRectsHeuristic::BottomLeft => 3,
                                MaxRectsHeuristic::ContactPoint => 4,
                            };
                            if ui.combo("MaxRects Heuristic", &mut idx, &opts, |v: &&str| {
                                std::borrow::Cow::from(*v)
                            }) {
                                self.state.cfg.mr_heuristic = match idx {
                                    0 => MaxRectsHeuristic::BestAreaFit,
                                    1 => MaxRectsHeuristic::BestShortSideFit,
                                    2 => MaxRectsHeuristic::BestLongSideFit,
                                    3 => MaxRectsHeuristic::BottomLeft,
                                    _ => MaxRectsHeuristic::ContactPoint,
                                };
                            }
                            let mut mr_ref = self.state.cfg.mr_reference;
                            ui.checkbox("Use MaxRects Reference Split/Prune", &mut mr_ref);
                            self.state.cfg.mr_reference = mr_ref;
                        }
                        AlgorithmFamily::Guillotine => {
                            let choices = ["BAF", "BSSF", "BLSF", "WAF", "WSSF", "WLSF"];
                            let splits = [
                                "ShorterLeftoverAxis",
                                "LongerLeftoverAxis",
                                "MinArea",
                                "MaxArea",
                                "ShorterAxis",
                                "LongerAxis",
                            ];
                            let mut cidx = match self.state.cfg.g_choice {
                                GuillotineChoice::BestAreaFit => 0,
                                GuillotineChoice::BestShortSideFit => 1,
                                GuillotineChoice::BestLongSideFit => 2,
                                GuillotineChoice::WorstAreaFit => 3,
                                GuillotineChoice::WorstShortSideFit => 4,
                                GuillotineChoice::WorstLongSideFit => 5,
                            } as i32;
                            if ui.combo("G Choice", &mut cidx, &choices, choices.len() as i32) {
                                self.state.cfg.g_choice = match cidx {
                                    0 => GuillotineChoice::BestAreaFit,
                                    1 => GuillotineChoice::BestShortSideFit,
                                    2 => GuillotineChoice::BestLongSideFit,
                                    3 => GuillotineChoice::WorstAreaFit,
                                    4 => GuillotineChoice::WorstShortSideFit,
                                    _ => GuillotineChoice::WorstLongSideFit,
                                };
                            }
                            let mut sidx = match self.state.cfg.g_split {
                                GuillotineSplit::SplitShorterLeftoverAxis => 0,
                                GuillotineSplit::SplitLongerLeftoverAxis => 1,
                                GuillotineSplit::SplitMinimizeArea => 2,
                                GuillotineSplit::SplitMaximizeArea => 3,
                                GuillotineSplit::SplitShorterAxis => 4,
                                GuillotineSplit::SplitLongerAxis => 5,
                            } as i32;
                            if ui.combo("G Split", &mut sidx, &splits, splits.len() as i32) {
                                self.state.cfg.g_split = match sidx {
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
                            let mut idx = match self.state.cfg.auto_mode {
                                AutoMode::Fast => 0,
                                AutoMode::Quality => 1,
                            } as i32;
                            if ui.combo("Auto Mode", &mut idx, &opts, opts.len() as i32) {
                                self.state.cfg.auto_mode = if idx == 0 {
                                    AutoMode::Fast
                                } else {
                                    AutoMode::Quality
                                };
                            }
                            let mut ms = self.state.cfg.time_budget_ms.unwrap_or(0) as i32;
                            ui.input_int("Time Budget (ms)", &mut ms).build();
                            self.state.cfg.time_budget_ms = Some(ms.max(0) as u64);
                        }
                    }
                }

                ui.separator();
                if ui.button("Pack") {
                    self.do_pack();
                }
                ui.same_line();
                if ui.button("Export") {
                    self.do_export();
                }
                if let Some(err) = &self.state.last_error {
                    ui.text_colored([1.0, 0.2, 0.2, 1.0], err);
                }
            });

        // Preview
        ui.window("Preview")
            .size([720.0, 700.0], Condition::FirstUseEver)
            .build(|| {
                if self.state.previews.is_empty() {
                    ui.text("No preview. Pack to generate.");
                    return;
                }
                let pages = self.state.previews.len();
                let mut page = self.state.selected_page as i32;
                let _ = ui.slider("Page", 0, (pages as i32 - 1).max(0), &mut page);
                self.state.selected_page = page.clamp(0, (pages as i32 - 1).max(0)) as usize;
                let mut zoom = self.state.zoom;
                let _ = ui.slider("Zoom", 0.1f32, 4.0f32, &mut zoom);
                self.state.zoom = zoom;
                ui.separator();
                let pp = &mut self.state.previews[self.state.selected_page];
                let size = [pp.width as f32 * zoom, pp.height as f32 * zoom];
                dear_imgui_rs::Image::new(ui, &mut *pp.tex, size).build();
            });
    }

    fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let delta = now - self.imgui.last_frame;
        self.imgui.last_frame = now;
        self.imgui
            .context
            .io_mut()
            .set_delta_time(delta.as_secs_f32());

        // Pre-warm any textures that want updates (not strictly needed here since we update on pack)
        for pp in &mut self.state.previews {
            if let Ok(res) = self.imgui.renderer.update_texture(&*pp.tex) {
                res.apply_to(&mut *pp.tex);
            }
        }

        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.surface_desc);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => {
                return Ok(());
            }
            Err(e) => return Err(Box::new(e)),
        };
        self.imgui
            .platform
            .prepare_frame(&self.window, &mut self.imgui.context);
        let ui = self.imgui.context.frame();
        // Fullscreen dockspace for docking layout (no enforced layout)
        let _dockspace_id = ui.dockspace_over_main_viewport();

        // Defer side-effecting actions until after the frame to avoid borrows on self conflicts
        let mut want_pick_input = false;
        let mut want_pick_files = false;
        let mut want_pick_output = false;
        let mut want_pack = false;
        let mut want_export = false;
        let mut want_reload = false;

        // Inputs & Config window
        ui.window("Inputs & Config")
            .size([460.0, 520.0], Condition::FirstUseEver)
            .build(|| {
                if ui.button("Pick Input Folder…") {
                    want_pick_input = true;
                }
                ui.same_line();
                if ui.button("Pick Files…") {
                    want_pick_files = true;
                }
                ui.same_line();
                if let Some(dir) = &self.state.input_dir {
                    ui.text(format!("{dir:?}"));
                } else {
                    ui.text("<none>");
                }
                if ui.button("Pick Output Folder…") {
                    want_pick_output = true;
                }
                ui.same_line();
                if let Some(dir) = &self.state.output_dir {
                    ui.text(format!("{dir:?}"));
                } else {
                    ui.text("<none>");
                }
                if ui.button("Reload") {
                    want_reload = true;
                }
                ui.separator();

                ui.text(format!("Inputs loaded: {}", self.state.inputs.len()));
                ui.separator();

                // Dimensions
                {
                    let mut w = self.state.cfg.max_width as i32;
                    let mut h = self.state.cfg.max_height as i32;
                    let _ = ui.input_int("Max Width", &mut w);
                    let _ = ui.input_int("Max Height", &mut h);
                    self.state.cfg.max_width = w.max(1) as u32;
                    self.state.cfg.max_height = h.max(1) as u32;
                }

                // booleans
                {
                    let mut allow_rot = self.state.cfg.allow_rotation;
                    let mut force_max = self.state.cfg.force_max_dimensions;
                    let mut pow2 = self.state.cfg.power_of_two;
                    let mut square = self.state.cfg.square;
                    let mut trim = self.state.cfg.trim;
                    let mut outlines = self.state.cfg.texture_outlines;
                    ui.checkbox("Allow Rotation", &mut allow_rot);
                    ui.checkbox("Force Max Dimensions", &mut force_max);
                    ui.checkbox("Power of Two", &mut pow2);
                    ui.checkbox("Square", &mut square);
                    ui.checkbox("Trim", &mut trim);
                    ui.checkbox("Debug Outlines", &mut outlines);
                    self.state.cfg.allow_rotation = allow_rot;
                    self.state.cfg.force_max_dimensions = force_max;
                    self.state.cfg.power_of_two = pow2;
                    self.state.cfg.square = square;
                    self.state.cfg.trim = trim;
                    self.state.cfg.texture_outlines = outlines;
                }

                // paddings/extrude/trim threshold
                {
                    let mut border = self.state.cfg.border_padding as i32;
                    let mut texpad = self.state.cfg.texture_padding as i32;
                    let mut extrude = self.state.cfg.texture_extrusion as i32;
                    let mut trim_th = self.state.cfg.trim_threshold as i32;
                    let _ = ui.input_int("Border Padding", &mut border);
                    let _ = ui.input_int("Texture Padding", &mut texpad);
                    let _ = ui.input_int("Extrude", &mut extrude);
                    let _ = ui.input_int("Trim Threshold", &mut trim_th);
                    self.state.cfg.border_padding = border.max(0) as u32;
                    self.state.cfg.texture_padding = texpad.max(0) as u32;
                    self.state.cfg.texture_extrusion = extrude.max(0) as u32;
                    self.state.cfg.trim_threshold = trim_th.clamp(0, 255) as u8;
                }

                // Algorithm family + heuristics
                {
                    // Family combo
                    let families = ["Skyline", "MaxRects", "Guillotine", "Auto"];
                    let mut current: usize = match self.state.cfg.family {
                        AlgorithmFamily::Skyline => 0,
                        AlgorithmFamily::MaxRects => 1,
                        AlgorithmFamily::Guillotine => 2,
                        AlgorithmFamily::Auto => 3,
                    };
                    if ui.combo("Algorithm", &mut current, &families, |v: &&str| {
                        std::borrow::Cow::from(*v)
                    }) {
                        self.state.cfg.family = match current {
                            0 => AlgorithmFamily::Skyline,
                            1 => AlgorithmFamily::MaxRects,
                            2 => AlgorithmFamily::Guillotine,
                            _ => AlgorithmFamily::Auto,
                        };
                    }

                    match self.state.cfg.family {
                        AlgorithmFamily::Skyline => {
                            let opts = ["BottomLeft", "MinWaste"];
                            let mut idx: usize = match self.state.cfg.skyline_heuristic {
                                SkylineHeuristic::BottomLeft => 0,
                                SkylineHeuristic::MinWaste => 1,
                            };
                            if ui.combo("Skyline Heuristic", &mut idx, &opts, |v: &&str| {
                                std::borrow::Cow::from(*v)
                            }) {
                                self.state.cfg.skyline_heuristic = match idx {
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
                            let mut idx: usize = match self.state.cfg.mr_heuristic {
                                MaxRectsHeuristic::BestAreaFit => 0,
                                MaxRectsHeuristic::BestShortSideFit => 1,
                                MaxRectsHeuristic::BestLongSideFit => 2,
                                MaxRectsHeuristic::BottomLeft => 3,
                                MaxRectsHeuristic::ContactPoint => 4,
                            };
                            if ui.combo("MaxRects Heuristic", &mut idx, &opts, |v: &&str| {
                                std::borrow::Cow::from(*v)
                            }) {
                                self.state.cfg.mr_heuristic = match idx {
                                    0 => MaxRectsHeuristic::BestAreaFit,
                                    1 => MaxRectsHeuristic::BestShortSideFit,
                                    2 => MaxRectsHeuristic::BestLongSideFit,
                                    3 => MaxRectsHeuristic::BottomLeft,
                                    _ => MaxRectsHeuristic::ContactPoint,
                                };
                            }
                            let mut mr_ref = self.state.cfg.mr_reference;
                            ui.checkbox("Use MaxRects Reference Split/Prune", &mut mr_ref);
                            self.state.cfg.mr_reference = mr_ref;
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
                            let mut cidx: usize = match self.state.cfg.g_choice {
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
                                self.state.cfg.g_choice = match cidx {
                                    0 => GuillotineChoice::BestAreaFit,
                                    1 => GuillotineChoice::BestShortSideFit,
                                    2 => GuillotineChoice::BestLongSideFit,
                                    3 => GuillotineChoice::WorstAreaFit,
                                    4 => GuillotineChoice::WorstShortSideFit,
                                    _ => GuillotineChoice::WorstLongSideFit,
                                };
                            }
                            let mut sidx: usize = match self.state.cfg.g_split {
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
                                self.state.cfg.g_split = match sidx {
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
                            let mut idx: usize = match self.state.cfg.auto_mode {
                                AutoMode::Fast => 0,
                                AutoMode::Quality => 1,
                            };
                            if ui.combo("Auto Mode", &mut idx, &opts, |v: &&str| {
                                std::borrow::Cow::from(*v)
                            }) {
                                self.state.cfg.auto_mode = if idx == 0 {
                                    AutoMode::Fast
                                } else {
                                    AutoMode::Quality
                                };
                            }
                            let mut ms = self.state.cfg.time_budget_ms.unwrap_or(0) as i32;
                            let _ = ui.input_int("Time Budget (ms)", &mut ms);
                            self.state.cfg.time_budget_ms = Some(ms.max(0) as u64);
                        }
                    }
                }

                ui.separator();
                let _ = ui
                    .input_text("Atlas Name", &mut self.state.atlas_name)
                    .build();
                if ui.button("Pack") {
                    want_pack = true;
                }
                ui.same_line();
                if ui.button("Export") {
                    want_export = true;
                }
                if let Some(err) = &self.state.last_error {
                    ui.text_colored([1.0, 0.2, 0.2, 1.0], err);
                }
            });

        // Preview window
        ui.window("Preview")
            .size([720.0, 700.0], Condition::FirstUseEver)
            .build(|| {
                if self.state.previews.is_empty() {
                    ui.text("No preview. Pack to generate.");
                    return;
                }
                let pages = self.state.previews.len();
                let mut page = self.state.selected_page as i32;
                let _ = ui.slider("Page", 0, (pages as i32 - 1).max(0), &mut page);
                self.state.selected_page = page.clamp(0, (pages as i32 - 1).max(0)) as usize;
                let mut zoom = self.state.zoom;
                let _ = ui.slider("Zoom", 0.1f32, 4.0f32, &mut zoom);
                self.state.zoom = zoom;
                ui.separator();
                let pp = &mut self.state.previews[self.state.selected_page];
                let size = [pp.width as f32 * zoom, pp.height as f32 * zoom];
                dear_imgui_rs::Image::new(ui, &mut *pp.tex, size).build();
            });

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        self.imgui
            .platform
            .prepare_render_with_ui(&ui, &self.window);
        let draw_data = self.imgui.context.render();
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.13,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.imgui.renderer.new_frame()?;
            self.imgui
                .renderer
                .render_draw_data(draw_data, &mut rpass)?;
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        // Apply deferred actions
        if want_pick_input {
            self.pick_input_dir();
        }
        if want_pick_files {
            if let Some(files) = rfd::FileDialog::new().set_directory(".").pick_files() {
                if let Err(e) = self.load_inputs_from_paths(&files) {
                    self.set_error(e.to_string());
                }
            }
        }
        if want_pick_output {
            self.pick_output_dir();
        }
        if want_reload {
            if let Err(e) = self.load_inputs() {
                self.set_error(e.to_string());
            }
        }
        if want_pack {
            self.do_pack();
        }
        if want_export {
            self.do_export();
        }
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            match AppWindow::new(event_loop) {
                Ok(window) => {
                    self.window = Some(window);
                }
                Err(e) => {
                    eprintln!("Failed to create window: {e}");
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_mut() else {
            return;
        };
        let full_event: winit::event::Event<()> = winit::event::Event::WindowEvent {
            window_id,
            event: event.clone(),
        };
        window
            .imgui
            .platform
            .handle_event(&mut window.imgui.context, &window.window, &full_event);
        match event {
            WindowEvent::Resized(sz) => {
                window.resize(sz);
                window.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let sz = window.window.inner_size();
                window.resize(sz);
                window.window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = window.render() {
                    error!("Render error: {e}");
                }
                window.window.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.window.request_redraw();
        }
    }
}

fn is_image_path(path: &Path) -> bool {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
    {
        Some(ext)
            if matches!(
                ext.as_str(),
                "png" | "jpg" | "jpeg" | "bmp" | "gif" | "tif" | "tiff"
            ) =>
        {
            true
        }
        _ => false,
    }
}

fn main() {
    // Init tracing (RUST_LOG controls verbosity)
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App { window: None };
    event_loop.run_app(&mut app).unwrap();
}
