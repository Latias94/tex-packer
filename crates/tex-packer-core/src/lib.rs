//! Core texture-atlas packing library.
//!
//! The crate exposes validated offline and runtime workflow facades. Placement,
//! compositing, and preparation details remain implementation-private.
//!
//! ```
//! use image::{DynamicImage, RgbaImage};
//! use tex_packer_core::config::OfflineConfig;
//! use tex_packer_core::error::Result;
//! use tex_packer_core::offline::{InputImage, OfflinePacker};
//!
//! # fn main() -> Result<()> {
//! let packer = OfflinePacker::new(OfflineConfig::default());
//! let output = packer.pack_images(vec![InputImage {
//!     key: "hero".into(),
//!     image: DynamicImage::ImageRgba8(RgbaImage::new(16, 16)),
//! }])?;
//! assert_eq!(output.atlas().stats().num_frames, 1);
//! # Ok(())
//! # }
//! ```

mod compositing;
pub mod config;
pub mod error;
pub mod export;
mod export_manifest;
mod export_plist;
mod free_space;
mod geometry;
pub mod model;
pub mod offline;
mod packer;
mod packing_plan;
mod pipeline;
mod preparation;
pub mod runtime;
mod runtime_atlas;
mod runtime_placement;

pub use config::{OfflineConfig, PackingStrategy, PageConfig, RuntimeConfig};
pub use error::{Result, TexPackerError};
pub use model::{
    Atlas, AtlasDocument, Frame, FrameId, Page, PageId, Rect, Region, RegionId, ResolvedFrame,
};
pub use offline::{InputImage, LayoutItem, OfflinePacker, PackOutput, RenderedPage};
pub use runtime::{
    AtlasSession, RuntimeAtlas, RuntimeImageUpdate, RuntimePlacement, RuntimeStats, UpdateRegion,
};
