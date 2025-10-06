use super::Packer;
use crate::config::{GuillotineChoice, GuillotineSplit, PackerConfig};
use crate::model::{Frame, Rect};

pub struct GuillotinePacker {
    config: PackerConfig,
    free: Vec<Rect>,
    used: Vec<Rect>,
    choice: GuillotineChoice,
    split: GuillotineSplit,
}

impl GuillotinePacker {
    pub fn new(config: PackerConfig, choice: GuillotineChoice, split: GuillotineSplit) -> Self {
        let pad = config.border_padding;
        let w = config.max_width.saturating_sub(pad.saturating_mul(2));
        let h = config.max_height.saturating_sub(pad.saturating_mul(2));
        let border = Rect::new(pad, pad, w, h);
        Self {
            config,
            free: vec![border],
            used: Vec::new(),
            choice,
            split,
        }
    }

    fn score(choice: &GuillotineChoice, fr: &Rect, w: u32, h: u32) -> i32 {
        let area_fit = (fr.w * fr.h) as i32 - (w * h) as i32;
        let leftover_h = fr.w as i32 - w as i32;
        let leftover_v = fr.h as i32 - h as i32;
        let short_fit = leftover_h.abs().min(leftover_v.abs());
        let long_fit = leftover_h.abs().max(leftover_v.abs());
        match choice {
            GuillotineChoice::BestAreaFit => area_fit,
            GuillotineChoice::BestShortSideFit => short_fit,
            GuillotineChoice::BestLongSideFit => long_fit,
            GuillotineChoice::WorstAreaFit => -area_fit,
            GuillotineChoice::WorstShortSideFit => -short_fit,
            GuillotineChoice::WorstLongSideFit => -long_fit,
        }
    }

    fn choose(&self, w: u32, h: u32) -> Option<(usize, Rect, bool)> {
        let mut best_idx = None;
        let mut best_score = i32::MAX;
        let mut best_rect = Rect::new(0, 0, 0, 0);
        let mut best_rot = false;
        for (i, fr) in self.free.iter().enumerate() {
            if fr.w >= w && fr.h >= h {
                let s = Self::score(&self.choice, fr, w, h);
                if s < best_score {
                    best_score = s;
                    best_idx = Some(i);
                    best_rect = Rect::new(fr.x, fr.y, w, h);
                    best_rot = false;
                }
            }
            if self.config.allow_rotation && fr.w >= h && fr.h >= w {
                let s = Self::score(&self.choice, fr, h, w);
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

    fn split(&self, fr: &Rect, placed: &Rect) -> (Option<Rect>, Option<Rect>) {
        // Compute leftover widths/heights (right/bottom), as in Jylänki's SplitFreeRectAlongAxis.
        let w_right = (fr.x + fr.w).saturating_sub(placed.x + placed.w);
        let h_bottom = (fr.y + fr.h).saturating_sub(placed.y + placed.h);

        // Choose split axis based on heuristic comparing leftover along right vs bottom.
        let split_horizontal = match self.split {
            GuillotineSplit::SplitShorterLeftoverAxis => h_bottom < w_right,
            GuillotineSplit::SplitLongerLeftoverAxis => h_bottom > w_right,
            GuillotineSplit::SplitMinimizeArea => (w_right * fr.h) <= (fr.w * h_bottom),
            GuillotineSplit::SplitMaximizeArea => (w_right * fr.h) >= (fr.w * h_bottom),
            GuillotineSplit::SplitShorterAxis => fr.h < fr.w,
            GuillotineSplit::SplitLongerAxis => fr.h > fr.w,
        };

        // Form the two new rectangles: bottom and right. Dimensions depend on split axis.
        let mut bottom = Rect::new(fr.x, placed.y + placed.h, 0, fr.h.saturating_sub(placed.h));
        let mut right = Rect::new(placed.x + placed.w, fr.y, fr.w.saturating_sub(placed.w), 0);
        if split_horizontal {
            bottom.w = fr.w;
            right.h = placed.h;
        } else {
            bottom.w = placed.w;
            right.h = fr.h;
        }
        let r1 = if bottom.w > 0 && bottom.h > 0 {
            Some(bottom)
        } else {
            None
        };
        let r2 = if right.w > 0 && right.h > 0 {
            Some(right)
        } else {
            None
        };
        (r1, r2)
    }

    fn place(&mut self, idx: usize, placed: &Rect) {
        let fr = self.free[idx];
        self.free.swap_remove(idx);
        let (a, b) = self.split(&fr, placed);
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
        let mut i = 0;
        while i < self.free.len() {
            let mut j = i + 1;
            let a = self.free[i];
            let a_x2 = a.x + a.w;
            let a_y2 = a.y + a.h;
            let mut remove_i = false;
            while j < self.free.len() {
                let b = self.free[j];
                let b_x2 = b.x + b.w;
                let b_y2 = b.y + b.h;
                if a.x >= b.x && a.y >= b.y && a_x2 <= b_x2 && a_y2 <= b_y2 {
                    remove_i = true;
                    break;
                }
                if b.x >= a.x && b.y >= a.y && b_x2 <= a_x2 && b_y2 <= a_y2 {
                    self.free.remove(j);
                    continue;
                }
                j += 1;
            }
            if remove_i {
                self.free.remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn merge_free_list(&mut self) {
        let mut merged = true;
        while merged {
            merged = false;
            'outer: for i in 0..self.free.len() {
                for j in i + 1..self.free.len() {
                    let a = self.free[i];
                    let b = self.free[j];
                    // horizontal merge (same y, height, contiguous in x)
                    if a.y == b.y && a.h == b.h {
                        if a.x + a.w == b.x {
                            self.free[i] = Rect::new(a.x, a.y, a.w + b.w, a.h);
                            self.free.remove(j);
                            merged = true;
                            break 'outer;
                        } else if b.x + b.w == a.x {
                            self.free[i] = Rect::new(b.x, a.y, a.w + b.w, a.h);
                            self.free.remove(j);
                            merged = true;
                            break 'outer;
                        }
                    }
                    // vertical merge (same x, width, contiguous in y)
                    if a.x == b.x && a.w == b.w {
                        if a.y + a.h == b.y {
                            self.free[i] = Rect::new(a.x, a.y, a.w, a.h + b.h);
                            self.free.remove(j);
                            merged = true;
                            break 'outer;
                        } else if b.y + b.h == a.y {
                            self.free[i] = Rect::new(a.x, b.y, a.w, a.h + b.h);
                            self.free.remove(j);
                            merged = true;
                            break 'outer;
                        }
                    }
                }
            }
        }
    }
}

impl<K: Clone> Packer<K> for GuillotinePacker {
    fn can_pack(&self, rect: &Rect) -> bool {
        let w = rect.w + self.config.texture_padding + self.config.texture_extrusion * 2;
        let h = rect.h + self.config.texture_padding + self.config.texture_extrusion * 2;
        self.choose(w, h).is_some()
    }

    fn pack(&mut self, key: K, rect: &Rect) -> Option<Frame<K>> {
        let w = rect.w + self.config.texture_padding + self.config.texture_extrusion * 2;
        let h = rect.h + self.config.texture_padding + self.config.texture_extrusion * 2;
        if let Some((idx, place, rotated)) = self.choose(w, h) {
            self.place(idx, &place);
            let pad_half = self.config.texture_padding / 2;
            let off = self.config.texture_extrusion + pad_half;
            let frame_rect = Rect::new(
                place.x.saturating_add(off),
                place.y.saturating_add(off),
                rect.w,
                rect.h,
            );
            Some(Frame {
                key,
                frame: frame_rect,
                rotated,
                trimmed: false,
                source: *rect,
                source_size: (rect.w, rect.h),
            })
        } else {
            None
        }
    }
}
