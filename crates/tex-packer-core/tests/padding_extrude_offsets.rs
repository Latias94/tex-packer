use tex_packer_core::config::{
    AlgorithmFamily, AutoMode, GuillotineChoice, GuillotineSplit, MaxRectsHeuristic, PackerConfig,
    SkylineHeuristic, SortOrder,
};
use tex_packer_core::model::Rect;
use tex_packer_core::packer::guillotine::GuillotinePacker;
use tex_packer_core::packer::maxrects::MaxRectsPacker;
use tex_packer_core::packer::skyline::SkylinePacker;
use tex_packer_core::packer::Packer;

fn cfg_base() -> PackerConfig {
    PackerConfig {
        max_width: 512,
        max_height: 512,
        allow_rotation: false,
        force_max_dimensions: false,
        border_padding: 0,
        texture_padding: 4,
        texture_extrusion: 2,
        trim: false,
        trim_threshold: 0,
        texture_outlines: false,
        power_of_two: false,
        square: false,
        use_waste_map: false,
        family: AlgorithmFamily::Skyline,
        mr_heuristic: MaxRectsHeuristic::BestAreaFit,
        skyline_heuristic: SkylineHeuristic::BottomLeft,
        g_choice: GuillotineChoice::BestAreaFit,
        g_split: GuillotineSplit::SplitShorterLeftoverAxis,
        auto_mode: AutoMode::Quality,
        sort_order: SortOrder::AreaDesc,
        time_budget_ms: None,
        parallel: false,
        mr_reference: false,
        auto_mr_ref_time_ms_threshold: None,
        auto_mr_ref_input_threshold: None,
    }
}

fn expanded_slot(f: &Rect, pad: u32, extrude: u32) -> Rect {
    let pad_half = pad / 2;
    let off = extrude + pad_half;
    Rect::new(
        f.x.saturating_sub(off),
        f.y.saturating_sub(off),
        f.w + extrude * 2 + pad,
        f.h + extrude * 2 + pad,
    )
}

fn disjoint(a: &Rect, b: &Rect) -> bool {
    let ax2 = a.x + a.w;
    let ay2 = a.y + a.h;
    let bx2 = b.x + b.w;
    let by2 = b.y + b.h;
    a.x >= bx2 || b.x >= ax2 || a.y >= by2 || b.y >= ay2
}

#[test]
fn skyline_offsets_produce_disjoint_slots() {
    let cfg = cfg_base();
    let mut p = SkylinePacker::new(cfg.clone());
    let r = Rect::new(0, 0, 40, 40);
    let frames = [
        <SkylinePacker as Packer<String>>::pack(&mut p, "a".into(), &r).expect("place a"),
        <SkylinePacker as Packer<String>>::pack(&mut p, "b".into(), &r).expect("place b"),
    ];
    assert_eq!(frames.len(), 2);

    let s0 = expanded_slot(&frames[0].frame, cfg.texture_padding, cfg.texture_extrusion);
    let s1 = expanded_slot(&frames[1].frame, cfg.texture_padding, cfg.texture_extrusion);
    assert!(
        disjoint(&s0, &s1),
        "expanded slots (content+padding+extrude) must not overlap"
    );
}

#[test]
fn maxrects_offsets_produce_disjoint_slots() {
    let mut cfg = cfg_base();
    cfg.family = AlgorithmFamily::MaxRects;
    let mut p = MaxRectsPacker::new(cfg.clone(), MaxRectsHeuristic::BestAreaFit);
    let r = Rect::new(0, 0, 40, 40);
    let frames = [
        <MaxRectsPacker as Packer<String>>::pack(&mut p, "a".into(), &r).expect("place a"),
        <MaxRectsPacker as Packer<String>>::pack(&mut p, "b".into(), &r).expect("place b"),
    ];
    assert_eq!(frames.len(), 2);

    let s0 = expanded_slot(&frames[0].frame, cfg.texture_padding, cfg.texture_extrusion);
    let s1 = expanded_slot(&frames[1].frame, cfg.texture_padding, cfg.texture_extrusion);
    assert!(
        disjoint(&s0, &s1),
        "expanded slots (content+padding+extrude) must not overlap"
    );
}

#[test]
fn guillotine_offsets_produce_disjoint_slots() {
    let mut cfg = cfg_base();
    cfg.family = AlgorithmFamily::Guillotine;
    let mut p = GuillotinePacker::new(
        cfg.clone(),
        GuillotineChoice::BestAreaFit,
        GuillotineSplit::SplitShorterLeftoverAxis,
    );
    let r = Rect::new(0, 0, 40, 40);
    let frames = [
        <GuillotinePacker as Packer<String>>::pack(&mut p, "a".into(), &r).expect("place a"),
        <GuillotinePacker as Packer<String>>::pack(&mut p, "b".into(), &r).expect("place b"),
    ];
    assert_eq!(frames.len(), 2);

    let s0 = expanded_slot(&frames[0].frame, cfg.texture_padding, cfg.texture_extrusion);
    let s1 = expanded_slot(&frames[1].frame, cfg.texture_padding, cfg.texture_extrusion);
    assert!(
        disjoint(&s0, &s1),
        "expanded slots (content+padding+extrude) must not overlap"
    );
}
