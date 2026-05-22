use tex_packer_core::config::{PackerConfig, SkylineHeuristic};
use tex_packer_core::model::Rect;
use tex_packer_core::packer::Packer;
use tex_packer_core::packer::skyline::SkylinePacker;

#[test]
fn skyline_rotates_when_only_rotated_fits() {
    // Page inner area: 16x9 (no border), allow rotation.
    // Rect is 8x14; unrotated (8x14) doesn't fit (14>9), rotated (14x8) fits (14<=16, 8<=9).
    let cfg = PackerConfig {
        max_width: 16,
        max_height: 12,
        allow_rotation: true,
        skyline_heuristic: SkylineHeuristic::BottomLeft,
        texture_padding: 0,
        ..Default::default()
    };

    let mut p = SkylinePacker::new(cfg);
    let r = Rect::new(0, 0, 8, 14);
    let f = <SkylinePacker as Packer<String>>::pack(&mut p, "R".into(), &r)
        .expect("rotated fit should succeed");
    assert!(f.rotated, "should rotate because only rotated fits");
    assert_eq!(f.frame.w, 14);
    assert_eq!(f.frame.h, 8);
}
