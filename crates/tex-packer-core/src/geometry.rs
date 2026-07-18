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

/// Normalized visible dimensions accepted by the physical placement engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContentSize {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl ContentSize {
    pub(crate) fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Authoritative physical geometry produced by one successful placement attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PhysicalPlacement {
    pub(crate) content: Rect,
    pub(crate) allocation: Rect,
    pub(crate) rotated: bool,
}

/// Geometry derived from content dimensions and validated page configuration.
///
/// The reserved size includes visible content, texture padding, and extrusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlacementGeometry {
    pub(crate) reserved_w: u32,
    pub(crate) reserved_h: u32,
    content_w: u32,
    content_h: u32,
    offset: u32,
    trailing_extra: u32,
}

impl PlacementGeometry {
    pub(crate) fn new(content: ContentSize, config: &PageConfig) -> Option<Self> {
        let content_w = content.width;
        let content_h = content.height;
        let (reserved_w, reserved_h) = config.checked_reservation(content_w, content_h)?;
        Some(Self {
            reserved_w,
            reserved_h,
            content_w,
            content_h,
            offset: config.content_offset(),
            trailing_extra: config.trailing_extra(),
        })
    }

    pub(crate) fn reserved_size(self) -> (u32, u32) {
        (self.reserved_w, self.reserved_h)
    }

    pub(crate) fn from_size(
        content_width: u32,
        content_height: u32,
        config: &PageConfig,
    ) -> Option<Self> {
        Self::new(ContentSize::new(content_width, content_height), config)
    }

    fn rotated_content_size(self, rotated: bool) -> (u32, u32) {
        if rotated {
            (self.content_h, self.content_w)
        } else {
            (self.content_w, self.content_h)
        }
    }

    fn content_rect(self, allocation: Rect, rotated: bool) -> Rect {
        let (frame_w, frame_h) = self.rotated_content_size(rotated);
        let content = Rect::new(
            allocation.x.saturating_add(self.offset),
            allocation.y.saturating_add(self.offset),
            frame_w,
            frame_h,
        );
        debug_assert_eq!(
            allocation.w as u64,
            content.w as u64 + self.offset as u64 + self.trailing_extra as u64
        );
        debug_assert_eq!(
            allocation.h as u64,
            content.h as u64 + self.offset as u64 + self.trailing_extra as u64
        );
        content
    }

    pub(crate) fn complete(self, allocation: Rect, rotated: bool) -> PhysicalPlacement {
        PhysicalPlacement {
            content: self.content_rect(allocation, rotated),
            allocation,
            rotated,
        }
    }

    pub(crate) fn frame<K>(
        self,
        key: K,
        source: Rect,
        allocation: &Rect,
        rotated: bool,
    ) -> Frame<K> {
        Frame {
            key,
            frame: self.content_rect(*allocation, rotated),
            rotated,
            trimmed: false,
            source,
            source_size: (source.w, source.h),
        }
    }
}
