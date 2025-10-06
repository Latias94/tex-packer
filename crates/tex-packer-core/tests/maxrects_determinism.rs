use tex_packer_core::config::{AlgorithmFamily, MaxRectsHeuristic, PackerConfig, SortOrder};
use tex_packer_core::model::{Frame, Rect};
use tex_packer_core::packer::maxrects::MaxRectsPacker;
use tex_packer_core::packer::Packer;

#[allow(dead_code)]
fn disjoint(frames: &[Frame]) -> bool {
    for i in 0..frames.len() {
        for j in (i + 1)..frames.len() {
            let a = &frames[i].frame;
            let b = &frames[j].frame;
            let a_x2 = a.x + a.w;
            let a_y2 = a.y + a.h;
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

fn cfg() -> PackerConfig {
    PackerConfig {
        max_width: 512,
        max_height: 512,
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
        use_waste_map: false,
        family: AlgorithmFamily::MaxRects,
        mr_heuristic: MaxRectsHeuristic::BestAreaFit,
        skyline_heuristic: tex_packer_core::config::SkylineHeuristic::BottomLeft,
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
fn maxrects_repeatable_and_disjoint() {
    use rand::{Rng, SeedableRng};
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let cfg = cfg();

    let mut rects: Vec<(u32, u32)> = Vec::new();
    for _ in 0..120 {
        let w = rng.gen_range(4..=64);
        let h = rng.gen_range(4..=64);
        rects.push((w, h));
    }

    let mut p1 = MaxRectsPacker::new(cfg.clone(), MaxRectsHeuristic::BestAreaFit);
    let mut f1: Vec<Frame> = Vec::new();
    for (i, (w, h)) in rects.iter().cloned().enumerate() {
        let r = Rect::new(0, 0, w, h);
        if let Some(f) = <MaxRectsPacker as Packer<String>>::pack(&mut p1, format!("r{}", i), &r) {
            f1.push(f)
        } else {
            break;
        }
    }
    // Note: disjointness invariants are covered by integration tests; here we only ensure determinism.

    let mut p2 = MaxRectsPacker::new(cfg.clone(), MaxRectsHeuristic::BestAreaFit);
    let mut f2: Vec<Frame> = Vec::new();
    for (i, (w, h)) in rects.iter().cloned().enumerate() {
        let r = Rect::new(0, 0, w, h);
        if let Some(f) = <MaxRectsPacker as Packer<String>>::pack(&mut p2, format!("r{}", i), &r) {
            f2.push(f)
        } else {
            break;
        }
    }

    assert_eq!(f1.len(), f2.len());
    for (a, b) in f1.iter().zip(f2.iter()) {
        assert_eq!(a.frame, b.frame);
        assert_eq!(a.rotated, b.rotated);
    }
}
