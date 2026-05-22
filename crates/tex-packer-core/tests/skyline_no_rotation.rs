use tex_packer_core::config::{AlgorithmFamily, PackerConfig, SkylineHeuristic};
use tex_packer_core::model::Rect;
use tex_packer_core::packer::Packer;
use tex_packer_core::packer::skyline::SkylinePacker;

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

#[test]
fn skyline_rejects_span_that_would_overlap_full_height_segment() {
    let cfg = PackerConfig {
        max_width: 10,
        max_height: 10,
        family: AlgorithmFamily::Skyline,
        allow_rotation: false,
        texture_padding: 0,
        texture_extrusion: 0,
        skyline_heuristic: SkylineHeuristic::BottomLeft,
        ..Default::default()
    };

    let mut packer = SkylinePacker::new(cfg);
    let full_height = Rect::new(0, 0, 5, 10);
    let too_wide_for_remaining_side = Rect::new(0, 0, 6, 1);

    <SkylinePacker as Packer<String>>::pack(&mut packer, "full".into(), &full_height)
        .expect("first rectangle should fit");

    assert!(
        <SkylinePacker as Packer<String>>::pack(
            &mut packer,
            "overlap".into(),
            &too_wide_for_remaining_side,
        )
        .is_none(),
        "Skyline must not reuse the bottom row of a full-height segment"
    );
}
