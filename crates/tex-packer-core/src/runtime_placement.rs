mod guillotine;
mod shelf;
mod skyline;

use std::collections::HashMap;

use self::guillotine::RuntimeGuillotine;
use self::shelf::RuntimeShelfPlacement;
use self::skyline::RuntimeSkyline;
use crate::config::{PageConfig, RuntimeStrategy};
use crate::error::{Result, TexPackerError};
use crate::geometry::{PlacementGeometry, usable_area};
use crate::model::{Frame, FrameId, Page, PageId, Rect, Region, RegionId};
use crate::runtime::RuntimePlacement;

pub(crate) struct RuntimePage {
    id: PageId,
    width: u32,
    height: u32,
    regions: HashMap<RegionId, Region>,
    frames: HashMap<FrameId, Frame>,
    next_region_id: u32,
    next_frame_id: u32,
    allow_rotation: bool,
    allocator: RuntimeAllocator,
}

pub(crate) struct PreparedPageAppend {
    allocator: RuntimeAllocator,
    placement: RuntimePlacement,
    next_region_id: u32,
    next_frame_id: u32,
}

impl PreparedPageAppend {
    pub(crate) const fn placement(&self) -> &RuntimePlacement {
        &self.placement
    }
}

#[derive(Clone)]
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
            regions: HashMap::new(),
            frames: HashMap::new(),
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

    pub(crate) fn prepare_append(
        &self,
        key: &str,
        geometry: PlacementGeometry,
        source: Rect,
    ) -> Result<Option<PreparedPageAppend>> {
        let Some((allocation, rotated)) = self.allocator.choose(
            self.allow_rotation,
            geometry.reserved_w,
            geometry.reserved_h,
        ) else {
            return Ok(None);
        };
        let physical = geometry.complete(allocation, rotated);
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
            key.to_owned(),
            region.id(),
            false,
            source,
            (source.w, source.h),
        );

        let page_bounds = Rect::new(0, 0, self.width, self.height);
        if !physical.allocation.contains(&physical.content)
            || !page_bounds.contains(&physical.allocation)
            || self
                .regions
                .values()
                .any(|region| region.allocation().intersects(&physical.allocation))
        {
            return Err(TexPackerError::InvariantViolation {
                context: format!("runtime page {}", self.id),
                reason: format!("placement for key '{key}' violates page geometry invariants"),
            });
        }

        let mut allocator = self.allocator.clone();
        allocator.place(&physical.allocation);
        Ok(Some(PreparedPageAppend {
            allocator,
            placement: RuntimePlacement::new(self.id, frame, region),
            next_region_id,
            next_frame_id,
        }))
    }

    pub(crate) fn commit_append(&mut self, prepared: PreparedPageAppend) -> RuntimePlacement {
        let PreparedPageAppend {
            allocator,
            placement,
            next_region_id,
            next_frame_id,
        } = prepared;
        self.allocator = allocator;
        self.next_region_id = next_region_id;
        self.next_frame_id = next_frame_id;
        let replaced_region = self
            .regions
            .insert(placement.region_id(), placement.region().clone());
        let replaced_frame = self
            .frames
            .insert(placement.frame_id(), placement.frame().clone());
        debug_assert!(
            replaced_region.is_none() && replaced_frame.is_none(),
            "prepared runtime identities must remain unique"
        );
        placement
    }

    pub(crate) fn evict(&mut self, frame_id: FrameId, region_id: RegionId) -> Option<Rect> {
        let frame = self.frames.get(&frame_id)?;
        if frame.region_id() != region_id {
            return None;
        }
        let allocation = self.regions.get(&region_id)?.allocation();
        self.frames.remove(&frame_id);
        self.regions.remove(&region_id);
        self.allocator.add_free(allocation);
        Some(allocation)
    }

    pub(crate) fn placement(
        &self,
        frame_id: FrameId,
        region_id: RegionId,
    ) -> Option<RuntimePlacement> {
        let frame = self.frames.get(&frame_id)?;
        if frame.region_id() != region_id {
            return None;
        }
        let region = self.regions.get(&region_id)?;
        Some(RuntimePlacement::new(
            self.id,
            frame.clone(),
            region.clone(),
        ))
    }

    pub(crate) fn len(&self) -> usize {
        self.frames.len()
    }

    pub(crate) fn region_count(&self) -> usize {
        self.regions.len()
    }

    pub(crate) fn content_area(&self) -> u128 {
        self.regions
            .values()
            .map(|region| region.content().area())
            .sum()
    }

    pub(crate) fn allocation_area(&self) -> u128 {
        self.regions
            .values()
            .map(|region| region.allocation().area())
            .sum()
    }

    pub(crate) fn rotated_regions(&self) -> usize {
        self.regions
            .values()
            .filter(|region| region.rotated())
            .count()
    }

    pub(crate) fn free_area_and_rects(&self) -> (u64, usize) {
        self.allocator.free_area_and_rects()
    }

    pub(crate) fn snapshot(&self) -> Result<Page> {
        let mut regions: Vec<_> = self.regions.values().cloned().collect();
        let mut frames: Vec<_> = self.frames.values().cloned().collect();
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
