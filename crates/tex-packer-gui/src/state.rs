//! Application state

use crate::presets::PackerPreset;
use crate::stats::PackStats;
use dear_imgui_rs::texture::TextureData;
use std::path::PathBuf;
use tex_packer_core::prelude::*;
use tracing::{error, info};

/// Preview page with texture
pub struct PreviewPage {
    pub tex: Box<TextureData>,
    pub width: u32,
    pub height: u32,
}

impl Default for PreviewPage {
    fn default() -> Self {
        let mut tex = TextureData::new();
        tex.create(dear_imgui_rs::texture::TextureFormat::RGBA32, 1, 1);
        tex.set_data(&[0x00, 0x00, 0x00, 0xFF]);
        Self {
            tex,
            width: 1,
            height: 1,
        }
    }
}

/// Main application state
pub struct AppState {
    // IO
    pub input_dir: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub inputs: Vec<InputImage>,

    // Preset system
    pub presets: Vec<PackerPreset>,
    pub selected_preset_idx: usize,
    pub selected_size_idx: usize,
    pub is_custom_preset: bool, // True when user modifies config

    // Config (from preset or custom)
    pub cfg: PackerConfig,

    // Result
    pub result: Option<PackOutput>,
    pub previews: Vec<PreviewPage>,
    pub stats: Option<PackStats>,

    // UI state
    pub selected_page: usize,
    pub atlas_name: String,
    pub fit_to_window: bool,
    pub zoom: f32,
    pub show_advanced: bool,

    // Errors
    pub last_error: Option<String>,

    // Dock layout
    pub layout_built: bool,
}

impl Default for AppState {
    fn default() -> Self {
        let presets = PackerPreset::all();
        let default_preset = PackerPreset::default();
        let cfg = default_preset.config.clone();

        Self {
            input_dir: None,
            output_dir: None,
            inputs: Vec::new(),

            presets,
            selected_preset_idx: 0, // Quality is default
            selected_size_idx: 1,   // 2048x2048 is default
            is_custom_preset: false,

            cfg,

            result: None,
            previews: Vec::new(),
            stats: None,

            selected_page: 0,
            atlas_name: "atlas".into(),
            fit_to_window: true,
            zoom: 1.0,
            show_advanced: false,

            last_error: None,
            layout_built: false,
        }
    }
}

impl AppState {
    pub fn set_error(&mut self, msg: impl ToString) {
        let s = msg.to_string();
        error!("{s}");
        self.last_error = Some(s);
    }

    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    /// Apply a preset by index
    pub fn apply_preset(&mut self, preset_idx: usize) {
        if let Some(preset) = self.presets.get(preset_idx) {
            self.cfg = preset.config.clone();
            self.selected_preset_idx = preset_idx;
            self.is_custom_preset = false;
            info!("Applied preset: {}", preset.name);
        }
    }

    /// Mark config as custom (user modified)
    pub fn mark_custom(&mut self) {
        self.is_custom_preset = true;
    }

    /// Get current preset
    pub fn current_preset(&self) -> &PackerPreset {
        &self.presets[self.selected_preset_idx]
    }

    /// Get recommended sizes for current preset
    pub fn recommended_sizes(&self) -> &[(u32, u32)] {
        &self.current_preset().recommended_sizes
    }

    /// Apply a recommended size
    pub fn apply_size(&mut self, size_idx: usize) {
        let sizes = self.recommended_sizes();
        if let Some(&(w, h)) = sizes.get(size_idx) {
            self.cfg.max_width = w;
            self.cfg.max_height = h;
            self.selected_size_idx = size_idx;
        }
    }

    pub fn pick_input_dir(&mut self) {
        if let Some(d) = rfd::FileDialog::new().set_directory(".").pick_folder() {
            self.input_dir = Some(d);
            if let Err(e) = self.load_inputs() {
                self.set_error(e.to_string());
            }
        }
    }

    pub fn pick_files(&mut self) {
        if let Some(files) = rfd::FileDialog::new().set_directory(".").pick_files() {
            if let Err(e) = self.load_inputs_from_paths(&files) {
                self.set_error(e.to_string());
            }
        }
    }

    pub fn pick_output_dir(&mut self) {
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
                let img = image::ImageReader::open(path)?.decode()?;
                self.inputs.push(InputImage { key, image: img });
            }
        }
        info!("Loaded {} images (files)", self.inputs.len());
        Ok(())
    }

    pub fn load_inputs(&mut self) -> anyhow::Result<()> {
        self.inputs.clear();
        let Some(dir) = &self.input_dir else {
            return Ok(());
        };
        let mut count = 0usize;
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let path = entry.path();
                if is_image_path(&path) {
                    let key = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    let img = image::ImageReader::open(&path)?.decode()?;
                    self.inputs.push(InputImage { key, image: img });
                    count += 1;
                }
            }
        }
        info!("Loaded {} images", count);
        Ok(())
    }

    pub fn clear_result(&mut self) {
        self.result = None;
        self.previews.clear();
        self.stats = None;
        self.selected_page = 0;
    }

    pub fn do_pack(&mut self) {
        self.clear_result();
        self.clear_error();

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

        let num_images = inputs.len();
        let start = std::time::Instant::now();

        match pack_images(inputs, self.cfg.clone()) {
            Ok(out) => {
                let pack_time_ms = start.elapsed().as_millis() as u64;

                // Create previews
                let mut previews = Vec::with_capacity(out.pages.len());
                for p in &out.pages {
                    let mut tex = TextureData::new();
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

                // Calculate stats
                let stats = PackStats::from_output(&out, num_images, pack_time_ms);
                info!("{}", stats.status_string());

                self.previews = previews;
                self.stats = Some(stats);
                self.result = Some(out);
            }
            Err(e) => {
                self.set_error(format!("Pack error: {e:?}"));
            }
        }
    }

    pub fn do_export(&mut self) {
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
        if let Err(e) = std::fs::write(&json_path, serde_json::to_string_pretty(&json).unwrap()) {
            self.set_error(format!("Failed writing {:?}: {e}", json_path));
            return;
        }

        info!("Exported atlas to {:?}", outdir);
    }
}

fn is_image_path(path: &std::path::Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase()),
        Some(ext) if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "gif" | "tif" | "tiff")
    )
}

