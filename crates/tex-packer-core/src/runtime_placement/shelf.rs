use crate::config::ShelfPolicy;
use crate::geometry::{bottom_ex_u32, right_ex_u32, span_end_ex};
use crate::model::Rect;

pub(super) struct RuntimeShelfPlacement {
    border: Rect,
    policy: ShelfPolicy,
    shelves: Vec<RuntimeShelf>,
    next_y: u32,
}

#[derive(Clone, Debug)]
struct RuntimeShelf {
    y: u32,
    h: u32,
    segs: Vec<(u32, u32)>,
}

impl RuntimeShelfPlacement {
    pub(super) fn new(border: Rect, policy: ShelfPolicy) -> Self {
        Self {
            border,
            policy,
            shelves: Vec::new(),
            next_y: border.y,
        }
    }

    pub(super) fn free_area_and_rects(&self) -> (u64, usize) {
        let mut area = 0u64;
        let mut rects = 0usize;
        for shelf in &self.shelves {
            rects += shelf.segs.len();
            for (_, width) in &shelf.segs {
                area += (*width as u64) * (shelf.h as u64);
            }
        }
        (area, rects)
    }

    pub(super) fn choose(&self, allow_rotation: bool, w: u32, h: u32) -> Option<(Rect, bool)> {
        if let Some(rect) = self.try_existing_shelf(w, h) {
            return Some((rect, false));
        }
        if allow_rotation && let Some(rect) = self.try_existing_shelf(h, w) {
            return Some((rect, true));
        }
        if let Some(rect) = self.try_new_shelf(w, h) {
            return Some((rect, false));
        }
        if allow_rotation && let Some(rect) = self.try_new_shelf(h, w) {
            return Some((rect, true));
        }
        None
    }

    pub(super) fn place(&mut self, slot: &Rect) {
        if let Some(shelf) = self
            .shelves
            .iter_mut()
            .find(|shelf| shelf.y == slot.y && shelf.h >= slot.h)
        {
            consume_from_shelf(shelf, slot, &self.border);
        } else {
            let mut shelf = RuntimeShelf {
                y: slot.y,
                h: slot.h,
                segs: vec![(self.border.x, self.border.w)],
            };
            consume_from_shelf(&mut shelf, slot, &self.border);
            self.shelves.push(shelf);
            self.next_y = self.next_y.max(bottom_ex_u32(slot));
        }
    }

    pub(super) fn add_free(&mut self, rect: Rect) {
        if let Some(shelf) = self
            .shelves
            .iter_mut()
            .find(|shelf| shelf.y == rect.y && shelf.h == rect.h)
        {
            shelf.segs.push((rect.x, rect.w));
            merge_shelf_segments(shelf);
        } else {
            self.shelves.push(RuntimeShelf {
                y: rect.y,
                h: rect.h,
                segs: vec![(rect.x, rect.w)],
            });
        }
    }

    fn try_existing_shelf(&self, w: u32, h: u32) -> Option<Rect> {
        let shelves: Box<dyn Iterator<Item = &RuntimeShelf> + '_> = match self.policy {
            ShelfPolicy::FirstFit => Box::new(self.shelves.iter()),
            ShelfPolicy::NextFit => Box::new(self.shelves.iter().rev().take(1)),
        };

        for shelf in shelves {
            if h <= shelf.h
                && let Some((x, _width)) = shelf.segs.iter().find(|(x, width)| {
                    *width >= w && span_end_ex(*x, w) <= right_ex_u32(&self.border)
                })
            {
                return Some(Rect::new(*x, shelf.y, w, h));
            }
        }
        None
    }

    fn try_new_shelf(&self, w: u32, h: u32) -> Option<Rect> {
        if w <= self.border.w && span_end_ex(self.next_y, h) <= bottom_ex_u32(&self.border) {
            Some(Rect::new(self.border.x, self.next_y, w, h))
        } else {
            None
        }
    }
}

fn consume_from_shelf(shelf: &mut RuntimeShelf, slot: &Rect, border: &Rect) {
    let mut index = 0;
    while index < shelf.segs.len() {
        let (seg_x, seg_w) = shelf.segs[index];
        if slot.x >= seg_x && right_ex_u32(slot) <= span_end_ex(seg_x, seg_w) {
            shelf.segs.remove(index);
            let left_w = slot.x.saturating_sub(seg_x);
            let right_x = right_ex_u32(slot);
            let right_w = span_end_ex(seg_x, seg_w).saturating_sub(right_x);
            if left_w > 0 {
                shelf.segs.push((seg_x, left_w));
            }
            if right_w > 0 {
                shelf.segs.push((right_x, right_w));
            }
            break;
        } else {
            index += 1;
        }
    }
    merge_shelf_segments(shelf);
    shelf
        .segs
        .retain(|(x, w)| *w > 0 && *x >= border.x && span_end_ex(*x, *w) <= right_ex_u32(border));
}

fn merge_shelf_segments(shelf: &mut RuntimeShelf) {
    shelf.segs.sort_by_key(|(x, _)| *x);
    let mut out: Vec<(u32, u32)> = Vec::new();
    for (x, w) in shelf.segs.drain(..) {
        if let Some((last_x, last_w)) = out.last_mut()
            && span_end_ex(*last_x, *last_w) == x
        {
            *last_w += w;
            continue;
        }
        out.push((x, w));
    }
    shelf.segs = out;
}
