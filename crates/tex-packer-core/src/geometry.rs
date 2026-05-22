use crate::config::PackerConfig;
use crate::model::{Frame, Rect};

/// Validated, derived geometry and page sizing context for a packing run.
///
/// This is intentionally crate-private: public callers still build `PackerConfig`,
/// while internal modules use the precomputed invariants instead of recomputing
/// border, padding, extrusion, and final page-size policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PackingContext {
    max_width: u32,
    max_height: u32,
    border_padding: u32,
    force_max_dimensions: bool,
    power_of_two: bool,
    square: bool,
    usable: Rect,
    reserved_extra: u32,
    frame_offset: u32,
    trailing_extra: u32,
}

impl PackingContext {
    pub(crate) fn new(cfg: &PackerConfig) -> Self {
        let border_padding = cfg.border_padding;
        let usable = usable_area(cfg);
        let reserved_extra = cfg
            .texture_extrusion
            .saturating_mul(2)
            .saturating_add(cfg.texture_padding);
        let frame_offset = cfg
            .texture_extrusion
            .saturating_add(cfg.texture_padding / 2);
        let trailing_padding = cfg.texture_padding.saturating_sub(cfg.texture_padding / 2);
        let trailing_extra = cfg.texture_extrusion.saturating_add(trailing_padding);

        Self {
            max_width: cfg.max_width,
            max_height: cfg.max_height,
            border_padding,
            force_max_dimensions: cfg.force_max_dimensions,
            power_of_two: cfg.power_of_two,
            square: cfg.square,
            usable,
            reserved_extra,
            frame_offset,
            trailing_extra,
        }
    }

    pub(crate) fn usable_area(&self) -> Rect {
        self.usable
    }

    pub(crate) fn reserved_extra(&self) -> u32 {
        self.reserved_extra
    }

    pub(crate) fn frame_offset(&self) -> u32 {
        self.frame_offset
    }

    pub(crate) fn border_padding(&self) -> u32 {
        self.border_padding
    }

    pub(crate) fn max_dimensions(&self) -> (u32, u32) {
        (self.max_width, self.max_height)
    }

    pub(crate) fn compute_page_size(&self, frames: &[Frame]) -> (u32, u32) {
        if self.force_max_dimensions {
            return self.max_dimensions();
        }

        let mut page_w = 0u32;
        let mut page_h = 0u32;
        for frame in frames {
            page_w = page_w.max(
                right_ex_u32(&frame.frame)
                    .saturating_add(self.trailing_extra)
                    .saturating_add(self.border_padding),
            );
            page_h = page_h.max(
                bottom_ex_u32(&frame.frame)
                    .saturating_add(self.trailing_extra)
                    .saturating_add(self.border_padding),
            );
        }

        if self.power_of_two {
            page_w = next_pow2(page_w.max(1));
            page_h = next_pow2(page_h.max(1));
        }
        if self.square {
            let m = page_w.max(page_h);
            page_w = m;
            page_h = m;
        }

        (page_w, page_h)
    }
}

pub(crate) fn usable_area(cfg: &PackerConfig) -> Rect {
    let pad = cfg.border_padding;
    Rect::new(
        pad,
        pad,
        cfg.max_width.saturating_sub(pad.saturating_mul(2)),
        cfg.max_height.saturating_sub(pad.saturating_mul(2)),
    )
}

fn next_pow2(mut v: u32) -> u32 {
    if v <= 1 {
        return 1;
    }
    v -= 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v + 1
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
    pub fn new(content: &Rect, cfg: &PackerConfig) -> Self {
        Self::from_size(content.w, content.h, cfg)
    }

    pub fn from_size(content_w: u32, content_h: u32, cfg: &PackerConfig) -> Self {
        Self::from_context(content_w, content_h, &PackingContext::new(cfg))
    }

    pub(crate) fn from_context(content_w: u32, content_h: u32, ctx: &PackingContext) -> Self {
        Self {
            reserved_w: content_w.saturating_add(ctx.reserved_extra()),
            reserved_h: content_h.saturating_add(ctx.reserved_extra()),
            content_w,
            content_h,
            offset: ctx.frame_offset(),
        }
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
