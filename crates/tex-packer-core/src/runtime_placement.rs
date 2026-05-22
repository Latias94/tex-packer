use crate::config::{GuillotineChoice, GuillotineSplit, PackerConfig, SkylineHeuristic};
use crate::free_space::{guillotine_score, guillotine_split, merge_adjacent, prune_contained};
use crate::geometry::{PackingContext, bottom_ex_u32, contains_rect, right_ex_u32, span_end_ex};
use crate::model::{Frame, Rect};
use crate::runtime::{RuntimeStrategy, ShelfPolicy};
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
    Guillotine {
        free: Vec<Rect>,
        choice: GuillotineChoice,
        split: GuillotineSplit,
    },
    Shelf {
        border: Rect,
        policy: ShelfPolicy,
        shelves: Vec<RuntimeShelf>,
        next_y: u32,
    },
    Skyline {
        border: Rect,
        heuristic: SkylineHeuristic,
        skylines: Vec<RuntimeSkylineNode>,
    },
}

#[derive(Clone, Debug)]
struct RuntimeShelf {
    y: u32,
    h: u32,
    segs: Vec<(u32, u32)>,
}

#[derive(Clone, Copy, Debug)]
struct RuntimeSkylineNode {
    x: u32,
    y: u32,
    w: u32,
}

impl RuntimePage {
    pub(crate) fn new(
        id: usize,
        width: u32,
        height: u32,
        cfg: &PackerConfig,
        strategy: &RuntimeStrategy,
    ) -> Self {
        let ctx = PackingContext::new(cfg);
        let usable = if ctx.max_dimensions() == (width, height) {
            ctx.usable_area()
        } else {
            let pad = ctx.border_padding();
            Rect::new(
                pad,
                pad,
                width.saturating_sub(pad.saturating_mul(2)),
                height.saturating_sub(pad.saturating_mul(2)),
            )
        };
        let placement = match strategy {
            RuntimeStrategy::Guillotine => RuntimePlacement::Guillotine {
                free: vec![usable],
                choice: cfg.g_choice.clone(),
                split: cfg.g_split.clone(),
            },
            RuntimeStrategy::Shelf(policy) => RuntimePlacement::Shelf {
                border: usable,
                policy: *policy,
                shelves: Vec::new(),
                next_y: usable.y,
            },
            RuntimeStrategy::Skyline(heuristic) => RuntimePlacement::Skyline {
                border: usable,
                heuristic: heuristic.clone(),
                skylines: vec![RuntimeSkylineNode {
                    x: usable.x,
                    y: usable.y,
                    w: usable.w,
                }],
            },
        };
        Self {
            id,
            width,
            height,
            used: HashMap::new(),
            allow_rotation: cfg.allow_rotation,
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
        match &self.placement {
            RuntimePlacement::Guillotine { free, .. } => {
                let area = free.iter().map(|r| (r.w as u64) * (r.h as u64)).sum();
                (area, free.len())
            }
            RuntimePlacement::Shelf { shelves, .. } => {
                let mut area = 0u64;
                let mut rects = 0usize;
                for shelf in shelves {
                    rects += shelf.segs.len();
                    for (_, w) in &shelf.segs {
                        area += (*w as u64) * (shelf.h as u64);
                    }
                }
                (area, rects)
            }
            RuntimePlacement::Skyline {
                border, skylines, ..
            } => {
                let bottom_ex = bottom_ex_u32(border);
                let area = skylines
                    .iter()
                    .map(|node| (node.w as u64) * (bottom_ex.saturating_sub(node.y) as u64))
                    .sum();
                (area, skylines.len())
            }
        }
    }

    pub(crate) fn choose(&self, w: u32, h: u32) -> Option<(Rect, bool)> {
        match &self.placement {
            RuntimePlacement::Guillotine { free, choice, .. } => {
                let mut best_idx = None;
                let mut best = Rect::new(0, 0, 0, 0);
                let mut best_s = i128::MAX;
                let mut best_s2 = i128::MAX;
                let mut best_rot = false;
                for (i, fr) in free.iter().enumerate() {
                    if fr.w >= w && fr.h >= h {
                        let (s1, s2) = score_choice(choice, fr, w, h);
                        if s1 < best_s || (s1 == best_s && s2 < best_s2) {
                            best_s = s1;
                            best_s2 = s2;
                            best_idx = Some(i);
                            best = Rect::new(fr.x, fr.y, w, h);
                            best_rot = false;
                        }
                    }
                    if self.allow_rotation && fr.w >= h && fr.h >= w {
                        let (s1, s2) = score_choice(choice, fr, h, w);
                        if s1 < best_s || (s1 == best_s && s2 < best_s2) {
                            best_s = s1;
                            best_s2 = s2;
                            best_idx = Some(i);
                            best = Rect::new(fr.x, fr.y, h, w);
                            best_rot = true;
                        }
                    }
                }
                best_idx.map(|_| (best, best_rot))
            }
            RuntimePlacement::Shelf {
                border,
                policy,
                shelves,
                next_y,
            } => choose_shelf(self.allow_rotation, border, *policy, shelves, *next_y, w, h),
            RuntimePlacement::Skyline {
                border,
                heuristic,
                skylines,
            } => choose_skyline(self.allow_rotation, border, heuristic, skylines, w, h),
        }
    }

    pub(crate) fn place(&mut self, key: &str, slot: &Rect, frame: &Frame<String>, rotated: bool) {
        match &mut self.placement {
            RuntimePlacement::Guillotine { free, split, .. } => {
                // remove chosen free and split
                let mut idx = None;
                for (i, fr) in free.iter().enumerate() {
                    if fr.x == slot.x && fr.y == slot.y && fr.w >= slot.w && fr.h >= slot.h {
                        idx = Some(i);
                        break;
                    }
                }
                if let Some(i) = idx {
                    // emulate original split on matched free[i]
                    let fr = free[i];
                    free.swap_remove(i);
                    let (a, b) = split_rect(split, &fr, slot);
                    if let Some(r) = a {
                        free.push(r);
                    }
                    if let Some(r) = b {
                        free.push(r);
                    }
                    prune_free_list(free);
                    merge_free_list(free);
                }
            }
            RuntimePlacement::Shelf {
                border,
                shelves,
                next_y,
                ..
            } => {
                // consume from shelf at slot.y, or create new shelf and consume
                if let Some(sh) = shelves.iter_mut().find(|s| s.y == slot.y && s.h >= slot.h) {
                    consume_from_shelf(sh, slot, border);
                } else {
                    let mut sh = RuntimeShelf {
                        y: slot.y,
                        h: slot.h,
                        segs: vec![(border.x, border.w)],
                    };
                    consume_from_shelf(&mut sh, slot, border);
                    shelves.push(sh);
                    *next_y = (*next_y).max(bottom_ex_u32(slot));
                }
            }
            RuntimePlacement::Skyline { skylines, .. } => {
                place_skyline(skylines, slot);
            }
        }
        self.used
            .insert(key.to_string(), (*slot, rotated, frame.clone()));
    }

    pub(crate) fn add_free(&mut self, r: Rect) {
        match &mut self.placement {
            RuntimePlacement::Guillotine { free, .. } => {
                free.push(r);
                prune_free_list(free);
                merge_free_list(free);
            }
            RuntimePlacement::Shelf { shelves, .. } => {
                if let Some(sh) = shelves.iter_mut().find(|s| s.y == r.y && s.h == r.h) {
                    sh.segs.push((r.x, r.w));
                    merge_shelf_segments(sh);
                } else {
                    shelves.push(RuntimeShelf {
                        y: r.y,
                        h: r.h,
                        segs: vec![(r.x, r.w)],
                    });
                }
            }
            RuntimePlacement::Skyline { .. } => {
                // Skyline doesn't support add_free (eviction not optimized)
            }
        }
    }

    // guillotine prune/split helpers moved to free functions below
}

fn score_choice(choice: &GuillotineChoice, fr: &Rect, w: u32, h: u32) -> (i128, i128) {
    guillotine_score(choice, fr, w, h)
}

fn split_rect(split: &GuillotineSplit, fr: &Rect, placed: &Rect) -> (Option<Rect>, Option<Rect>) {
    guillotine_split(split, fr, placed)
}

// ---------- helpers for page modes ----------

fn prune_free_list(free: &mut Vec<Rect>) {
    prune_contained(free);
}

fn merge_free_list(free: &mut Vec<Rect>) {
    merge_adjacent(free);
}

fn choose_shelf(
    allow_rot: bool,
    border: &Rect,
    policy: ShelfPolicy,
    shelves: &Vec<RuntimeShelf>,
    next_y: u32,
    w: u32,
    h: u32,
) -> Option<(Rect, bool)> {
    let try_in = |rw: u32, rh: u32| -> Option<Rect> {
        match policy {
            ShelfPolicy::FirstFit => {
                for sh in shelves {
                    if rh <= sh.h {
                        if let Some((sx, _sw)) = sh.segs.iter().find(|(sx, sw)| {
                            *sw >= rw && span_end_ex(*sx, rw) <= right_ex_u32(border)
                        }) {
                            return Some(Rect::new(*sx, sh.y, rw, rh));
                        }
                    }
                }
                None
            }
            ShelfPolicy::NextFit => {
                if let Some(sh) = shelves.last() {
                    if rh <= sh.h {
                        if let Some((sx, _sw)) = sh.segs.iter().find(|(sx, sw)| {
                            *sw >= rw && span_end_ex(*sx, rw) <= right_ex_u32(border)
                        }) {
                            return Some(Rect::new(*sx, sh.y, rw, rh));
                        }
                    }
                }
                None
            }
        }
    };
    if let Some(r) = try_in(w, h) {
        return Some((r, false));
    }
    if allow_rot {
        if let Some(r) = try_in(h, w) {
            return Some((r, true));
        }
    }
    let try_new = |rw: u32, rh: u32| -> Option<Rect> {
        if rw <= border.w && span_end_ex(next_y, rh) <= bottom_ex_u32(border) {
            Some(Rect::new(border.x, next_y, rw, rh))
        } else {
            None
        }
    };
    if let Some(r) = try_new(w, h) {
        return Some((r, false));
    }
    if allow_rot {
        if let Some(r) = try_new(h, w) {
            return Some((r, true));
        }
    }
    None
}

fn consume_from_shelf(sh: &mut RuntimeShelf, slot: &Rect, border: &Rect) {
    let mut i = 0;
    while i < sh.segs.len() {
        let (sx, sw) = sh.segs[i];
        if slot.x >= sx && right_ex_u32(slot) <= span_end_ex(sx, sw) {
            sh.segs.remove(i);
            let left_w = slot.x.saturating_sub(sx);
            let right_x = right_ex_u32(slot);
            let right_w = span_end_ex(sx, sw).saturating_sub(right_x);
            if left_w > 0 {
                sh.segs.push((sx, left_w));
            }
            if right_w > 0 {
                sh.segs.push((right_x, right_w));
            }
            break;
        } else {
            i += 1;
        }
    }
    merge_shelf_segments(sh);
    sh.segs
        .retain(|(x, w)| *w > 0 && *x >= border.x && span_end_ex(*x, *w) <= right_ex_u32(border));
}

fn merge_shelf_segments(sh: &mut RuntimeShelf) {
    sh.segs.sort_by_key(|(x, _)| *x);
    let mut out: Vec<(u32, u32)> = Vec::new();
    for (x, w) in sh.segs.drain(..) {
        if let Some((lx, lw)) = out.last_mut() {
            if span_end_ex(*lx, *lw) == x {
                *lw += w;
                continue;
            }
        }
        out.push((x, w));
    }
    sh.segs = out;
}

// Skyline helper functions
fn choose_skyline(
    allow_rotation: bool,
    border: &Rect,
    heuristic: &SkylineHeuristic,
    skylines: &[RuntimeSkylineNode],
    w: u32,
    h: u32,
) -> Option<(Rect, bool)> {
    match heuristic {
        SkylineHeuristic::BottomLeft => {
            find_skyline_bottom_left(allow_rotation, border, skylines, w, h)
        }
        SkylineHeuristic::MinWaste => {
            find_skyline_min_waste(allow_rotation, border, skylines, w, h)
        }
    }
}

fn can_put_skyline(
    skylines: &[RuntimeSkylineNode],
    border: &Rect,
    mut i: usize,
    w: u32,
    h: u32,
) -> Option<Rect> {
    if i >= skylines.len() {
        return None;
    }
    let mut rect = Rect::new(skylines[i].x, 0, w, h);
    let mut width_left = rect.w;
    loop {
        rect.y = rect.y.max(skylines[i].y);
        if !contains_rect(border, &rect) {
            return None;
        }
        if skylines[i].w >= width_left {
            return Some(rect);
        }
        width_left = width_left.saturating_sub(skylines[i].w);
        i += 1;
        if i >= skylines.len() {
            return None;
        }
    }
}

fn find_skyline_bottom_left(
    allow_rotation: bool,
    border: &Rect,
    skylines: &[RuntimeSkylineNode],
    w: u32,
    h: u32,
) -> Option<(Rect, bool)> {
    let mut best_bottom = u32::MAX;
    let mut best_width = u32::MAX;
    let mut best_index: Option<usize> = None;
    let mut best_rect = Rect::new(0, 0, 0, 0);
    let mut best_rot = false;

    for i in 0..skylines.len() {
        if let Some(r) = can_put_skyline(skylines, border, i, w, h) {
            if r.bottom() < best_bottom || (r.bottom() == best_bottom && skylines[i].w < best_width)
            {
                best_bottom = r.bottom();
                best_width = skylines[i].w;
                best_index = Some(i);
                best_rect = r;
                best_rot = false;
            }
        }
        if allow_rotation {
            if let Some(r) = can_put_skyline(skylines, border, i, h, w) {
                if r.bottom() < best_bottom
                    || (r.bottom() == best_bottom && skylines[i].w < best_width)
                {
                    best_bottom = r.bottom();
                    best_width = skylines[i].w;
                    best_index = Some(i);
                    best_rect = r;
                    best_rot = true;
                }
            }
        }
    }

    best_index.map(|_| (best_rect, best_rot))
}

fn find_skyline_min_waste(
    allow_rotation: bool,
    border: &Rect,
    skylines: &[RuntimeSkylineNode],
    w: u32,
    h: u32,
) -> Option<(Rect, bool)> {
    let mut best_waste = u128::MAX;
    let mut best_bottom = u32::MAX;
    let mut best_index: Option<usize> = None;
    let mut best_rect = Rect::new(0, 0, 0, 0);
    let mut best_rot = false;

    for i in 0..skylines.len() {
        if let Some(r) = can_put_skyline(skylines, border, i, w, h) {
            let waste = compute_waste(skylines, i, &r);
            if waste < best_waste || (waste == best_waste && r.bottom() < best_bottom) {
                best_waste = waste;
                best_bottom = r.bottom();
                best_index = Some(i);
                best_rect = r;
                best_rot = false;
            }
        }
        if allow_rotation {
            if let Some(r) = can_put_skyline(skylines, border, i, h, w) {
                let waste = compute_waste(skylines, i, &r);
                if waste < best_waste || (waste == best_waste && r.bottom() < best_bottom) {
                    best_waste = waste;
                    best_bottom = r.bottom();
                    best_index = Some(i);
                    best_rect = r;
                    best_rot = true;
                }
            }
        }
    }

    best_index.map(|_| (best_rect, best_rot))
}

fn compute_waste(skylines: &[RuntimeSkylineNode], start_idx: usize, rect: &Rect) -> u128 {
    let mut waste = 0u128;
    let rect_right = right_ex_u32(rect);
    let mut i = start_idx;
    while i < skylines.len() && skylines[i].x < rect_right {
        if rect.y > skylines[i].y {
            let overlap_w = rect_right
                .min(span_end_ex(skylines[i].x, skylines[i].w))
                .saturating_sub(skylines[i].x.max(rect.x));
            let overlap_h = rect.y.saturating_sub(skylines[i].y);
            waste += overlap_w as u128 * overlap_h as u128;
        }
        i += 1;
    }
    waste
}

fn place_skyline(skylines: &mut Vec<RuntimeSkylineNode>, slot: &Rect) {
    let slot_right = right_ex_u32(slot);
    let Some(mut idx) = skylines
        .iter()
        .position(|node| span_end_ex(node.x, node.w) > slot.x)
    else {
        return;
    };

    if skylines[idx].x < slot.x {
        skylines[idx].w = slot.x - skylines[idx].x;
        idx += 1;
    }

    while idx < skylines.len() && skylines[idx].x < slot_right {
        let node_right = span_end_ex(skylines[idx].x, skylines[idx].w);
        if node_right <= slot_right {
            skylines.remove(idx);
        } else {
            let shrink = slot_right - skylines[idx].x;
            skylines[idx].x += shrink;
            skylines[idx].w -= shrink;
            break;
        }
    }

    skylines.insert(
        idx,
        RuntimeSkylineNode {
            x: slot.x,
            y: bottom_ex_u32(slot),
            w: slot.w,
        },
    );
    merge_skyline_nodes(skylines);
}

fn merge_skyline_nodes(skylines: &mut Vec<RuntimeSkylineNode>) {
    let mut i = 0;
    while i < skylines.len().saturating_sub(1) {
        if skylines[i].y == skylines[i + 1].y {
            skylines[i].w += skylines[i + 1].w;
            skylines.remove(i + 1);
        } else {
            i += 1;
        }
    }
}
