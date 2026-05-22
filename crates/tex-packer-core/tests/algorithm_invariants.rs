use std::collections::HashMap;

use tex_packer_core::prelude::*;

#[derive(Clone)]
struct OfflineCase {
    name: &'static str,
    cfg: PackerConfig,
}

#[derive(Clone)]
struct RuntimeCase {
    name: &'static str,
    cfg: PackerConfig,
    strategy: RuntimeStrategy,
}

#[test]
fn offline_algorithms_satisfy_shared_frame_invariants() {
    let items = vec![
        ("hero", 41, 29),
        ("enemy", 23, 47),
        ("tree", 31, 31),
        ("rock", 19, 37),
        ("coin", 13, 13),
        ("banner", 55, 17),
        ("tower", 21, 53),
        ("spark", 9, 25),
    ];

    for case in offline_cases() {
        let atlas = pack_layout(items.clone(), case.cfg.clone()).expect(case.name);
        assert_atlas_invariants(case.name, &atlas, &expected_sizes(&items));

        let repeated = pack_layout(items.clone(), case.cfg).expect(case.name);
        assert_eq!(
            canonical_frames(&atlas),
            canonical_frames(&repeated),
            "{} should be deterministic",
            case.name
        );
    }
}

#[test]
fn offline_multi_page_invariants_are_page_local() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(32, 32)
        .force_max_dimensions(true)
        .allow_rotation(false)
        .texture_padding(0)
        .texture_extrusion(0)
        .family(AlgorithmFamily::Skyline)
        .build();
    let items = vec![("a", 32, 32), ("b", 32, 32), ("c", 32, 32)];

    let atlas = pack_layout(items.clone(), cfg).expect("multi-page skyline layout");

    assert_eq!(atlas.pages.len(), 3);
    assert_atlas_invariants("offline multi-page", &atlas, &expected_sizes(&items));
    assert!(
        atlas.pages.iter().all(|page| page.frames.len() == 1),
        "each full-page frame must be isolated on its own page"
    );
}

#[test]
fn runtime_strategies_satisfy_shared_frame_invariants() {
    let items = vec![
        ("hero", 34, 21),
        ("enemy", 19, 39),
        ("tree", 25, 25),
        ("rock", 17, 31),
        ("coin", 11, 11),
        ("banner", 45, 13),
        ("tower", 15, 43),
    ];

    for case in runtime_cases() {
        let atlas = run_runtime_case(&case, &items);
        assert_atlas_invariants(case.name, &atlas, &expected_sizes(&items));

        let repeated = run_runtime_case(&case, &items);
        assert_eq!(
            canonical_frames(&atlas),
            canonical_frames(&repeated),
            "{} should be deterministic",
            case.name
        );
    }
}

#[test]
fn runtime_multi_page_invariants_are_page_local() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(32, 32)
        .allow_rotation(false)
        .texture_padding(0)
        .texture_extrusion(0)
        .build();
    let items = vec![("a", 32, 32), ("b", 32, 32), ("c", 32, 32)];
    let case = RuntimeCase {
        name: "runtime guillotine multi-page",
        cfg,
        strategy: RuntimeStrategy::Guillotine,
    };

    let atlas = run_runtime_case(&case, &items);

    assert_eq!(atlas.pages.len(), 3);
    assert_atlas_invariants(case.name, &atlas, &expected_sizes(&items));
    assert!(
        atlas.pages.iter().all(|page| page.frames.len() == 1),
        "each full-page frame must be isolated on its own page"
    );
}

fn offline_cases() -> Vec<OfflineCase> {
    vec![
        OfflineCase {
            name: "offline skyline bottom-left",
            cfg: base_cfg()
                .family(AlgorithmFamily::Skyline)
                .skyline_heuristic(SkylineHeuristic::BottomLeft)
                .build(),
        },
        OfflineCase {
            name: "offline skyline min-waste with waste map",
            cfg: base_cfg()
                .family(AlgorithmFamily::Skyline)
                .skyline_heuristic(SkylineHeuristic::MinWaste)
                .use_waste_map(true)
                .build(),
        },
        OfflineCase {
            name: "offline maxrects best-area-fit",
            cfg: base_cfg()
                .family(AlgorithmFamily::MaxRects)
                .mr_heuristic(MaxRectsHeuristic::BestAreaFit)
                .build(),
        },
        OfflineCase {
            name: "offline maxrects reference best-area-fit",
            cfg: base_cfg()
                .family(AlgorithmFamily::MaxRects)
                .mr_heuristic(MaxRectsHeuristic::BestAreaFit)
                .mr_reference(true)
                .build(),
        },
        OfflineCase {
            name: "offline guillotine best-area-fit",
            cfg: base_cfg()
                .family(AlgorithmFamily::Guillotine)
                .g_choice(GuillotineChoice::BestAreaFit)
                .g_split(GuillotineSplit::SplitShorterLeftoverAxis)
                .build(),
        },
        OfflineCase {
            name: "offline auto quality",
            cfg: base_cfg()
                .family(AlgorithmFamily::Auto)
                .auto_mode(AutoMode::Quality)
                .time_budget_ms(Some(50))
                .build(),
        },
    ]
}

fn runtime_cases() -> Vec<RuntimeCase> {
    let cfg = base_cfg().build();
    vec![
        RuntimeCase {
            name: "runtime guillotine",
            cfg: cfg.clone(),
            strategy: RuntimeStrategy::Guillotine,
        },
        RuntimeCase {
            name: "runtime shelf next-fit",
            cfg: cfg.clone(),
            strategy: RuntimeStrategy::Shelf(ShelfPolicy::NextFit),
        },
        RuntimeCase {
            name: "runtime shelf first-fit",
            cfg: cfg.clone(),
            strategy: RuntimeStrategy::Shelf(ShelfPolicy::FirstFit),
        },
        RuntimeCase {
            name: "runtime skyline bottom-left",
            cfg: cfg.clone(),
            strategy: RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft),
        },
        RuntimeCase {
            name: "runtime skyline min-waste",
            cfg,
            strategy: RuntimeStrategy::Skyline(SkylineHeuristic::MinWaste),
        },
    ]
}

fn base_cfg() -> PackerConfigBuilder {
    PackerConfig::builder()
        .with_max_dimensions(96, 96)
        .force_max_dimensions(true)
        .allow_rotation(true)
        .border_padding(3)
        .texture_padding(2)
        .texture_extrusion(1)
        .sort_order(SortOrder::AreaDesc)
}

fn run_runtime_case(case: &RuntimeCase, items: &[(&str, u32, u32)]) -> Atlas<String> {
    let mut session = AtlasSession::new(case.cfg.clone(), case.strategy.clone());
    for (key, w, h) in items {
        session
            .append((*key).to_string(), *w, *h)
            .unwrap_or_else(|err| panic!("{} should append {key}: {err:?}", case.name));
    }
    session.snapshot_atlas()
}

fn expected_sizes(items: &[(&str, u32, u32)]) -> HashMap<String, (u32, u32)> {
    items
        .iter()
        .map(|(key, w, h)| ((*key).to_string(), (*w, *h)))
        .collect()
}

fn assert_atlas_invariants(
    label: &str,
    atlas: &Atlas<String>,
    expected: &HashMap<String, (u32, u32)>,
) {
    let mut seen = HashMap::new();

    for page in &atlas.pages {
        assert!(
            page.width > 0 && page.height > 0,
            "{label}: page {} must have positive dimensions",
            page.id
        );
        assert_page_frames_within_bounds(label, page);
        assert_page_frames_disjoint(label, page);

        for frame in &page.frames {
            let Some(&(source_w, source_h)) = expected.get(&frame.key) else {
                panic!("{label}: unexpected frame key {}", frame.key);
            };
            assert_eq!(
                frame.source,
                Rect::new(0, 0, source_w, source_h),
                "{label}: source rect for {} must preserve input dimensions",
                frame.key
            );
            assert_eq!(
                frame.source_size,
                (source_w, source_h),
                "{label}: source size for {} must preserve input dimensions",
                frame.key
            );

            let expected_frame_size = if frame.rotated {
                (source_h, source_w)
            } else {
                (source_w, source_h)
            };
            assert_eq!(
                (frame.frame.w, frame.frame.h),
                expected_frame_size,
                "{label}: frame size for {} must match rotation flag",
                frame.key
            );

            assert!(
                seen.insert(frame.key.clone(), page.id).is_none(),
                "{label}: duplicate frame key {}",
                frame.key
            );
        }
    }

    assert_eq!(
        seen.len(),
        expected.len(),
        "{label}: all expected frames must be packed"
    );
}

fn assert_page_frames_within_bounds(label: &str, page: &Page<String>) {
    for frame in &page.frames {
        let rect = &frame.frame;
        assert!(rect.w > 0 && rect.h > 0, "{label}: zero-sized frame");
        assert!(
            right_ex(rect) <= page.width as u64 && bottom_ex(rect) <= page.height as u64,
            "{label}: frame {} {:?} must fit within page {}x{}",
            frame.key,
            rect,
            page.width,
            page.height
        );
    }
}

fn assert_page_frames_disjoint(label: &str, page: &Page<String>) {
    for i in 0..page.frames.len() {
        for j in (i + 1)..page.frames.len() {
            let a = &page.frames[i].frame;
            let b = &page.frames[j].frame;
            assert!(
                !intersects(a, b),
                "{label}: frames {} {:?} and {} {:?} overlap on page {}",
                page.frames[i].key,
                a,
                page.frames[j].key,
                b,
                page.id
            );
        }
    }
}

type CanonicalFrame = (usize, String, Rect, bool, Rect, (u32, u32));

fn canonical_frames(atlas: &Atlas<String>) -> Vec<CanonicalFrame> {
    let mut out = atlas
        .pages
        .iter()
        .flat_map(|page| {
            page.frames.iter().map(|frame| {
                (
                    page.id,
                    frame.key.clone(),
                    frame.frame,
                    frame.rotated,
                    frame.source,
                    frame.source_size,
                )
            })
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    out
}

fn intersects(a: &Rect, b: &Rect) -> bool {
    !(a.x as u64 >= right_ex(b)
        || b.x as u64 >= right_ex(a)
        || a.y as u64 >= bottom_ex(b)
        || b.y as u64 >= bottom_ex(a))
}

fn right_ex(rect: &Rect) -> u64 {
    rect.x as u64 + rect.w as u64
}

fn bottom_ex(rect: &Rect) -> u64 {
    rect.y as u64 + rect.h as u64
}
