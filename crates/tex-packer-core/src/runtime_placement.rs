mod guillotine;
mod shelf;
mod skyline;

use std::collections::HashMap;

use self::guillotine::RuntimeGuillotine;
use self::shelf::RuntimeShelfPlacement;
use self::skyline::RuntimeSkyline;
use crate::config::{PageConfig, RuntimeStrategy};
use crate::error::{Result, TexPackerError};
use crate::geometry::{PhysicalPlacement, usable_area};
use crate::model::{Frame, FrameId, Page, PageId, Rect, Region, RegionId};
use crate::runtime::RuntimePlacement;

#[derive(Debug, Clone)]
struct RuntimeEntry {
    region: Region,
    frame: Frame,
}

pub(crate) struct RuntimePage {
    id: PageId,
    width: u32,
    height: u32,
    used: HashMap<String, RuntimeEntry>,
    next_region_id: u32,
    next_frame_id: u32,
    allow_rotation: bool,
    allocator: RuntimeAllocator,
}

enum RuntimeAllocator {
    Guillotine(RuntimeGuillotine),
    Shelf(RuntimeShelfPlacement),
    Skyline(RuntimeSkyline),
}

impl RuntimeAllocator {
    fn free_area_and_rects(&self) -> (u64, usize) {
        match self {
            Self::Guillotine(strategy) => strategy.free_area_and_rects(),
            Self::Shelf(strategy) => strategy.free_area_and_rects(),
            Self::Skyline(strategy) => strategy.free_area_and_rects(),
        }
    }

    fn choose(&self, allow_rotation: bool, w: u32, h: u32) -> Option<(Rect, bool)> {
        match self {
            Self::Guillotine(strategy) => strategy.choose(allow_rotation, w, h),
            Self::Shelf(strategy) => strategy.choose(allow_rotation, w, h),
            Self::Skyline(strategy) => strategy.choose(allow_rotation, w, h),
        }
    }

    fn place(&mut self, slot: &Rect) {
        match self {
            Self::Guillotine(strategy) => strategy.place(slot),
            Self::Shelf(strategy) => strategy.place(slot),
            Self::Skyline(strategy) => strategy.place(slot),
        }
    }

    fn add_free(&mut self, rect: Rect) {
        match self {
            Self::Guillotine(strategy) => strategy.add_free(rect),
            Self::Shelf(strategy) => strategy.add_free(rect),
            Self::Skyline(strategy) => strategy.add_free(rect),
        }
    }
}

impl RuntimePage {
    pub(crate) fn new(
        id: PageId,
        width: u32,
        height: u32,
        page_config: &PageConfig,
        strategy: &RuntimeStrategy,
    ) -> Self {
        let usable = if page_config.max_dimensions() == (width, height) {
            usable_area(page_config)
        } else {
            let padding = page_config.border_padding();
            Rect::new(
                padding,
                padding,
                width.saturating_sub(padding.saturating_mul(2)),
                height.saturating_sub(padding.saturating_mul(2)),
            )
        };
        let allocator = match strategy {
            RuntimeStrategy::Guillotine { choice, split } => {
                RuntimeAllocator::Guillotine(RuntimeGuillotine::new(usable, *choice, *split))
            }
            RuntimeStrategy::Shelf { policy } => {
                RuntimeAllocator::Shelf(RuntimeShelfPlacement::new(usable, *policy))
            }
            RuntimeStrategy::Skyline { heuristic } => {
                RuntimeAllocator::Skyline(RuntimeSkyline::new(usable, *heuristic))
            }
        };
        Self {
            id,
            width,
            height,
            used: HashMap::new(),
            next_region_id: 0,
            next_frame_id: 0,
            allow_rotation: page_config.allow_rotation(),
            allocator,
        }
    }

    pub(crate) const fn id(&self) -> PageId {
        self.id
    }

    pub(crate) const fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub(crate) fn choose(&self, w: u32, h: u32) -> Option<(Rect, bool)> {
        self.allocator.choose(self.allow_rotation, w, h)
    }

    pub(crate) fn place(
        &mut self,
        key: String,
        physical: PhysicalPlacement,
        source: Rect,
    ) -> Result<RuntimePlacement> {
        let next_region_id = self
            .next_region_id
            .checked_add(1)
            .ok_or_else(|| identity_exhausted(self.id, "region", self.next_region_id))?;
        let next_frame_id = self
            .next_frame_id
            .checked_add(1)
            .ok_or_else(|| identity_exhausted(self.id, "frame", self.next_frame_id))?;

        let region = Region::new(
            RegionId::new(self.next_region_id),
            physical.content,
            physical.allocation,
            physical.rotated,
        );
        let frame = Frame::new(
            FrameId::new(self.next_frame_id),
            key.clone(),
            region.id(),
            false,
            source,
            (source.w, source.h),
        );

        self.allocator.place(&physical.allocation);
        self.next_region_id = next_region_id;
        self.next_frame_id = next_frame_id;
        self.used.insert(
            key,
            RuntimeEntry {
                region: region.clone(),
                frame: frame.clone(),
            },
        );

        Ok(RuntimePlacement::new(self.id, frame, region))
    }

    pub(crate) fn evict(&mut self, key: &str) -> Option<Rect> {
        let entry = self.used.remove(key)?;
        let allocation = entry.region.allocation();
        self.allocator.add_free(allocation);
        Some(allocation)
    }

    pub(crate) fn placement(&self, key: &str) -> Option<RuntimePlacement> {
        let entry = self.used.get(key)?;
        Some(RuntimePlacement::new(
            self.id,
            entry.frame.clone(),
            entry.region.clone(),
        ))
    }

    pub(crate) fn contains(&self, key: &str) -> bool {
        self.used.contains_key(key)
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = &str> {
        self.used.keys().map(String::as_str)
    }

    pub(crate) fn len(&self) -> usize {
        self.used.len()
    }

    pub(crate) fn content_area(&self) -> u128 {
        self.used
            .values()
            .map(|entry| entry.region.content().area())
            .sum()
    }

    pub(crate) fn allocation_area(&self) -> u128 {
        self.used
            .values()
            .map(|entry| entry.region.allocation().area())
            .sum()
    }

    pub(crate) fn rotated_regions(&self) -> usize {
        self.used
            .values()
            .filter(|entry| entry.region.rotated())
            .count()
    }

    pub(crate) fn free_area_and_rects(&self) -> (u64, usize) {
        self.allocator.free_area_and_rects()
    }

    pub(crate) fn snapshot(&self) -> Result<Page> {
        let mut regions: Vec<_> = self
            .used
            .values()
            .map(|entry| entry.region.clone())
            .collect();
        let mut frames: Vec<_> = self
            .used
            .values()
            .map(|entry| entry.frame.clone())
            .collect();
        regions.sort_unstable_by_key(Region::id);
        frames.sort_unstable_by_key(Frame::id);
        Page::try_new(self.id, self.width, self.height, regions, frames)
    }
}

fn identity_exhausted(page_id: PageId, kind: &str, value: u32) -> TexPackerError {
    TexPackerError::InvariantViolation {
        context: format!("runtime page {page_id}"),
        reason: format!("{kind} identity space exhausted at {value}"),
    }
}
