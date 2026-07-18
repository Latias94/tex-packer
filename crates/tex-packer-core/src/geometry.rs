use crate::config::PageConfig;
use crate::model::{Frame, Rect};

pub(crate) fn usable_area(config: &PageConfig) -> Rect {
    let padding = config.border_padding();
    let (width, height) = config.usable_dimensions();
    Rect::new(padding, padding, width, height)
}

/// Returns the exclusive right edge (`x + w`) as a widened integer.
#[inline]
pub(crate) fn right_ex(rect: &Rect) -> u64 {
    rect.x as u64 + rect.w as u64
}

/// Returns the exclusive bottom edge (`y + h`) as a widened integer.
#[inline]
pub(crate) fn bottom_ex(rect: &Rect) -> u64 {
    rect.y as u64 + rect.h as u64
}

/// Returns the exclusive right edge as `u32`, saturating only at the coordinate type limit.
#[inline]
pub(crate) fn right_ex_u32(rect: &Rect) -> u32 {
    rect.x.saturating_add(rect.w)
}

/// Returns the exclusive bottom edge as `u32`, saturating only at the coordinate type limit.
#[inline]
pub(crate) fn bottom_ex_u32(rect: &Rect) -> u32 {
    rect.y.saturating_add(rect.h)
}

/// Returns the exclusive end of a one-dimensional span as `u32`.
#[inline]
pub(crate) fn span_end_ex(start: u32, len: u32) -> u32 {
    start.saturating_add(len)
}

/// Returns true when `inner` is fully inside `outer` using exclusive edges.
#[inline]
pub(crate) fn contains_rect(outer: &Rect, inner: &Rect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && right_ex(inner) <= right_ex(outer)
        && bottom_ex(inner) <= bottom_ex(outer)
}

/// Returns true when two positive-area rectangles overlap using exclusive edges.
#[inline]
pub(crate) fn intersects(a: &Rect, b: &Rect) -> bool {
    !(a.x as u64 >= right_ex(b)
        || b.x as u64 >= right_ex(a)
        || a.y as u64 >= bottom_ex(b)
        || b.y as u64 >= bottom_ex(a))
}

/// Area of a width/height pair using widened arithmetic.
#[inline]
pub(crate) fn area_u128(w: u32, h: u32) -> u128 {
    w as u128 * h as u128
}

/// Area of a rectangle using widened arithmetic.
#[inline]
pub(crate) fn rect_area_u128(rect: &Rect) -> u128 {
    area_u128(rect.w, rect.h)
}

/// Remaining area score after placing a `w` x `h` rectangle in `free`.
#[inline]
pub(crate) fn area_fit_score(free: &Rect, w: u32, h: u32) -> i128 {
    rect_area_u128(free) as i128 - area_u128(w, h) as i128
}

/// One-dimensional exclusive overlap length.
#[inline]
pub(crate) fn overlap_1d(a_start: u32, a_end_ex: u32, b_start: u32, b_end_ex: u32) -> u32 {
    a_end_ex.min(b_end_ex).saturating_sub(a_start.max(b_start))
}

/// Geometry derived from source content and packing configuration.
///
/// The reserved size is the rectangle consumed by the packing algorithm. It includes
/// frame content, texture padding, and extrusion. The public frame rectangle remains
/// the visible content area inside that reserved slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacementGeometry {
    pub reserved_w: u32,
    pub reserved_h: u32,
    content_w: u32,
    content_h: u32,
    offset: u32,
}

impl PlacementGeometry {
    pub fn new(content: &Rect, config: &PageConfig) -> Option<Self> {
        Self::from_size(content.w, content.h, config)
    }

    pub fn from_size(content_w: u32, content_h: u32, config: &PageConfig) -> Option<Self> {
        let (reserved_w, reserved_h) = config.checked_reservation(content_w, content_h)?;
        Some(Self {
            reserved_w,
            reserved_h,
            content_w,
            content_h,
            offset: config.content_offset(),
        })
    }

    pub fn rotated_content_size(&self, rotated: bool) -> (u32, u32) {
        if rotated {
            (self.content_h, self.content_w)
        } else {
            (self.content_w, self.content_h)
        }
    }

    pub fn frame_rect(&self, reserved_slot: &Rect, rotated: bool) -> Rect {
        let (frame_w, frame_h) = self.rotated_content_size(rotated);
        Rect::new(
            reserved_slot.x.saturating_add(self.offset),
            reserved_slot.y.saturating_add(self.offset),
            frame_w,
            frame_h,
        )
    }

    pub fn frame<K>(&self, key: K, source: Rect, reserved_slot: &Rect, rotated: bool) -> Frame<K> {
        Frame {
            key,
            frame: self.frame_rect(reserved_slot, rotated),
            rotated,
            trimmed: false,
            source,
            source_size: (source.w, source.h),
        }
    }
}
