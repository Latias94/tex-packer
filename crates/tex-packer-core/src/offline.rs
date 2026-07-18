use image::{DynamicImage, RgbaImage};

use crate::config::OfflineConfig;
use crate::error::Result;
use crate::model::{Atlas, PackStats, PageId, Rect};

/// A decoded image and its logical atlas key.
#[derive(Debug)]
pub struct InputImage {
    pub key: String,
    pub image: DynamicImage,
}

/// A size-only input with optional source reconstruction metadata.
#[derive(Debug, Clone)]
pub struct LayoutItem {
    pub key: String,
    pub w: u32,
    pub h: u32,
    pub source: Option<Rect>,
    pub source_size: Option<(u32, u32)>,
    pub trimmed: bool,
}

/// Rendered pixels for one page in the accompanying atlas.
pub struct RenderedPage {
    pub(crate) page_id: PageId,
    pub(crate) rgba: RgbaImage,
}

impl RenderedPage {
    pub fn page_id(&self) -> PageId {
        self.page_id
    }

    pub fn rgba(&self) -> &RgbaImage {
        &self.rgba
    }

    pub fn into_rgba(self) -> RgbaImage {
        self.rgba
    }
}

/// A validated atlas and its rendered page payloads.
pub struct PackOutput {
    pub(crate) atlas: Atlas,
    pub(crate) pages: Vec<RenderedPage>,
}

impl PackOutput {
    pub fn atlas(&self) -> &Atlas {
        &self.atlas
    }

    pub fn pages(&self) -> &[RenderedPage] {
        &self.pages
    }

    pub fn into_parts(self) -> (Atlas, Vec<RenderedPage>) {
        (self.atlas, self.pages)
    }

    pub fn stats(&self) -> PackStats {
        self.atlas.stats()
    }
}

/// Stateful entry point for all offline packing workflows.
pub struct OfflinePacker {
    config: OfflineConfig,
}

impl OfflinePacker {
    pub fn new(config: OfflineConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &OfflineConfig {
        &self.config
    }

    /// Packs decoded images and renders the resulting atlas pages.
    pub fn pack_images(&self, inputs: Vec<InputImage>) -> Result<PackOutput> {
        crate::pipeline::pack_images_impl(inputs, &self.config)
    }

    /// Packs decoded images without rendering page pixels.
    pub fn layout_images(&self, inputs: Vec<InputImage>) -> Result<Atlas> {
        crate::pipeline::layout_images_impl(inputs, &self.config)
    }

    /// Packs caller-supplied layout records without pixel preprocessing or deduplication.
    pub fn pack_layout(&self, items: Vec<LayoutItem>) -> Result<Atlas> {
        crate::pipeline::pack_layout_impl(items, &self.config)
    }
}
