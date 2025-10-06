use serde::{Deserialize, Serialize};

/// Axis-aligned rectangle (pixels). `x,y` is top-left; `w,h` are sizes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }
    /// Inclusive right edge coordinate (`x + w - 1`).
    pub fn right(&self) -> u32 {
        self.x + self.w.saturating_sub(1)
    }
    /// Inclusive bottom edge coordinate (`y + h - 1`).
    pub fn bottom(&self) -> u32 {
        self.y + self.h.saturating_sub(1)
    }
    /// Returns true if `r` is fully inside `self` (inclusive edges).
    pub fn contains(&self, r: &Rect) -> bool {
        r.x >= self.x && r.y >= self.y && r.right() <= self.right() && r.bottom() <= self.bottom()
    }
}

/// A placed frame within a page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame<K = String> {
    /// User-specified key (e.g., filename or asset path).
    pub key: K,
    /// Placed rectangle within the page (post-rotation width/height).
    pub frame: Rect,
    /// True if the frame was rotated 90° when placed.
    pub rotated: bool,
    /// True if the source was trimmed.
    pub trimmed: bool,
    /// Source sub-rect within the original image after trimming.
    pub source: Rect,
    /// Original (untrimmed) image size.
    pub source_size: (u32, u32),
}

/// A single atlas page (logical record).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<K = String> {
    pub id: usize,
    pub width: u32,
    pub height: u32,
    pub frames: Vec<Frame<K>>,
}

/// Atlas-level metadata (common fields used by exporters/templates).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    /// Schema version for JSON metadata formats (e.g., json-array/json-hash).
    /// Allows downstream tooling to handle future additive changes.
    /// String to allow non-integer versions like "1.0"; current: "1".
    pub schema_version: String,
    pub app: String,
    pub version: String,
    pub format: String,
    pub scale: f32,
    pub power_of_two: bool,
    pub square: bool,
    pub max_dim: (u32, u32),
    pub padding: (u32, u32),
    pub extrude: u32,
    pub allow_rotation: bool,
    pub trim_mode: String,
    pub background_color: Option<[u8; 4]>,
}

/// Atlas of pages and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atlas<K = String> {
    pub pages: Vec<Page<K>>,
    pub meta: Meta,
}
