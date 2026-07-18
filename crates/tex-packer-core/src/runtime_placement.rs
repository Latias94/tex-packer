mod guillotine;
mod shelf;
mod skyline;

use self::guillotine::RuntimeGuillotine;
use self::shelf::RuntimeShelfPlacement;
use self::skyline::RuntimeSkyline;
use crate::config::{PageConfig, RuntimeStrategy};
use crate::geometry::usable_area;
use crate::model::{Frame, Rect};
use std::collections::HashMap;

pub(crate) struct RuntimePage {
    pub(crate) id: usize,
    pub(crate) width: u32,
    pub(crate) height: u32,
    // Used map of reserved slots (expanded by padding/extrude)
    pub(crate) used: HashMap<String, (Rect, bool, Frame<String>)>, // (reserved_slot, rotated, frame)
    allow_rotation: bool,
    placement: RuntimePlacement,
}

enum RuntimePlacement {
    Guillotine(RuntimeGuillotine),
    Shelf(RuntimeShelfPlacement),
    Skyline(RuntimeSkyline),
}

impl RuntimePlacement {
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
        id: usize,
        width: u32,
        height: u32,
        page_config: &PageConfig,
        strategy: &RuntimeStrategy,
    ) -> Self {
        let usable = if page_config.max_dimensions() == (width, height) {
            usable_area(page_config)
        } else {
            let pad = page_config.border_padding();
            Rect::new(
                pad,
                pad,
                width.saturating_sub(pad.saturating_mul(2)),
                height.saturating_sub(pad.saturating_mul(2)),
            )
        };
        let placement = match strategy {
            RuntimeStrategy::Guillotine { choice, split } => {
                RuntimePlacement::Guillotine(RuntimeGuillotine::new(usable, *choice, *split))
            }
            RuntimeStrategy::Shelf { policy } => {
                RuntimePlacement::Shelf(RuntimeShelfPlacement::new(usable, *policy))
            }
            RuntimeStrategy::Skyline { heuristic } => {
                RuntimePlacement::Skyline(RuntimeSkyline::new(usable, *heuristic))
            }
        };
        Self {
            id,
            width,
            height,
            used: HashMap::new(),
            allow_rotation: page_config.allow_rotation(),
            placement,
        }
    }

    pub(crate) fn used_area(&self) -> u64 {
        self.used
            .values()
            .map(|(slot, _rot, _frame)| (slot.w as u64) * (slot.h as u64))
            .sum()
    }

    pub(crate) fn free_area_and_rects(&self) -> (u64, usize) {
        self.placement.free_area_and_rects()
    }

    pub(crate) fn choose(&self, w: u32, h: u32) -> Option<(Rect, bool)> {
        self.placement.choose(self.allow_rotation, w, h)
    }

    pub(crate) fn place(&mut self, key: &str, slot: &Rect, frame: &Frame<String>, rotated: bool) {
        self.placement.place(slot);
        self.used
            .insert(key.to_string(), (*slot, rotated, frame.clone()));
    }

    pub(crate) fn add_free(&mut self, rect: Rect) {
        self.placement.add_free(rect);
    }
}
