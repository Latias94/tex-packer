use crate::config::{GuillotineChoice, GuillotineSplit, PackerConfig};
use crate::free_space::{guillotine_score, guillotine_split, merge_adjacent, prune_contained};
use crate::model::Rect;

pub(super) struct RuntimeGuillotine {
    free: Vec<Rect>,
    choice: GuillotineChoice,
    split: GuillotineSplit,
}

impl RuntimeGuillotine {
    pub(super) fn new(usable: Rect, cfg: &PackerConfig) -> Self {
        Self {
            free: vec![usable],
            choice: cfg.g_choice.clone(),
            split: cfg.g_split.clone(),
        }
    }

    pub(super) fn free_area_and_rects(&self) -> (u64, usize) {
        let area = self
            .free
            .iter()
            .map(|rect| (rect.w as u64) * (rect.h as u64))
            .sum();
        (area, self.free.len())
    }

    pub(super) fn choose(&self, allow_rotation: bool, w: u32, h: u32) -> Option<(Rect, bool)> {
        let mut best_idx = None;
        let mut best = Rect::new(0, 0, 0, 0);
        let mut best_s = i128::MAX;
        let mut best_s2 = i128::MAX;
        let mut best_rot = false;

        for (idx, free_rect) in self.free.iter().enumerate() {
            if free_rect.w >= w && free_rect.h >= h {
                let (score, secondary) = guillotine_score(&self.choice, free_rect, w, h);
                if score < best_s || (score == best_s && secondary < best_s2) {
                    best_s = score;
                    best_s2 = secondary;
                    best_idx = Some(idx);
                    best = Rect::new(free_rect.x, free_rect.y, w, h);
                    best_rot = false;
                }
            }

            if allow_rotation && free_rect.w >= h && free_rect.h >= w {
                let (score, secondary) = guillotine_score(&self.choice, free_rect, h, w);
                if score < best_s || (score == best_s && secondary < best_s2) {
                    best_s = score;
                    best_s2 = secondary;
                    best_idx = Some(idx);
                    best = Rect::new(free_rect.x, free_rect.y, h, w);
                    best_rot = true;
                }
            }
        }

        best_idx.map(|_| (best, best_rot))
    }

    pub(super) fn place(&mut self, slot: &Rect) {
        let Some(idx) = self.free.iter().position(|free| {
            free.x == slot.x && free.y == slot.y && free.w >= slot.w && free.h >= slot.h
        }) else {
            return;
        };

        let free_rect = self.free[idx];
        self.free.swap_remove(idx);
        let (a, b) = guillotine_split(&self.split, &free_rect, slot);
        if let Some(rect) = a {
            self.free.push(rect);
        }
        if let Some(rect) = b {
            self.free.push(rect);
        }
        self.prune_and_merge();
    }

    pub(super) fn add_free(&mut self, rect: Rect) {
        self.free.push(rect);
        self.prune_and_merge();
    }

    fn prune_and_merge(&mut self) {
        prune_contained(&mut self.free);
        merge_adjacent(&mut self.free);
    }
}
