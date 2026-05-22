use tex_packer_core::config::{AlgorithmFamily, GuillotineChoice, GuillotineSplit, PackerConfig};
use tex_packer_core::model::Rect;
use tex_packer_core::packer::Packer;
use tex_packer_core::packer::guillotine::GuillotinePacker;

#[test]
fn guillotine_rotates_when_only_rotated_fits() {
    let cfg = PackerConfig {
        max_width: 16,
        max_height: 12,
        allow_rotation: true,
        family: AlgorithmFamily::Guillotine,
        ..Default::default()
    };

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
