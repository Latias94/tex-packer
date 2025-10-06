use crate::config::PackerConfig;
use crate::config::{AlgorithmFamily, AutoMode, SortOrder};
use crate::error::{Result, TexPackerError};
use crate::model::{Atlas, Frame, Meta, Page, Rect};
use crate::packer::{
    guillotine::GuillotinePacker, maxrects::MaxRectsPacker, skyline::SkylinePacker, Packer,
};
use image::{DynamicImage, Rgba, RgbaImage};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tracing::instrument;

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

#[instrument(skip_all)]
/// Packs `inputs` into atlas pages using configuration `cfg` and returns metadata and RGBA pages.
///
/// Notes:
/// - Sorting is stable for deterministic results.
/// - When `family` is `Auto`, a small portfolio is tried and the best result is chosen (pages first, then total area).
/// - `time_budget_ms` can limit Auto evaluation time; `parallel` may evaluate in parallel when enabled.
pub fn pack_images(inputs: Vec<InputImage>, cfg: PackerConfig) -> Result<PackOutput> {
    if inputs.is_empty() {
        return Err(TexPackerError::Empty);
    }

    // Preprocess once
    let prepared = prepare_inputs(&inputs, &cfg);

    // Auto portfolio
    if matches!(cfg.family, AlgorithmFamily::Auto) {
        return pack_auto(&prepared, cfg);
    }

    pack_prepared(&prepared, &cfg)
}

pub fn compute_trim_rect(rgba: &RgbaImage, threshold: u8) -> (Option<Rect>, Rect) {
    let (w, h) = rgba.dimensions();
    let mut x1 = 0;
    let mut y1 = 0;
    let mut x2 = w.saturating_sub(1);
    let mut y2 = h.saturating_sub(1);
    // left
    while x1 < w {
        let mut all_transparent = true;
        for y in 0..h {
            if rgba.get_pixel(x1, y)[3] > threshold {
                all_transparent = false;
                break;
            }
        }
        if all_transparent {
            x1 += 1;
        } else {
            break;
        }
    }
    if x1 >= w {
        return (None, Rect::new(0, 0, w, h));
    }
    // right
    while x2 > x1 {
        let mut all_transparent = true;
        for y in 0..h {
            if rgba.get_pixel(x2, y)[3] > threshold {
                all_transparent = false;
                break;
            }
        }
        if all_transparent {
            x2 -= 1;
        } else {
            break;
        }
    }
    // top
    while y1 < h {
        let mut all_transparent = true;
        for x in x1..=x2 {
            if rgba.get_pixel(x, y1)[3] > threshold {
                all_transparent = false;
                break;
            }
        }
        if all_transparent {
            y1 += 1;
        } else {
            break;
        }
    }
    // bottom
    while y2 > y1 {
        let mut all_transparent = true;
        for x in x1..=x2 {
            if rgba.get_pixel(x, y2)[3] > threshold {
                all_transparent = false;
                break;
            }
        }
        if all_transparent {
            y2 -= 1;
        } else {
            break;
        }
    }
    let tw = x2 - x1 + 1;
    let th = y2 - y1 + 1;
    (Some(Rect::new(0, 0, tw, th)), Rect::new(x1, y1, tw, th))
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

#[allow(clippy::too_many_arguments)]
fn blit_image(
    src: &RgbaImage,
    canvas: &mut RgbaImage,
    dx: u32,
    dy: u32,
    sx: u32,
    sy: u32,
    sw: u32,
    sh: u32,
    rotated: bool,
    extrude: u32,
    outlines: bool,
) {
    let (cw, ch) = canvas.dimensions();
    // destination (rendered) size may differ when rotated
    let (rw, rh) = if rotated { (sh, sw) } else { (sw, sh) };

    // main blit
    for yy in 0..rh {
        for xx in 0..rw {
            let (ix, iy) = if rotated {
                (sx + yy, sy + (sh - 1 - xx))
            } else {
                (sx + xx, sy + yy)
            };
            if dx + xx < cw && dy + yy < ch {
                let px = *src.get_pixel(ix, iy);
                canvas.put_pixel(dx + xx, dy + yy, px);
            }
        }
    }

    if outlines {
        // red outline on frame bounds
        let red = Rgba([255, 0, 0, 255]);
        for xx in 0..rw {
            if dx + xx < cw && dy < ch {
                canvas.put_pixel(dx + xx, dy, red);
            }
            let by = dy + rh.saturating_sub(1);
            if dx + xx < cw && by < ch {
                canvas.put_pixel(dx + xx, by, red);
            }
        }
        for yy in 0..rh {
            if dx < cw && dy + yy < ch {
                canvas.put_pixel(dx, dy + yy, red);
            }
            let rx = dx + rw.saturating_sub(1);
            if rx < cw && dy + yy < ch {
                canvas.put_pixel(rx, dy + yy, red);
            }
        }
    }

    if extrude > 0 {
        // edges
        for e in 1..=extrude {
            // top row
            if dy >= e && dy < ch {
                for xx in 0..rw {
                    if dx + xx < cw {
                        let p = *canvas.get_pixel(dx + xx, dy);
                        if dy >= e {
                            canvas.put_pixel(dx + xx, dy - e, p);
                        }
                    }
                }
            }
            // bottom row
            if dy + rh - 1 < ch && dy + rh - 1 + e < ch {
                for xx in 0..rw {
                    if dx + xx < cw {
                        let p = *canvas.get_pixel(dx + xx, dy + rh - 1);
                        canvas.put_pixel(dx + xx, dy + rh - 1 + e, p);
                    }
                }
            }
            // left col
            if dx >= e && dx < cw {
                for yy in 0..rh {
                    if dy + yy < ch {
                        let p = *canvas.get_pixel(dx, dy + yy);
                        canvas.put_pixel(dx - e, dy + yy, p);
                    }
                }
            }
            // right col
            if dx + rw - 1 < cw && dx + rw - 1 + e < cw {
                for yy in 0..rh {
                    if dy + yy < ch {
                        let p = *canvas.get_pixel(dx + rw - 1, dy + yy);
                        canvas.put_pixel(dx + rw - 1 + e, dy + yy, p);
                    }
                }
            }
        }
        // corners (copy the corner pixel) with bounds guards
        let c00 = if dx < cw && dy < ch {
            *canvas.get_pixel(dx, dy)
        } else {
            Rgba([0, 0, 0, 0])
        };
        let c10 = if dx + rw > 0 && dx + rw - 1 < cw && dy < ch {
            *canvas.get_pixel(dx + rw - 1, dy)
        } else {
            Rgba([0, 0, 0, 0])
        };
        let c01 = if dx < cw && dy + rh > 0 && dy + rh - 1 < ch {
            *canvas.get_pixel(dx, dy + rh - 1)
        } else {
            Rgba([0, 0, 0, 0])
        };
        let c11 = if dx + rw > 0 && dx + rw - 1 < cw && dy + rh > 0 && dy + rh - 1 < ch {
            *canvas.get_pixel(dx + rw - 1, dy + rh - 1)
        } else {
            Rgba([0, 0, 0, 0])
        };
        if dx >= 1 && dy >= 1 {
            for ex in 1..=extrude {
                for ey in 1..=extrude {
                    if dx >= ex && dy >= ey {
                        canvas.put_pixel(dx - ex, dy - ey, c00);
                    }
                }
            }
        }
        if dy >= 1 && dx + rw - 1 < cw {
            for ex in 1..=extrude {
                for ey in 1..=extrude {
                    if dy >= ey && dx + rw - 1 + ex < cw {
                        canvas.put_pixel(dx + rw - 1 + ex, dy - ey, c10);
                    }
                }
            }
        }
        if dx >= 1 && dy + rh - 1 < ch {
            for ex in 1..=extrude {
                for ey in 1..=extrude {
                    if dx >= ex && dy + rh - 1 + ey < ch {
                        canvas.put_pixel(dx - ex, dy + rh - 1 + ey, c01);
                    }
                }
            }
        }
        if dx + rw - 1 < cw && dy + rh - 1 < ch {
            for ex in 1..=extrude {
                for ey in 1..=extrude {
                    if dx + rw - 1 + ex < cw && dy + rh - 1 + ey < ch {
                        canvas.put_pixel(dx + rw - 1 + ex, dy + rh - 1 + ey, c11);
                    }
                }
            }
        }
    }
}

// ---------- helpers for multi-run (auto) ----------

struct Prep {
    key: String,
    rgba: RgbaImage,
    rect: Rect,
    trimmed: bool,
    source: Rect,
    orig_size: (u32, u32),
}

fn prepare_inputs(inputs: &[InputImage], cfg: &PackerConfig) -> Vec<Prep> {
    let mut out = Vec::with_capacity(inputs.len());
    for inp in inputs.iter() {
        let rgba = inp.image.to_rgba8();
        let (iw, ih) = rgba.dimensions();
        let (rect, trimmed, source) = if cfg.trim {
            let (trim_rect_opt, src_rect) = compute_trim_rect(&rgba, cfg.trim_threshold);
            match trim_rect_opt {
                Some(r) => (Rect::new(0, 0, r.w, r.h), true, src_rect),
                None => (Rect::new(0, 0, iw, ih), false, Rect::new(0, 0, iw, ih)),
            }
        } else {
            (Rect::new(0, 0, iw, ih), false, Rect::new(0, 0, iw, ih))
        };
        out.push(Prep {
            key: inp.key.clone(),
            rgba,
            rect,
            trimmed,
            source,
            orig_size: (iw, ih),
        });
    }
    // stable sort per config
    match cfg.sort_order {
        SortOrder::None => {}
        SortOrder::NameAsc => {
            out.sort_by(|a, b| a.key.cmp(&b.key));
        }
        SortOrder::AreaDesc => {
            out.sort_by(|a, b| {
                (b.rect.w * b.rect.h)
                    .cmp(&(a.rect.w * a.rect.h))
                    .then_with(|| a.key.cmp(&b.key))
            });
        }
        SortOrder::MaxSideDesc => {
            out.sort_by(|a, b| {
                b.rect
                    .w
                    .max(b.rect.h)
                    .cmp(&a.rect.w.max(a.rect.h))
                    .then_with(|| a.key.cmp(&b.key))
            });
        }
        SortOrder::HeightDesc => {
            out.sort_by(|a, b| b.rect.h.cmp(&a.rect.h).then_with(|| a.key.cmp(&b.key)));
        }
        SortOrder::WidthDesc => {
            out.sort_by(|a, b| b.rect.w.cmp(&a.rect.w).then_with(|| a.key.cmp(&b.key)));
        }
    }
    out
}

fn pack_prepared(prepared: &[Prep], cfg: &PackerConfig) -> Result<PackOutput> {
    let mut pages: Vec<OutputPage> = Vec::new();
    let mut atlas_pages: Vec<Page> = Vec::new();

    // Map for quick lookup during compositing
    let prep_map: HashMap<String, &Prep> = prepared.iter().map(|p| (p.key.clone(), p)).collect();

    // Remaining indices to place (in sorted order)
    let mut remaining: Vec<usize> = (0..prepared.len()).collect();
    let mut page_id = 0usize;

    while !remaining.is_empty() {
        let mut packer: Box<dyn Packer<String>> = match cfg.family {
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
        };
        let mut frames: Vec<Frame> = Vec::new();

        loop {
            let mut placed_any = false;
            let mut remove_set: HashSet<usize> = HashSet::new();
            for &idx in &remaining {
                let p = &prepared[idx];
                if !packer.can_pack(&p.rect) {
                    continue;
                }
                if let Some(mut f) = packer.pack(p.key.clone(), &p.rect) {
                    f.trimmed = p.trimmed;
                    f.source = p.source;
                    f.source_size = p.orig_size;
                    frames.push(f);
                    remove_set.insert(idx);
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

        if frames.is_empty() {
            return Err(TexPackerError::OutOfSpace);
        }

        // Compute final page size; include right/bottom reserved margin (extrude + ceil(padding/2))
        let pad_half = cfg.texture_padding / 2;
        let pad_rem = cfg.texture_padding - pad_half; // ceil division remainder
        let right_extra = cfg.texture_extrusion + pad_rem;
        let bottom_extra = cfg.texture_extrusion + pad_rem;

        let mut page_w = if cfg.force_max_dimensions {
            cfg.max_width
        } else {
            0
        };
        let mut page_h = if cfg.force_max_dimensions {
            cfg.max_height
        } else {
            0
        };
        for f in &frames {
            page_w = page_w.max(f.frame.right() + 1 + right_extra + cfg.border_padding);
            page_h = page_h.max(f.frame.bottom() + 1 + bottom_extra + cfg.border_padding);
        }
        if cfg.power_of_two {
            page_w = next_pow2(page_w.max(1));
            page_h = next_pow2(page_h.max(1));
        }
        if cfg.square {
            let m = page_w.max(page_h);
            page_w = m;
            page_h = m;
        }

        let mut canvas = RgbaImage::new(page_w, page_h);
        for f in &frames {
            if let Some(prep) = prep_map.get(&f.key) {
                blit_image(
                    &prep.rgba,
                    &mut canvas,
                    f.frame.x,
                    f.frame.y,
                    prep.source.x,
                    prep.source.y,
                    prep.source.w,
                    prep.source.h,
                    f.rotated,
                    cfg.texture_extrusion,
                    cfg.texture_outlines,
                );
            }
        }
        let page = Page {
            id: page_id,
            width: page_w,
            height: page_h,
            frames: frames.clone(),
        };
        pages.push(OutputPage {
            page: page.clone(),
            rgba: canvas,
        });
        atlas_pages.push(page);
        page_id += 1;
    }

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
    let atlas = Atlas {
        pages: atlas_pages,
        meta,
    };
    Ok(PackOutput { atlas, pages })
}

fn pack_auto(prepared: &[Prep], base: PackerConfig) -> Result<PackOutput> {
    let mut candidates: Vec<PackerConfig> = Vec::new();
    let n_inputs = prepared.len();
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
            let results: Vec<(PackOutput, u64, u32)> = candidates
                .par_iter()
                .filter_map(|cand| pack_prepared(prepared, cand).ok())
                .map(|out| {
                    let pages = out.atlas.pages.len() as u32;
                    let total_area: u64 = out
                        .atlas
                        .pages
                        .iter()
                        .map(|p| (p.width as u64) * (p.height as u64))
                        .sum();
                    (out, total_area, pages)
                })
                .collect();
            let best = results.into_iter().min_by(|a, b| match a.2.cmp(&b.2) {
                // pages asc
                std::cmp::Ordering::Equal => a.1.cmp(&b.1),
                other => other,
            });
            return best.map(|x| x.0).ok_or(TexPackerError::OutOfSpace);
        }
    }

    // Sequential path with optional time budget
    let mut best: Option<(PackOutput, u64, u32)> = None; // (output, total_area, pages)
    for cand in candidates.into_iter() {
        if budget_ms > 0 && start.elapsed().as_millis() as u64 > budget_ms {
            break;
        }
        if let Ok(out) = pack_prepared(prepared, &cand) {
            let pages = out.atlas.pages.len() as u32;
            let total_area: u64 = out
                .atlas
                .pages
                .iter()
                .map(|p| (p.width as u64) * (p.height as u64))
                .sum();
            match &mut best {
                None => best = Some((out, total_area, pages)),
                Some((bo, barea, bpages)) => {
                    if pages < *bpages || (pages == *bpages && total_area < *barea) {
                        *bo = out;
                        *barea = total_area;
                        *bpages = pages;
                    }
                }
            }
        }
    }
    best.map(|x| x.0).ok_or(TexPackerError::OutOfSpace)
}

// ---------------- Layout-only API ----------------

/// Packs sizes into pages without compositing pixel data.
/// Inputs are (key, width, height). Returns an Atlas with pages and frames; no RGBA pages.
pub fn pack_layout<K: Into<String>>(
    inputs: Vec<(K, u32, u32)>,
    cfg: PackerConfig,
) -> Result<Atlas<String>> {
    if inputs.is_empty() {
        return Err(TexPackerError::Empty);
    }
    // Build lightweight preps
    struct PrepL {
        key: String,
        rect: Rect,
        trimmed: bool,
        source: Rect,
        orig_size: (u32, u32),
    }
    let mut prepared: Vec<PrepL> = inputs
        .into_iter()
        .map(|(k, w, h)| {
            let key = k.into();
            let rect = Rect::new(0, 0, w, h);
            let source = Rect::new(0, 0, w, h);
            PrepL {
                key,
                rect,
                trimmed: false,
                source,
                orig_size: (w, h),
            }
        })
        .collect();
    // Sort like pack_images
    match cfg.sort_order {
        SortOrder::None => {}
        SortOrder::NameAsc => prepared.sort_by(|a, b| a.key.cmp(&b.key)),
        SortOrder::AreaDesc => prepared.sort_by(|a, b| {
            (b.rect.w * b.rect.h)
                .cmp(&(a.rect.w * a.rect.h))
                .then_with(|| a.key.cmp(&b.key))
        }),
        SortOrder::MaxSideDesc => prepared.sort_by(|a, b| {
            b.rect
                .w
                .max(b.rect.h)
                .cmp(&a.rect.w.max(a.rect.h))
                .then_with(|| a.key.cmp(&b.key))
        }),
        SortOrder::HeightDesc => {
            prepared.sort_by(|a, b| b.rect.h.cmp(&a.rect.h).then_with(|| a.key.cmp(&b.key)))
        }
        SortOrder::WidthDesc => {
            prepared.sort_by(|a, b| b.rect.w.cmp(&a.rect.w).then_with(|| a.key.cmp(&b.key)))
        }
    }

    let mut remaining: Vec<usize> = (0..prepared.len()).collect();
    let mut atlas_pages: Vec<Page> = Vec::new();
    let mut page_id = 0usize;
    while !remaining.is_empty() {
        let mut packer: Box<dyn Packer<String>> = match cfg.family {
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
        };
        let mut frames: Vec<Frame> = Vec::new();
        loop {
            let mut placed_any = false;
            let mut remove_set: HashSet<usize> = HashSet::new();
            for &idx in &remaining {
                let p = &prepared[idx];
                if !packer.can_pack(&p.rect) {
                    continue;
                }
                if let Some(mut f) = packer.pack(p.key.clone(), &p.rect) {
                    f.trimmed = p.trimmed;
                    f.source = p.source;
                    f.source_size = p.orig_size;
                    frames.push(f);
                    remove_set.insert(idx);
                    placed_any = true;
                }
            }
            if !placed_any {
                break;
            }
            if !remove_set.is_empty() {
                remaining.retain(|i| !remove_set.contains(i));
            }
        }
        if frames.is_empty() {
            return Err(TexPackerError::OutOfSpace);
        }

        // Compute page size same as pack_prepared
        let pad_half = cfg.texture_padding / 2;
        let pad_rem = cfg.texture_padding - pad_half;
        let right_extra = cfg.texture_extrusion + pad_rem;
        let bottom_extra = cfg.texture_extrusion + pad_rem;
        let mut page_w = if cfg.force_max_dimensions {
            cfg.max_width
        } else {
            0
        };
        let mut page_h = if cfg.force_max_dimensions {
            cfg.max_height
        } else {
            0
        };
        for f in &frames {
            page_w = page_w.max(f.frame.right() + 1 + right_extra + cfg.border_padding);
            page_h = page_h.max(f.frame.bottom() + 1 + bottom_extra + cfg.border_padding);
        }
        if cfg.power_of_two {
            page_w = next_pow2(page_w.max(1));
            page_h = next_pow2(page_h.max(1));
        }
        if cfg.square {
            let m = page_w.max(page_h);
            page_w = m;
            page_h = m;
        }

        let page = Page {
            id: page_id,
            width: page_w,
            height: page_h,
            frames: frames.clone(),
        };
        atlas_pages.push(page);
        page_id += 1;
    }

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
    Ok(Atlas {
        pages: atlas_pages,
        meta,
    })
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
    if items.is_empty() {
        return Err(TexPackerError::Empty);
    }
    struct PrepL {
        key: String,
        rect: Rect,
        trimmed: bool,
        source: Rect,
        orig_size: (u32, u32),
    }
    let mut prepared: Vec<PrepL> = items
        .into_iter()
        .map(|it| {
            let key = it.key.into();
            let rect = Rect::new(0, 0, it.w, it.h);
            let source = it.source.unwrap_or(Rect::new(0, 0, it.w, it.h));
            let orig = it.source_size.unwrap_or((it.w, it.h));
            PrepL {
                key,
                rect,
                trimmed: it.trimmed,
                source,
                orig_size: orig,
            }
        })
        .collect();
    match cfg.sort_order {
        SortOrder::None => {}
        SortOrder::NameAsc => prepared.sort_by(|a, b| a.key.cmp(&b.key)),
        SortOrder::AreaDesc => prepared.sort_by(|a, b| {
            (b.rect.w * b.rect.h)
                .cmp(&(a.rect.w * a.rect.h))
                .then_with(|| a.key.cmp(&b.key))
        }),
        SortOrder::MaxSideDesc => prepared.sort_by(|a, b| {
            b.rect
                .w
                .max(b.rect.h)
                .cmp(&a.rect.w.max(a.rect.h))
                .then_with(|| a.key.cmp(&b.key))
        }),
        SortOrder::HeightDesc => {
            prepared.sort_by(|a, b| b.rect.h.cmp(&a.rect.h).then_with(|| a.key.cmp(&b.key)))
        }
        SortOrder::WidthDesc => {
            prepared.sort_by(|a, b| b.rect.w.cmp(&a.rect.w).then_with(|| a.key.cmp(&b.key)))
        }
    }

    let mut remaining: Vec<usize> = (0..prepared.len()).collect();
    let mut atlas_pages: Vec<Page> = Vec::new();
    let mut page_id = 0usize;
    while !remaining.is_empty() {
        let mut packer: Box<dyn Packer<String>> = match cfg.family {
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
        };
        let mut frames: Vec<Frame> = Vec::new();
        loop {
            let mut placed_any = false;
            let mut remove_set: HashSet<usize> = HashSet::new();
            for &idx in &remaining {
                let p = &prepared[idx];
                if !packer.can_pack(&p.rect) {
                    continue;
                }
                if let Some(mut f) = packer.pack(p.key.clone(), &p.rect) {
                    f.trimmed = p.trimmed;
                    f.source = p.source;
                    f.source_size = p.orig_size;
                    frames.push(f);
                    remove_set.insert(idx);
                    placed_any = true;
                }
            }
            if !placed_any {
                break;
            }
            if !remove_set.is_empty() {
                remaining.retain(|i| !remove_set.contains(i));
            }
        }
        if frames.is_empty() {
            return Err(TexPackerError::OutOfSpace);
        }

        let pad_half = cfg.texture_padding / 2;
        let pad_rem = cfg.texture_padding - pad_half;
        let right_extra = cfg.texture_extrusion + pad_rem;
        let bottom_extra = cfg.texture_extrusion + pad_rem;
        let mut page_w = if cfg.force_max_dimensions {
            cfg.max_width
        } else {
            0
        };
        let mut page_h = if cfg.force_max_dimensions {
            cfg.max_height
        } else {
            0
        };
        for f in &frames {
            page_w = page_w.max(f.frame.right() + 1 + right_extra + cfg.border_padding);
            page_h = page_h.max(f.frame.bottom() + 1 + bottom_extra + cfg.border_padding);
        }
        if cfg.power_of_two {
            page_w = next_pow2(page_w.max(1));
            page_h = next_pow2(page_h.max(1));
        }
        if cfg.square {
            let m = page_w.max(page_h);
            page_w = m;
            page_h = m;
        }

        let page = Page {
            id: page_id,
            width: page_w,
            height: page_h,
            frames: frames.clone(),
        };
        atlas_pages.push(page);
        page_id += 1;
    }

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
    Ok(Atlas {
        pages: atlas_pages,
        meta,
    })
}
