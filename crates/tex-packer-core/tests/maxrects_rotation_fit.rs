use tex_packer_core::config::{AlgorithmFamily, MaxRectsHeuristic, PackerConfig};
use tex_packer_core::model::Rect;
use tex_packer_core::packer::Packer;
use tex_packer_core::packer::maxrects::MaxRectsPacker;

#[test]
fn maxrects_rotates_when_only_rotated_fits() {
    let mut cfg = PackerConfig::default();
    cfg.max_width = 16;
    cfg.max_height = 12;
    cfg.allow_rotation = true;
    cfg.family = AlgorithmFamily::MaxRects;

    let mut p = MaxRectsPacker::new(cfg, MaxRectsHeuristic::BestAreaFit);
    let r = Rect::new(0, 0, 8, 14);
    let f = <MaxRectsPacker as Packer<String>>::pack(&mut p, "R".into(), &r)
        .expect("rotated fit should succeed");
    assert!(f.rotated, "should rotate because only rotated fits");
    assert_eq!(f.frame.w, 14);
    assert_eq!(f.frame.h, 8);
}
