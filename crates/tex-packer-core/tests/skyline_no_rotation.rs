use tex_packer_core::config::{AlgorithmFamily, PackerConfig, SkylineHeuristic};
use tex_packer_core::model::Rect;
use tex_packer_core::packer::skyline::SkylinePacker;
use tex_packer_core::packer::Packer;

#[test]
fn skyline_respects_allow_rotation_false() {
    // Configure Skyline with rotation disabled
    let cfg = PackerConfig {
        family: AlgorithmFamily::Skyline,
        allow_rotation: false,
        skyline_heuristic: SkylineHeuristic::BottomLeft,
        ..Default::default()
    };

    let mut p = SkylinePacker::new(cfg);
    // A tall rectangle that could be rotated if allowed
    let r = Rect::new(0, 0, 64, 128);
    let f = <SkylinePacker as Packer<String>>::pack(&mut p, "tall".into(), &r)
        .expect("should place without rotation");
    assert_eq!(f.frame.w, 64);
    assert_eq!(f.frame.h, 128);
    assert!(
        !f.rotated,
        "rotation must be false when allow_rotation=false"
    );
}
