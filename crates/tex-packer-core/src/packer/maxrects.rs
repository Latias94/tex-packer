use super::Packer;
use crate::config::{MaxRectsHeuristic, PackerConfig};
use crate::model::{Frame, Rect};

pub struct MaxRectsPacker {
    config: PackerConfig,
    border: Rect,
    free: Vec<Rect>,
    used: Vec<Rect>,
    heuristic: MaxRectsHeuristic,
}

impl MaxRectsPacker {
    pub fn new(config: PackerConfig, heuristic: MaxRectsHeuristic) -> Self {
        let pad = config.border_padding;
        let w = config.max_width.saturating_sub(pad.saturating_mul(2));
        let h = config.max_height.saturating_sub(pad.saturating_mul(2));
        let border = Rect::new(pad, pad, w, h);
        Self {
            config,
            border,
            free: vec![border],
            used: Vec::new(),
            heuristic,
        }
    }

    fn rect_right_ex(r: &Rect) -> u32 {
        r.x + r.w
    }
    fn rect_bottom_ex(r: &Rect) -> u32 {
        r.y + r.h
    }

    fn intersects(a: &Rect, b: &Rect) -> bool {
        !(a.x >= Self::rect_right_ex(b)
            || b.x >= Self::rect_right_ex(a)
            || a.y >= Self::rect_bottom_ex(b)
            || b.y >= Self::rect_bottom_ex(a))
    }

    fn place_rect(&mut self, node: &Rect) {
        if self.config.mr_reference {
            return self.place_rect_ref(node);
        }
        // split all free rectangles that intersect with node
        let mut new_free: Vec<Rect> = Vec::new();
        for fr in self.free.iter() {
            if !Self::intersects(fr, node) {
                new_free.push(*fr);
                continue;
            }
            let fr_x2 = fr.x + fr.w;
            let fr_y2 = fr.y + fr.h;
            let n_x2 = node.x + node.w;
            let n_y2 = node.y + node.h;

            let ix1 = fr.x.max(node.x);
            let iy1 = fr.y.max(node.y);
            let ix2 = fr_x2.min(n_x2);
            let iy2 = fr_y2.min(n_y2);

            // above
            if iy1 > fr.y {
                let h = iy1 - fr.y;
                new_free.push(Rect::new(fr.x, fr.y, fr.w, h));
            }
            // below
            if iy2 < fr_y2 {
                let h = fr_y2 - iy2;
                new_free.push(Rect::new(fr.x, iy2, fr.w, h));
            }
            // left
            if ix1 > fr.x {
                let w = ix1 - fr.x;
                let y = iy1;
                let h = iy2.saturating_sub(iy1);
                if h > 0 {
                    new_free.push(Rect::new(fr.x, y, w, h));
                }
            }
            // right
            if ix2 < fr_x2 {
                let w = fr_x2 - ix2;
                let x = ix2;
                let y = iy1;
                let h = iy2.saturating_sub(iy1);
                if h > 0 {
                    new_free.push(Rect::new(x, y, w, h));
                }
            }
        }

        self.free = new_free;
        self.prune_free_list();
        self.used.push(*node);
    }

    fn place_rect_ref(&mut self, node: &Rect) {
        let mut new_free: Vec<Rect> = Vec::new();
        let mut i = 0usize;
        while i < self.free.len() {
            let fr = self.free[i];
            if Self::intersects(&fr, node) {
                // remove this free rect; split into parts added to new_free
                self.free.swap_remove(i);
                self.split_free_node_ref(fr, node, &mut new_free);
            } else {
                i += 1;
            }
        }
        // Prune new_free against existing free; and remove dominated among new_free
        self.prune_new_vs_old(&mut new_free);
        self.prune_within(&mut new_free);
        // Merge new into free, then final prune pass
        self.free.extend(new_free);
        self.prune_free_list();
        self.used.push(*node);
    }

    fn split_free_node_ref(&self, fr: Rect, node: &Rect, out: &mut Vec<Rect>) {
        let fr_x2 = fr.x + fr.w;
        let fr_y2 = fr.y + fr.h;
        let n_x2 = node.x + node.w;
        let n_y2 = node.y + node.h;

        // Left
        if node.x > fr.x && node.x < fr_x2 {
            let w = node.x - fr.x;
            out.push(Rect::new(fr.x, fr.y, w, fr.h));
        }
        // Right
        if n_x2 < fr_x2 {
            let x = n_x2;
            let w = fr_x2 - n_x2;
            out.push(Rect::new(x, fr.y, w, fr.h));
        }
        // Top
        if node.y > fr.y && node.y < fr_y2 {
            let h = node.y - fr.y;
            out.push(Rect::new(fr.x, fr.y, fr.w, h));
        }
        // Bottom
        if n_y2 < fr_y2 {
            let y = n_y2;
            let h = fr_y2 - n_y2;
            out.push(Rect::new(fr.x, y, fr.w, h));
        }
        // filter zero areas handled by prune later
    }

    fn prune_new_vs_old(&mut self, new_free: &mut Vec<Rect>) {
        // Remove any new rect fully contained in any existing free rect
        new_free.retain(|nr| !self.free.iter().any(|of| of.contains(nr)) && nr.w > 0 && nr.h > 0);
        // Remove any existing free rect fully contained in any remaining new rect
        let mut i = 0;
        while i < self.free.len() {
            if new_free.iter().any(|nr| nr.contains(&self.free[i])) {
                self.free.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn prune_within(&self, v: &mut Vec<Rect>) {
        let mut i = 0;
        while i < v.len() {
            let a = v[i];
            let a_x2 = a.x + a.w;
            let a_y2 = a.y + a.h;
            let mut remove_i = false;
            let mut j = 0;
            while j < v.len() {
                if i == j {
                    j += 1;
                    continue;
                }
                let b = v[j];
                let b_x2 = b.x + b.w;
                let b_y2 = b.y + b.h;
                if a.x >= b.x && a.y >= b.y && a_x2 <= b_x2 && a_y2 <= b_y2 {
                    remove_i = true;
                    break;
                }
                j += 1;
            }
            if remove_i {
                v.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn prune_free_list(&mut self) {
        let mut i = 0;
        while i < self.free.len() {
            let mut j = i + 1;
            let a = self.free[i];
            let a_right = Self::rect_right_ex(&a);
            let a_bottom = Self::rect_bottom_ex(&a);
            let mut remove_i = false;
            while j < self.free.len() {
                let b = self.free[j];
                let b_right = Self::rect_right_ex(&b);
                let b_bottom = Self::rect_bottom_ex(&b);
                // if a inside b
                if a.x >= b.x && a.y >= b.y && a_right <= b_right && a_bottom <= b_bottom {
                    remove_i = true;
                    break;
                }
                // if b inside a
                if b.x >= a.x && b.y >= a.y && b_right <= a_right && b_bottom <= a_bottom {
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

    fn score(&self, fr: &Rect, w: u32, h: u32) -> (i32, i32) {
        let leftover_h = fr.w as i32 - w as i32;
        let leftover_v = fr.h as i32 - h as i32;
        let short_fit = leftover_h.abs().min(leftover_v.abs());
        let long_fit = leftover_h.abs().max(leftover_v.abs());
        let area_fit = (fr.w * fr.h) as i32 - (w * h) as i32;
        match self.heuristic {
            MaxRectsHeuristic::BestAreaFit => (area_fit, short_fit),
            MaxRectsHeuristic::BestShortSideFit => (short_fit, long_fit),
            MaxRectsHeuristic::BestLongSideFit => (long_fit, short_fit),
            MaxRectsHeuristic::BottomLeft => (fr.y as i32, fr.x as i32),
            MaxRectsHeuristic::ContactPoint => {
                // maximize contact score: use negative for minimization
                let contact = self.contact_point_score(fr.x, fr.y, w, h);
                (-(contact as i32), area_fit)
            }
        }
    }

    fn find_position(&self, w: u32, h: u32) -> Option<(Rect, bool)> {
        let mut best_score1 = i32::MAX;
        let mut best_score2 = i32::MAX;
        let mut best_rect = Rect::new(0, 0, 0, 0);
        let mut best_rot = false;
        let mut best_top = u32::MAX; // tie-break: prefer smaller top side (y + h)
        let mut best_left = u32::MAX; // then prefer smaller x

        for fr in &self.free {
            // normal
            if fr.w >= w && fr.h >= h {
                let (s1, s2) = self.score(fr, w, h);
                let top = fr.y.saturating_add(h);
                if s1 < best_score1
                    || (s1 == best_score1
                        && (s2 < best_score2
                            || (s2 == best_score2
                                && (top < best_top || (top == best_top && fr.x < best_left)))))
                {
                    best_score1 = s1;
                    best_score2 = s2;
                    best_top = top;
                    best_left = fr.x;
                    best_rect = Rect::new(fr.x, fr.y, w, h);
                    best_rot = false;
                }
                // perfect fit early-out
                if fr.w == w && fr.h == h {
                    return Some((Rect::new(fr.x, fr.y, w, h), false));
                }
            }
            // rotated
            if self.config.allow_rotation && fr.w >= h && fr.h >= w {
                let (s1, s2) = self.score(fr, h, w);
                let top = fr.y.saturating_add(w);
                if s1 < best_score1
                    || (s1 == best_score1
                        && (s2 < best_score2
                            || (s2 == best_score2
                                && (top < best_top || (top == best_top && fr.x < best_left)))))
                {
                    best_score1 = s1;
                    best_score2 = s2;
                    best_top = top;
                    best_left = fr.x;
                    best_rect = Rect::new(fr.x, fr.y, h, w);
                    best_rot = true;
                }
                // perfect fit early-out (rotated)
                if fr.w == h && fr.h == w {
                    return Some((Rect::new(fr.x, fr.y, h, w), true));
                }
            }
        }

        if best_rect.w == 0 || best_rect.h == 0 {
            None
        } else {
            Some((best_rect, best_rot))
        }
    }

    fn contact_point_score(&self, x: u32, y: u32, w: u32, h: u32) -> u32 {
        let node = Rect::new(x, y, w, h);
        let mut score = 0u32;
        // contact with borders
        let border_right = self.border.x + self.border.w;
        let border_bottom = self.border.y + self.border.h;
        if node.x == self.border.x {
            score += node.h;
        }
        if node.y == self.border.y {
            score += node.w;
        }
        if node.x + node.w == border_right {
            score += node.h;
        }
        if node.y + node.h == border_bottom {
            score += node.w;
        }

        // contact with used rectangles
        for u in &self.used {
            // vertical contact (left/right edges)
            if node.x == u.x + u.w || u.x == node.x + node.w {
                let overlap = overlap_1d(node.y, node.y + node.h, u.y, u.y + u.h);
                score += overlap;
            }
            // horizontal contact (top/bottom edges)
            if node.y == u.y + u.h || u.y == node.y + node.h {
                let overlap = overlap_1d(node.x, node.x + node.w, u.x, u.x + u.w);
                score += overlap;
            }
        }
        score
    }

    pub fn free_list_len(&self) -> usize {
        self.free.len()
    }
}

fn overlap_1d(a1: u32, a2: u32, b1: u32, b2: u32) -> u32 {
    let start = a1.max(b1);
    let end = a2.min(b2);
    end.saturating_sub(start)
}

impl<K: Clone> Packer<K> for MaxRectsPacker {
    fn can_pack(&self, rect: &Rect) -> bool {
        let w = rect.w + self.config.texture_padding + self.config.texture_extrusion * 2;
        let h = rect.h + self.config.texture_padding + self.config.texture_extrusion * 2;
        self.find_position(w, h).is_some()
    }

    fn pack(&mut self, key: K, rect: &Rect) -> Option<Frame<K>> {
        let w = rect.w + self.config.texture_padding + self.config.texture_extrusion * 2;
        let h = rect.h + self.config.texture_padding + self.config.texture_extrusion * 2;
        if let Some((place, rotated)) = self.find_position(w, h) {
            self.place_rect(&place);
            // Report atlas frame rectangle in stored orientation (post-rotation dimensions),
            // and offset content inside reserved slot by extrude + half padding (symmetric)
            let (fw, fh) = if rotated {
                (rect.h, rect.w)
            } else {
                (rect.w, rect.h)
            };
            let pad_half = self.config.texture_padding / 2;
            let off = self.config.texture_extrusion + pad_half;
            let frame = Rect::new(
                place.x.saturating_add(off),
                place.y.saturating_add(off),
                fw,
                fh,
            );
            Some(Frame {
                key,
                frame,
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
