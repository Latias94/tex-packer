use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::config::PageConfig;
use crate::error::{Result, TexPackerError};

/// Axis-aligned pixel rectangle. `(x, y)` is the top-left corner and `(w, h)` is its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub const fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    /// Inclusive right edge coordinate.
    pub const fn right(self) -> u32 {
        self.x.saturating_add(self.w.saturating_sub(1))
    }

    /// Inclusive bottom edge coordinate.
    pub const fn bottom(self) -> u32 {
        self.y.saturating_add(self.h.saturating_sub(1))
    }

    pub const fn is_empty(self) -> bool {
        self.w == 0 || self.h == 0
    }

    pub const fn area(self) -> u128 {
        self.w as u128 * self.h as u128
    }

    /// Returns whether `inner` is a positive-area rectangle fully inside this rectangle.
    pub fn contains(self, inner: &Rect) -> bool {
        !self.is_empty()
            && !inner.is_empty()
            && inner.x >= self.x
            && inner.y >= self.y
            && inner.right_exclusive() <= self.right_exclusive()
            && inner.bottom_exclusive() <= self.bottom_exclusive()
    }

    /// Returns whether two positive-area rectangles overlap. Touching edges do not overlap.
    pub fn intersects(self, other: &Rect) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && (self.x as u64) < other.right_exclusive()
            && (other.x as u64) < self.right_exclusive()
            && (self.y as u64) < other.bottom_exclusive()
            && (other.y as u64) < self.bottom_exclusive()
    }

    const fn right_exclusive(self) -> u64 {
        self.x as u64 + self.w as u64
    }

    const fn bottom_exclusive(self) -> u64 {
        self.y as u64 + self.h as u64
    }
}

macro_rules! identity_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u32);

        impl $name {
            pub const fn new(value: u32) -> Self {
                Self(value)
            }

            pub const fn get(self) -> u32 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

identity_type!(
    /// Stable atlas page identity. It is not a collection index.
    PageId
);
identity_type!(
    /// Stable physical-region identity scoped to one page.
    RegionId
);
identity_type!(
    /// Stable logical-frame identity scoped to one page.
    FrameId
);

/// One physical allocation on an atlas page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    id: RegionId,
    content: Rect,
    allocation: Rect,
    rotated: bool,
}

impl Region {
    pub const fn new(id: RegionId, content: Rect, allocation: Rect, rotated: bool) -> Self {
        Self {
            id,
            content,
            allocation,
            rotated,
        }
    }

    pub const fn id(&self) -> RegionId {
        self.id
    }

    pub const fn content(&self) -> Rect {
        self.content
    }

    pub const fn allocation(&self) -> Rect {
        self.allocation
    }

    pub const fn rotated(&self) -> bool {
        self.rotated
    }
}

/// One logical sprite that resolves to a physical region on the same page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    id: FrameId,
    key: String,
    region_id: RegionId,
    trimmed: bool,
    source: Rect,
    source_size: (u32, u32),
}

impl Frame {
    pub fn new(
        id: FrameId,
        key: String,
        region_id: RegionId,
        trimmed: bool,
        source: Rect,
        source_size: (u32, u32),
    ) -> Self {
        Self {
            id,
            key,
            region_id,
            trimmed,
            source,
            source_size,
        }
    }

    pub const fn id(&self) -> FrameId {
        self.id
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub const fn region_id(&self) -> RegionId {
        self.region_id
    }

    pub const fn trimmed(&self) -> bool {
        self.trimmed
    }

    pub const fn source(&self) -> Rect {
        self.source
    }

    pub const fn source_size(&self) -> (u32, u32) {
        self.source_size
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PageIndex {
    region_by_id: HashMap<RegionId, usize>,
    frame_by_id: HashMap<FrameId, usize>,
    frame_region_slots: Vec<usize>,
}

/// Validated page aggregate containing physical regions and logical frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    id: PageId,
    width: u32,
    height: u32,
    regions: Vec<Region>,
    frames: Vec<Frame>,
    index: PageIndex,
}

impl Page {
    pub fn try_new(
        id: PageId,
        width: u32,
        height: u32,
        regions: Vec<Region>,
        frames: Vec<Frame>,
    ) -> Result<Self> {
        let page_context = format!("page {id}");
        if width == 0 || height == 0 {
            return Err(invariant(
                page_context,
                format!("page dimensions must be positive, got {width}x{height}"),
            ));
        }

        let page_bounds = Rect::new(0, 0, width, height);
        let mut region_by_id = HashMap::with_capacity(regions.len());
        for (slot, region) in regions.iter().enumerate() {
            let context = region_context(id, region.id);
            if region_by_id.insert(region.id, slot).is_some() {
                return Err(invariant(context, "duplicate region identity"));
            }
            if region.content.is_empty() {
                return Err(invariant(context, "content rectangle must be non-empty"));
            }
            if region.allocation.is_empty() {
                return Err(invariant(context, "allocation rectangle must be non-empty"));
            }
            if !region.allocation.contains(&region.content) {
                return Err(invariant(
                    context,
                    format!(
                        "content rectangle {:?} must lie inside allocation {:?}",
                        region.content, region.allocation
                    ),
                ));
            }
            if !page_bounds.contains(&region.allocation) {
                return Err(invariant(
                    context,
                    format!(
                        "allocation rectangle {:?} must lie inside page bounds {width}x{height}",
                        region.allocation
                    ),
                ));
            }
        }

        for first in 0..regions.len() {
            for second in (first + 1)..regions.len() {
                if regions[first]
                    .allocation
                    .intersects(&regions[second].allocation)
                {
                    return Err(invariant(
                        page_context.clone(),
                        format!(
                            "allocations for regions {} and {} overlap",
                            regions[first].id, regions[second].id
                        ),
                    ));
                }
            }
        }

        let mut frame_by_id = HashMap::with_capacity(frames.len());
        let mut frame_region_slots = Vec::with_capacity(frames.len());
        let mut region_references = vec![0usize; regions.len()];
        for (slot, frame) in frames.iter().enumerate() {
            let context = frame_context(id, frame);
            if frame_by_id.insert(frame.id, slot).is_some() {
                return Err(invariant(context, "duplicate frame identity"));
            }

            let Some(&region_slot) = region_by_id.get(&frame.region_id) else {
                return Err(invariant(
                    context,
                    format!("references missing region {}", frame.region_id),
                ));
            };
            let region = &regions[region_slot];

            if frame.source.is_empty() {
                return Err(invariant(context, "source rectangle must be non-empty"));
            }
            let source_bounds = Rect::new(0, 0, frame.source_size.0, frame.source_size.1);
            if !source_bounds.contains(&frame.source) {
                return Err(invariant(
                    context,
                    format!(
                        "source rectangle {:?} must lie inside source size {}x{}",
                        frame.source, frame.source_size.0, frame.source_size.1
                    ),
                ));
            }

            let expected_content_size = if region.rotated {
                (frame.source.h, frame.source.w)
            } else {
                (frame.source.w, frame.source.h)
            };
            if (region.content.w, region.content.h) != expected_content_size {
                return Err(invariant(
                    context,
                    format!(
                        "source size {}x{} does not match region content {}x{} with rotated={}",
                        frame.source.w,
                        frame.source.h,
                        region.content.w,
                        region.content.h,
                        region.rotated
                    ),
                ));
            }

            frame_region_slots.push(region_slot);
            region_references[region_slot] += 1;
        }

        for (region, references) in regions.iter().zip(region_references) {
            if references == 0 {
                return Err(invariant(
                    region_context(id, region.id),
                    "physical region is not referenced by any frame",
                ));
            }
        }

        Ok(Self {
            id,
            width,
            height,
            regions,
            frames,
            index: PageIndex {
                region_by_id,
                frame_by_id,
                frame_region_slots,
            },
        })
    }

    pub const fn id(&self) -> PageId {
        self.id
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub const fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn regions(&self) -> &[Region] {
        &self.regions
    }

    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    pub fn region(&self, id: RegionId) -> Option<&Region> {
        self.index
            .region_by_id
            .get(&id)
            .and_then(|&slot| self.regions.get(slot))
    }

    pub fn frame(&self, id: FrameId) -> Option<&Frame> {
        self.index
            .frame_by_id
            .get(&id)
            .and_then(|&slot| self.frames.get(slot))
    }

    pub fn resolved_frames(
        &self,
    ) -> impl ExactSizeIterator<Item = ResolvedFrame<'_>> + DoubleEndedIterator + '_ {
        self.frames
            .iter()
            .zip(self.index.frame_region_slots.iter().copied())
            .map(move |(frame, region_slot)| ResolvedFrame {
                page_id: self.id,
                page_size: (self.width, self.height),
                frame,
                region: &self.regions[region_slot],
            })
    }
}

/// Borrowed logical frame together with its authoritative physical region.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedFrame<'a> {
    page_id: PageId,
    page_size: (u32, u32),
    frame: &'a Frame,
    region: &'a Region,
}

impl<'a> ResolvedFrame<'a> {
    pub const fn page_id(self) -> PageId {
        self.page_id
    }

    pub const fn page_size(self) -> (u32, u32) {
        self.page_size
    }

    pub const fn frame(self) -> &'a Frame {
        self.frame
    }

    pub const fn region(self) -> &'a Region {
        self.region
    }
}

/// Descriptive metadata for an atlas run. Native schema versioning belongs to [`AtlasDocument`].
#[derive(Debug, Clone, PartialEq)]
pub struct Meta {
    app: String,
    version: String,
    format: String,
    scale: f32,
    power_of_two: bool,
    square: bool,
    max_dimensions: (u32, u32),
    padding: (u32, u32),
    extrude: u32,
    allow_rotation: bool,
    trim_mode: String,
    background_color: Option<[u8; 4]>,
}

impl Meta {
    pub fn app(&self) -> &str {
        &self.app
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn format(&self) -> &str {
        &self.format
    }

    pub const fn scale(&self) -> f32 {
        self.scale
    }

    pub const fn power_of_two(&self) -> bool {
        self.power_of_two
    }

    pub const fn square(&self) -> bool {
        self.square
    }

    pub const fn max_dimensions(&self) -> (u32, u32) {
        self.max_dimensions
    }

    /// Returns `(border_padding, texture_padding)`.
    pub const fn padding(&self) -> (u32, u32) {
        self.padding
    }

    pub const fn extrude(&self) -> u32 {
        self.extrude
    }

    pub const fn allow_rotation(&self) -> bool {
        self.allow_rotation
    }

    pub fn trim_mode(&self) -> &str {
        &self.trim_mode
    }

    pub const fn background_color(&self) -> Option<[u8; 4]> {
        self.background_color
    }

    pub(crate) fn for_run(
        page_config: &PageConfig,
        power_of_two: bool,
        square: bool,
        trim_mode: impl Into<String>,
    ) -> Self {
        Self {
            power_of_two,
            square,
            max_dimensions: page_config.max_dimensions(),
            padding: (page_config.border_padding(), page_config.texture_padding()),
            extrude: page_config.texture_extrusion(),
            allow_rotation: page_config.allow_rotation(),
            trim_mode: trim_mode.into(),
            ..Self::default()
        }
    }

    fn validate(&self) -> Result<()> {
        for (name, value) in [
            ("app", self.app.as_str()),
            ("version", self.version.as_str()),
            ("format", self.format.as_str()),
            ("trim_mode", self.trim_mode.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(invariant(
                    "atlas metadata",
                    format!("{name} must not be empty"),
                ));
            }
        }
        if !self.scale.is_finite() || self.scale <= 0.0 {
            return Err(invariant(
                "atlas metadata",
                format!("scale must be finite and positive, got {}", self.scale),
            ));
        }
        if self.max_dimensions.0 == 0 || self.max_dimensions.1 == 0 {
            return Err(invariant(
                "atlas metadata",
                format!(
                    "maximum dimensions must be positive, got {}x{}",
                    self.max_dimensions.0, self.max_dimensions.1
                ),
            ));
        }

        let border = self.padding.0;
        let texture_padding = self.padding.1;
        let doubled_border = border.checked_mul(2).ok_or_else(|| {
            invariant(
                "atlas metadata",
                format!("border padding {border} overflows page geometry"),
            )
        })?;
        let usable_width = self.max_dimensions.0.checked_sub(doubled_border);
        let usable_height = self.max_dimensions.1.checked_sub(doubled_border);
        let (Some(usable_width), Some(usable_height)) = (usable_width, usable_height) else {
            return Err(invariant(
                "atlas metadata",
                format!(
                    "border padding {border} exceeds maximum dimensions {}x{}",
                    self.max_dimensions.0, self.max_dimensions.1
                ),
            ));
        };
        if usable_width == 0 || usable_height == 0 {
            return Err(invariant(
                "atlas metadata",
                "border padding leaves no usable page area",
            ));
        }

        let allocation_extra = self
            .extrude
            .checked_mul(2)
            .and_then(|extrusion| extrusion.checked_add(texture_padding))
            .ok_or_else(|| {
                invariant(
                    "atlas metadata",
                    format!(
                        "texture padding {texture_padding} and extrusion {} overflow allocation geometry",
                        self.extrude
                    ),
                )
            })?;
        let minimum_reservation = allocation_extra.checked_add(1).ok_or_else(|| {
            invariant(
                "atlas metadata",
                "minimum texture reservation overflows allocation geometry",
            )
        })?;
        if minimum_reservation > usable_width || minimum_reservation > usable_height {
            return Err(invariant(
                "atlas metadata",
                format!(
                    "padding and extrusion require at least {minimum_reservation}x{minimum_reservation} usable pixels, got {usable_width}x{usable_height}"
                ),
            ));
        }
        Ok(())
    }
}

impl Default for Meta {
    fn default() -> Self {
        Self {
            app: "tex-packer".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            format: "RGBA8888".into(),
            scale: 1.0,
            power_of_two: false,
            square: false,
            max_dimensions: (1024, 1024),
            padding: (0, 2),
            extrude: 0,
            allow_rotation: true,
            trim_mode: "none".into(),
            background_color: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AtlasIndex {
    page_by_id: HashMap<PageId, usize>,
}

/// Validated texture-atlas aggregate.
#[derive(Debug, Clone, PartialEq)]
pub struct Atlas {
    pages: Vec<Page>,
    meta: Meta,
    index: AtlasIndex,
}

impl Atlas {
    pub fn try_new(pages: Vec<Page>, meta: Meta) -> Result<Self> {
        meta.validate()?;
        let mut page_by_id = HashMap::with_capacity(pages.len());
        for (slot, page) in pages.iter().enumerate() {
            if page_by_id.insert(page.id, slot).is_some() {
                return Err(invariant(
                    format!("page {}", page.id),
                    "duplicate page identity",
                ));
            }
        }

        Ok(Self {
            pages,
            meta,
            index: AtlasIndex { page_by_id },
        })
    }

    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    pub fn page(&self, id: PageId) -> Option<&Page> {
        self.index
            .page_by_id
            .get(&id)
            .and_then(|&slot| self.pages.get(slot))
    }

    pub const fn meta(&self) -> &Meta {
        &self.meta
    }

    pub fn stats(&self) -> PackStats {
        let mut num_frames = 0usize;
        let mut num_regions = 0usize;
        let mut num_aliases = 0usize;
        let mut num_rotated_regions = 0usize;
        let mut num_trimmed_frames = 0usize;
        let mut page_area = 0u128;
        let mut content_area = 0u128;
        let mut allocation_area = 0u128;

        for page in &self.pages {
            num_frames += page.frames.len();
            num_regions += page.regions.len();
            num_aliases += page.frames.len().saturating_sub(page.regions.len());
            page_area += u128::from(page.width) * u128::from(page.height);

            for region in &page.regions {
                content_area += region.content.area();
                allocation_area += region.allocation.area();
                num_rotated_regions += usize::from(region.rotated);
            }
            for frame in &page.frames {
                num_trimmed_frames += usize::from(frame.trimmed);
            }
        }

        PackStats {
            num_pages: self.pages.len(),
            num_frames,
            num_regions,
            num_aliases,
            num_rotated_regions,
            num_trimmed_frames,
            page_area,
            content_area,
            allocation_area,
            content_occupancy: occupancy(content_area, page_area),
            allocation_occupancy: occupancy(allocation_area, page_area),
        }
    }
}

/// Statistics derived from unique physical regions and logical frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PackStats {
    pub num_pages: usize,
    pub num_frames: usize,
    pub num_regions: usize,
    pub num_aliases: usize,
    pub num_rotated_regions: usize,
    pub num_trimmed_frames: usize,
    pub page_area: u128,
    pub content_area: u128,
    pub allocation_area: u128,
    pub content_occupancy: f64,
    pub allocation_occupancy: f64,
}

impl PackStats {
    pub fn summary(self) -> String {
        format!(
            "Pages: {}, Frames: {}, Regions: {}, Aliases: {}, Content occupancy: {:.2}%, Allocation occupancy: {:.2}%, Page area: {}, Content area: {}, Allocation area: {}, Rotated regions: {}, Trimmed frames: {}",
            self.num_pages,
            self.num_frames,
            self.num_regions,
            self.num_aliases,
            self.content_occupancy * 100.0,
            self.allocation_occupancy * 100.0,
            self.page_area,
            self.content_area,
            self.allocation_area,
            self.num_rotated_regions,
            self.num_trimmed_frames,
        )
    }

    pub const fn unallocated_area(self) -> u128 {
        self.page_area.saturating_sub(self.allocation_area)
    }

    pub fn allocation_waste_percentage(self) -> f64 {
        if self.page_area == 0 {
            0.0
        } else {
            self.unallocated_area() as f64 / self.page_area as f64 * 100.0
        }
    }
}

fn occupancy(area: u128, page_area: u128) -> f64 {
    if page_area == 0 {
        0.0
    } else {
        area as f64 / page_area as f64
    }
}

/// Reversible, versioned persistence representation of an [`Atlas`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AtlasDocument {
    schema_version: u32,
    meta: MetaDocument,
    pages: Vec<PageDocument>,
}

impl AtlasDocument {
    pub const SCHEMA_VERSION: u32 = 2;

    pub fn from_atlas(atlas: &Atlas) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            meta: MetaDocument::from(&atlas.meta),
            pages: atlas.pages.iter().map(PageDocument::from).collect(),
        }
    }

    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn try_into_atlas(self) -> Result<Atlas> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(TexPackerError::InvalidDocument {
                reason: format!(
                    "unsupported schema version {}; expected {}",
                    self.schema_version,
                    Self::SCHEMA_VERSION
                ),
            });
        }

        let pages = self
            .pages
            .into_iter()
            .map(PageDocument::try_into_page)
            .collect::<Result<Vec<_>>>()?;
        Atlas::try_new(pages, self.meta.into()).map_err(as_document_error)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct MetaDocument {
    app: String,
    version: String,
    format: String,
    scale: f32,
    power_of_two: bool,
    square: bool,
    max_dimensions: (u32, u32),
    padding: (u32, u32),
    extrude: u32,
    allow_rotation: bool,
    trim_mode: String,
    background_color: Option<[u8; 4]>,
}

impl From<&Meta> for MetaDocument {
    fn from(meta: &Meta) -> Self {
        Self {
            app: meta.app.clone(),
            version: meta.version.clone(),
            format: meta.format.clone(),
            scale: meta.scale,
            power_of_two: meta.power_of_two,
            square: meta.square,
            max_dimensions: meta.max_dimensions,
            padding: meta.padding,
            extrude: meta.extrude,
            allow_rotation: meta.allow_rotation,
            trim_mode: meta.trim_mode.clone(),
            background_color: meta.background_color,
        }
    }
}

impl From<MetaDocument> for Meta {
    fn from(document: MetaDocument) -> Self {
        Self {
            app: document.app,
            version: document.version,
            format: document.format,
            scale: document.scale,
            power_of_two: document.power_of_two,
            square: document.square,
            max_dimensions: document.max_dimensions,
            padding: document.padding,
            extrude: document.extrude,
            allow_rotation: document.allow_rotation,
            trim_mode: document.trim_mode,
            background_color: document.background_color,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PageDocument {
    id: u32,
    width: u32,
    height: u32,
    regions: Vec<RegionDocument>,
    frames: Vec<FrameDocument>,
}

impl PageDocument {
    fn try_into_page(self) -> Result<Page> {
        Page::try_new(
            PageId::new(self.id),
            self.width,
            self.height,
            self.regions
                .into_iter()
                .map(RegionDocument::into_region)
                .collect(),
            self.frames
                .into_iter()
                .map(FrameDocument::into_frame)
                .collect(),
        )
        .map_err(as_document_error)
    }
}

impl From<&Page> for PageDocument {
    fn from(page: &Page) -> Self {
        Self {
            id: page.id.get(),
            width: page.width,
            height: page.height,
            regions: page.regions.iter().map(RegionDocument::from).collect(),
            frames: page.frames.iter().map(FrameDocument::from).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RegionDocument {
    id: u32,
    content: RectDocument,
    allocation: RectDocument,
    rotated: bool,
}

impl RegionDocument {
    fn into_region(self) -> Region {
        Region::new(
            RegionId::new(self.id),
            self.content.into(),
            self.allocation.into(),
            self.rotated,
        )
    }
}

impl From<&Region> for RegionDocument {
    fn from(region: &Region) -> Self {
        Self {
            id: region.id.get(),
            content: region.content.into(),
            allocation: region.allocation.into(),
            rotated: region.rotated,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct FrameDocument {
    id: u32,
    key: String,
    region_id: u32,
    trimmed: bool,
    source: RectDocument,
    source_size: (u32, u32),
}

impl FrameDocument {
    fn into_frame(self) -> Frame {
        Frame::new(
            FrameId::new(self.id),
            self.key,
            RegionId::new(self.region_id),
            self.trimmed,
            self.source.into(),
            self.source_size,
        )
    }
}

impl From<&Frame> for FrameDocument {
    fn from(frame: &Frame) -> Self {
        Self {
            id: frame.id.get(),
            key: frame.key.clone(),
            region_id: frame.region_id.get(),
            trimmed: frame.trimmed,
            source: frame.source.into(),
            source_size: frame.source_size,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RectDocument {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl From<Rect> for RectDocument {
    fn from(rect: Rect) -> Self {
        Self {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
        }
    }
}

impl From<RectDocument> for Rect {
    fn from(document: RectDocument) -> Self {
        Self::new(document.x, document.y, document.w, document.h)
    }
}

fn invariant(context: impl Into<String>, reason: impl Into<String>) -> TexPackerError {
    TexPackerError::InvariantViolation {
        context: context.into(),
        reason: reason.into(),
    }
}

fn region_context(page_id: PageId, region_id: RegionId) -> String {
    format!("page {page_id}, region {region_id}")
}

fn frame_context(page_id: PageId, frame: &Frame) -> String {
    format!("page {page_id}, frame {} (key {:?})", frame.id, frame.key)
}

fn as_document_error(error: TexPackerError) -> TexPackerError {
    TexPackerError::InvalidDocument {
        reason: error.to_string(),
    }
}
