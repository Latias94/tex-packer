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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RuntimePageMetrics {
    pub(crate) num_frames: usize,
    pub(crate) num_regions: usize,
    pub(crate) num_rotated_regions: usize,
    pub(crate) page_area: u128,
    pub(crate) content_area: u128,
    pub(crate) allocation_area: u128,
    pub(crate) allocator_free_area: u128,
    pub(crate) num_free_rects: usize,
}

pub(crate) struct PreparedPageAppend {
    allocation: Rect,
    placement: RuntimePlacement,
    next_region_id: u32,
    next_frame_id: u32,
}

impl PreparedPageAppend {
    pub(crate) const fn placement(&self) -> &RuntimePlacement {
        &self.placement
    }
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
        {
            return Err(TexPackerError::InvariantViolation {
                context: format!("runtime page {}", self.id),
                reason: format!("placement for key '{key}' violates page geometry invariants"),
            });
        }
        debug_assert!(
            !self
                .regions
                .values()
                .any(|region| region.allocation().intersects(&physical.allocation)),
            "runtime allocator must propose a non-overlapping allocation"
        );

        Ok(Some(PreparedPageAppend {
            allocation: physical.allocation,
            placement: RuntimePlacement::new(self.id, frame, region),
            next_region_id,
            next_frame_id,
        }))
    }

    pub(crate) fn commit_append(&mut self, prepared: PreparedPageAppend) -> RuntimePlacement {
        let PreparedPageAppend {
            allocation,
            placement,
            next_region_id,
            next_frame_id,
        } = prepared;
        self.allocator.place(&allocation);
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

    pub(crate) fn evict(&mut self, frame_id: FrameId) -> Option<Rect> {
        let region_id = self.frames.get(&frame_id)?.region_id();
        let allocation = self.regions.get(&region_id)?.allocation();
        self.frames.remove(&frame_id);
        self.regions.remove(&region_id);
        self.allocator.add_free(allocation);
        Some(allocation)
    }

    pub(crate) fn placement(&self, frame_id: FrameId) -> Option<RuntimePlacement> {
        let frame = self.frames.get(&frame_id)?;
        let region_id = frame.region_id();
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

    pub(crate) fn metrics(&self) -> RuntimePageMetrics {
        let (content_area, allocation_area, num_rotated_regions) = self.regions.values().fold(
            (0u128, 0u128, 0usize),
            |(content_area, allocation_area, rotated), region| {
                (
                    content_area + region.content().area(),
                    allocation_area + region.allocation().area(),
                    rotated + usize::from(region.rotated()),
                )
            },
        );
        let (allocator_free_area, num_free_rects) = self.allocator.free_area_and_rects();
        RuntimePageMetrics {
            num_frames: self.frames.len(),
            num_regions: self.regions.len(),
            num_rotated_regions,
            page_area: u128::from(self.width) * u128::from(self.height),
            content_area,
            allocation_area,
            allocator_free_area: u128::from(allocator_free_area),
            num_free_rects,
        }
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
