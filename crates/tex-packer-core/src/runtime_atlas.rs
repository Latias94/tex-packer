use std::collections::HashMap;

use image::{Rgba, RgbaImage};

use crate::config::RuntimeConfig;
use crate::error::{Result, TexPackerError};
use crate::model::{Atlas, PageId, Rect};
use crate::runtime::{AtlasSession, PreparedAppend, RuntimePlacement, RuntimeStats};

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

enum PreparedPixels {
    Patch {
        page_index: usize,
        region: UpdateRegion,
        pixels: RgbaImage,
    },
    New(RuntimeImagePage),
}

/// Runtime atlas with pixel data management.
pub struct RuntimeAtlas {
    session: AtlasSession,
    pages: Vec<RuntimeImagePage>,
    page_slots: HashMap<PageId, usize>,
    background_color: Rgba<u8>,
    outlines: bool,
}

impl RuntimeAtlas {
    pub fn new(cfg: RuntimeConfig) -> Self {
        Self {
            session: AtlasSession::new(cfg),
            pages: Vec::new(),
            page_slots: HashMap::new(),
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
        let prepared_append = self.session.prepare_append(key, width, height)?;
        let (prepared_pixels, dirty_region) = self.prepare_pixels(&prepared_append, image)?;
        let placement = self.commit_image_append(prepared_append, prepared_pixels);

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
        let page_slot = *self.page_slots.get(&page_id)?;
        self.pages.get(page_slot).map(|page| &page.image)
    }

    pub fn get_page_image_mut(&mut self, page_id: PageId) -> Option<&mut RgbaImage> {
        let page_slot = *self.page_slots.get(&page_id)?;
        self.pages.get_mut(page_slot).map(|page| &mut page.image)
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

    fn prepare_pixels(
        &self,
        prepared_append: &PreparedAppend,
        image: &RgbaImage,
    ) -> Result<(PreparedPixels, UpdateRegion)> {
        let placement = prepared_append.placement();
        let page_id = placement.page_id();
        let extrusion = self.session.cfg.page_config().texture_extrusion();
        let page_size = prepared_append.page_size();
        let dirty_region = validate_blit(placement, image, page_size, extrusion)?;

        if let Some(&page_index) = self.page_slots.get(&page_id) {
            let page =
                self.pages
                    .get(page_index)
                    .ok_or_else(|| TexPackerError::InvariantViolation {
                        context: format!("runtime image page {page_id}"),
                        reason: format!("page index references missing slot {page_index}"),
                    })?;
            if page.image.dimensions() != page_size {
                return Err(TexPackerError::InvariantViolation {
                    context: format!("runtime image page {page_id}"),
                    reason: format!(
                        "pixel buffer dimensions {:?} do not match geometry page {:?}",
                        page.image.dimensions(),
                        page_size
                    ),
                });
            }

            let mut pixels = RgbaImage::from_pixel(
                dirty_region.width,
                dirty_region.height,
                self.background_color,
            );
            let content = placement.content();
            blit_staged(
                image,
                &mut pixels,
                Rect::new(
                    content.x - dirty_region.x,
                    content.y - dirty_region.y,
                    content.w,
                    content.h,
                ),
                placement.rotated(),
                extrusion,
                self.outlines,
            );
            Ok((
                PreparedPixels::Patch {
                    page_index,
                    region: dirty_region,
                    pixels,
                },
                dirty_region,
            ))
        } else {
            let mut pixels = RgbaImage::from_pixel(page_size.0, page_size.1, self.background_color);
            blit_staged(
                image,
                &mut pixels,
                placement.content(),
                placement.rotated(),
                extrusion,
                self.outlines,
            );
            Ok((
                PreparedPixels::New(RuntimeImagePage {
                    id: page_id,
                    image: pixels,
                }),
                dirty_region,
            ))
        }
    }

    fn commit_pixels(&mut self, prepared: PreparedPixels) {
        match prepared {
            PreparedPixels::Patch {
                page_index,
                region,
                pixels,
            } => {
                let page = &mut self.pages[page_index].image;
                for (x, y, pixel) in pixels.enumerate_pixels() {
                    page.put_pixel(region.x + x, region.y + y, *pixel);
                }
            }
            PreparedPixels::New(page) => {
                let page_id = page.id;
                let page_slot = self.pages.len();
                self.pages.push(page);
                let replaced = self.page_slots.insert(page_id, page_slot);
                debug_assert!(replaced.is_none(), "new pixel page identity must be unique");
            }
        }
    }

    fn commit_image_append(
        &mut self,
        prepared_append: PreparedAppend,
        prepared_pixels: PreparedPixels,
    ) -> RuntimePlacement {
        let placement = self.session.commit_append(prepared_append);
        self.commit_pixels(prepared_pixels);
        placement
    }

    fn clear_region(&mut self, region: UpdateRegion) {
        let background_color = self.background_color;
        let Some(page) = self.get_page_image_mut(region.page_id) else {
            return;
        };

        for y in region.y..region.y.saturating_add(region.height).min(page.height()) {
            for x in region.x..region.x.saturating_add(region.width).min(page.width()) {
                page.put_pixel(x, y, background_color);
            }
        }
    }
}

fn validate_blit(
    placement: &RuntimePlacement,
    image: &RgbaImage,
    page_size: (u32, u32),
    extrusion: u32,
) -> Result<UpdateRegion> {
    let page_id = placement.page_id();
    let content = placement.content();
    let source_size = image.dimensions();
    let expected_content_size = if placement.rotated() {
        (source_size.1, source_size.0)
    } else {
        source_size
    };
    if content.is_empty() || (content.w, content.h) != expected_content_size {
        return Err(blit_invariant(
            page_id,
            format!(
                "content dimensions {:?} do not match source {:?} with rotated={}",
                (content.w, content.h),
                source_size,
                placement.rotated()
            ),
        ));
    }

    let Some(start_x) = content.x.checked_sub(extrusion) else {
        return Err(blit_invariant(page_id, "extrusion crosses the left edge"));
    };
    let Some(start_y) = content.y.checked_sub(extrusion) else {
        return Err(blit_invariant(page_id, "extrusion crosses the top edge"));
    };
    let end_x = u64::from(content.x) + u64::from(content.w) + u64::from(extrusion);
    let end_y = u64::from(content.y) + u64::from(content.h) + u64::from(extrusion);
    if end_x > u64::from(page_size.0) || end_y > u64::from(page_size.1) {
        return Err(blit_invariant(
            page_id,
            format!(
                "destination including extrusion exceeds page dimensions {:?}",
                page_size
            ),
        ));
    }

    let region = UpdateRegion {
        page_id,
        x: start_x,
        y: start_y,
        width: u32::try_from(end_x - u64::from(start_x))
            .map_err(|_| blit_invariant(page_id, "dirty width exceeds u32"))?,
        height: u32::try_from(end_y - u64::from(start_y))
            .map_err(|_| blit_invariant(page_id, "dirty height exceeds u32"))?,
    };
    let dirty_rect = Rect::new(region.x, region.y, region.width, region.height);
    if !placement.allocation().contains(&dirty_rect) {
        return Err(blit_invariant(
            page_id,
            "dirty rectangle must lie inside the reserved allocation",
        ));
    }
    Ok(region)
}

fn blit_staged(
    image: &RgbaImage,
    pixels: &mut RgbaImage,
    content: Rect,
    rotated: bool,
    extrusion: u32,
    outlines: bool,
) {
    let (source_width, source_height) = image.dimensions();
    crate::compositing::blit_rgba(
        image,
        pixels,
        crate::compositing::BlitRect::new(content.x, content.y, content.w, content.h),
        crate::compositing::BlitRect::new(0, 0, source_width, source_height),
        crate::compositing::BlitOptions {
            rotated,
            extrude: extrusion,
            outlines,
        },
    );
}

fn blit_invariant(page_id: PageId, reason: impl Into<String>) -> TexPackerError {
    TexPackerError::InvariantViolation {
        context: format!("runtime image page {page_id}"),
        reason: reason.into(),
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
