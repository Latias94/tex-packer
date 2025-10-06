use super::Packer;
use crate::config::{GuillotineChoice, GuillotineSplit, PackerConfig, SkylineHeuristic};
use crate::model::{Frame, Rect};

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
        self.x + self.w.saturating_sub(1)
    }
}

pub struct SkylinePacker {
    config: PackerConfig,
    border: Rect,
    skylines: Vec<SkylineNode>,
    heuristic: SkylineHeuristic,
    waste: Option<WasteMap>,
}

impl SkylinePacker {
    pub fn new(config: PackerConfig) -> Self {
        let pad = config.border_padding;
        let w = config.max_width.saturating_sub(pad.saturating_mul(2));
        let h = config.max_height.saturating_sub(pad.saturating_mul(2));
        Self {
            config: config.clone(),
            border: Rect::new(pad, pad, w, h),
            skylines: vec![SkylineNode { x: pad, y: pad, w }],
            heuristic: config.skyline_heuristic.clone(),
            waste: if config.use_waste_map {
                Some(WasteMap::new(
                    Rect::new(pad, pad, w, h),
                    config.allow_rotation,
                    config.g_choice.clone(),
                    config.g_split.clone(),
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
            if !self.border.contains(&rect) {
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

    fn find_skyline(&self, w: u32, h: u32) -> Option<(usize, Rect)> {
        match self.heuristic {
            SkylineHeuristic::BottomLeft => self.find_bottom_left(w, h),
            SkylineHeuristic::MinWaste => self.find_min_waste(w, h),
        }
    }

    fn find_bottom_left(&self, w: u32, h: u32) -> Option<(usize, Rect)> {
        let mut best_bottom = u32::MAX;
        let mut best_width = u32::MAX;
        let mut best_index: Option<usize> = None;
        let mut best_rect = Rect::new(0, 0, 0, 0);

        for i in 0..self.skylines.len() {
            if let Some(r) = self.can_put(i, w, h) {
                if r.bottom() < best_bottom
                    || (r.bottom() == best_bottom && self.skylines[i].w < best_width)
                {
                    best_bottom = r.bottom();
                    best_width = self.skylines[i].w;
                    best_index = Some(i);
                    best_rect = r;
                }
            }
            if self.config.allow_rotation {
                if let Some(r) = self.can_put(i, h, w) {
                    if r.bottom() < best_bottom
                        || (r.bottom() == best_bottom && self.skylines[i].w < best_width)
                    {
                        best_bottom = r.bottom();
                        best_width = self.skylines[i].w;
                        best_index = Some(i);
                        best_rect = r;
                    }
                }
            }
        }
        best_index.map(|idx| (idx, best_rect))
    }

    fn wasted_area_for(&self, start: usize, r: &Rect) -> u32 {
        let mut area: u32 = 0;
        let mut width_left = r.w;
        let mut i = start;
        let base_y = r.y;
        while width_left > 0 && i < self.skylines.len() {
            let seg = &self.skylines[i];
            let use_w = width_left.min(seg.w);
            if seg.y > base_y {
                area = area.saturating_add((seg.y - base_y) * use_w);
            }
            width_left -= use_w;
            i += 1;
        }
        area
    }

    fn find_min_waste(&self, w: u32, h: u32) -> Option<(usize, Rect)> {
        let mut best_waste = u32::MAX;
        let mut best_bottom = u32::MAX;
        let mut best_index: Option<usize> = None;
        let mut best_rect = Rect::new(0, 0, 0, 0);
        for i in 0..self.skylines.len() {
            if let Some(r) = self.can_put(i, w, h) {
                let waste = self.wasted_area_for(i, &r);
                if waste < best_waste || (waste == best_waste && r.bottom() < best_bottom) {
                    best_waste = waste;
                    best_bottom = r.bottom();
                    best_index = Some(i);
                    best_rect = r;
                }
            }
            if self.config.allow_rotation {
                if let Some(r) = self.can_put(i, h, w) {
                    let waste = self.wasted_area_for(i, &r);
                    if waste < best_waste || (waste == best_waste && r.bottom() < best_bottom) {
                        best_waste = waste;
                        best_bottom = r.bottom();
                        best_index = Some(i);
                        best_rect = r;
                    }
                }
            }
        }
        best_index.map(|idx| (idx, best_rect))
    }

    fn split(&mut self, index: usize, rect: &Rect) {
        // Clamp the new skyline y to border.bottom() to avoid going past the page bottom when the
        // placed rectangle touches the bottom edge.
        let mut new_y = rect.bottom().saturating_add(1);
        if new_y > self.border.bottom() {
            new_y = self.border.bottom();
        }
        let skyline = SkylineNode {
            x: rect.x,
            y: new_y,
            w: rect.w,
        };
        // ensure within border
        debug_assert!(skyline.right() <= self.border.right());
        debug_assert!(skyline.y <= self.border.bottom());

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
        let mut i = 1;
        while i < self.skylines.len() {
            if self.skylines[i - 1].y == self.skylines[i].y {
                let w = self.skylines[i].w;
                self.skylines[i - 1].w = self.skylines[i - 1].w.saturating_add(w);
                self.skylines.remove(i);
            } else {
                i += 1;
            }
        }
    }
}

impl<K: Clone> Packer<K> for SkylinePacker {
    fn can_pack(&self, rect: &Rect) -> bool {
        let w = rect.w + self.config.texture_padding + self.config.texture_extrusion * 2;
        let h = rect.h + self.config.texture_padding + self.config.texture_extrusion * 2;
        if let Some(wm) = &self.waste {
            if wm.can_fit(w, h) {
                return true;
            }
        }
        self.find_skyline(w, h).is_some()
    }

    fn pack(&mut self, key: K, rect: &Rect) -> Option<Frame<K>> {
        let mut w = rect.w + self.config.texture_padding + self.config.texture_extrusion * 2;
        let mut h = rect.h + self.config.texture_padding + self.config.texture_extrusion * 2;

        // Try waste map first
        if let Some(wm) = &mut self.waste {
            if let Some((place, rotated)) = wm.try_pack(w, h) {
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
                return Some(Frame {
                    key,
                    frame,
                    rotated,
                    trimmed: false,
                    source: *rect,
                    source_size: (rect.w, rect.h),
                });
            }
        }

        if let Some((i, place)) = self.find_skyline(w, h) {
            self.split(i, &place);
            self.merge();
            self.add_waste_areas(i, &place);
            let rotated = w != place.w;

            // Compute content frame size (post-rotation)
            let (fw, fh) = if rotated {
                (rect.h, rect.w)
            } else {
                (rect.w, rect.h)
            };
            // Offset content inside the reserved slot by extrude + half padding (symmetric)
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
            // try rotated if not allowed above
            if !self.config.allow_rotation {
                std::mem::swap(&mut w, &mut h);
                if let Some((i, place)) = self.find_skyline(w, h) {
                    self.split(i, &place);
                    self.merge();
                    self.add_waste_areas(i, &place);
                    let rotated = true;
                    let (fw, fh) = (rect.h, rect.w);
                    let pad_half = self.config.texture_padding / 2;
                    let off = self.config.texture_extrusion + pad_half;
                    let frame = Rect::new(
                        place.x.saturating_add(off),
                        place.y.saturating_add(off),
                        fw,
                        fh,
                    );
                    return Some(Frame {
                        key,
                        frame,
                        rotated,
                        trimmed: false,
                        source: *rect,
                        source_size: (rect.w, rect.h),
                    });
                }
            }
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
        let rect_right = rect.x + rect.w; // exclusive-style right for internal calcs
        let mut i = index;
        while i < self.skylines.len() && self.skylines[i].x < rect_right {
            let seg = self.skylines[i];
            // If this segment lies completely left of rect_left, or starts at/after rect_right, no need to continue.
            if seg.x >= rect_right {
                break;
            }
            if seg.x + seg.w <= rect_left {
                break;
            }

            let left_side = seg.x.max(rect_left);
            let right_side = (seg.x + seg.w).min(rect_right);
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
    fn can_fit(&self, w: u32, h: u32) -> bool {
        self.choose(w, h).is_some()
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
        let mut best_s = i32::MAX;
        let mut best_s2 = i32::MAX;
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
            if !intersects(&fr, node) {
                new_free.push(fr);
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
            // left strip within overlap band
            if ix1 > fr.x {
                let w = ix1 - fr.x;
                let y = iy1;
                let h = iy2.saturating_sub(iy1);
                if h > 0 {
                    new_free.push(Rect::new(fr.x, y, w, h));
                }
            }
            // right strip within overlap band
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
        let mut i = 0;
        while i < self.free.len() {
            let a = self.free[i];
            let a_x2 = a.x + a.w;
            let a_y2 = a.y + a.h;
            let mut remove_i = false;
            let mut j = i + 1;
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
    fn merge(&mut self) {
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

fn score_choice(choice: &GuillotineChoice, fr: &Rect, w: u32, h: u32) -> (i32, i32) {
    let area_fit = (fr.w * fr.h) as i32 - (w * h) as i32;
    let leftover_h = fr.w as i32 - w as i32;
    let leftover_v = fr.h as i32 - h as i32;
    let short_fit = leftover_h.abs().min(leftover_v.abs());
    let long_fit = leftover_h.abs().max(leftover_v.abs());
    match choice {
        GuillotineChoice::BestAreaFit => (area_fit, short_fit),
        GuillotineChoice::BestShortSideFit => (short_fit, long_fit),
        GuillotineChoice::BestLongSideFit => (long_fit, short_fit),
        GuillotineChoice::WorstAreaFit => (-area_fit, -short_fit),
        GuillotineChoice::WorstShortSideFit => (-short_fit, -long_fit),
        GuillotineChoice::WorstLongSideFit => (-long_fit, -short_fit),
    }
}

// Note: split_decision was removed; current WasteMap uses subtractive splitting only.

#[inline]
fn intersects(a: &Rect, b: &Rect) -> bool {
    let ax2 = a.x + a.w;
    let ay2 = a.y + a.h;
    let bx2 = b.x + b.w;
    let by2 = b.y + b.h;
    !(a.x >= bx2 || b.x >= ax2 || a.y >= by2 || b.y >= ay2)
}
