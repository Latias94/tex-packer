use crate::config::PackerConfig;
use crate::error::{Result, TexPackerError};
use crate::model::Frame;
use crate::runtime::{AtlasSession, RuntimeStats, RuntimeStrategy};
use image::{Rgba, RgbaImage};

/// Region that needs to be updated on GPU texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateRegion {
    /// Page ID that needs updating.
    pub page_id: usize,
    /// X coordinate of the region.
    pub x: u32,
    /// Y coordinate of the region.
    pub y: u32,
    /// Width of the region.
    pub width: u32,
    /// Height of the region.
    pub height: u32,
}

impl UpdateRegion {
    /// Create an empty update region.
    pub fn empty() -> Self {
        Self {
            page_id: 0,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }

    /// Check if this region is empty.
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Get the area of this region in pixels.
    pub fn area(&self) -> u64 {
        (self.width as u64) * (self.height as u64)
    }
}

/// Runtime atlas with pixel data management.
///
/// This extends `AtlasSession` by managing actual pixel data in addition to geometry.
/// Useful for game engines that need to dynamically update GPU textures.
pub struct RuntimeAtlas {
    session: AtlasSession,
    pages: Vec<RgbaImage>,
    background_color: Rgba<u8>,
}

impl RuntimeAtlas {
    /// Create a new runtime atlas with pixel data management.
    pub fn new(cfg: PackerConfig, strategy: RuntimeStrategy) -> Self {
        Self {
            session: AtlasSession::new(cfg, strategy),
            pages: Vec::new(),
            background_color: Rgba([0, 0, 0, 0]), // Transparent by default
        }
    }

    /// Set the background color for new pages.
    pub fn with_background_color(mut self, color: Rgba<u8>) -> Self {
        self.background_color = color;
        self
    }

    /// Append a texture with its pixel data.
    /// Returns (page_id, frame, update_region).
    pub fn append_with_image(
        &mut self,
        key: String,
        image: &RgbaImage,
    ) -> Result<(usize, Frame<String>, UpdateRegion)> {
        let (w, h) = image.dimensions();
        let (page_id, frame) = self.session.append(key, w, h)?;

        // Ensure page exists
        self.ensure_page(page_id);

        // Blit image to page
        let update_region = self.blit_to_page(page_id, &frame, image)?;

        Ok((page_id, frame, update_region))
    }

    /// Append a texture by dimensions only (no pixel data).
    /// Returns (page_id, frame).
    pub fn append(&mut self, key: String, w: u32, h: u32) -> Result<(usize, Frame<String>)> {
        self.session.append(key, w, h)
    }

    /// Evict a texture and optionally clear its region.
    /// Returns the region that was cleared (if clear=true).
    pub fn evict_with_clear(
        &mut self,
        page_id: usize,
        key: &str,
        clear: bool,
    ) -> Option<UpdateRegion> {
        // Get reserved slot before evicting (covers padding/extrude)
        let slot_region = if clear {
            self.session
                .get_reserved_slot(key)
                .map(|(pid, slot)| UpdateRegion {
                    page_id: pid,
                    x: slot.x,
                    y: slot.y,
                    width: slot.w,
                    height: slot.h,
                })
        } else {
            None
        };

        // Evict from session
        if self.session.evict(page_id, key) {
            // Clear pixels if requested
            if clear {
                if let Some(region) = slot_region {
                    self.clear_region(region);
                    return Some(region);
                }
            }
            Some(UpdateRegion::empty())
        } else {
            None
        }
    }

    /// Evict a texture by key and optionally clear its region.
    pub fn evict_by_key_with_clear(&mut self, key: &str, clear: bool) -> Option<UpdateRegion> {
        // Get reserved slot before evicting
        let slot_region = if clear {
            self.session
                .get_reserved_slot(key)
                .map(|(page_id, slot)| UpdateRegion {
                    page_id,
                    x: slot.x,
                    y: slot.y,
                    width: slot.w,
                    height: slot.h,
                })
        } else {
            None
        };

        if self.session.evict_by_key(key) {
            if clear {
                if let Some(region) = slot_region {
                    self.clear_region(region);
                    return Some(region);
                }
            }
            Some(UpdateRegion::empty())
        } else {
            None
        }
    }

    /// Get a reference to the pixel data of a page.
    pub fn get_page_image(&self, page_id: usize) -> Option<&RgbaImage> {
        self.pages.get(page_id)
    }

    /// Get a mutable reference to the pixel data of a page.
    pub fn get_page_image_mut(&mut self, page_id: usize) -> Option<&mut RgbaImage> {
        self.pages.get_mut(page_id)
    }

    /// Get the number of pages with pixel data.
    pub fn num_pages(&self) -> usize {
        self.pages.len()
    }

    // Delegate query methods to session
    pub fn get_frame(&self, key: &str) -> Option<(usize, &Frame<String>)> {
        self.session.get_frame(key)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.session.contains(key)
    }

    pub fn keys(&self) -> Vec<&str> {
        self.session.keys()
    }

    pub fn texture_count(&self) -> usize {
        self.session.texture_count()
    }

    pub fn stats(&self) -> RuntimeStats {
        self.session.stats()
    }

    pub fn snapshot_atlas(&self) -> crate::model::Atlas<String> {
        self.session.snapshot_atlas()
    }

    /// Ensure a page exists, creating it if necessary.
    fn ensure_page(&mut self, page_id: usize) {
        while self.pages.len() <= page_id {
            let page_img = RgbaImage::from_pixel(
                self.session.cfg.max_width,
                self.session.cfg.max_height,
                self.background_color,
            );
            self.pages.push(page_img);
        }
    }

    /// Blit an image to a page at the frame's position.
    fn blit_to_page(
        &mut self,
        page_id: usize,
        frame: &Frame<String>,
        image: &RgbaImage,
    ) -> Result<UpdateRegion> {
        let page = self
            .pages
            .get_mut(page_id)
            .ok_or_else(|| TexPackerError::InvalidConfig("Page not found".into()))?;

        let (src_w, src_h) = image.dimensions();
        let dst_x = frame.frame.x;
        let dst_y = frame.frame.y;

        // Reuse core compositing (with extrusion and optional outlines)
        let extrude = self.session.cfg.texture_extrusion;
        let outlines = self.session.cfg.texture_outlines;
        crate::compositing::blit_rgba(
            image,
            page,
            dst_x,
            dst_y,
            0,
            0,
            src_w,
            src_h,
            frame.rotated,
            extrude,
            outlines,
        );

        // Return the minimal update region including extrusion
        let start_x = dst_x.saturating_sub(extrude);
        let start_y = dst_y.saturating_sub(extrude);
        let mut width = frame.frame.w + extrude.saturating_mul(2);
        let mut height = frame.frame.h + extrude.saturating_mul(2);
        // Clamp to page bounds
        if start_x + width > page.width() {
            width = page.width() - start_x;
        }
        if start_y + height > page.height() {
            height = page.height() - start_y;
        }

        Ok(UpdateRegion {
            page_id,
            x: start_x,
            y: start_y,
            width,
            height,
        })
    }

    /// Clear a region on a page.
    fn clear_region(&mut self, region: UpdateRegion) {
        if let Some(page) = self.pages.get_mut(region.page_id) {
            for y in region.y..(region.y + region.height).min(page.height()) {
                for x in region.x..(region.x + region.width).min(page.width()) {
                    page.put_pixel(x, y, self.background_color);
                }
            }
        }
    }
}
