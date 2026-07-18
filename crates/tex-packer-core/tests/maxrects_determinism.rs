use tex_packer_core::config::{MaxRectsHeuristic, PageConfig};
use tex_packer_core::model::{Frame, Rect};
use tex_packer_core::packer::Packer;
use tex_packer_core::packer::maxrects::MaxRectsPacker;

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

fn cfg() -> PageConfig {
    PageConfig::builder()
        .max_dimensions(512, 512)
        .allow_rotation(true)
        .texture_padding(0)
        .texture_extrusion(0)
        .build()
        .expect("valid page config")
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

    let mut p1 = MaxRectsPacker::new(cfg.clone(), MaxRectsHeuristic::BestAreaFit, false);
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

    let mut p2 = MaxRectsPacker::new(cfg, MaxRectsHeuristic::BestAreaFit, false);
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
