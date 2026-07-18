use std::collections::HashMap;

use crate::config::RuntimeConfig;
use crate::error::{Result, TexPackerError};
use crate::geometry::PlacementGeometry;
use crate::model::{Atlas, Frame, FrameId, Meta, PageId, Rect, Region, RegionId};
use crate::runtime_placement::{PreparedPageAppend, RuntimePage};

pub use crate::runtime_atlas::{RuntimeAtlas, RuntimeImageUpdate, UpdateRegion};

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
    page_slots: HashMap<PageId, usize>,
    key_index: HashMap<String, RuntimeHandle>,
    next_page_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeHandle {
    page_id: PageId,
    frame_id: FrameId,
    region_id: RegionId,
}

enum PreparedAppendTarget {
    Existing {
        page_index: usize,
        page_append: PreparedPageAppend,
    },
    New {
        page: RuntimePage,
        next_page_id: u32,
        placement: RuntimePlacement,
    },
}

pub(crate) struct PreparedAppend {
    target: PreparedAppendTarget,
    page_size: (u32, u32),
    key: String,
}

impl PreparedAppend {
    pub(crate) const fn placement(&self) -> &RuntimePlacement {
        match &self.target {
            PreparedAppendTarget::Existing { page_append, .. } => page_append.placement(),
            PreparedAppendTarget::New { placement, .. } => placement,
        }
    }

    pub(crate) const fn page_size(&self) -> (u32, u32) {
        self.page_size
    }
}

impl AtlasSession {
    pub fn new(cfg: RuntimeConfig) -> Self {
        Self {
            cfg,
            pages: Vec::new(),
            page_slots: HashMap::new(),
            key_index: HashMap::new(),
            next_page_id: 0,
        }
    }

    pub fn append(&mut self, key: String, w: u32, h: u32) -> Result<RuntimePlacement> {
        let prepared = self.prepare_append(key, w, h)?;
        Ok(self.commit_append(prepared))
    }

    pub(crate) fn prepare_append(&self, key: String, w: u32, h: u32) -> Result<PreparedAppend> {
        if self.key_index.contains_key(&key) {
            return Err(TexPackerError::DuplicateKey { key });
        }
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

        for (page_index, page) in self.pages.iter().enumerate() {
            if let Some(page_append) = page.prepare_append(&key, geometry, source)? {
                return Ok(PreparedAppend {
                    target: PreparedAppendTarget::Existing {
                        page_index,
                        page_append,
                    },
                    page_size: page.size(),
                    key,
                });
            }
        }

        let page_id = PageId::new(self.next_page_id);
        let next_page_id =
            self.next_page_id
                .checked_add(1)
                .ok_or_else(|| TexPackerError::InvariantViolation {
                    context: "runtime atlas".into(),
                    reason: format!("page identity space exhausted at {}", self.next_page_id),
                })?;
        let page_config = self.cfg.page_config();
        let mut page = RuntimePage::new(
            page_id,
            page_config.max_width(),
            page_config.max_height(),
            page_config,
            self.cfg.strategy(),
        );
        let page_size = page.size();
        if let Some(page_append) = page.prepare_append(&key, geometry, source)? {
            let placement = page.commit_append(page_append);
            return Ok(PreparedAppend {
                target: PreparedAppendTarget::New {
                    page,
                    next_page_id,
                    placement,
                },
                page_size,
                key,
            });
        }

        Err(TexPackerError::OutOfSpace {
            key,
            width: w,
            height: h,
            pages_attempted: self.pages.len() + 1,
        })
    }

    pub(crate) fn commit_append(&mut self, prepared: PreparedAppend) -> RuntimePlacement {
        let placement = match prepared.target {
            PreparedAppendTarget::Existing {
                page_index,
                page_append,
            } => self.pages[page_index].commit_append(page_append),
            PreparedAppendTarget::New {
                page,
                next_page_id,
                placement,
            } => {
                let page_id = page.id();
                let page_slot = self.pages.len();
                self.pages.push(page);
                let replaced_slot = self.page_slots.insert(page_id, page_slot);
                debug_assert!(replaced_slot.is_none(), "new page identity must be unique");
                self.next_page_id = next_page_id;
                placement
            }
        };
        let handle = RuntimeHandle {
            page_id: placement.page_id(),
            frame_id: placement.frame_id(),
            region_id: placement.region_id(),
        };
        let replaced_handle = self.key_index.insert(prepared.key, handle);
        debug_assert!(
            replaced_handle.is_none(),
            "prepared runtime key must remain unique"
        );
        placement
    }

    pub fn evict(&mut self, page_id: PageId, key: &str) -> bool {
        let Some(handle) = self.key_index.get(key).copied() else {
            return false;
        };
        if handle.page_id != page_id {
            return false;
        }
        self.evict_handle(key, handle)
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
        let handle = self.key_index.get(key)?;
        let page_slot = *self.page_slots.get(&handle.page_id)?;
        self.pages
            .get(page_slot)?
            .placement(handle.frame_id, handle.region_id)
    }

    /// Finds the physical allocation reserved for a key.
    pub fn get_reserved_slot(&self, key: &str) -> Option<(PageId, Rect)> {
        self.get_frame(key)
            .map(|placement| (placement.page_id(), placement.allocation()))
    }

    /// Evicts a texture by key without requiring its page identity.
    pub fn evict_by_key(&mut self, key: &str) -> bool {
        let Some(handle) = self.key_index.get(key).copied() else {
            return false;
        };
        self.evict_handle(key, handle)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.key_index.contains_key(key)
    }

    pub fn keys(&self) -> Vec<&str> {
        let mut entries: Vec<_> = self.key_index.iter().collect();
        entries.sort_unstable_by_key(|(_, handle)| (handle.page_id, handle.frame_id));
        entries.into_iter().map(|(key, _)| key.as_str()).collect()
    }

    pub fn texture_count(&self) -> usize {
        self.pages.iter().map(RuntimePage::len).sum()
    }

    pub fn stats(&self) -> RuntimeStats {
        let num_pages = self.pages.len();
        let num_frames = self.texture_count();
        let num_regions = self.pages.iter().map(RuntimePage::region_count).sum();
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

    fn evict_handle(&mut self, key: &str, handle: RuntimeHandle) -> bool {
        let Some(&page_slot) = self.page_slots.get(&handle.page_id) else {
            return false;
        };
        let Some(page) = self.pages.get_mut(page_slot) else {
            return false;
        };
        if page.evict(handle.frame_id, handle.region_id).is_none() {
            return false;
        }
        let removed = self.key_index.remove(key);
        debug_assert_eq!(removed, Some(handle));
        true
    }
}

fn occupancy(area: u128, page_area: u128) -> f64 {
    if page_area == 0 {
        0.0
    } else {
        area as f64 / page_area as f64
    }
}
