use crate::config::{
    AutoMode, GuillotineChoice, GuillotineSplit, MaxRectsHeuristic, OfflineConfig, PackingStrategy,
    PageConfig, SkylineHeuristic,
};
use crate::error::{Result, TexPackerError};
use crate::geometry::{ContentSize, PhysicalPlacement, bottom_ex_u32, right_ex_u32};
use crate::model::{Atlas, Frame, FrameId, Meta, Page, PageId, Rect, Region, RegionId};
use crate::offline::{InputImage, LayoutItem, PackOutput, RenderedPage};
use crate::packer::PlacementEngine;
use crate::packing_plan::PackingPlan;
use crate::preparation::{PreparedItem, prepare_images, prepare_layout_items};
use image::RgbaImage;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tracing::instrument;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

#[instrument(skip_all)]
pub(crate) fn pack_images_impl(
    inputs: Vec<InputImage>,
    config: &OfflineConfig,
) -> Result<PackOutput> {
    if inputs.is_empty() {
        return Err(TexPackerError::Empty);
    }

    let input_keys = input_keys(&inputs);
    let prepared = prepare_images(inputs, config)?;
    reject_empty_prepared(&prepared, input_keys)?;

    OfflinePipeline::new(config).pack_images(&prepared)
}

pub(crate) fn layout_images_impl(inputs: Vec<InputImage>, config: &OfflineConfig) -> Result<Atlas> {
    if inputs.is_empty() {
        return Err(TexPackerError::Empty);
    }

    let input_keys = input_keys(&inputs);
    let prepared = prepare_images(inputs, config)?;
    reject_empty_prepared(&prepared, input_keys)?;

    OfflinePipeline::new(config).layout_images(&prepared)
}

fn input_keys(inputs: &[InputImage]) -> Vec<String> {
    inputs.iter().map(|input| input.key.clone()).collect()
}

fn reject_empty_prepared<T>(prepared: &[PreparedItem<T>], keys: Vec<String>) -> Result<()> {
    if prepared.is_empty() {
        return Err(TexPackerError::NoPackableInputs { keys });
    }
    Ok(())
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
    fn to_region(&self, id: RegionId) -> Region {
        Region::new(id, self.content, self.allocation, self.rotated)
    }

    fn logical_frame<T>(
        &self,
        prepared: &[PreparedItem<T>],
        item_index: usize,
        frame_id: FrameId,
        region_id: RegionId,
    ) -> Frame {
        let item = &prepared[item_index];
        Frame::new(
            frame_id,
            item.key.clone(),
            region_id,
            item.trimmed,
            item.source,
            item.orig_size,
        )
    }
}

#[derive(Clone)]
struct PackedPage {
    id: PageId,
    width: u32,
    height: u32,
    regions: Vec<PackedRegion>,
}

impl PackedPage {
    fn to_page<T>(&self, prepared: &[PreparedItem<T>]) -> Result<Page> {
        let regions = self
            .regions
            .iter()
            .enumerate()
            .map(|(region_index, packed_region)| {
                let region_id = checked_region_id(self.id, region_index)?;
                Ok(packed_region.to_region(region_id))
            })
            .collect::<Result<Vec<_>>>()?;

        let mut logical_items = self
            .regions
            .iter()
            .enumerate()
            .flat_map(|(region_index, region)| {
                std::iter::once(region.canonical_item_index)
                    .chain(region.alias_item_indices.iter().copied())
                    .map(move |item_index| (item_index, region_index))
            })
            .collect::<Vec<_>>();
        logical_items.sort_by_key(|(item_index, _)| *item_index);

        let frames = logical_items
            .into_iter()
            .enumerate()
            .map(|(frame_index, (item_index, region_index))| {
                let frame_id = checked_frame_id(self.id, frame_index)?;
                let region_id = checked_region_id(self.id, region_index)?;
                Ok(self.regions[region_index]
                    .logical_frame(prepared, item_index, frame_id, region_id))
            })
            .collect::<Result<Vec<_>>>()?;

        Page::try_new(self.id, self.width, self.height, regions, frames)
    }
}

fn checked_page_id(page_index: usize) -> Result<PageId> {
    u32::try_from(page_index)
        .map(PageId::new)
        .map_err(|_| identity_overflow("page", page_index, None))
}

fn checked_region_id(page_id: PageId, region_index: usize) -> Result<RegionId> {
    u32::try_from(region_index)
        .map(RegionId::new)
        .map_err(|_| identity_overflow("region", region_index, Some(page_id)))
}

fn checked_frame_id(page_id: PageId, frame_index: usize) -> Result<FrameId> {
    u32::try_from(frame_index)
        .map(FrameId::new)
        .map_err(|_| identity_overflow("frame", frame_index, Some(page_id)))
}

fn identity_overflow(kind: &str, index: usize, page_id: Option<PageId>) -> TexPackerError {
    let context = page_id.map_or_else(|| "atlas".to_string(), |id| format!("page {id}"));
    TexPackerError::InvariantViolation {
        context,
        reason: format!("{kind} index {index} exceeds the u32 identity range"),
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
        let (atlas, packed_pages) = self.pack_decoded_images(prepared)?;
        self.render_output(prepared, atlas, &packed_pages)
    }

    fn layout_images(&self, prepared: &[PreparedItem<RgbaImage>]) -> Result<Atlas> {
        self.pack_decoded_images(prepared).map(|(atlas, _)| atlas)
    }

    fn pack_decoded_images(
        &self,
        prepared: &[PreparedItem<RgbaImage>],
    ) -> Result<(Atlas, Vec<PackedPage>)> {
        let plan = PackingPlan::deduplicated(prepared);
        let packed_pages = self.pack_pages(prepared, &plan)?;
        let atlas = self.build_atlas(prepared, &packed_pages)?;
        Ok((atlas, packed_pages))
    }

    fn pack_layout<T: Sync>(&self, prepared: &[PreparedItem<T>]) -> Result<Atlas> {
        let plan = PackingPlan::one_per_item(prepared.len());
        let packed_pages = self.pack_pages(prepared, &plan)?;
        self.build_atlas(prepared, &packed_pages)
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

    fn render_output(
        &self,
        prepared: &[PreparedItem<RgbaImage>],
        atlas: Atlas,
        packed_pages: &[PackedPage],
    ) -> Result<PackOutput> {
        let pages = packed_pages
            .iter()
            .map(|packed_page| {
                let page = atlas.page(packed_page.id).ok_or_else(|| {
                    TexPackerError::InvariantViolation {
                        context: format!("page {}", packed_page.id),
                        reason: "rendered page does not resolve in the validated atlas".into(),
                    }
                })?;
                if page.size() != (packed_page.width, packed_page.height) {
                    return Err(TexPackerError::InvariantViolation {
                        context: format!("page {}", packed_page.id),
                        reason: "rendered page dimensions differ from the validated atlas".into(),
                    });
                }
                Ok(render_output_page(prepared, packed_page, self.config))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(PackOutput { atlas, pages })
    }

    fn build_atlas<T>(
        &self,
        prepared: &[PreparedItem<T>],
        packed_pages: &[PackedPage],
    ) -> Result<Atlas> {
        build_atlas(prepared, packed_pages, self.config)
    }
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
        let page_id = checked_page_id(pages.len())?;

        pages.push(PackedPage {
            id: page_id,
            width: page_w,
            height: page_h,
            regions,
        });
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
) -> RenderedPage {
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

    RenderedPage {
        page_id: packed_page.id,
        rgba: canvas,
    }
}

fn build_atlas<T>(
    prepared: &[PreparedItem<T>],
    packed_pages: &[PackedPage],
    config: &OfflineConfig,
) -> Result<Atlas> {
    let page_config = config.page_config();
    let atlas_pages = packed_pages
        .iter()
        .map(|page| page.to_page(prepared))
        .collect::<Result<Vec<_>>>()?;
    let trim_mode = if config.trim_enabled() {
        "trim"
    } else {
        "none"
    };
    let meta = Meta::for_run(
        page_config,
        config.power_of_two(),
        config.square(),
        trim_mode,
    );

    Atlas::try_new(atlas_pages, meta)
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

pub(crate) fn pack_layout_impl(items: Vec<LayoutItem>, config: &OfflineConfig) -> Result<Atlas> {
    if items.is_empty() {
        return Err(TexPackerError::Empty);
    }
    let prepared = prepare_layout_items(items, config)?;

    OfflinePipeline::new(config).pack_layout(&prepared)
}
