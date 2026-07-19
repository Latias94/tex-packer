use crate::config::{GuillotineChoice, GuillotineSplit, PageConfig};
use crate::free_space::{guillotine_score, guillotine_split, merge_adjacent, prune_contained};
use crate::geometry::{ContentSize, PhysicalPlacement, PlacementGeometry, usable_area};
use crate::model::Rect;

pub(super) struct GuillotinePacker {
    page_config: PageConfig,
    free: Vec<Rect>,
    choice: GuillotineChoice,
    split: GuillotineSplit,
}

impl GuillotinePacker {
    pub(super) fn new(
        page_config: PageConfig,
        choice: GuillotineChoice,
        split: GuillotineSplit,
    ) -> Self {
        let border = usable_area(&page_config);
        Self {
            page_config,
            free: vec![border],
            choice,
            split,
        }
    }

    fn choose(&self, w: u32, h: u32) -> Option<(usize, Rect, bool)> {
        let mut best_idx = None;
        let mut best_score = i128::MAX;
        let mut best_rect = Rect::new(0, 0, 0, 0);
        let mut best_rot = false;
        for (i, fr) in self.free.iter().enumerate() {
            if fr.w >= w && fr.h >= h {
                let s = guillotine_score(&self.choice, fr, w, h).0;
                if s < best_score {
                    best_score = s;
                    best_idx = Some(i);
                    best_rect = Rect::new(fr.x, fr.y, w, h);
                    best_rot = false;
                }
            }
            if self.page_config.allow_rotation() && fr.w >= h && fr.h >= w {
                let s = guillotine_score(&self.choice, fr, h, w).0;
                if s < best_score {
                    best_score = s;
                    best_idx = Some(i);
                    best_rect = Rect::new(fr.x, fr.y, h, w);
                    best_rot = true;
                }
            }
        }
        best_idx.map(|idx| (idx, best_rect, best_rot))
    }

    fn place(&mut self, idx: usize, placed: &Rect) {
        let fr = self.free[idx];
        self.free.swap_remove(idx);
        let (a, b) = guillotine_split(&self.split, &fr, placed);
        if let Some(r) = a {
            self.free.push(r);
        }
        if let Some(r) = b {
            self.free.push(r);
        }
        self.prune_free_list();
        self.merge_free_list();
    }

    fn prune_free_list(&mut self) {
        prune_contained(&mut self.free);
    }

    fn merge_free_list(&mut self) {
        merge_adjacent(&mut self.free);
    }

    pub(super) fn try_place(&mut self, content: ContentSize) -> Option<PhysicalPlacement> {
        let geometry = PlacementGeometry::new(content, &self.page_config)?;
        let (reserved_w, reserved_h) = geometry.reserved_size();
        let (index, allocation, rotated) = self.choose(reserved_w, reserved_h)?;
        self.place(index, &allocation);
        Some(geometry.complete(allocation, rotated))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::rect_area_u128;

    fn page_config(width: u32, height: u32) -> PageConfig {
        PageConfig::builder()
            .max_dimensions(width, height)
            .allow_rotation(true)
            .texture_padding(0)
            .texture_extrusion(0)
            .build()
            .expect("valid test page")
    }

    fn engine(width: u32, height: u32) -> GuillotinePacker {
        GuillotinePacker::new(
            page_config(width, height),
            GuillotineChoice::BestAreaFit,
            GuillotineSplit::SplitShorterLeftoverAxis,
        )
    }

    #[test]
    fn rotates_when_only_rotated_allocation_fits() {
        let mut engine = engine(16, 12);
        let placement = engine
            .try_place(ContentSize::new(8, 14))
            .expect("rotated allocation should fit");
        assert!(placement.rotated);
        assert_eq!((placement.content.w, placement.content.h), (14, 8));
        assert_eq!((placement.allocation.w, placement.allocation.h), (14, 8));
    }

    #[test]
    fn failed_search_does_not_change_free_state() {
        let mut engine = engine(16, 12);
        let before = engine.free.clone();
        assert!(engine.try_place(ContentSize::new(17, 13)).is_none());
        assert_eq!(engine.free, before);
    }

    #[test]
    fn successful_search_consumes_allocation_once() {
        let mut engine = engine(32, 32);
        let free_area_before = engine.free.iter().map(rect_area_u128).sum::<u128>();
        let placement = engine
            .try_place(ContentSize::new(7, 5))
            .expect("allocation should fit");
        let free_area_after = engine.free.iter().map(rect_area_u128).sum::<u128>();
        assert_eq!(
            free_area_before - free_area_after,
            rect_area_u128(&placement.allocation)
        );
    }
}
