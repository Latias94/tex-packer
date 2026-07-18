use std::time::Duration;

use tex_packer_core::config::{
    AutoMode, GuillotineChoice, GuillotineSplit, MaxRectsHeuristic, OfflineConfig, PackingStrategy,
    PageConfig, RuntimeConfig, RuntimeStrategy, ShelfPolicy, SkylineHeuristic, SortOrder,
    TransparentPolicy,
};

#[test]
fn defaults_match_the_validated_builder_defaults() {
    let page = PageConfig::builder().build().expect("default page");
    let offline = OfflineConfig::builder().build().expect("default offline");
    let runtime = RuntimeConfig::builder().build().expect("default runtime");

    assert_eq!(page, PageConfig::default());
    assert_eq!(offline, OfflineConfig::default());
    assert_eq!(runtime, RuntimeConfig::default());
    assert_eq!(page.max_dimensions(), (1024, 1024));
    assert!(page.allow_rotation());
    assert_eq!(page.border_padding(), 0);
    assert_eq!(page.texture_padding(), 2);
    assert_eq!(page.texture_extrusion(), 0);
    assert!(offline.trim_enabled());
    assert_eq!(offline.trim_threshold(), Some(0));
    assert_eq!(offline.transparent_policy(), Some(TransparentPolicy::Keep));
    assert_eq!(offline.sort_order(), SortOrder::AreaDesc);
}

#[test]
fn immutable_configs_expose_values_through_accessors() {
    let page = PageConfig::builder()
        .max_dimensions(2048, 1024)
        .allow_rotation(false)
        .border_padding(3)
        .texture_padding(5)
        .texture_extrusion(2)
        .build()
        .expect("valid page");
    let offline = OfflineConfig::builder()
        .page_config(page.clone())
        .force_max_dimensions(true)
        .power_of_two(true)
        .square(true)
        .trim(false)
        .trim_threshold(99)
        .transparent_policy(TransparentPolicy::Skip)
        .outlines(true)
        .sort_order(SortOrder::NameAsc)
        .build()
        .expect("valid offline config");

    assert_eq!(offline.page_config(), &page);
    assert!(offline.force_max_dimensions());
    assert!(offline.power_of_two());
    assert!(offline.square());
    assert!(!offline.trim_enabled());
    assert_eq!(offline.trim_threshold(), None);
    assert_eq!(offline.transparent_policy(), None);
    assert!(offline.outlines());
    assert_eq!(offline.sort_order(), SortOrder::NameAsc);
}

#[test]
fn packing_strategy_variants_only_contain_relevant_options() {
    let strategies = [
        PackingStrategy::Skyline {
            heuristic: SkylineHeuristic::MinWaste,
            use_waste_map: true,
        },
        PackingStrategy::MaxRects {
            heuristic: MaxRectsHeuristic::ContactPoint,
            reference: true,
        },
        PackingStrategy::Guillotine {
            choice: GuillotineChoice::WorstAreaFit,
            split: GuillotineSplit::SplitLongerAxis,
        },
        PackingStrategy::Auto {
            mode: AutoMode::Quality,
            time_budget: Some(Duration::from_millis(250)),
            parallel: true,
            reference_time_threshold: Some(Duration::from_millis(200)),
            reference_input_threshold: Some(800),
        },
    ];

    for strategy in strategies {
        let config = OfflineConfig::builder()
            .strategy(strategy)
            .build()
            .expect("valid strategy");
        assert_eq!(config.strategy(), &strategy);
    }
}

#[test]
fn runtime_configuration_contains_no_offline_policy() {
    let strategy = RuntimeStrategy::Shelf {
        policy: ShelfPolicy::FirstFit,
    };
    let config = RuntimeConfig::builder()
        .page_config(PageConfig::default())
        .strategy(strategy)
        .build()
        .expect("valid runtime config");

    assert_eq!(config.page_config(), &PageConfig::default());
    assert_eq!(config.strategy(), &strategy);
}

#[test]
fn runtime_strategy_variants_own_their_specific_options() {
    let strategies = [
        RuntimeStrategy::Guillotine {
            choice: GuillotineChoice::BestShortSideFit,
            split: GuillotineSplit::SplitMinimizeArea,
        },
        RuntimeStrategy::Shelf {
            policy: ShelfPolicy::NextFit,
        },
        RuntimeStrategy::Skyline {
            heuristic: SkylineHeuristic::BottomLeft,
        },
    ];

    for strategy in strategies {
        let config = RuntimeConfig::builder()
            .strategy(strategy)
            .build()
            .expect("valid strategy");
        assert_eq!(config.strategy(), &strategy);
    }
}

#[test]
fn zero_page_dimensions_are_rejected() {
    assert!(
        PageConfig::builder()
            .max_dimensions(0, 1024)
            .build()
            .is_err()
    );
    assert!(
        PageConfig::builder()
            .max_dimensions(1024, 0)
            .build()
            .is_err()
    );
}

#[test]
fn border_exhaustion_is_rejected() {
    let error = PageConfig::builder()
        .max_dimensions(8, 8)
        .border_padding(4)
        .build()
        .expect_err("border consumes the page");

    assert!(error.to_string().contains("no usable area"));
}

#[test]
fn arithmetic_overflow_is_rejected() {
    let border_error = PageConfig::builder()
        .max_dimensions(u32::MAX, u32::MAX)
        .border_padding(u32::MAX)
        .texture_padding(0)
        .build()
        .expect_err("border multiplication overflows");
    let reservation_error = PageConfig::builder()
        .max_dimensions(u32::MAX, u32::MAX)
        .texture_padding(0)
        .texture_extrusion(u32::MAX)
        .build()
        .expect_err("reservation multiplication overflows");

    assert!(border_error.to_string().contains("overflows"));
    assert!(reservation_error.to_string().contains("overflow"));
}

#[test]
fn impossible_minimum_reservation_is_rejected() {
    let error = PageConfig::builder()
        .max_dimensions(8, 8)
        .texture_padding(0)
        .texture_extrusion(4)
        .build()
        .expect_err("a one-pixel item needs a 9x9 reservation");

    assert!(error.to_string().contains("at least 9x9"));
}

#[test]
fn exact_minimum_reservation_is_accepted() {
    let page = PageConfig::builder()
        .max_dimensions(5, 5)
        .border_padding(1)
        .texture_padding(0)
        .texture_extrusion(1)
        .build()
        .expect("the usable 3x3 area exactly fits one reserved pixel");

    assert_eq!(page.max_dimensions(), (5, 5));
}

#[test]
fn power_of_two_rounding_overflow_is_rejected_unless_forcing_max_dimensions() {
    let page = PageConfig::builder()
        .max_dimensions(u32::MAX, u32::MAX)
        .texture_padding(0)
        .build()
        .expect("page geometry itself is representable");

    assert!(
        OfflineConfig::builder()
            .page_config(page.clone())
            .power_of_two(true)
            .build()
            .is_err()
    );
    assert!(
        OfflineConfig::builder()
            .page_config(page)
            .force_max_dimensions(true)
            .power_of_two(true)
            .build()
            .is_ok()
    );
}

#[test]
fn zero_auto_budget_is_normalized_to_disabled() {
    let config = OfflineConfig::builder()
        .strategy(PackingStrategy::Auto {
            mode: AutoMode::Fast,
            time_budget: Some(Duration::ZERO),
            parallel: false,
            reference_time_threshold: Some(Duration::ZERO),
            reference_input_threshold: Some(0),
        })
        .build()
        .expect("zero budget keeps the existing disabled behavior");

    assert_eq!(
        config.strategy(),
        &PackingStrategy::Auto {
            mode: AutoMode::Fast,
            time_budget: None,
            parallel: false,
            reference_time_threshold: Some(Duration::ZERO),
            reference_input_threshold: Some(0),
        }
    );
}
