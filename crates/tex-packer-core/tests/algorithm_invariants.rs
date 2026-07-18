use std::collections::HashMap;

use tex_packer_core::prelude::*;
use tex_packer_core::{FrameId, PageId, RegionId};

#[derive(Clone)]
struct OfflineCase {
    name: &'static str,
    cfg: OfflineConfig,
}

#[derive(Clone)]
struct RuntimeCase {
    name: &'static str,
    cfg: RuntimeConfig,
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
    let page = PageConfig::builder()
        .max_dimensions(32, 32)
        .allow_rotation(false)
        .texture_padding(0)
        .texture_extrusion(0)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .force_max_dimensions(true)
        .strategy(skyline_strategy(SkylineHeuristic::BottomLeft, false))
        .build()
        .expect("valid offline config");
    let items = vec![("a", 32, 32), ("b", 32, 32), ("c", 32, 32)];

    let atlas = pack_layout(items.clone(), cfg).expect("multi-page skyline layout");

    assert_eq!(atlas.pages().len(), 3);
    assert_atlas_invariants("offline multi-page", &atlas, &expected_sizes(&items));
    assert!(
        atlas.pages().iter().all(|page| page.frames().len() == 1),
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
    let page = PageConfig::builder()
        .max_dimensions(32, 32)
        .allow_rotation(false)
        .texture_padding(0)
        .texture_extrusion(0)
        .build()
        .expect("valid page config");
    let items = vec![("a", 32, 32), ("b", 32, 32), ("c", 32, 32)];
    let case = RuntimeCase {
        name: "runtime guillotine multi-page",
        cfg: runtime_config(
            page,
            RuntimeStrategy::Guillotine {
                choice: GuillotineChoice::BestAreaFit,
                split: GuillotineSplit::SplitShorterLeftoverAxis,
            },
        ),
    };

    let atlas = run_runtime_case(&case, &items);

    assert_eq!(atlas.pages().len(), 3);
    assert_atlas_invariants(case.name, &atlas, &expected_sizes(&items));
    assert!(
        atlas.pages().iter().all(|page| page.frames().len() == 1),
        "each full-page frame must be isolated on its own page"
    );
}

fn offline_cases() -> Vec<OfflineCase> {
    vec![
        OfflineCase {
            name: "offline skyline bottom-left",
            cfg: offline_config(skyline_strategy(SkylineHeuristic::BottomLeft, false)),
        },
        OfflineCase {
            name: "offline skyline min-waste with waste map",
            cfg: offline_config(skyline_strategy(SkylineHeuristic::MinWaste, true)),
        },
        OfflineCase {
            name: "offline maxrects best-area-fit",
            cfg: offline_config(PackingStrategy::MaxRects {
                heuristic: MaxRectsHeuristic::BestAreaFit,
                reference: false,
            }),
        },
        OfflineCase {
            name: "offline maxrects reference best-area-fit",
            cfg: offline_config(PackingStrategy::MaxRects {
                heuristic: MaxRectsHeuristic::BestAreaFit,
                reference: true,
            }),
        },
        OfflineCase {
            name: "offline guillotine best-area-fit",
            cfg: offline_config(PackingStrategy::Guillotine {
                choice: GuillotineChoice::BestAreaFit,
                split: GuillotineSplit::SplitShorterLeftoverAxis,
            }),
        },
        OfflineCase {
            name: "offline auto quality",
            cfg: offline_config(PackingStrategy::Auto {
                mode: AutoMode::Quality,
                time_budget: Some(std::time::Duration::from_millis(50)),
                parallel: false,
                reference_time_threshold: None,
                reference_input_threshold: None,
            }),
        },
    ]
}

fn runtime_cases() -> Vec<RuntimeCase> {
    let page = base_page_config();
    vec![
        RuntimeCase {
            name: "runtime guillotine",
            cfg: runtime_config(
                page.clone(),
                RuntimeStrategy::Guillotine {
                    choice: GuillotineChoice::BestAreaFit,
                    split: GuillotineSplit::SplitShorterLeftoverAxis,
                },
            ),
        },
        RuntimeCase {
            name: "runtime shelf next-fit",
            cfg: runtime_config(
                page.clone(),
                RuntimeStrategy::Shelf {
                    policy: ShelfPolicy::NextFit,
                },
            ),
        },
        RuntimeCase {
            name: "runtime shelf first-fit",
            cfg: runtime_config(
                page.clone(),
                RuntimeStrategy::Shelf {
                    policy: ShelfPolicy::FirstFit,
                },
            ),
        },
        RuntimeCase {
            name: "runtime skyline bottom-left",
            cfg: runtime_config(
                page.clone(),
                RuntimeStrategy::Skyline {
                    heuristic: SkylineHeuristic::BottomLeft,
                },
            ),
        },
        RuntimeCase {
            name: "runtime skyline min-waste",
            cfg: runtime_config(
                page,
                RuntimeStrategy::Skyline {
                    heuristic: SkylineHeuristic::MinWaste,
                },
            ),
        },
    ]
}

fn base_page_config() -> PageConfig {
    PageConfig::builder()
        .max_dimensions(96, 96)
        .allow_rotation(true)
        .border_padding(3)
        .texture_padding(2)
        .texture_extrusion(1)
        .build()
        .expect("valid shared page config")
}

fn offline_config(strategy: PackingStrategy) -> OfflineConfig {
    OfflineConfig::builder()
        .page_config(base_page_config())
        .force_max_dimensions(true)
        .sort_order(SortOrder::AreaDesc)
        .strategy(strategy)
        .build()
        .expect("valid offline config")
}

fn skyline_strategy(heuristic: SkylineHeuristic, use_waste_map: bool) -> PackingStrategy {
    PackingStrategy::Skyline {
        heuristic,
        use_waste_map,
    }
}

fn runtime_config(page: PageConfig, strategy: RuntimeStrategy) -> RuntimeConfig {
    RuntimeConfig::builder()
        .page_config(page)
        .strategy(strategy)
        .build()
        .expect("valid runtime config")
}

fn run_runtime_case(case: &RuntimeCase, items: &[(&str, u32, u32)]) -> Atlas {
    let mut session = AtlasSession::new(case.cfg.clone());
    for (key, w, h) in items {
        session
            .append((*key).to_string(), *w, *h)
            .unwrap_or_else(|err| panic!("{} should append {key}: {err:?}", case.name));
    }
    session
        .snapshot_atlas()
        .unwrap_or_else(|err| panic!("{} should snapshot: {err:?}", case.name))
}

fn expected_sizes(items: &[(&str, u32, u32)]) -> HashMap<String, (u32, u32)> {
    items
        .iter()
        .map(|(key, w, h)| ((*key).to_string(), (*w, *h)))
        .collect()
}

fn assert_atlas_invariants(label: &str, atlas: &Atlas, expected: &HashMap<String, (u32, u32)>) {
    let mut seen = HashMap::new();

    for page in atlas.pages() {
        assert!(
            page.width() > 0 && page.height() > 0,
            "{label}: page {} must have positive dimensions",
            page.id()
        );
        assert_page_regions_within_bounds(label, page);
        assert_page_regions_disjoint(label, page);

        for resolved in page.resolved_frames() {
            let frame = resolved.frame();
            let region = resolved.region();
            let Some(&(source_w, source_h)) = expected.get(frame.key()) else {
                panic!("{label}: unexpected frame key {}", frame.key());
            };
            assert_eq!(
                frame.source(),
                Rect::new(0, 0, source_w, source_h),
                "{label}: source rect for {} must preserve input dimensions",
                frame.key()
            );
            assert_eq!(
                frame.source_size(),
                (source_w, source_h),
                "{label}: source size for {} must preserve input dimensions",
                frame.key()
            );

            let expected_frame_size = if region.rotated() {
                (source_h, source_w)
            } else {
                (source_w, source_h)
            };
            assert_eq!(
                (region.content().w, region.content().h),
                expected_frame_size,
                "{label}: frame size for {} must match rotation flag",
                frame.key()
            );
            assert_eq!(page.region(frame.region_id()), Some(region));

            assert!(
                seen.insert(frame.key().to_string(), page.id()).is_none(),
                "{label}: duplicate frame key {}",
                frame.key()
            );
        }
    }

    assert_eq!(
        seen.len(),
        expected.len(),
        "{label}: all expected frames must be packed"
    );
}

fn assert_page_regions_within_bounds(label: &str, page: &Page) {
    for region in page.regions() {
        let content = region.content();
        let allocation = region.allocation();
        assert!(content.w > 0 && content.h > 0, "{label}: zero-sized region");
        assert!(
            right_ex(&allocation) <= u64::from(page.width())
                && bottom_ex(&allocation) <= u64::from(page.height()),
            "{label}: allocation for region {} {:?} must fit within page {}x{}",
            region.id(),
            allocation,
            page.width(),
            page.height()
        );
        assert!(allocation.contains(&content));
    }
}

fn assert_page_regions_disjoint(label: &str, page: &Page) {
    for i in 0..page.regions().len() {
        for j in (i + 1)..page.regions().len() {
            let a = page.regions()[i].allocation();
            let b = page.regions()[j].allocation();
            assert!(
                !intersects(&a, &b),
                "{label}: region allocations {} {:?} and {} {:?} overlap on page {}",
                page.regions()[i].id(),
                a,
                page.regions()[j].id(),
                b,
                page.id()
            );
        }
    }
}

type CanonicalFrame = (
    PageId,
    FrameId,
    String,
    RegionId,
    Rect,
    bool,
    Rect,
    (u32, u32),
);

fn canonical_frames(atlas: &Atlas) -> Vec<CanonicalFrame> {
    let mut out = atlas
        .pages()
        .iter()
        .flat_map(|page| {
            page.resolved_frames().map(|resolved| {
                let frame = resolved.frame();
                let region = resolved.region();
                (
                    page.id(),
                    frame.id(),
                    frame.key().to_string(),
                    region.id(),
                    region.content(),
                    region.rotated(),
                    frame.source(),
                    frame.source_size(),
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
