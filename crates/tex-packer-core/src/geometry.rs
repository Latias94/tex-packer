use crate::config::PackerConfig;
use crate::model::{Frame, Rect};

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
        let reserved_extra = cfg
            .texture_extrusion
            .saturating_mul(2)
            .saturating_add(cfg.texture_padding);
        Self {
            reserved_w: content_w.saturating_add(reserved_extra),
            reserved_h: content_h.saturating_add(reserved_extra),
            content_w,
            content_h,
            offset: cfg
                .texture_extrusion
                .saturating_add(cfg.texture_padding / 2),
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
