use image::{Rgba, RgbaImage};

use crate::config::RuntimeConfig;
use crate::error::{Result, TexPackerError};
use crate::model::{Atlas, PageId, Rect};
use crate::runtime::{AtlasSession, RuntimePlacement, RuntimeStats};

/// Region that needs to be updated on a GPU texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateRegion {
    pub page_id: PageId,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl UpdateRegion {
    pub const fn empty() -> Self {
        Self {
            page_id: PageId::new(0),
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub const fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

/// Result of uploading an image into a runtime atlas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeImageUpdate {
    placement: RuntimePlacement,
    dirty_region: UpdateRegion,
}

impl RuntimeImageUpdate {
    pub const fn placement(&self) -> &RuntimePlacement {
        &self.placement
    }

    pub const fn dirty_region(&self) -> UpdateRegion {
        self.dirty_region
    }
}

struct RuntimeImagePage {
    id: PageId,
    image: RgbaImage,
}

/// Runtime atlas with pixel data management.
pub struct RuntimeAtlas {
    session: AtlasSession,
    pages: Vec<RuntimeImagePage>,
    background_color: Rgba<u8>,
    outlines: bool,
}

impl RuntimeAtlas {
    pub fn new(cfg: RuntimeConfig) -> Self {
        Self {
            session: AtlasSession::new(cfg),
            pages: Vec::new(),
            background_color: Rgba([0, 0, 0, 0]),
            outlines: false,
        }
    }

    pub fn with_background_color(mut self, color: Rgba<u8>) -> Self {
        self.background_color = color;
        self
    }

    pub fn with_outlines(mut self, enabled: bool) -> Self {
        self.outlines = enabled;
        self
    }

    pub fn append_with_image(
        &mut self,
        key: String,
        image: &RgbaImage,
    ) -> Result<RuntimeImageUpdate> {
        let (width, height) = image.dimensions();
        let placement = self.session.append(key, width, height)?;
        self.ensure_page(placement.page_id())?;
        let dirty_region = self.blit_to_page(&placement, image)?;

        Ok(RuntimeImageUpdate {
            placement,
            dirty_region,
        })
    }

    pub fn append(&mut self, key: String, w: u32, h: u32) -> Result<RuntimePlacement> {
        self.session.append(key, w, h)
    }

    pub fn evict_with_clear(
        &mut self,
        page_id: PageId,
        key: &str,
        clear: bool,
    ) -> Option<UpdateRegion> {
        let allocation = clear.then(|| self.session.get_reserved_slot(key)).flatten();

        if !self.session.evict(page_id, key) {
            return None;
        }

        if let Some((allocation_page_id, rect)) = allocation {
            let region = update_region(allocation_page_id, rect);
            self.clear_region(region);
            Some(region)
        } else {
            Some(UpdateRegion::empty())
        }
    }

    pub fn evict_by_key_with_clear(&mut self, key: &str, clear: bool) -> Option<UpdateRegion> {
        let allocation = clear.then(|| self.session.get_reserved_slot(key)).flatten();

        if !self.session.evict_by_key(key) {
            return None;
        }

        if let Some((page_id, rect)) = allocation {
            let region = update_region(page_id, rect);
            self.clear_region(region);
            Some(region)
        } else {
            Some(UpdateRegion::empty())
        }
    }

    pub fn get_page_image(&self, page_id: PageId) -> Option<&RgbaImage> {
        self.pages
            .iter()
            .find(|page| page.id == page_id)
            .map(|page| &page.image)
    }

    pub fn get_page_image_mut(&mut self, page_id: PageId) -> Option<&mut RgbaImage> {
        self.pages
            .iter_mut()
            .find(|page| page.id == page_id)
            .map(|page| &mut page.image)
    }

    pub fn num_pages(&self) -> usize {
        self.pages.len()
    }

    pub fn get_frame(&self, key: &str) -> Option<RuntimePlacement> {
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

    pub fn snapshot_atlas(&self) -> Result<Atlas> {
        self.session.snapshot_atlas()
    }

    fn ensure_page(&mut self, page_id: PageId) -> Result<()> {
        if self.pages.iter().any(|page| page.id == page_id) {
            return Ok(());
        }

        let (width, height) =
            self.session
                .page_size(page_id)
                .ok_or_else(|| TexPackerError::InvariantViolation {
                    context: format!("runtime image page {page_id}"),
                    reason: "geometry page is missing".into(),
                })?;
        self.pages.push(RuntimeImagePage {
            id: page_id,
            image: RgbaImage::from_pixel(width, height, self.background_color),
        });
        Ok(())
    }

    fn blit_to_page(
        &mut self,
        placement: &RuntimePlacement,
        image: &RgbaImage,
    ) -> Result<UpdateRegion> {
        let page_id = placement.page_id();
        let page = self
            .pages
            .iter_mut()
            .find(|page| page.id == page_id)
            .map(|page| &mut page.image)
            .ok_or_else(|| TexPackerError::InvariantViolation {
                context: format!("runtime image page {page_id}"),
                reason: "pixel buffer is missing".into(),
            })?;

        let content = placement.content();
        let (source_width, source_height) = image.dimensions();
        let extrusion = self.session.cfg.page_config().texture_extrusion();
        let destination =
            crate::compositing::BlitRect::new(content.x, content.y, content.w, content.h);
        let source = crate::compositing::BlitRect::new(0, 0, source_width, source_height);
        let options = crate::compositing::BlitOptions {
            rotated: placement.rotated(),
            extrude: extrusion,
            outlines: self.outlines,
        };
        crate::compositing::blit_rgba(image, page, destination, source, options);

        let start_x = content.x.saturating_sub(extrusion);
        let start_y = content.y.saturating_sub(extrusion);
        let width = content
            .w
            .saturating_add(extrusion.saturating_mul(2))
            .min(page.width().saturating_sub(start_x));
        let height = content
            .h
            .saturating_add(extrusion.saturating_mul(2))
            .min(page.height().saturating_sub(start_y));

        Ok(UpdateRegion {
            page_id,
            x: start_x,
            y: start_y,
            width,
            height,
        })
    }

    fn clear_region(&mut self, region: UpdateRegion) {
        let Some(page) = self
            .pages
            .iter_mut()
            .find(|page| page.id == region.page_id)
            .map(|page| &mut page.image)
        else {
            return;
        };

        for y in region.y..region.y.saturating_add(region.height).min(page.height()) {
            for x in region.x..region.x.saturating_add(region.width).min(page.width()) {
                page.put_pixel(x, y, self.background_color);
            }
        }
    }
}

fn update_region(page_id: PageId, rect: Rect) -> UpdateRegion {
    UpdateRegion {
        page_id,
        x: rect.x,
        y: rect.y,
        width: rect.w,
        height: rect.h,
    }
}
