use super::Packer;
use crate::config::{GuillotineChoice, GuillotineSplit, PageConfig};
use crate::free_space::{guillotine_score, guillotine_split, merge_adjacent, prune_contained};
use crate::geometry::{PlacementGeometry, usable_area};
use crate::model::{Frame, Rect};

pub struct GuillotinePacker {
    page_config: PageConfig,
    free: Vec<Rect>,
    used: Vec<Rect>,
    choice: GuillotineChoice,
    split: GuillotineSplit,
}

impl GuillotinePacker {
    pub fn new(page_config: PageConfig, choice: GuillotineChoice, split: GuillotineSplit) -> Self {
        let border = usable_area(&page_config);
        Self {
            page_config,
            free: vec![border],
            used: Vec::new(),
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
        self.used.push(*placed);
    }

    fn prune_free_list(&mut self) {
        prune_contained(&mut self.free);
    }

    fn merge_free_list(&mut self) {
        merge_adjacent(&mut self.free);
    }
}

impl<K: Clone> Packer<K> for GuillotinePacker {
    fn can_pack(&self, rect: &Rect) -> bool {
        let Some(geometry) = PlacementGeometry::new(rect, &self.page_config) else {
            return false;
        };
        self.choose(geometry.reserved_w, geometry.reserved_h)
            .is_some()
    }

    fn pack(&mut self, key: K, rect: &Rect) -> Option<Frame<K>> {
        let geometry = PlacementGeometry::new(rect, &self.page_config)?;
        if let Some((idx, place, rotated)) = self.choose(geometry.reserved_w, geometry.reserved_h) {
            self.place(idx, &place);
            Some(geometry.frame(key, *rect, &place, rotated))
        } else {
            None
        }
    }
}
