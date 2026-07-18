use crate::config::{PackerConfig, SkylineHeuristic};
use crate::error::{Result, TexPackerError};
use crate::geometry::PlacementGeometry;
use crate::model::{Atlas, Frame, Meta, Page, Rect};
use crate::runtime_placement::RuntimePage;

#[derive(Debug, Clone)]
pub enum RuntimeStrategy {
    Guillotine,
    Shelf(ShelfPolicy),
    Skyline(SkylineHeuristic),
}

#[derive(Debug, Clone, Copy)]
pub enum ShelfPolicy {
    NextFit,
    FirstFit,
}

/// Runtime statistics for an atlas session.
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    /// Number of pages currently allocated.
    pub num_pages: usize,
    /// Number of textures currently in the atlas.
    pub num_textures: usize,
    /// Total area of all pages (width * height * num_pages).
    pub total_page_area: u64,
    /// Total area occupied by textures (sum of all used slots).
    pub total_used_area: u64,
    /// Total free area available for new textures.
    pub total_free_area: u64,
    /// Occupancy ratio: used_area / total_page_area (0.0 to 1.0).
    pub occupancy: f64,
    /// Number of free rectangles/segments (fragmentation indicator).
    pub num_free_rects: usize,
}

impl RuntimeStats {
    /// Returns a human-readable summary of the statistics.
    pub fn summary(&self) -> String {
        format!(
            "Pages: {}, Textures: {}, Occupancy: {:.2}%, Free: {} px² ({} rects), Used: {} px²",
            self.num_pages,
            self.num_textures,
            self.occupancy * 100.0,
            self.total_free_area,
            self.num_free_rects,
            self.total_used_area,
        )
    }

    /// Returns the fragmentation ratio (higher = more fragmented).
    /// Calculated as: num_free_rects / (total_free_area / 1000)
    /// Lower is better (fewer, larger free blocks).
    pub fn fragmentation(&self) -> f64 {
        if self.total_free_area == 0 {
            0.0
        } else {
            (self.num_free_rects as f64) / ((self.total_free_area as f64) / 1000.0).max(1.0)
        }
    }

    /// Returns the waste percentage (0.0 to 100.0).
    pub fn waste_percentage(&self) -> f64 {
        if self.total_page_area > 0 {
            (self.total_free_area as f64 / self.total_page_area as f64) * 100.0
        } else {
            0.0
        }
    }
}

pub struct AtlasSession {
    pub(crate) cfg: PackerConfig,
    _strategy: RuntimeStrategy,
    pages: Vec<RuntimePage>,
    next_id: usize,
}

impl AtlasSession {
    pub fn new(cfg: PackerConfig, strategy: RuntimeStrategy) -> Self {
        Self {
            cfg,
            _strategy: strategy,
            pages: Vec::new(),
            next_id: 0,
        }
    }

    fn new_page(&mut self) -> RuntimePage {
        let id = self.next_id;
        self.next_id += 1;
        RuntimePage::new(
            id,
            self.cfg.max_width,
            self.cfg.max_height,
            &self.cfg,
            &self._strategy,
        )
    }

    pub fn append(&mut self, key: String, w: u32, h: u32) -> Result<(usize, Frame<String>)> {
        let geometry = PlacementGeometry::from_size(w, h, &self.cfg);
        // Try existing pages
        for idx in 0..self.pages.len() {
            let (slot, rotated, id);
            {
                let p = &self.pages[idx];
                if let Some((s, r)) = p.choose(geometry.reserved_w, geometry.reserved_h) {
                    slot = s;
                    rotated = r;
                    id = p.id;
                } else {
                    continue;
                }
            }
            let frame = geometry.frame(key.clone(), Rect::new(0, 0, w, h), &slot, rotated);
            let p = &mut self.pages[idx];
            p.place(&key, &slot, &frame, rotated);
            return Ok((id, frame));
        }
        // Grow: add a new page and place
        let mut page = self.new_page();
        if let Some((slot, rotated)) = page.choose(geometry.reserved_w, geometry.reserved_h) {
            let frame = geometry.frame(key.clone(), Rect::new(0, 0, w, h), &slot, rotated);
            page.place(&key, &slot, &frame, rotated);
            let id = page.id;
            self.pages.push(page);
            return Ok((id, frame));
        }
        Err(TexPackerError::OutOfSpace {
            key,
            width: w,
            height: h,
            pages_attempted: self.pages.len() + 1,
        })
    }

    pub fn evict(&mut self, page_id: usize, key: &str) -> bool {
        if let Some(p) = self.pages.iter_mut().find(|p| p.id == page_id)
            && let Some((slot, _rot, _frame)) = p.used.remove(key)
        {
            p.add_free(slot);
            return true;
        }
        false
    }

    pub fn snapshot_atlas(&self) -> Atlas<String> {
        let mut pages: Vec<Page<String>> = Vec::new();
        for p in &self.pages {
            let mut frames: Vec<Frame<String>> = Vec::new();
            for (_slot, _rot, f) in p.used.values() {
                frames.push(f.clone());
            }
            pages.push(Page {
                id: p.id,
                width: p.width,
                height: p.height,
                frames,
            });
        }
        let meta = Meta {
            schema_version: "1".into(),
            app: "tex-packer".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            format: "RGBA8888".into(),
            scale: 1.0,
            power_of_two: self.cfg.power_of_two,
            square: self.cfg.square,
            max_dim: (self.cfg.max_width, self.cfg.max_height),
            padding: (self.cfg.border_padding, self.cfg.texture_padding),
            extrude: self.cfg.texture_extrusion,
            allow_rotation: self.cfg.allow_rotation,
            trim_mode: if self.cfg.trim { "trim" } else { "none" }.into(),
            background_color: None,
        };
        Atlas { pages, meta }
    }

    /// Find a frame by its key.
    /// Returns the page ID and a reference to the frame if found.
    pub fn get_frame(&self, key: &str) -> Option<(usize, &Frame<String>)> {
        for page in &self.pages {
            if let Some((_slot, _rot, frame)) = page.used.get(key) {
                return Some((page.id, frame));
            }
        }
        None
    }

    /// Find the reserved slot rectangle for a texture key (pre-offset area including padding/extrude).
    pub fn get_reserved_slot(&self, key: &str) -> Option<(usize, Rect)> {
        for page in &self.pages {
            if let Some((slot, _rot, _frame)) = page.used.get(key) {
                return Some((page.id, *slot));
            }
        }
        None
    }

    /// Evict a texture by its key without needing to know the page ID.
    /// Returns true if the texture was found and evicted.
    pub fn evict_by_key(&mut self, key: &str) -> bool {
        for page in &mut self.pages {
            if let Some((slot, _rot, _frame)) = page.used.remove(key) {
                page.add_free(slot);
                return true;
            }
        }
        false
    }

    /// Check if a texture with the given key exists.
    pub fn contains(&self, key: &str) -> bool {
        self.pages.iter().any(|p| p.used.contains_key(key))
    }

    /// Get all texture keys currently in the atlas.
    pub fn keys(&self) -> Vec<&str> {
        self.pages
            .iter()
            .flat_map(|p| p.used.keys().map(|k| k.as_str()))
            .collect()
    }

    /// Get the number of textures currently in the atlas.
    pub fn texture_count(&self) -> usize {
        self.pages.iter().map(|p| p.used.len()).sum()
    }

    /// Get runtime statistics about the atlas session.
    pub fn stats(&self) -> RuntimeStats {
        let num_pages = self.pages.len();
        let num_textures = self.texture_count();

        let mut total_used_area = 0u64;
        let mut total_free_area = 0u64;
        let mut num_free_rects = 0;

        for page in &self.pages {
            total_used_area += page.used_area();
            let (free_area, free_rects) = page.free_area_and_rects();
            total_free_area += free_area;
            num_free_rects += free_rects;
        }

        let total_page_area = if num_pages > 0 {
            (self.cfg.max_width as u64) * (self.cfg.max_height as u64) * (num_pages as u64)
        } else {
            0
        };

        let occupancy = if total_page_area > 0 {
            total_used_area as f64 / total_page_area as f64
        } else {
            0.0
        };

        RuntimeStats {
            num_pages,
            num_textures,
            total_page_area,
            total_used_area,
            total_free_area,
            occupancy,
            num_free_rects,
        }
    }
}
