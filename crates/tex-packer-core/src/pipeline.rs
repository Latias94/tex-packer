use crate::config::PackerConfig;
use crate::config::{AlgorithmFamily, AutoMode};
use crate::error::{Result, TexPackerError};
use crate::geometry::PackingContext;
use crate::model::{Atlas, Frame, Meta, Page, Rect};
use crate::packer::{
    Packer, guillotine::GuillotinePacker, maxrects::MaxRectsPacker, skyline::SkylinePacker,
};
use crate::packing_plan::PackingPlan;
use crate::preparation::{PreparedItem, prepare_images, prepare_layout, prepare_layout_items};
use image::{DynamicImage, RgbaImage};
use std::collections::HashSet;
use std::time::Instant;
use tracing::instrument;

pub use crate::preparation::compute_trim_rect;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// In-memory image to pack (key + decoded image).
pub struct InputImage {
    pub key: String,
    pub image: DynamicImage,
}

/// Output RGBA page and its logical page record.
pub struct OutputPage {
    pub page: Page,
    pub rgba: RgbaImage,
}

/// Output of a packing run: atlas metadata and RGBA pages.
pub struct PackOutput {
    pub atlas: Atlas,
    pub pages: Vec<OutputPage>,
}

impl PackOutput {
    /// Computes packing statistics for this output.
    /// This is a convenience method that delegates to `atlas.stats()`.
    pub fn stats(&self) -> crate::model::PackStats {
        self.atlas.stats()
    }
}

#[instrument(skip_all)]
/// Packs `inputs` into atlas pages using configuration `cfg` and returns metadata and RGBA pages.
///
/// Notes:
/// - Sorting is stable for deterministic results.
/// - When `family` is `Auto`, a small portfolio is tried and the best result is chosen (pages first, then total area).
/// - `time_budget_ms` can limit Auto evaluation time; `parallel` may evaluate in parallel when enabled.
pub fn pack_images(inputs: Vec<InputImage>, cfg: PackerConfig) -> Result<PackOutput> {
    // Validate configuration first
    cfg.validate()?;

    if inputs.is_empty() {
        return Err(TexPackerError::Empty);
    }

    let prepared = prepare_images(&inputs, &cfg);

    pack_prepared(&prepared, &cfg)
}

#[derive(Clone)]
struct PackedRegion {
    canonical_item_index: usize,
    alias_item_indices: Vec<usize>,
    frame: Rect,
    rotated: bool,
}

impl PackedRegion {
    fn logical_frame<T>(&self, prepared: &[PreparedItem<T>], item_index: usize) -> Frame {
        let item = &prepared[item_index];
        Frame {
            key: item.key.clone(),
            frame: self.frame,
            rotated: self.rotated,
            trimmed: item.trimmed,
            source: item.source,
            source_size: item.orig_size,
        }
    }
}

#[derive(Clone)]
struct PackedPage {
    id: usize,
    width: u32,
    height: u32,
    regions: Vec<PackedRegion>,
}

impl PackedPage {
    fn public_frames<T>(&self, prepared: &[PreparedItem<T>]) -> Vec<Frame> {
        let mut frames = self
            .regions
            .iter()
            .flat_map(|region| {
                std::iter::once(region.canonical_item_index)
                    .chain(region.alias_item_indices.iter().copied())
                    .map(|item_index| (item_index, region.logical_frame(prepared, item_index)))
            })
            .collect::<Vec<_>>();
        frames.sort_by_key(|(item_index, _)| *item_index);
        frames.into_iter().map(|(_, frame)| frame).collect()
    }

    fn to_page<T>(&self, prepared: &[PreparedItem<T>]) -> Page {
        Page {
            id: self.id,
            width: self.width,
            height: self.height,
            frames: self.public_frames(prepared),
        }
    }
}

struct OfflinePipeline<'a> {
    cfg: &'a PackerConfig,
}

impl<'a> OfflinePipeline<'a> {
    fn new(cfg: &'a PackerConfig) -> Self {
        Self { cfg }
    }

    fn pack_images(&self, prepared: &[PreparedItem<RgbaImage>]) -> Result<PackOutput> {
        let plan = PackingPlan::deduplicated(prepared);
        let packed_pages = self.pack_pages(prepared, &plan)?;
        Ok(self.build_output(prepared, &packed_pages))
    }

    fn pack_layout<T: Sync>(&self, prepared: &[PreparedItem<T>]) -> Result<Atlas> {
        let plan = PackingPlan::one_per_item(prepared.len());
        let packed_pages = self.pack_pages(prepared, &plan)?;
        Ok(self.build_atlas(prepared, &packed_pages))
    }

    fn pack_pages<T: Sync>(
        &self,
        prepared: &[PreparedItem<T>],
        plan: &PackingPlan,
    ) -> Result<Vec<PackedPage>> {
        if matches!(self.cfg.family, AlgorithmFamily::Auto) {
            return pack_auto_pages(prepared, plan, self.cfg.clone());
        }

        self.pack_pages_for_family(prepared, plan)
    }

    fn pack_pages_for_family<T>(
        &self,
        prepared: &[PreparedItem<T>],
        plan: &PackingPlan,
    ) -> Result<Vec<PackedPage>> {
        pack_pages_for_family(prepared, plan, self.cfg)
    }

    fn build_output(
        &self,
        prepared: &[PreparedItem<RgbaImage>],
        packed_pages: &[PackedPage],
    ) -> PackOutput {
        let pages = packed_pages
            .iter()
            .map(|packed_page| render_output_page(prepared, packed_page, self.cfg))
            .collect();
        let atlas = self.build_atlas(prepared, packed_pages);

        PackOutput { atlas, pages }
    }

    fn build_atlas<T>(&self, prepared: &[PreparedItem<T>], packed_pages: &[PackedPage]) -> Atlas {
        build_atlas(prepared, packed_pages, self.cfg)
    }
}

fn pack_prepared(prepared: &[PreparedItem<RgbaImage>], cfg: &PackerConfig) -> Result<PackOutput> {
    OfflinePipeline::new(cfg).pack_images(prepared)
}

fn pack_pages_for_family<T>(
    prepared: &[PreparedItem<T>],
    plan: &PackingPlan,
    cfg: &PackerConfig,
) -> Result<Vec<PackedPage>> {
    let mut pages: Vec<PackedPage> = Vec::new();
    let mut remaining: Vec<usize> = (0..plan.len()).collect();
    let mut page_id = 0usize;

    while !remaining.is_empty() {
        let mut packer = create_packer(cfg);
        let mut regions: Vec<PackedRegion> = Vec::new();

        loop {
            let mut placed_any = false;
            let mut remove_set: HashSet<usize> = HashSet::new();
            for &group_index in &remaining {
                let group = plan.group(group_index);
                let item = &prepared[group.canonical_item_index()];
                if !packer.can_pack(&item.rect) {
                    continue;
                }
                if let Some(frame) = packer.pack(item.key.clone(), &item.rect) {
                    regions.push(PackedRegion {
                        canonical_item_index: group.canonical_item_index(),
                        alias_item_indices: group.alias_item_indices().to_vec(),
                        frame: frame.frame,
                        rotated: frame.rotated,
                    });
                    remove_set.insert(group_index);
                    placed_any = true;
                }
            }
            if !placed_any {
                break;
            }
            // Retain only indices not placed
            if !remove_set.is_empty() {
                remaining.retain(|i| !remove_set.contains(i));
            }
        }

        if regions.is_empty() {
            let remaining_items = remaining
                .iter()
                .map(|&group_index| plan.group(group_index).logical_item_count())
                .sum::<usize>();
            let placed = prepared.len() - remaining_items;
            return Err(TexPackerError::OutOfSpaceGeneric {
                placed,
                total: prepared.len(),
            });
        }

        let physical_frames = regions
            .iter()
            .map(|region| region.logical_frame(prepared, region.canonical_item_index))
            .collect::<Vec<_>>();
        let (page_w, page_h) = compute_page_size(&physical_frames, cfg);

        pages.push(PackedPage {
            id: page_id,
            width: page_w,
            height: page_h,
            regions,
        });
        page_id += 1;
    }

    Ok(pages)
}

fn create_packer(cfg: &PackerConfig) -> Box<dyn Packer<String>> {
    match cfg.family {
        AlgorithmFamily::Skyline => Box::new(SkylinePacker::new(cfg.clone())),
        AlgorithmFamily::MaxRects => {
            Box::new(MaxRectsPacker::new(cfg.clone(), cfg.mr_heuristic.clone()))
        }
        AlgorithmFamily::Guillotine => Box::new(GuillotinePacker::new(
            cfg.clone(),
            cfg.g_choice.clone(),
            cfg.g_split.clone(),
        )),
        AlgorithmFamily::Auto => unreachable!(),
    }
}

fn render_output_page(
    prepared: &[PreparedItem<RgbaImage>],
    packed_page: &PackedPage,
    cfg: &PackerConfig,
) -> OutputPage {
    let mut canvas = RgbaImage::new(packed_page.width, packed_page.height);
    for region in &packed_page.regions {
        let prep = &prepared[region.canonical_item_index];
        let dst = crate::compositing::BlitRect::new(
            region.frame.x,
            region.frame.y,
            region.frame.w,
            region.frame.h,
        );
        let src = crate::compositing::BlitRect::new(
            prep.source.x,
            prep.source.y,
            prep.source.w,
            prep.source.h,
        );
        let options = crate::compositing::BlitOptions {
            rotated: region.rotated,
            extrude: cfg.texture_extrusion,
            outlines: cfg.texture_outlines,
        };
        crate::compositing::blit_rgba(&prep.payload, &mut canvas, dst, src, options);
    }

    OutputPage {
        page: packed_page.to_page(prepared),
        rgba: canvas,
    }
}

fn build_atlas<T>(
    prepared: &[PreparedItem<T>],
    packed_pages: &[PackedPage],
    cfg: &PackerConfig,
) -> Atlas {
    let atlas_pages = packed_pages
        .iter()
        .map(|page| page.to_page(prepared))
        .collect();
    let meta = Meta {
        schema_version: "1".into(),
        app: "tex-packer".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        format: "RGBA8888".into(),
        scale: 1.0,
        power_of_two: cfg.power_of_two,
        square: cfg.square,
        max_dim: (cfg.max_width, cfg.max_height),
        padding: (cfg.border_padding, cfg.texture_padding),
        extrude: cfg.texture_extrusion,
        allow_rotation: cfg.allow_rotation,
        trim_mode: if cfg.trim { "trim" } else { "none" }.into(),
        background_color: None,
    };

    Atlas {
        pages: atlas_pages,
        meta,
    }
}

fn total_packed_area(pages: &[PackedPage]) -> u64 {
    pages
        .iter()
        .map(|p| (p.width as u64) * (p.height as u64))
        .sum()
}

fn pack_auto_pages<T: Sync>(
    prepared: &[PreparedItem<T>],
    plan: &PackingPlan,
    base: PackerConfig,
) -> Result<Vec<PackedPage>> {
    let mut candidates: Vec<PackerConfig> = Vec::new();
    let n_inputs = plan.len();
    let budget_ms = base.time_budget_ms.unwrap_or(0);
    let thr_time = base.auto_mr_ref_time_ms_threshold.unwrap_or(200);
    let thr_inputs = base.auto_mr_ref_input_threshold.unwrap_or(800);
    let enable_mr_ref = matches!(base.auto_mode, AutoMode::Quality)
        && (budget_ms >= thr_time || n_inputs >= thr_inputs);
    match base.auto_mode {
        AutoMode::Fast => {
            let mut s_bl = base.clone();
            s_bl.family = AlgorithmFamily::Skyline;
            s_bl.skyline_heuristic = crate::config::SkylineHeuristic::BottomLeft;
            candidates.push(s_bl);
            let mut mr_baf = base.clone();
            mr_baf.family = AlgorithmFamily::MaxRects;
            mr_baf.mr_heuristic = crate::config::MaxRectsHeuristic::BestAreaFit;
            mr_baf.mr_reference = false;
            candidates.push(mr_baf);
        }
        AutoMode::Quality => {
            let mut s_mw = base.clone();
            s_mw.family = AlgorithmFamily::Skyline;
            s_mw.skyline_heuristic = crate::config::SkylineHeuristic::MinWaste;
            candidates.push(s_mw);
            let mut mr_baf = base.clone();
            mr_baf.family = AlgorithmFamily::MaxRects;
            mr_baf.mr_heuristic = crate::config::MaxRectsHeuristic::BestAreaFit;
            mr_baf.mr_reference = enable_mr_ref;
            candidates.push(mr_baf);
            let mut mr_bl = base.clone();
            mr_bl.family = AlgorithmFamily::MaxRects;
            mr_bl.mr_heuristic = crate::config::MaxRectsHeuristic::BottomLeft;
            mr_bl.mr_reference = enable_mr_ref;
            candidates.push(mr_bl);
            let mut mr_cp = base.clone();
            mr_cp.family = AlgorithmFamily::MaxRects;
            mr_cp.mr_heuristic = crate::config::MaxRectsHeuristic::ContactPoint;
            mr_cp.mr_reference = enable_mr_ref;
            candidates.push(mr_cp);
            let mut g = base.clone();
            g.family = AlgorithmFamily::Guillotine;
            g.g_choice = crate::config::GuillotineChoice::BestAreaFit;
            g.g_split = crate::config::GuillotineSplit::SplitShorterLeftoverAxis;
            candidates.push(g);
        }
    }
    let start = Instant::now();

    // Parallel path (optional)
    #[cfg(feature = "parallel")]
    {
        if base.parallel {
            let results: Vec<(Vec<PackedPage>, u64, u32)> = candidates
                .par_iter()
                .filter_map(|cand| pack_pages_for_family(prepared, plan, cand).ok())
                .map(|pages| {
                    let page_count = pages.len() as u32;
                    let total_area = total_packed_area(&pages);
                    (pages, total_area, page_count)
                })
                .collect();
            let best = results.into_iter().min_by(|a, b| match a.2.cmp(&b.2) {
                // pages asc
                std::cmp::Ordering::Equal => a.1.cmp(&b.1),
                other => other,
            });
            return best.map(|x| x.0).ok_or(TexPackerError::OutOfSpaceGeneric {
                placed: 0,
                total: prepared.len(),
            });
        }
    }

    // Sequential path with optional time budget
    let mut best: Option<(Vec<PackedPage>, u64, u32)> = None; // (pages, total_area, page count)
    for cand in candidates.into_iter() {
        if budget_ms > 0 && start.elapsed().as_millis() as u64 > budget_ms {
            break;
        }
        if let Ok(packed_pages) = pack_pages_for_family(prepared, plan, &cand) {
            let pages = packed_pages.len() as u32;
            let total_area = total_packed_area(&packed_pages);
            match &mut best {
                None => best = Some((packed_pages, total_area, pages)),
                Some((bo, barea, bpages)) => {
                    if pages < *bpages || (pages == *bpages && total_area < *barea) {
                        *bo = packed_pages;
                        *barea = total_area;
                        *bpages = pages;
                    }
                }
            }
        }
    }
    best.map(|x| x.0).ok_or(TexPackerError::OutOfSpaceGeneric {
        placed: 0,
        total: prepared.len(),
    })
}

// ---------------- Layout-only API ----------------

/// Packs sizes into pages without compositing pixel data.
/// Inputs are (key, width, height). Returns an Atlas with pages and frames; no RGBA pages.
pub fn pack_layout<K: Into<String>>(
    inputs: Vec<(K, u32, u32)>,
    cfg: PackerConfig,
) -> Result<Atlas<String>> {
    // Validate configuration first
    cfg.validate()?;

    if inputs.is_empty() {
        return Err(TexPackerError::Empty);
    }
    let prepared = prepare_layout(inputs, &cfg);

    OfflinePipeline::new(&cfg).pack_layout(&prepared)
}

/// Layout-only item with optional source/source_size to propagate trimming metadata.
#[derive(Debug, Clone)]
pub struct LayoutItem<K = String> {
    pub key: K,
    pub w: u32,
    pub h: u32,
    pub source: Option<Rect>,
    pub source_size: Option<(u32, u32)>,
    pub trimmed: bool,
}

/// Packs layout-only items (with optional source/source_size metadata) into pages.
pub fn pack_layout_items<K: Into<String>>(
    items: Vec<LayoutItem<K>>,
    cfg: PackerConfig,
) -> Result<Atlas<String>> {
    // Validate configuration first
    cfg.validate()?;

    if items.is_empty() {
        return Err(TexPackerError::Empty);
    }
    let prepared = prepare_layout_items(items, &cfg);

    OfflinePipeline::new(&cfg).pack_layout(&prepared)
}

/// Compute final page dimensions given placed frames and config.
fn compute_page_size(frames: &[Frame], cfg: &PackerConfig) -> (u32, u32) {
    PackingContext::new(cfg).compute_page_size(frames)
}
