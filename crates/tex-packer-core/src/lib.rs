//! Core library for packing textures into atlases.
//!
//! - Algorithms: Skyline (BL/MW + optional Waste Map), MaxRects (BAF/BSSF/BLSF/BL/CP), Guillotine (choice + split)
//! - Pipeline: `pack_images` takes in-memory images and returns pages + metadata
//! - Data model is serde-serializable; exporters are provided in helpers and the CLI crate.
//!
//! Quick example:
//! ```ignore
//! use image::ImageReader;
//! use tex_packer_core::{InputImage, PackerConfig, pack_images};
//! # fn main() -> anyhow::Result<()> {
//! let img1 = ImageReader::open("a.png")?.decode()?;
//! let img2 = ImageReader::open("b.png")?.decode()?;
//! let inputs = vec![
//!   InputImage { key: "a".into(), image: img1 },
//!   InputImage { key: "b".into(), image: img2 },
//! ];
//! let cfg = PackerConfig { max_width: 1024, max_height: 1024, ..Default::default() };
//! let out = pack_images(inputs, cfg)?;
//! println!("pages: {}", out.pages.len());
//! # Ok(()) }
//! ```

pub mod config;
pub mod error;
pub mod export;
pub mod export_plist;
pub mod model;
pub mod packer;
pub mod pipeline;
pub mod runtime;
pub mod runtime_atlas;

pub use config::*;
pub use error::*;
pub use export::*;
pub use export_plist::*;
pub use model::*;
pub use packer::*;
pub use pipeline::*;

/// Convenience prelude for common types and functions.
/// Importing `tex_packer_core::prelude::*` brings the primary APIs into scope.
pub mod prelude {
    pub use crate::config::{
        AlgorithmFamily, AutoMode, GuillotineChoice, GuillotineSplit, MaxRectsHeuristic,
        PackerConfig, PackerConfigBuilder, SkylineHeuristic, SortOrder,
    };
    pub use crate::model::{Atlas, Frame, Meta, Page, PackStats, Rect};
    pub use crate::pipeline::LayoutItem;
    pub use crate::runtime::{AtlasSession, RuntimeStats, RuntimeStrategy, ShelfPolicy};
    pub use crate::runtime_atlas::{RuntimeAtlas, UpdateRegion};
    pub use crate::{
        pack_images, pack_layout, pack_layout_items, InputImage, OutputPage, PackOutput,
    };
}
