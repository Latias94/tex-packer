use tex_packer_core::config::{GuillotineChoice, GuillotineSplit, PageConfig};
use tex_packer_core::model::Rect;
use tex_packer_core::packer::Packer;
use tex_packer_core::packer::guillotine::GuillotinePacker;

#[test]
fn guillotine_rotates_when_only_rotated_fits() {
    let cfg = PageConfig::builder()
        .max_dimensions(16, 12)
        .allow_rotation(true)
        .build()
        .expect("valid page config");

    let mut p = GuillotinePacker::new(
        cfg,
        GuillotineChoice::BestAreaFit,
        GuillotineSplit::SplitShorterLeftoverAxis,
    );
    let r = Rect::new(0, 0, 8, 14);
    let f = <GuillotinePacker as Packer<String>>::pack(&mut p, "R".into(), &r)
        .expect("rotated fit should succeed");
    assert!(f.rotated, "should rotate because only rotated fits");
    assert_eq!(f.frame.w, 14);
    assert_eq!(f.frame.h, 8);
}
