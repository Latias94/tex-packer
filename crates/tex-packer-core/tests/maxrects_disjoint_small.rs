use tex_packer_core::config::{MaxRectsHeuristic, PageConfig};
use tex_packer_core::model::{Frame, Rect};
use tex_packer_core::packer::Packer;
use tex_packer_core::packer::maxrects::MaxRectsPacker;

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

#[test]
fn maxrects_disjoint_on_small_set() {
    let cfg = PageConfig::builder()
        .max_dimensions(256, 256)
        .allow_rotation(true)
        .texture_padding(0)
        .texture_extrusion(0)
        .build()
        .expect("valid page config");

    let mut p = MaxRectsPacker::new(cfg, MaxRectsHeuristic::BestAreaFit, false);
    let rects = vec![
        (64, 64),
        (32, 64),
        (64, 32),
        (48, 48),
        (16, 80),
        (80, 16),
        (40, 40),
        (30, 50),
        (50, 30),
    ];
    let mut frames: Vec<Frame> = Vec::new();
    for (i, (w, h)) in rects.into_iter().enumerate() {
        let r = Rect::new(0, 0, w, h);
        if let Some(f) = <MaxRectsPacker as Packer<String>>::pack(&mut p, format!("r{}", i), &r) {
            frames.push(f);
        } else {
            break;
        }
    }
    assert!(disjoint(&frames));
}
