use crate::config::RuntimeConfig;
pub use crate::config::{RuntimeStrategy, ShelfPolicy};
use crate::error::{Result, TexPackerError};
use crate::geometry::PlacementGeometry;
use crate::model::{Atlas, Frame, FrameId, Meta, PageId, Rect, Region, RegionId};
use crate::runtime_placement::RuntimePage;

/// Owned logical and physical context returned by runtime placement operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePlacement {
    page_id: PageId,
    frame: Frame,
    region: Region,
}

impl RuntimePlacement {
    pub(crate) const fn new(page_id: PageId, frame: Frame, region: Region) -> Self {
        Self {
            page_id,
            frame,
            region,
        }
    }

    pub const fn page_id(&self) -> PageId {
        self.page_id
    }

    pub const fn frame_id(&self) -> FrameId {
        self.frame.id()
    }

    pub const fn region_id(&self) -> RegionId {
        self.region.id()
    }

    pub const fn frame(&self) -> &Frame {
        &self.frame
    }

    pub const fn region(&self) -> &Region {
        &self.region
    }

    pub const fn content(&self) -> Rect {
        self.region.content()
    }

    pub const fn allocation(&self) -> Rect {
        self.region.allocation()
    }

    pub const fn rotated(&self) -> bool {
        self.region.rotated()
    }
}

/// KTD10 atlas metrics plus runtime allocator fragmentation data.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeStats {
    pub num_pages: usize,
    pub num_frames: usize,
    pub num_regions: usize,
    pub num_aliases: usize,
    pub num_rotated_regions: usize,
    pub num_trimmed_frames: usize,
    pub page_area: u128,
    pub content_area: u128,
    pub allocation_area: u128,
    pub content_occupancy: f64,
    pub allocation_occupancy: f64,
    /// Space currently reported by runtime placement strategies.
    pub allocator_free_area: u128,
    /// Number of allocator free rectangles or segments.
    pub num_free_rects: usize,
}

impl RuntimeStats {
    pub fn summary(&self) -> String {
        format!(
            "Pages: {}, Frames: {}, Regions: {}, Content occupancy: {:.2}%, Allocation occupancy: {:.2}%, Allocator free: {} px² ({} rects)",
            self.num_pages,
            self.num_frames,
            self.num_regions,
            self.content_occupancy * 100.0,
            self.allocation_occupancy * 100.0,
            self.allocator_free_area,
            self.num_free_rects,
        )
    }

    /// Returns the runtime allocator fragmentation ratio.
    pub fn fragmentation(&self) -> f64 {
        if self.allocator_free_area == 0 {
            0.0
        } else {
            self.num_free_rects as f64 / (self.allocator_free_area as f64 / 1000.0).max(1.0)
        }
    }

    /// Returns the percentage of page area not occupied by physical allocations.
    pub fn waste_percentage(&self) -> f64 {
        if self.page_area == 0 {
            0.0
        } else {
            self.page_area.saturating_sub(self.allocation_area) as f64 / self.page_area as f64
                * 100.0
        }
    }
}

pub struct AtlasSession {
    pub(crate) cfg: RuntimeConfig,
    pages: Vec<RuntimePage>,
    next_page_id: u32,
}

impl AtlasSession {
    pub fn new(cfg: RuntimeConfig) -> Self {
        Self {
            cfg,
            pages: Vec::new(),
            next_page_id: 0,
        }
    }

    fn new_page(&mut self) -> Result<RuntimePage> {
        let id = PageId::new(self.next_page_id);
        self.next_page_id =
            self.next_page_id
                .checked_add(1)
                .ok_or_else(|| TexPackerError::InvariantViolation {
                    context: "runtime atlas".into(),
                    reason: format!("page identity space exhausted at {}", self.next_page_id),
                })?;
        let page = self.cfg.page_config();
        Ok(RuntimePage::new(
            id,
            page.max_width(),
            page.max_height(),
            page,
            self.cfg.strategy(),
        ))
    }

    pub fn append(&mut self, key: String, w: u32, h: u32) -> Result<RuntimePlacement> {
        if w == 0 || h == 0 {
            return Err(TexPackerError::InvalidDimensions {
                width: w,
                height: h,
            });
        }

        let page_config = self.cfg.page_config();
        let Some(geometry) = PlacementGeometry::from_size(w, h, page_config) else {
            return Err(TexPackerError::TextureTooLarge {
                key,
                width: w,
                height: h,
                max_width: page_config.max_width(),
                max_height: page_config.max_height(),
            });
        };
        let source = Rect::new(0, 0, w, h);

        for page in &mut self.pages {
            let Some((allocation, rotated)) = page.choose(geometry.reserved_w, geometry.reserved_h)
            else {
                continue;
            };
            let physical = geometry.complete(allocation, rotated);
            return page.place(key, physical, source);
        }

        let mut page = self.new_page()?;
        if let Some((allocation, rotated)) = page.choose(geometry.reserved_w, geometry.reserved_h) {
            let physical = geometry.complete(allocation, rotated);
            let placement = page.place(key.clone(), physical, source)?;
            self.pages.push(page);
            return Ok(placement);
        }

        Err(TexPackerError::OutOfSpace {
            key,
            width: w,
            height: h,
            pages_attempted: self.pages.len() + 1,
        })
    }

    pub fn evict(&mut self, page_id: PageId, key: &str) -> bool {
        self.pages
            .iter_mut()
            .find(|page| page.id() == page_id)
            .and_then(|page| page.evict(key))
            .is_some()
    }

    pub fn snapshot_atlas(&self) -> Result<Atlas> {
        let mut page_refs: Vec<_> = self.pages.iter().collect();
        page_refs.sort_unstable_by_key(|page| page.id());
        let pages = page_refs
            .into_iter()
            .map(RuntimePage::snapshot)
            .collect::<Result<Vec<_>>>()?;
        let page_config = self.cfg.page_config();
        let meta = Meta::for_run(page_config, false, false, "none");
        Atlas::try_new(pages, meta)
    }

    /// Finds the resolved runtime placement for a key.
    pub fn get_frame(&self, key: &str) -> Option<RuntimePlacement> {
        self.pages.iter().find_map(|page| page.placement(key))
    }

    /// Finds the physical allocation reserved for a key.
    pub fn get_reserved_slot(&self, key: &str) -> Option<(PageId, Rect)> {
        self.get_frame(key)
            .map(|placement| (placement.page_id(), placement.allocation()))
    }

    /// Evicts a texture by key without requiring its page identity.
    pub fn evict_by_key(&mut self, key: &str) -> bool {
        self.pages
            .iter_mut()
            .find_map(|page| page.evict(key))
            .is_some()
    }

    pub fn contains(&self, key: &str) -> bool {
        self.pages.iter().any(|page| page.contains(key))
    }

    pub fn keys(&self) -> Vec<&str> {
        self.pages.iter().flat_map(RuntimePage::keys).collect()
    }

    pub fn texture_count(&self) -> usize {
        self.pages.iter().map(RuntimePage::len).sum()
    }

    pub fn stats(&self) -> RuntimeStats {
        let num_pages = self.pages.len();
        let num_frames = self.texture_count();
        let num_regions = num_frames;
        let page_area = self
            .pages
            .iter()
            .map(|page| {
                let (width, height) = page.size();
                u128::from(width) * u128::from(height)
            })
            .sum();
        let content_area = self.pages.iter().map(RuntimePage::content_area).sum();
        let allocation_area = self.pages.iter().map(RuntimePage::allocation_area).sum();
        let num_rotated_regions = self.pages.iter().map(RuntimePage::rotated_regions).sum();
        let (allocator_free_area, num_free_rects) = self
            .pages
            .iter()
            .map(RuntimePage::free_area_and_rects)
            .fold((0u128, 0usize), |(area, count), (page_area, page_count)| {
                (area + u128::from(page_area), count + page_count)
            });

        RuntimeStats {
            num_pages,
            num_frames,
            num_regions,
            num_aliases: 0,
            num_rotated_regions,
            num_trimmed_frames: 0,
            page_area,
            content_area,
            allocation_area,
            content_occupancy: occupancy(content_area, page_area),
            allocation_occupancy: occupancy(allocation_area, page_area),
            allocator_free_area,
            num_free_rects,
        }
    }

    pub(crate) fn page_size(&self, page_id: PageId) -> Option<(u32, u32)> {
        self.pages
            .iter()
            .find(|page| page.id() == page_id)
            .map(RuntimePage::size)
    }
}

fn occupancy(area: u128, page_area: u128) -> f64 {
    if page_area == 0 {
        0.0
    } else {
        area as f64 / page_area as f64
    }
}
