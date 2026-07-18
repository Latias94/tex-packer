use crate::config::{GuillotineChoice, GuillotineSplit, PageConfig, SkylineHeuristic};
use crate::free_space::{
    guillotine_score, merge_adjacent, prune_contained, subtract_intersections,
};
use crate::geometry::{
    ContentSize, PhysicalPlacement, PlacementGeometry, contains_rect, right_ex_u32, span_end_ex,
    usable_area,
};
use crate::model::Rect;

#[derive(Clone, Copy, Debug)]
struct SkylineNode {
    x: u32,
    y: u32,
    w: u32,
}

impl SkylineNode {
    #[inline]
    fn left(&self) -> u32 {
        self.x
    }
    #[inline]
    fn right(&self) -> u32 {
        self.right_ex().saturating_sub(1)
    }
    #[inline]
    fn right_ex(&self) -> u32 {
        span_end_ex(self.x, self.w)
    }
}

pub(super) struct SkylinePacker {
    page_config: PageConfig,
    border: Rect,
    skylines: Vec<SkylineNode>,
    heuristic: SkylineHeuristic,
    waste: Option<WasteMap>,
}

impl SkylinePacker {
    pub(super) fn new(
        page_config: PageConfig,
        heuristic: SkylineHeuristic,
        use_waste_map: bool,
    ) -> Self {
        let usable = usable_area(&page_config);
        let allow_rotation = page_config.allow_rotation();
        Self {
            page_config,
            border: usable,
            skylines: vec![SkylineNode {
                x: usable.x,
                y: usable.y,
                w: usable.w,
            }],
            heuristic,
            waste: if use_waste_map {
                Some(WasteMap::new(
                    usable,
                    allow_rotation,
                    GuillotineChoice::default(),
                    GuillotineSplit::default(),
                ))
            } else {
                None
            },
        }
    }

    fn can_put(&self, mut i: usize, w: u32, h: u32) -> Option<Rect> {
        let mut rect = Rect::new(self.skylines[i].x, 0, w, h);
        let mut width_left = rect.w;
        loop {
            rect.y = rect.y.max(self.skylines[i].y);
            if !contains_rect(&self.border, &rect) {
                return None;
            }
            if self.skylines[i].w >= width_left {
                return Some(rect);
            }
            width_left -= self.skylines[i].w;
            i += 1;
            if i >= self.skylines.len() {
                return None;
            }
        }
    }

    fn find_skyline(&self, w: u32, h: u32) -> Option<(usize, Rect, bool)> {
        match self.heuristic {
            SkylineHeuristic::BottomLeft => self.find_bottom_left(w, h),
            SkylineHeuristic::MinWaste => self.find_min_waste(w, h),
        }
    }

    fn find_bottom_left(&self, w: u32, h: u32) -> Option<(usize, Rect, bool)> {
        let mut best_bottom = u32::MAX;
        let mut best_width = u32::MAX;
        let mut best_index: Option<usize> = None;
        let mut best_rect = Rect::new(0, 0, 0, 0);
        let mut best_rot = false;

        for i in 0..self.skylines.len() {
            if let Some(r) = self.can_put(i, w, h)
                && (r.bottom() < best_bottom
                    || (r.bottom() == best_bottom && self.skylines[i].w < best_width))
            {
                best_bottom = r.bottom();
                best_width = self.skylines[i].w;
                best_index = Some(i);
                best_rect = r;
                best_rot = false;
            }
            if self.page_config.allow_rotation()
                && let Some(r) = self.can_put(i, h, w)
                && (r.bottom() < best_bottom
                    || (r.bottom() == best_bottom && self.skylines[i].w < best_width))
            {
                best_bottom = r.bottom();
                best_width = self.skylines[i].w;
                best_index = Some(i);
                best_rect = r;
                best_rot = true;
            }
        }
        best_index.map(|idx| (idx, best_rect, best_rot))
    }

    fn wasted_area_for(&self, start: usize, r: &Rect) -> u128 {
        let mut area: u128 = 0;
        let mut width_left = r.w;
        let mut i = start;
        let base_y = r.y;
        while width_left > 0 && i < self.skylines.len() {
            let seg = &self.skylines[i];
            let use_w = width_left.min(seg.w);
            if base_y > seg.y {
                area = area.saturating_add((base_y - seg.y) as u128 * use_w as u128);
            }
            width_left -= use_w;
            i += 1;
        }
        area
    }

    fn find_min_waste(&self, w: u32, h: u32) -> Option<(usize, Rect, bool)> {
        let mut best_waste = u128::MAX;
        let mut best_bottom = u32::MAX;
        let mut best_index: Option<usize> = None;
        let mut best_rect = Rect::new(0, 0, 0, 0);
        let mut best_rot = false;
        for i in 0..self.skylines.len() {
            if let Some(r) = self.can_put(i, w, h) {
                let waste = self.wasted_area_for(i, &r);
                if waste < best_waste || (waste == best_waste && r.bottom() < best_bottom) {
                    best_waste = waste;
                    best_bottom = r.bottom();
                    best_index = Some(i);
                    best_rect = r;
                    best_rot = false;
                }
            }
            if self.page_config.allow_rotation()
                && let Some(r) = self.can_put(i, h, w)
            {
                let waste = self.wasted_area_for(i, &r);
                if waste < best_waste || (waste == best_waste && r.bottom() < best_bottom) {
                    best_waste = waste;
                    best_bottom = r.bottom();
                    best_index = Some(i);
                    best_rect = r;
                    best_rot = true;
                }
            }
        }
        best_index.map(|idx| (idx, best_rect, best_rot))
    }

    fn split(&mut self, index: usize, rect: &Rect) {
        let new_y = rect.y.saturating_add(rect.h);
        let skyline = SkylineNode {
            x: rect.x,
            y: new_y,
            w: rect.w,
        };
        // ensure within border
        debug_assert!(skyline.right() <= self.border.right());
        debug_assert!(skyline.y <= self.border.y.saturating_add(self.border.h));

        self.skylines.insert(index, skyline);

        let i = index + 1;
        while i < self.skylines.len() {
            if self.skylines[i - 1].left() <= self.skylines[i].left() {
                if self.skylines[i].left() <= self.skylines[i - 1].right() {
                    let shrink = self.skylines[i - 1].right() - self.skylines[i].left() + 1;
                    if self.skylines[i].w <= shrink {
                        self.skylines.remove(i);
                    } else {
                        self.skylines[i].x += shrink;
                        self.skylines[i].w -= shrink;
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn merge(&mut self) {
        // Correctness-first merge: merge only adjacent nodes with same y and contiguous x.
        if self.skylines.is_empty() {
            return;
        }
        let mut merged: Vec<SkylineNode> = Vec::with_capacity(self.skylines.len());
        for node in self.skylines.iter().copied() {
            if let Some(last) = merged.last_mut()
                && last.y == node.y
                && last.right_ex() == node.x
            {
                last.w = last.w.saturating_add(node.w);
                continue;
            }
            merged.push(node);
        }
        self.skylines = merged;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{PhysicalPlacement, intersects};
    use rand::{Rng, SeedableRng};

    fn page_config(width: u32, height: u32, allow_rotation: bool) -> PageConfig {
        PageConfig::builder()
            .max_dimensions(width, height)
            .allow_rotation(allow_rotation)
            .texture_padding(0)
            .texture_extrusion(0)
            .build()
            .expect("valid test page")
    }

    #[test]
    fn merge_does_not_bridge_gaps() {
        let mut p = SkylinePacker::new(PageConfig::default(), SkylineHeuristic::default(), false);
        p.skylines = vec![
            SkylineNode { x: 0, y: 10, w: 10 },
            SkylineNode { x: 12, y: 20, w: 2 },
            SkylineNode { x: 14, y: 10, w: 6 },
        ];
        p.merge();
        let nodes = p
            .skylines
            .iter()
            .map(|node| (node.x, node.y, node.w))
            .collect::<Vec<_>>();
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0], (0, 10, 10));
        assert_eq!(nodes[1], (12, 20, 2));
        assert_eq!(nodes[2], (14, 10, 6));
    }

    #[test]
    fn respects_disabled_rotation() {
        let mut engine = SkylinePacker::new(
            page_config(256, 256, false),
            SkylineHeuristic::BottomLeft,
            false,
        );
        let placement = engine
            .try_place(ContentSize::new(64, 128))
            .expect("content should fit without rotation");
        assert!(!placement.rotated);
        assert_eq!((placement.content.w, placement.content.h), (64, 128));
    }

    #[test]
    fn rotates_when_only_rotated_allocation_fits() {
        let mut engine = SkylinePacker::new(
            page_config(16, 12, true),
            SkylineHeuristic::BottomLeft,
            false,
        );
        let placement = engine
            .try_place(ContentSize::new(8, 14))
            .expect("rotated allocation should fit");
        assert!(placement.rotated);
        assert_eq!((placement.content.w, placement.content.h), (14, 8));
        assert_eq!((placement.allocation.w, placement.allocation.h), (14, 8));
    }

    #[test]
    fn rejects_span_across_a_full_height_segment() {
        let mut engine = SkylinePacker::new(
            page_config(10, 10, false),
            SkylineHeuristic::BottomLeft,
            false,
        );
        engine
            .try_place(ContentSize::new(5, 10))
            .expect("first allocation should fit");
        assert!(engine.try_place(ContentSize::new(6, 1)).is_none());
    }

    #[test]
    fn failed_search_does_not_change_skyline_or_waste_state() {
        let mut engine =
            SkylinePacker::new(page_config(16, 12, true), SkylineHeuristic::MinWaste, true);
        let skyline_before = engine
            .skylines
            .iter()
            .map(|node| (node.x, node.y, node.w))
            .collect::<Vec<_>>();
        let waste_before = engine.waste.as_ref().map(|waste| waste.free.clone());
        assert!(engine.try_place(ContentSize::new(17, 13)).is_none());
        let skyline_after = engine
            .skylines
            .iter()
            .map(|node| (node.x, node.y, node.w))
            .collect::<Vec<_>>();
        let waste_after = engine.waste.as_ref().map(|waste| waste.free.clone());
        assert_eq!(skyline_after, skyline_before);
        assert_eq!(waste_after, waste_before);
    }

    #[test]
    fn waste_map_is_repeatable_disjoint_and_not_worse_than_plain_skyline() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0xDEADBEEF);
        let sizes = (0..2000)
            .map(|_| ContentSize::new(rng.gen_range(4..=128), rng.gen_range(4..=128)))
            .collect::<Vec<_>>();
        let config = page_config(2048, 2048, true);

        let plain = pack_all(&config, &sizes, false);
        let waste = pack_all(&config, &sizes, true);
        let repeated = pack_all(&config, &sizes, true);

        assert_disjoint(&plain);
        assert_disjoint(&waste);
        assert_eq!(waste, repeated);
        assert!(content_area(&waste) >= content_area(&plain));
    }

    fn pack_all(
        config: &PageConfig,
        sizes: &[ContentSize],
        use_waste_map: bool,
    ) -> Vec<PhysicalPlacement> {
        let mut engine =
            SkylinePacker::new(config.clone(), SkylineHeuristic::MinWaste, use_waste_map);
        sizes
            .iter()
            .copied()
            .map_while(|size| engine.try_place(size))
            .collect()
    }

    fn assert_disjoint(placements: &[PhysicalPlacement]) {
        for (index, first) in placements.iter().enumerate() {
            for second in &placements[index + 1..] {
                assert!(!intersects(&first.allocation, &second.allocation));
            }
        }
    }

    fn content_area(placements: &[PhysicalPlacement]) -> u128 {
        placements
            .iter()
            .map(|placement| placement.content.w as u128 * placement.content.h as u128)
            .sum()
    }
}

impl SkylinePacker {
    pub(super) fn try_place(&mut self, content: ContentSize) -> Option<PhysicalPlacement> {
        let geometry = PlacementGeometry::new(content, &self.page_config)?;
        let (reserved_w, reserved_h) = geometry.reserved_size();

        // Try waste map first
        if let Some(wm) = &mut self.waste
            && let Some((allocation, rotated)) = wm.try_pack(reserved_w, reserved_h)
        {
            return Some(geometry.complete(allocation, rotated));
        }

        if let Some((i, allocation, rotated)) = self.find_skyline(reserved_w, reserved_h) {
            self.split(i, &allocation);
            self.merge();
            self.add_waste_areas(i, &allocation);

            Some(geometry.complete(allocation, rotated))
        } else {
            None
        }
    }
}

impl SkylinePacker {
    fn add_waste_areas(&mut self, index: usize, rect: &Rect) {
        if self.waste.is_none() {
            return;
        }
        // Align with Jylänki's SkylineBinPack::AddWasteMapArea behavior:
        // For each skyline segment overlapped by the placed rect width, add the
        // exact vertical gap between the segment.y and rect.y into waste-map.
        let wm = self.waste.as_mut().unwrap();
        let rect_left = rect.x;
        let rect_right = right_ex_u32(rect);
        let mut i = index;
        while i < self.skylines.len() && self.skylines[i].x < rect_right {
            let seg = self.skylines[i];
            // If this segment lies completely left of rect_left, or starts at/after rect_right, no need to continue.
            if seg.x >= rect_right {
                break;
            }
            if seg.right_ex() <= rect_left {
                break;
            }

            let left_side = seg.x.max(rect_left);
            let right_side = seg.right_ex().min(rect_right);
            if seg.y < rect.y {
                // Note: height is the vertical gap between segment top and rect top.
                let w = right_side.saturating_sub(left_side);
                let h = rect.y.saturating_sub(seg.y);
                if w > 0 && h > 0 {
                    wm.add_area(Rect::new(left_side, seg.y, w, h));
                }
            }
            i += 1;
        }
    }
}

// Minimal internal waste map structure
#[derive(Clone)]
struct WasteMap {
    free: Vec<Rect>,
    allow_rotation: bool,
    choice: GuillotineChoice,
}

impl WasteMap {
    fn new(
        _area: Rect,
        allow_rotation: bool,
        choice: GuillotineChoice,
        _split: GuillotineSplit,
    ) -> Self {
        // Start with an empty free list; Skyline will add waste areas after placements.
        Self {
            free: Vec::new(),
            allow_rotation,
            choice,
        }
    }
    fn try_pack(&mut self, w: u32, h: u32) -> Option<(Rect, bool)> {
        if let Some((idx, r, rot)) = self.choose(w, h) {
            self.place(idx, &r);
            Some((r, rot))
        } else {
            None
        }
    }
    fn choose(&self, w: u32, h: u32) -> Option<(usize, Rect, bool)> {
        let mut best_idx = None;
        let mut best_s = i128::MAX;
        let mut best_s2 = i128::MAX;
        let mut best = Rect::new(0, 0, 0, 0);
        let mut best_rot = false;
        for (i, fr) in self.free.iter().enumerate() {
            if fr.w >= w && fr.h >= h {
                let (s1, s2) = score_choice(&self.choice, fr, w, h);
                if s1 < best_s || (s1 == best_s && s2 < best_s2) {
                    best_s = s1;
                    best_s2 = s2;
                    best_idx = Some(i);
                    best = Rect::new(fr.x, fr.y, w, h);
                    best_rot = false;
                }
            }
            if self.allow_rotation && fr.w >= h && fr.h >= w {
                let (s1, s2) = score_choice(&self.choice, fr, h, w);
                if s1 < best_s || (s1 == best_s && s2 < best_s2) {
                    best_s = s1;
                    best_s2 = s2;
                    best_idx = Some(i);
                    best = Rect::new(fr.x, fr.y, h, w);
                    best_rot = true;
                }
            }
        }
        best_idx.map(|i| (i, best, best_rot))
    }
    fn place(&mut self, idx: usize, node: &Rect) {
        // Remove chosen free rectangle
        self.free.swap_remove(idx);

        // Subtract the placed node from all existing free rectangles to keep the list disjoint.
        let mut new_free: Vec<Rect> = Vec::with_capacity(self.free.len() + 2);
        for fr in self.free.drain(..) {
            new_free.extend(subtract_intersections([fr], node));
        }
        self.free = new_free;
        self.prune();
        self.merge();
    }
    fn add_area(&mut self, r: Rect) {
        self.push(r);
        self.prune();
        self.merge();
    }
    fn push(&mut self, r: Rect) {
        if r.w > 0 && r.h > 0 {
            self.free.push(r);
        }
    }
    fn prune(&mut self) {
        prune_contained(&mut self.free);
    }
    fn merge(&mut self) {
        merge_adjacent(&mut self.free);
    }
}

fn score_choice(choice: &GuillotineChoice, fr: &Rect, w: u32, h: u32) -> (i128, i128) {
    guillotine_score(choice, fr, w, h)
}

// Note: split_decision was removed; current WasteMap uses subtractive splitting only.
