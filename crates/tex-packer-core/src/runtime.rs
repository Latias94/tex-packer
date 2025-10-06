use crate::config::{GuillotineChoice, GuillotineSplit, PackerConfig};
use crate::error::{Result, TexPackerError};
use crate::model::{Atlas, Frame, Meta, Page, Rect};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub enum RuntimeStrategy {
    Guillotine,
    Shelf(ShelfPolicy),
}

#[derive(Debug, Clone, Copy)]
pub enum ShelfPolicy {
    NextFit,
    FirstFit,
}

pub struct AtlasSession {
    cfg: PackerConfig,
    _strategy: RuntimeStrategy,
    pages: Vec<RtPage>,
    next_id: usize,
}

struct RtPage {
    id: usize,
    width: u32,
    height: u32,
    // Used map of reserved slots (expanded by padding/extrude)
    used: HashMap<String, (Rect, bool, Frame<String>)>, // (reserved_slot, rotated, frame)
    allow_rotation: bool,
    mode: RtMode,
}

enum RtMode {
    Guillotine {
        free: Vec<Rect>,
        choice: GuillotineChoice,
        split: GuillotineSplit,
    },
    Shelf {
        border: Rect,
        policy: ShelfPolicy,
        shelves: Vec<Shelf>,
        next_y: u32,
    },
}

#[derive(Clone, Debug)]
struct Shelf {
    y: u32,
    h: u32,
    segs: Vec<(u32, u32)>,
}

impl AtlasSession {
    pub fn new(cfg: PackerConfig, strategy: RuntimeStrategy) -> Self {
        Self {
            cfg,
            _strategy: strategy,
            pages: Vec::new(),
            next_id: 0,
        }
    }

    fn new_page(&mut self) -> RtPage {
        let id = self.next_id;
        self.next_id += 1;
        let pad = self.cfg.border_padding;
        let w = self.cfg.max_width.saturating_sub(pad.saturating_mul(2));
        let h = self.cfg.max_height.saturating_sub(pad.saturating_mul(2));
        let mode = match self._strategy {
            RuntimeStrategy::Guillotine => RtMode::Guillotine {
                free: vec![Rect::new(pad, pad, w, h)],
                choice: self.cfg.g_choice.clone(),
                split: self.cfg.g_split.clone(),
            },
            RuntimeStrategy::Shelf(policy) => RtMode::Shelf {
                border: Rect::new(pad, pad, w, h),
                policy,
                shelves: Vec::new(),
                next_y: pad,
            },
        };
        RtPage {
            id,
            width: self.cfg.max_width,
            height: self.cfg.max_height,
            used: HashMap::new(),
            allow_rotation: self.cfg.allow_rotation,
            mode,
        }
    }

    pub fn append(&mut self, key: String, w: u32, h: u32) -> Result<(usize, Frame<String>)> {
        let reserve_w = w + self.cfg.texture_extrusion * 2 + self.cfg.texture_padding;
        let reserve_h = h + self.cfg.texture_extrusion * 2 + self.cfg.texture_padding;
        // Try existing pages
        for idx in 0..self.pages.len() {
            let (slot, rotated, id);
            {
                let p = &self.pages[idx];
                if let Some((s, r)) = p.choose(reserve_w, reserve_h) {
                    slot = s;
                    rotated = r;
                    id = p.id;
                } else {
                    continue;
                }
            }
            let frame = self.make_frame(&key, w, h, &slot, rotated);
            let p = &mut self.pages[idx];
            p.place(&key, &slot, &frame, rotated);
            return Ok((id, frame));
        }
        // Grow: add a new page and place
        let mut page = self.new_page();
        if let Some((slot, rotated)) = page.choose(reserve_w, reserve_h) {
            let frame = self.make_frame(&key, w, h, &slot, rotated);
            page.place(&key, &slot, &frame, rotated);
            let id = page.id;
            self.pages.push(page);
            return Ok((id, frame));
        }
        Err(TexPackerError::OutOfSpace)
    }

    pub fn evict(&mut self, page_id: usize, key: &str) -> bool {
        if let Some(p) = self.pages.iter_mut().find(|p| p.id == page_id) {
            if let Some((slot, _rot, _frame)) = p.used.remove(key) {
                p.add_free(slot);
                return true;
            }
        }
        false
    }

    pub fn snapshot_atlas(&self) -> Atlas<String> {
        let mut pages: Vec<Page<String>> = Vec::new();
        for p in &self.pages {
            let mut frames: Vec<Frame<String>> = Vec::new();
            for (_k, (_slot, _rot, f)) in p.used.iter() {
                frames.push(f.clone());
            }
            pages.push(Page {
                id: p.id,
                width: p.width,
                height: p.height,
                frames,
            });
        }
        let meta = Meta {
            schema_version: "1".into(),
            app: "tex-packer".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            format: "RGBA8888".into(),
            scale: 1.0,
            power_of_two: self.cfg.power_of_two,
            square: self.cfg.square,
            max_dim: (self.cfg.max_width, self.cfg.max_height),
            padding: (self.cfg.border_padding, self.cfg.texture_padding),
            extrude: self.cfg.texture_extrusion,
            allow_rotation: self.cfg.allow_rotation,
            trim_mode: if self.cfg.trim { "trim" } else { "none" }.into(),
            background_color: None,
        };
        Atlas { pages, meta }
    }

    fn make_frame(&self, key: &str, w: u32, h: u32, slot: &Rect, rotated: bool) -> Frame<String> {
        let pad_half = self.cfg.texture_padding / 2;
        let off = self.cfg.texture_extrusion + pad_half;
        let (fw, fh) = (w, h);
        let frame = Rect::new(slot.x + off, slot.y + off, fw, fh);
        let source = Rect::new(0, 0, w, h);
        Frame {
            key: key.to_string(),
            frame,
            rotated,
            trimmed: false,
            source,
            source_size: (w, h),
        }
    }
}

impl RtPage {
    fn choose(&self, w: u32, h: u32) -> Option<(Rect, bool)> {
        match &self.mode {
            RtMode::Guillotine { free, choice, .. } => {
                let mut best_idx = None;
                let mut best = Rect::new(0, 0, 0, 0);
                let mut best_s = i32::MAX;
                let mut best_s2 = i32::MAX;
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
            RtMode::Shelf {
                border,
                policy,
                shelves,
                next_y,
            } => choose_shelf(self.allow_rotation, border, *policy, shelves, *next_y, w, h),
        }
    }

    fn place(&mut self, key: &str, slot: &Rect, frame: &Frame<String>, rotated: bool) {
        match &mut self.mode {
            RtMode::Guillotine { free, split, .. } => {
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
            RtMode::Shelf {
                border,
                shelves,
                next_y,
                ..
            } => {
                // consume from shelf at slot.y, or create new shelf and consume
                if let Some(sh) = shelves.iter_mut().find(|s| s.y == slot.y && s.h >= slot.h) {
                    consume_from_shelf(sh, slot, border);
                } else {
                    let mut sh = Shelf {
                        y: slot.y,
                        h: slot.h,
                        segs: vec![(border.x, border.w)],
                    };
                    consume_from_shelf(&mut sh, slot, border);
                    shelves.push(sh);
                    *next_y = (*next_y).max(slot.y + slot.h);
                }
            }
        }
        self.used
            .insert(key.to_string(), (*slot, rotated, frame.clone()));
    }

    fn add_free(&mut self, r: Rect) {
        match &mut self.mode {
            RtMode::Guillotine { free, .. } => {
                free.push(r);
                prune_free_list(free);
                merge_free_list(free);
            }
            RtMode::Shelf { shelves, .. } => {
                if let Some(sh) = shelves.iter_mut().find(|s| s.y == r.y && s.h == r.h) {
                    sh.segs.push((r.x, r.w));
                    merge_shelf_segments(sh);
                } else {
                    shelves.push(Shelf {
                        y: r.y,
                        h: r.h,
                        segs: vec![(r.x, r.w)],
                    });
                }
            }
        }
    }

    // guillotine prune/split helpers moved to free functions below
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

fn split_rect(split: &GuillotineSplit, fr: &Rect, placed: &Rect) -> (Option<Rect>, Option<Rect>) {
    let w_right = (fr.x + fr.w).saturating_sub(placed.x + placed.w);
    let h_bottom = (fr.y + fr.h).saturating_sub(placed.y + placed.h);
    let split_horizontal = match split {
        GuillotineSplit::SplitShorterLeftoverAxis => h_bottom < w_right,
        GuillotineSplit::SplitLongerLeftoverAxis => h_bottom > w_right,
        GuillotineSplit::SplitMinimizeArea => (w_right * fr.h) <= (fr.w * h_bottom),
        GuillotineSplit::SplitMaximizeArea => (w_right * fr.h) >= (fr.w * h_bottom),
        GuillotineSplit::SplitShorterAxis => fr.h < fr.w,
        GuillotineSplit::SplitLongerAxis => fr.h > fr.w,
    };
    let mut bottom = Rect::new(fr.x, placed.y + placed.h, 0, fr.h.saturating_sub(placed.h));
    let mut right = Rect::new(placed.x + placed.w, fr.y, fr.w.saturating_sub(placed.w), 0);
    if split_horizontal {
        bottom.w = fr.w;
        right.h = placed.h;
    } else {
        bottom.w = placed.w;
        right.h = fr.h;
    }
    (
        if bottom.w > 0 && bottom.h > 0 {
            Some(bottom)
        } else {
            None
        },
        if right.w > 0 && right.h > 0 {
            Some(right)
        } else {
            None
        },
    )
}

// ---------- helpers for page modes ----------

fn prune_free_list(free: &mut Vec<Rect>) {
    let mut i = 0;
    while i < free.len() {
        let mut j = i + 1;
        let a = free[i];
        let a_x2 = a.x + a.w;
        let a_y2 = a.y + a.h;
        let mut remove_i = false;
        while j < free.len() {
            let b = free[j];
            let b_x2 = b.x + b.w;
            let b_y2 = b.y + b.h;
            if a.x >= b.x && a.y >= b.y && a_x2 <= b_x2 && a_y2 <= b_y2 {
                remove_i = true;
                break;
            }
            if b.x >= a.x && b.y >= a.y && b_x2 <= a_x2 && b_y2 <= a_y2 {
                free.remove(j);
                continue;
            }
            j += 1;
        }
        if remove_i {
            free.remove(i);
        } else {
            i += 1;
        }
    }
}

fn merge_free_list(free: &mut Vec<Rect>) {
    let mut merged = true;
    while merged {
        merged = false;
        'outer: for i in 0..free.len() {
            for j in i + 1..free.len() {
                let a = free[i];
                let b = free[j];
                if a.y == b.y && a.h == b.h {
                    if a.x + a.w == b.x {
                        free[i] = Rect::new(a.x, a.y, a.w + b.w, a.h);
                        free.remove(j);
                        merged = true;
                        break 'outer;
                    } else if b.x + b.w == a.x {
                        free[i] = Rect::new(b.x, a.y, a.w + b.w, a.h);
                        free.remove(j);
                        merged = true;
                        break 'outer;
                    }
                }
                if a.x == b.x && a.w == b.w {
                    if a.y + a.h == b.y {
                        free[i] = Rect::new(a.x, a.y, a.w, a.h + b.h);
                        free.remove(j);
                        merged = true;
                        break 'outer;
                    } else if b.y + b.h == a.y {
                        free[i] = Rect::new(a.x, b.y, a.w, a.h + b.h);
                        free.remove(j);
                        merged = true;
                        break 'outer;
                    }
                }
            }
        }
    }
}

fn choose_shelf(
    allow_rot: bool,
    border: &Rect,
    policy: ShelfPolicy,
    shelves: &Vec<Shelf>,
    next_y: u32,
    w: u32,
    h: u32,
) -> Option<(Rect, bool)> {
    let try_in = |rw: u32, rh: u32| -> Option<Rect> {
        match policy {
            ShelfPolicy::FirstFit => {
                for sh in shelves {
                    if rh <= sh.h {
                        if let Some((sx, _sw)) = sh
                            .segs
                            .iter()
                            .find(|(sx, sw)| *sw >= rw && *sx + rw <= border.x + border.w)
                        {
                            return Some(Rect::new(*sx, sh.y, rw, rh));
                        }
                    }
                }
                None
            }
            ShelfPolicy::NextFit => {
                if let Some(sh) = shelves.last() {
                    if rh <= sh.h {
                        if let Some((sx, _sw)) = sh
                            .segs
                            .iter()
                            .find(|(sx, sw)| *sw >= rw && *sx + rw <= border.x + border.w)
                        {
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
        if rw <= border.w && next_y + rh <= border.y + border.h {
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

fn consume_from_shelf(sh: &mut Shelf, slot: &Rect, border: &Rect) {
    let mut i = 0;
    while i < sh.segs.len() {
        let (sx, sw) = sh.segs[i];
        if slot.x >= sx && slot.x + slot.w <= sx + sw {
            sh.segs.remove(i);
            let left_w = slot.x.saturating_sub(sx);
            let right_x = slot.x + slot.w;
            let right_w = (sx + sw).saturating_sub(right_x);
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
        .retain(|(x, w)| *w > 0 && *x >= border.x && *x + *w <= border.x + border.w);
}

fn merge_shelf_segments(sh: &mut Shelf) {
    sh.segs.sort_by_key(|(x, _)| *x);
    let mut out: Vec<(u32, u32)> = Vec::new();
    for (x, w) in sh.segs.drain(..) {
        if let Some((lx, lw)) = out.last_mut() {
            if *lx + *lw == x {
                *lw += w;
                continue;
            }
        }
        out.push((x, w));
    }
    sh.segs = out;
}
