use crate::config::{
    AutoMode, GuillotineChoice, GuillotineSplit, MaxRectsHeuristic, OfflineConfig, PackingStrategy,
    PageConfig, SkylineHeuristic,
};
use crate::error::{Result, TexPackerError};
use crate::geometry::{ContentSize, PhysicalPlacement, bottom_ex_u32, right_ex_u32};
use crate::model::{Atlas, Frame, Meta, Page, Rect};
use crate::packer::PlacementEngine;
use crate::packing_plan::PackingPlan;
use crate::preparation::{PreparedItem, prepare_images, prepare_layout, prepare_layout_items};
use image::{DynamicImage, RgbaImage};
use std::collections::HashSet;
use std::time::{Duration, Instant};
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
/// - When the strategy is `Auto`, a small portfolio is tried and the best result is chosen (pages first, then total area).
/// - The time budget can limit Auto evaluation; parallel execution is used when enabled and available.
pub fn pack_images(inputs: Vec<InputImage>, config: OfflineConfig) -> Result<PackOutput> {
    if inputs.is_empty() {
        return Err(TexPackerError::Empty);
    }

    let prepared = prepare_images(&inputs, &config);

    pack_prepared(&prepared, &config)
}

#[derive(Clone)]
struct PackedRegion {
    canonical_item_index: usize,
    alias_item_indices: Vec<usize>,
    content: Rect,
    allocation: Rect,
    rotated: bool,
}

impl PackedRegion {
    fn logical_frame<T>(&self, prepared: &[PreparedItem<T>], item_index: usize) -> Frame {
        let item = &prepared[item_index];
        Frame {
            key: item.key.clone(),
            frame: self.content,
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

#[derive(Debug, Clone, Copy)]
struct PageSizing {
    max_dimensions: (u32, u32),
    border_padding: u32,
    force_max_dimensions: bool,
    power_of_two: bool,
    square: bool,
}

impl PageSizing {
    fn new(config: &OfflineConfig) -> Self {
        let page = config.page_config();
        Self {
            max_dimensions: page.max_dimensions(),
            border_padding: page.border_padding(),
            force_max_dimensions: config.force_max_dimensions(),
            power_of_two: config.power_of_two(),
            square: config.square(),
        }
    }

    fn compute(self, regions: &[PackedRegion]) -> (u32, u32) {
        if self.force_max_dimensions {
            return self.max_dimensions;
        }

        let mut width = 0;
        let mut height = 0;
        for region in regions {
            width = width.max(right_ex_u32(&region.allocation).saturating_add(self.border_padding));
            height =
                height.max(bottom_ex_u32(&region.allocation).saturating_add(self.border_padding));
        }

        if self.power_of_two {
            width = checked_next_power_of_two(width, self.max_dimensions.0);
            height = checked_next_power_of_two(height, self.max_dimensions.1);
        }
        if self.square {
            let side = width.max(height);
            width = side;
            height = side;
        }

        (width, height)
    }
}

fn checked_next_power_of_two(value: u32, validated_maximum: u32) -> u32 {
    value
        .max(1)
        .checked_next_power_of_two()
        .unwrap_or(validated_maximum)
}

struct OfflinePipeline<'a> {
    config: &'a OfflineConfig,
    page_sizing: PageSizing,
}

#[derive(Debug, Clone, Copy)]
struct AutoPackingPolicy {
    mode: AutoMode,
    time_budget: Option<Duration>,
    parallel: bool,
    reference_time_threshold: Option<Duration>,
    reference_input_threshold: Option<usize>,
}

impl AutoPackingPolicy {
    fn from_strategy(strategy: &PackingStrategy) -> Option<Self> {
        match *strategy {
            PackingStrategy::Auto {
                mode,
                time_budget,
                parallel,
                reference_time_threshold,
                reference_input_threshold,
            } => Some(Self {
                mode,
                time_budget,
                parallel,
                reference_time_threshold,
                reference_input_threshold,
            }),
            _ => None,
        }
    }
}

impl<'a> OfflinePipeline<'a> {
    fn new(config: &'a OfflineConfig) -> Self {
        Self {
            config,
            page_sizing: PageSizing::new(config),
        }
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
        if let Some(policy) = AutoPackingPolicy::from_strategy(self.config.strategy()) {
            return pack_auto_pages(
                prepared,
                plan,
                self.config.page_config(),
                self.page_sizing,
                policy,
            );
        }

        self.pack_pages_for_strategy(prepared, plan, self.config.strategy())
    }

    fn pack_pages_for_strategy<T>(
        &self,
        prepared: &[PreparedItem<T>],
        plan: &PackingPlan,
        strategy: &PackingStrategy,
    ) -> Result<Vec<PackedPage>> {
        pack_pages_for_strategy(
            prepared,
            plan,
            self.config.page_config(),
            self.page_sizing,
            strategy,
        )
    }

    fn build_output(
        &self,
        prepared: &[PreparedItem<RgbaImage>],
        packed_pages: &[PackedPage],
    ) -> PackOutput {
        let pages = packed_pages
            .iter()
            .map(|packed_page| render_output_page(prepared, packed_page, self.config))
            .collect();
        let atlas = self.build_atlas(prepared, packed_pages);

        PackOutput { atlas, pages }
    }

    fn build_atlas<T>(&self, prepared: &[PreparedItem<T>], packed_pages: &[PackedPage]) -> Atlas {
        build_atlas(prepared, packed_pages, self.config)
    }
}

fn pack_prepared(
    prepared: &[PreparedItem<RgbaImage>],
    config: &OfflineConfig,
) -> Result<PackOutput> {
    OfflinePipeline::new(config).pack_images(prepared)
}

fn pack_pages_for_strategy<T>(
    prepared: &[PreparedItem<T>],
    plan: &PackingPlan,
    page_config: &PageConfig,
    page_sizing: PageSizing,
    strategy: &PackingStrategy,
) -> Result<Vec<PackedPage>> {
    let mut pages: Vec<PackedPage> = Vec::new();
    let mut remaining: Vec<usize> = (0..plan.len()).collect();
    let mut page_id = 0usize;

    while !remaining.is_empty() {
        let mut engine = create_engine(page_config, strategy)?;
        let mut regions: Vec<PackedRegion> = Vec::new();

        loop {
            let mut placed_any = false;
            let mut remove_set: HashSet<usize> = HashSet::new();
            for &group_index in &remaining {
                let group = plan.group(group_index);
                let item = &prepared[group.canonical_item_index()];
                if let Some(PhysicalPlacement {
                    content,
                    allocation,
                    rotated,
                }) = engine.try_place(ContentSize::new(item.rect.w, item.rect.h))
                {
                    regions.push(PackedRegion {
                        canonical_item_index: group.canonical_item_index(),
                        alias_item_indices: group.alias_item_indices().to_vec(),
                        content,
                        allocation,
                        rotated,
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

        let (page_w, page_h) = page_sizing.compute(&regions);

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

fn create_engine(page_config: &PageConfig, strategy: &PackingStrategy) -> Result<PlacementEngine> {
    PlacementEngine::from_strategy(page_config, strategy).ok_or_else(|| {
        TexPackerError::InvalidConfig(
            "Auto must resolve to a concrete packing strategy before placement".into(),
        )
    })
}

fn render_output_page(
    prepared: &[PreparedItem<RgbaImage>],
    packed_page: &PackedPage,
    config: &OfflineConfig,
) -> OutputPage {
    let page_config = config.page_config();
    let mut canvas = RgbaImage::new(packed_page.width, packed_page.height);
    for region in &packed_page.regions {
        let prep = &prepared[region.canonical_item_index];
        let dst = crate::compositing::BlitRect::new(
            region.content.x,
            region.content.y,
            region.content.w,
            region.content.h,
        );
        let src = crate::compositing::BlitRect::new(
            prep.source.x,
            prep.source.y,
            prep.source.w,
            prep.source.h,
        );
        let options = crate::compositing::BlitOptions {
            rotated: region.rotated,
            extrude: page_config.texture_extrusion(),
            outlines: config.outlines(),
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
    config: &OfflineConfig,
) -> Atlas {
    let page_config = config.page_config();
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
        power_of_two: config.power_of_two(),
        square: config.square(),
        max_dim: page_config.max_dimensions(),
        padding: (page_config.border_padding(), page_config.texture_padding()),
        extrude: page_config.texture_extrusion(),
        allow_rotation: page_config.allow_rotation(),
        trim_mode: if config.trim_enabled() {
            "trim"
        } else {
            "none"
        }
        .into(),
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
    page_config: &PageConfig,
    page_sizing: PageSizing,
    policy: AutoPackingPolicy,
) -> Result<Vec<PackedPage>> {
    let n_inputs = plan.len();
    let reference_time_threshold = policy
        .reference_time_threshold
        .unwrap_or(Duration::from_millis(200));
    let reference_input_threshold = policy.reference_input_threshold.unwrap_or(800);
    let budget_ms = policy.time_budget.map_or(0, |budget| budget.as_millis());
    let reference_threshold_ms = reference_time_threshold.as_millis();
    let enable_mr_ref = matches!(policy.mode, AutoMode::Quality)
        && (budget_ms >= reference_threshold_ms || n_inputs >= reference_input_threshold);
    let candidates = auto_candidates(policy.mode, enable_mr_ref);
    let start = Instant::now();

    #[cfg(feature = "parallel")]
    {
        if policy.parallel {
            let results: Vec<(Vec<PackedPage>, u64, u32)> = candidates
                .par_iter()
                .filter_map(|strategy| {
                    pack_pages_for_strategy(prepared, plan, page_config, page_sizing, strategy).ok()
                })
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

    #[cfg(not(feature = "parallel"))]
    let _ = policy.parallel;

    let mut best: Option<(Vec<PackedPage>, u64, u32)> = None;
    for strategy in &candidates {
        if budget_ms > 0 && start.elapsed().as_millis() > budget_ms {
            break;
        }
        if let Ok(packed_pages) =
            pack_pages_for_strategy(prepared, plan, page_config, page_sizing, strategy)
        {
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

fn auto_candidates(mode: AutoMode, enable_mr_ref: bool) -> Vec<PackingStrategy> {
    match mode {
        AutoMode::Fast => vec![
            PackingStrategy::Skyline {
                heuristic: SkylineHeuristic::BottomLeft,
                use_waste_map: false,
            },
            PackingStrategy::MaxRects {
                heuristic: MaxRectsHeuristic::BestAreaFit,
                reference: false,
            },
        ],
        AutoMode::Quality => vec![
            PackingStrategy::Skyline {
                heuristic: SkylineHeuristic::MinWaste,
                use_waste_map: false,
            },
            PackingStrategy::MaxRects {
                heuristic: MaxRectsHeuristic::BestAreaFit,
                reference: enable_mr_ref,
            },
            PackingStrategy::MaxRects {
                heuristic: MaxRectsHeuristic::BottomLeft,
                reference: enable_mr_ref,
            },
            PackingStrategy::MaxRects {
                heuristic: MaxRectsHeuristic::ContactPoint,
                reference: enable_mr_ref,
            },
            PackingStrategy::Guillotine {
                choice: GuillotineChoice::BestAreaFit,
                split: GuillotineSplit::SplitShorterLeftoverAxis,
            },
        ],
    }
}

// ---------------- Layout-only API ----------------

/// Packs sizes into pages without compositing pixel data.
/// Inputs are (key, width, height). Returns an Atlas with pages and frames; no RGBA pages.
pub fn pack_layout<K: Into<String>>(
    inputs: Vec<(K, u32, u32)>,
    config: OfflineConfig,
) -> Result<Atlas<String>> {
    if inputs.is_empty() {
        return Err(TexPackerError::Empty);
    }
    let prepared = prepare_layout(inputs, &config);

    OfflinePipeline::new(&config).pack_layout(&prepared)
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
    config: OfflineConfig,
) -> Result<Atlas<String>> {
    if items.is_empty() {
        return Err(TexPackerError::Empty);
    }
    let prepared = prepare_layout_items(items, &config);

    OfflinePipeline::new(&config).pack_layout(&prepared)
}
