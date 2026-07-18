//! Core library for packing textures into atlases.
//!
//! - Algorithms: Skyline (BL/MW + optional Waste Map), MaxRects (BAF/BSSF/BLSF/BL/CP), Guillotine (choice + split)
//! - Pipeline: `pack_images` takes in-memory images and returns pages + metadata
//! - Data model is serde-serializable; exporters are provided in helpers and the CLI crate.
//!
//! Quick example:
//! ```ignore
//! use image::ImageReader;
//! use tex_packer_core::{InputImage, OfflineConfig, pack_images};
//! # fn main() -> anyhow::Result<()> {
//! let img1 = ImageReader::open("a.png")?.decode()?;
//! let img2 = ImageReader::open("b.png")?.decode()?;
//! let inputs = vec![
//!   InputImage { key: "a".into(), image: img1 },
//!   InputImage { key: "b".into(), image: img2 },
//! ];
//! let cfg = OfflineConfig::default();
//! let out = pack_images(inputs, cfg)?;
//! println!("pages: {}", out.pages.len());
//! # Ok(()) }
//! ```

pub mod compositing;
pub mod config;
pub mod error;
pub mod export;
mod export_manifest;
pub mod export_plist;
mod free_space;
mod geometry;
pub mod model;
mod packer;
mod packing_plan;
pub mod pipeline;
mod preparation;
pub mod runtime;
pub mod runtime_atlas;
mod runtime_placement;

pub use config::*;
pub use error::*;
pub use export::*;
pub use export_manifest::{TemplateContext, TemplatePage, TemplateSprite, to_template_context};
pub use export_plist::*;
pub use model::*;
pub use pipeline::*;
pub use preparation::compute_trim_rect;

/// Convenience prelude for common types and functions.
/// Importing `tex_packer_core::prelude::*` brings the primary APIs into scope.
pub mod prelude {
    pub use crate::config::{
        AutoMode, GuillotineChoice, GuillotineSplit, MaxRectsHeuristic, OfflineConfig,
        OfflineConfigBuilder, PackingStrategy, PageConfig, PageConfigBuilder, RuntimeConfig,
        RuntimeConfigBuilder, RuntimeStrategy, ShelfPolicy, SkylineHeuristic, SortOrder,
        TransparentPolicy,
    };
    pub use crate::model::{Atlas, Frame, Meta, PackStats, Page, Rect};
    pub use crate::pipeline::LayoutItem;
    pub use crate::runtime::{AtlasSession, RuntimeStats};
    pub use crate::runtime_atlas::{RuntimeAtlas, UpdateRegion};
    pub use crate::{
        InputImage, OutputPage, PackOutput, pack_images, pack_layout, pack_layout_items,
    };
}
