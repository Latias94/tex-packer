use tex_packer_core::config::{AlgorithmFamily, PackerConfig, SkylineHeuristic, SortOrder};
use tex_packer_core::model::{Frame, Rect};
use tex_packer_core::packer::skyline::SkylinePacker;
use tex_packer_core::packer::Packer;

fn area_of_frames(frames: &[Frame]) -> u64 {
    frames
        .iter()
        .map(|f| (f.frame.w as u64) * (f.frame.h as u64))
        .sum()
}

fn disjoint(frames: &[Frame]) -> bool {
    for i in 0..frames.len() {
        for j in (i + 1)..frames.len() {
            let a = &frames[i].frame;
            let b = &frames[j].frame;
            let a_x2 = a.x + a.w; // exclusive
            let a_y2 = a.y + a.h; // exclusive
            let b_x2 = b.x + b.w;
            let b_y2 = b.y + b.h;
            let overlap = !(a.x >= b_x2 || b.x >= a_x2 || a.y >= b_y2 || b.y >= a_y2);
            if overlap {
                return false;
            }
        }
    }
    true
}

fn make_cfg(use_waste_map: bool) -> PackerConfig {
    PackerConfig {
        max_width: 2048,
        max_height: 2048,
        allow_rotation: true,
        force_max_dimensions: false,
        border_padding: 0,
        texture_padding: 0,
        texture_extrusion: 0,
        trim: false,
        trim_threshold: 0,
        texture_outlines: false,
        power_of_two: false,
        square: false,
        use_waste_map,
        family: AlgorithmFamily::Skyline,
        mr_heuristic: tex_packer_core::config::MaxRectsHeuristic::BestAreaFit,
        skyline_heuristic: SkylineHeuristic::MinWaste,
        g_choice: tex_packer_core::config::GuillotineChoice::BestAreaFit,
        g_split: tex_packer_core::config::GuillotineSplit::SplitShorterLeftoverAxis,
        auto_mode: tex_packer_core::config::AutoMode::Quality,
        sort_order: SortOrder::AreaDesc,
        time_budget_ms: None,
        parallel: false,
        mr_reference: false,
        auto_mr_ref_time_ms_threshold: None,
        auto_mr_ref_input_threshold: None,
    }
}

#[test]
fn skyline_waste_map_improves_or_equal_occupancy() {
    // deterministic set of rectangles
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xDEADBEEF);
    use rand::{Rng, SeedableRng};

    let mut rects: Vec<(u32, u32)> = Vec::new();
    for _ in 0..2000u32 {
        let w = rng.gen_range(4..=128);
        let h = rng.gen_range(4..=128);
        rects.push((w, h));
    }

    // pack without waste map
    let cfg_plain = make_cfg(false);
    let mut pack_plain = SkylinePacker::new(cfg_plain.clone());
    let mut frames_plain: Vec<Frame> = Vec::new();
    for (idx, (w, h)) in rects.iter().cloned().enumerate() {
        let r = Rect::new(0, 0, w, h);
        if let Some(f) =
            <SkylinePacker as Packer<String>>::pack(&mut pack_plain, format!("r{}", idx), &r)
        {
            frames_plain.push(f);
        } else {
            break;
        }
    }
    assert!(disjoint(&frames_plain));
    let used_plain = area_of_frames(&frames_plain) as f64;
    let page_area = (cfg_plain.max_width as u64 * cfg_plain.max_height as u64) as f64;
    let occ_plain = if page_area > 0.0 {
        used_plain / page_area
    } else {
        0.0
    };

    // pack with waste map
    let cfg_waste = make_cfg(true);
    let mut pack_waste = SkylinePacker::new(cfg_waste.clone());
    let mut frames_waste: Vec<Frame> = Vec::new();
    for (idx, (w, h)) in rects.iter().cloned().enumerate() {
        let r = Rect::new(0, 0, w, h);
        if let Some(f) =
            <SkylinePacker as Packer<String>>::pack(&mut pack_waste, format!("r{}", idx), &r)
        {
            frames_waste.push(f);
        } else {
            break;
        }
    }
    assert!(disjoint(&frames_waste));
    let used_waste = area_of_frames(&frames_waste) as f64;
    let occ_waste = if page_area > 0.0 {
        used_waste / page_area
    } else {
        0.0
    };

    // The waste-map variant should not be worse in occupancy.
    assert!(
        occ_waste + 1e-9 >= occ_plain,
        "waste-map occupancy {} should be >= plain {}",
        occ_waste,
        occ_plain
    );
}
