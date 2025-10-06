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

/// Statistics about atlas packing efficiency.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PackStats {
    /// Total number of pages in the atlas.
    pub num_pages: usize,
    /// Total number of frames (textures) packed.
    pub num_frames: usize,
    /// Total area of all pages (sum of width * height for each page).
    pub total_page_area: u64,
    /// Total area used by all frames (sum of frame width * height).
    pub used_frame_area: u64,
    /// Occupancy ratio: used_frame_area / total_page_area (0.0 to 1.0).
    /// Higher is better (less wasted space).
    pub occupancy: f64,
    /// Average page dimensions.
    pub avg_page_width: f64,
    pub avg_page_height: f64,
    /// Largest page dimensions.
    pub max_page_width: u32,
    pub max_page_height: u32,
    /// Number of rotated frames.
    pub num_rotated: usize,
    /// Number of trimmed frames.
    pub num_trimmed: usize,
}

impl<K> Atlas<K> {
    /// Computes packing statistics for this atlas.
    pub fn stats(&self) -> PackStats {
        let num_pages = self.pages.len();
        let mut num_frames = 0;
        let mut total_page_area = 0u64;
        let mut used_frame_area = 0u64;
        let mut max_page_width = 0u32;
        let mut max_page_height = 0u32;
        let mut num_rotated = 0;
        let mut num_trimmed = 0;

        for page in &self.pages {
            let page_area = (page.width as u64) * (page.height as u64);
            total_page_area += page_area;
            max_page_width = max_page_width.max(page.width);
            max_page_height = max_page_height.max(page.height);

            for frame in &page.frames {
                num_frames += 1;
                let frame_area = (frame.frame.w as u64) * (frame.frame.h as u64);
                used_frame_area += frame_area;

                if frame.rotated {
                    num_rotated += 1;
                }
                if frame.trimmed {
                    num_trimmed += 1;
                }
            }
        }

        let occupancy = if total_page_area > 0 {
            used_frame_area as f64 / total_page_area as f64
        } else {
            0.0
        };

        let (avg_page_width, avg_page_height) = if num_pages > 0 {
            let total_width: u64 = self.pages.iter().map(|p| p.width as u64).sum();
            let total_height: u64 = self.pages.iter().map(|p| p.height as u64).sum();
            (
                total_width as f64 / num_pages as f64,
                total_height as f64 / num_pages as f64,
            )
        } else {
            (0.0, 0.0)
        };

        PackStats {
            num_pages,
            num_frames,
            total_page_area,
            used_frame_area,
            occupancy,
            avg_page_width,
            avg_page_height,
            max_page_width,
            max_page_height,
            num_rotated,
            num_trimmed,
        }
    }
}

impl PackStats {
    /// Returns a human-readable summary of the statistics.
    pub fn summary(&self) -> String {
        format!(
            "Pages: {}, Frames: {}, Occupancy: {:.2}%, Total Area: {} px², Used Area: {} px², Rotated: {}, Trimmed: {}",
            self.num_pages,
            self.num_frames,
            self.occupancy * 100.0,
            self.total_page_area,
            self.used_frame_area,
            self.num_rotated,
            self.num_trimmed,
        )
    }

    /// Returns wasted space in pixels.
    pub fn wasted_area(&self) -> u64 {
        self.total_page_area.saturating_sub(self.used_frame_area)
    }

    /// Returns wasted space as a percentage (0.0 to 100.0).
    pub fn waste_percentage(&self) -> f64 {
        if self.total_page_area > 0 {
            (self.wasted_area() as f64 / self.total_page_area as f64) * 100.0
        } else {
            0.0
        }
    }
}
