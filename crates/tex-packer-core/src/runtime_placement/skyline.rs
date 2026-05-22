use crate::config::SkylineHeuristic;
use crate::geometry::{bottom_ex_u32, contains_rect, right_ex_u32, span_end_ex};
use crate::model::Rect;

pub(super) struct RuntimeSkyline {
    border: Rect,
    heuristic: SkylineHeuristic,
    skylines: Vec<RuntimeSkylineNode>,
}

#[derive(Clone, Copy, Debug)]
struct RuntimeSkylineNode {
    x: u32,
    y: u32,
    w: u32,
}

impl RuntimeSkyline {
    pub(super) fn new(border: Rect, heuristic: SkylineHeuristic) -> Self {
        Self {
            border,
            heuristic,
            skylines: vec![RuntimeSkylineNode {
                x: border.x,
                y: border.y,
                w: border.w,
            }],
        }
    }

    pub(super) fn free_area_and_rects(&self) -> (u64, usize) {
        let bottom_ex = bottom_ex_u32(&self.border);
        let area = self
            .skylines
            .iter()
            .map(|node| (node.w as u64) * (bottom_ex.saturating_sub(node.y) as u64))
            .sum();
        (area, self.skylines.len())
    }

    pub(super) fn choose(&self, allow_rotation: bool, w: u32, h: u32) -> Option<(Rect, bool)> {
        match self.heuristic {
            SkylineHeuristic::BottomLeft => self.find_bottom_left(allow_rotation, w, h),
            SkylineHeuristic::MinWaste => self.find_min_waste(allow_rotation, w, h),
        }
    }

    pub(super) fn place(&mut self, slot: &Rect) {
        let slot_right = right_ex_u32(slot);
        let Some(mut idx) = self
            .skylines
            .iter()
            .position(|node| span_end_ex(node.x, node.w) > slot.x)
        else {
            return;
        };

        if self.skylines[idx].x < slot.x {
            self.skylines[idx].w = slot.x - self.skylines[idx].x;
            idx += 1;
        }

        while idx < self.skylines.len() && self.skylines[idx].x < slot_right {
            let node_right = span_end_ex(self.skylines[idx].x, self.skylines[idx].w);
            if node_right <= slot_right {
                self.skylines.remove(idx);
            } else {
                let shrink = slot_right - self.skylines[idx].x;
                self.skylines[idx].x += shrink;
                self.skylines[idx].w -= shrink;
                break;
            }
        }

        self.skylines.insert(
            idx,
            RuntimeSkylineNode {
                x: slot.x,
                y: bottom_ex_u32(slot),
                w: slot.w,
            },
        );
        self.merge_nodes();
    }

    pub(super) fn add_free(&mut self, _rect: Rect) {
        // Skyline does not support optimized eviction reuse in the current runtime strategy.
    }

    fn find_bottom_left(&self, allow_rotation: bool, w: u32, h: u32) -> Option<(Rect, bool)> {
        let mut best_bottom = u32::MAX;
        let mut best_width = u32::MAX;
        let mut best_index: Option<usize> = None;
        let mut best_rect = Rect::new(0, 0, 0, 0);
        let mut best_rot = false;

        for idx in 0..self.skylines.len() {
            if let Some(rect) = self.can_put(idx, w, h) {
                if rect.bottom() < best_bottom
                    || (rect.bottom() == best_bottom && self.skylines[idx].w < best_width)
                {
                    best_bottom = rect.bottom();
                    best_width = self.skylines[idx].w;
                    best_index = Some(idx);
                    best_rect = rect;
                    best_rot = false;
                }
            }
            if allow_rotation {
                if let Some(rect) = self.can_put(idx, h, w) {
                    if rect.bottom() < best_bottom
                        || (rect.bottom() == best_bottom && self.skylines[idx].w < best_width)
                    {
                        best_bottom = rect.bottom();
                        best_width = self.skylines[idx].w;
                        best_index = Some(idx);
                        best_rect = rect;
                        best_rot = true;
                    }
                }
            }
        }

        best_index.map(|_| (best_rect, best_rot))
    }

    fn find_min_waste(&self, allow_rotation: bool, w: u32, h: u32) -> Option<(Rect, bool)> {
        let mut best_waste = u128::MAX;
        let mut best_bottom = u32::MAX;
        let mut best_index: Option<usize> = None;
        let mut best_rect = Rect::new(0, 0, 0, 0);
        let mut best_rot = false;

        for idx in 0..self.skylines.len() {
            if let Some(rect) = self.can_put(idx, w, h) {
                let waste = self.compute_waste(idx, &rect);
                if waste < best_waste || (waste == best_waste && rect.bottom() < best_bottom) {
                    best_waste = waste;
                    best_bottom = rect.bottom();
                    best_index = Some(idx);
                    best_rect = rect;
                    best_rot = false;
                }
            }
            if allow_rotation {
                if let Some(rect) = self.can_put(idx, h, w) {
                    let waste = self.compute_waste(idx, &rect);
                    if waste < best_waste || (waste == best_waste && rect.bottom() < best_bottom) {
                        best_waste = waste;
                        best_bottom = rect.bottom();
                        best_index = Some(idx);
                        best_rect = rect;
                        best_rot = true;
                    }
                }
            }
        }

        best_index.map(|_| (best_rect, best_rot))
    }

    fn can_put(&self, mut idx: usize, w: u32, h: u32) -> Option<Rect> {
        if idx >= self.skylines.len() {
            return None;
        }
        let mut rect = Rect::new(self.skylines[idx].x, 0, w, h);
        let mut width_left = rect.w;
        loop {
            rect.y = rect.y.max(self.skylines[idx].y);
            if !contains_rect(&self.border, &rect) {
                return None;
            }
            if self.skylines[idx].w >= width_left {
                return Some(rect);
            }
            width_left = width_left.saturating_sub(self.skylines[idx].w);
            idx += 1;
            if idx >= self.skylines.len() {
                return None;
            }
        }
    }

    fn compute_waste(&self, start_idx: usize, rect: &Rect) -> u128 {
        let mut waste = 0u128;
        let rect_right = right_ex_u32(rect);
        let mut idx = start_idx;
        while idx < self.skylines.len() && self.skylines[idx].x < rect_right {
            if rect.y > self.skylines[idx].y {
                let overlap_w = rect_right
                    .min(span_end_ex(self.skylines[idx].x, self.skylines[idx].w))
                    .saturating_sub(self.skylines[idx].x.max(rect.x));
                let overlap_h = rect.y.saturating_sub(self.skylines[idx].y);
                waste += overlap_w as u128 * overlap_h as u128;
            }
            idx += 1;
        }
        waste
    }

    fn merge_nodes(&mut self) {
        let mut idx = 0;
        while idx < self.skylines.len().saturating_sub(1) {
            if self.skylines[idx].y == self.skylines[idx + 1].y {
                self.skylines[idx].w += self.skylines[idx + 1].w;
                self.skylines.remove(idx + 1);
            } else {
                idx += 1;
            }
        }
    }
}
