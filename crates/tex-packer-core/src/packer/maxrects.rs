use crate::config::{MaxRectsHeuristic, PageConfig};
use crate::free_space::{prune_contained, subtract_intersections};
use crate::geometry::{
    ContentSize, PhysicalPlacement, PlacementGeometry, area_fit_score, bottom_ex_u32,
    contains_rect, intersects, overlap_1d, right_ex_u32, usable_area,
};
use crate::model::Rect;

pub(super) struct MaxRectsPacker {
    page_config: PageConfig,
    border: Rect,
    free: Vec<Rect>,
    used: Vec<Rect>,
    heuristic: MaxRectsHeuristic,
    reference: bool,
}

impl MaxRectsPacker {
    pub(super) fn new(
        page_config: PageConfig,
        heuristic: MaxRectsHeuristic,
        reference: bool,
    ) -> Self {
        let border = usable_area(&page_config);
        Self {
            page_config,
            border,
            free: vec![border],
            used: Vec::new(),
            heuristic,
            reference,
        }
    }

    fn place_rect(&mut self, node: &Rect) {
        if self.reference {
            return self.place_rect_ref(node);
        }
        self.free = subtract_intersections(self.free.iter().copied(), node);
        self.prune_free_list();
        self.used.push(*node);
    }

    fn place_rect_ref(&mut self, node: &Rect) {
        let mut new_free: Vec<Rect> = Vec::new();
        let mut i = 0usize;
        while i < self.free.len() {
            let fr = self.free[i];
            if intersects(&fr, node) {
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
        let fr_x2 = right_ex_u32(&fr);
        let fr_y2 = bottom_ex_u32(&fr);
        let n_x2 = right_ex_u32(node);
        let n_y2 = bottom_ex_u32(node);

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
        new_free
            .retain(|nr| !self.free.iter().any(|of| contains_rect(of, nr)) && nr.w > 0 && nr.h > 0);
        // Remove any existing free rect fully contained in any remaining new rect
        let mut i = 0;
        while i < self.free.len() {
            if new_free.iter().any(|nr| contains_rect(nr, &self.free[i])) {
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
            let a_x2 = right_ex_u32(&a);
            let a_y2 = bottom_ex_u32(&a);
            let mut remove_i = false;
            let mut j = 0;
            while j < v.len() {
                if i == j {
                    j += 1;
                    continue;
                }
                let b = v[j];
                let b_x2 = right_ex_u32(&b);
                let b_y2 = bottom_ex_u32(&b);
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
        prune_contained(&mut self.free);
    }

    fn score(&self, fr: &Rect, w: u32, h: u32) -> (i128, i128) {
        let leftover_h = fr.w as i128 - w as i128;
        let leftover_v = fr.h as i128 - h as i128;
        let short_fit = leftover_h.abs().min(leftover_v.abs());
        let long_fit = leftover_h.abs().max(leftover_v.abs());
        let area_fit = area_fit_score(fr, w, h);
        match self.heuristic {
            MaxRectsHeuristic::BestAreaFit => (area_fit, short_fit),
            MaxRectsHeuristic::BestShortSideFit => (short_fit, long_fit),
            MaxRectsHeuristic::BestLongSideFit => (long_fit, short_fit),
            MaxRectsHeuristic::BottomLeft => (fr.y as i128, fr.x as i128),
            MaxRectsHeuristic::ContactPoint => {
                // maximize contact score: use negative for minimization
                let contact = self.contact_point_score(fr.x, fr.y, w, h);
                (-(contact as i128), area_fit)
            }
        }
    }

    fn find_position(&self, w: u32, h: u32) -> Option<(Rect, bool)> {
        let mut best_score1 = i128::MAX;
        let mut best_score2 = i128::MAX;
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
            if self.page_config.allow_rotation() && fr.w >= h && fr.h >= w {
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

    fn contact_point_score(&self, x: u32, y: u32, w: u32, h: u32) -> u64 {
        let node = Rect::new(x, y, w, h);
        let mut score = 0u64;
        // contact with borders
        let border_right = right_ex_u32(&self.border);
        let border_bottom = bottom_ex_u32(&self.border);
        if node.x == self.border.x {
            score += node.h as u64;
        }
        if node.y == self.border.y {
            score += node.w as u64;
        }
        if right_ex_u32(&node) == border_right {
            score += node.h as u64;
        }
        if bottom_ex_u32(&node) == border_bottom {
            score += node.w as u64;
        }

        // contact with used rectangles
        for u in &self.used {
            // vertical contact (left/right edges)
            if node.x == right_ex_u32(u) || u.x == right_ex_u32(&node) {
                let overlap = overlap_1d(node.y, bottom_ex_u32(&node), u.y, bottom_ex_u32(u));
                score += overlap as u64;
            }
            // horizontal contact (top/bottom edges)
            if node.y == bottom_ex_u32(u) || u.y == bottom_ex_u32(&node) {
                let overlap = overlap_1d(node.x, right_ex_u32(&node), u.x, right_ex_u32(u));
                score += overlap as u64;
            }
        }
        score
    }

    pub(super) fn try_place(&mut self, content: ContentSize) -> Option<PhysicalPlacement> {
        let geometry = PlacementGeometry::new(content, &self.page_config)?;
        let (reserved_w, reserved_h) = geometry.reserved_size();
        let (allocation, rotated) = self.find_position(reserved_w, reserved_h)?;
        self.place_rect(&allocation);
        Some(geometry.complete(allocation, rotated))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::intersects;
    use rand::{RngExt, SeedableRng};

    fn page_config(width: u32, height: u32) -> PageConfig {
        PageConfig::builder()
            .max_dimensions(width, height)
            .allow_rotation(true)
            .texture_padding(0)
            .texture_extrusion(0)
            .build()
            .expect("valid test page")
    }

    #[test]
    fn rotates_when_only_rotated_allocation_fits() {
        let mut engine =
            MaxRectsPacker::new(page_config(16, 12), MaxRectsHeuristic::BestAreaFit, false);
        let placement = engine
            .try_place(ContentSize::new(8, 14))
            .expect("rotated allocation should fit");
        assert!(placement.rotated);
        assert_eq!((placement.content.w, placement.content.h), (14, 8));
        assert_eq!((placement.allocation.w, placement.allocation.h), (14, 8));
    }

    #[test]
    fn failed_search_does_not_change_free_or_used_state() {
        let mut engine =
            MaxRectsPacker::new(page_config(16, 12), MaxRectsHeuristic::BestAreaFit, false);
        let free_before = engine.free.clone();
        let used_before = engine.used.clone();
        assert!(engine.try_place(ContentSize::new(17, 13)).is_none());
        assert_eq!(engine.free, free_before);
        assert_eq!(engine.used, used_before);
    }

    #[test]
    fn successful_search_commits_exactly_one_used_allocation() {
        for reference in [false, true] {
            let mut engine = MaxRectsPacker::new(
                page_config(32, 32),
                MaxRectsHeuristic::BestAreaFit,
                reference,
            );
            let placement = engine
                .try_place(ContentSize::new(7, 5))
                .expect("allocation should fit");
            assert_eq!(engine.used, vec![placement.allocation]);
        }
    }

    #[test]
    fn placements_are_repeatable_and_disjoint() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let sizes = (0..120)
            .map(|_| ContentSize::new(rng.random_range(4..=64), rng.random_range(4..=64)))
            .collect::<Vec<_>>();

        for reference in [false, true] {
            let config = page_config(512, 512);
            let mut first =
                MaxRectsPacker::new(config.clone(), MaxRectsHeuristic::BestAreaFit, reference);
            let first_placements = sizes
                .iter()
                .copied()
                .map_while(|size| first.try_place(size))
                .collect::<Vec<_>>();

            let mut repeated =
                MaxRectsPacker::new(config, MaxRectsHeuristic::BestAreaFit, reference);
            let repeated_placements = sizes
                .iter()
                .copied()
                .map_while(|size| repeated.try_place(size))
                .collect::<Vec<_>>();

            assert_eq!(first_placements, repeated_placements);
            for (index, first) in first_placements.iter().enumerate() {
                for second in &first_placements[index + 1..] {
                    assert!(!intersects(&first.allocation, &second.allocation));
                }
            }
        }
    }
}
