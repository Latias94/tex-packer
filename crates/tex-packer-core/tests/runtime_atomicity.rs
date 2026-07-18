use image::{Rgba, RgbaImage};
use tex_packer_core::config::{
    GuillotineChoice, GuillotineSplit, PageConfig, RuntimeConfig, RuntimeStrategy, ShelfPolicy,
    SkylineHeuristic,
};
use tex_packer_core::error::TexPackerError;
use tex_packer_core::model::{Atlas, PageId};
use tex_packer_core::runtime::{RuntimeAtlas, RuntimeStats};

#[derive(Debug, PartialEq)]
struct RuntimeState {
    atlas: Atlas,
    stats: RuntimeStats,
    keys: Vec<String>,
    pixel_page_count: usize,
    pixel_pages: Vec<(PageId, (u32, u32), Vec<u8>)>,
}

fn runtime_config(strategy: RuntimeStrategy) -> RuntimeConfig {
    let page = PageConfig::builder()
        .max_dimensions(96, 96)
        .border_padding(2)
        .texture_padding(4)
        .texture_extrusion(2)
        .allow_rotation(true)
        .build()
        .expect("valid runtime page config");
    RuntimeConfig::builder()
        .page_config(page)
        .strategy(strategy)
        .build()
        .expect("valid runtime config")
}

fn strategies() -> Vec<(&'static str, RuntimeStrategy)> {
    vec![
        (
            "guillotine",
            RuntimeStrategy::Guillotine {
                choice: GuillotineChoice::BestAreaFit,
                split: GuillotineSplit::SplitShorterLeftoverAxis,
            },
        ),
        (
            "shelf",
            RuntimeStrategy::Shelf {
                policy: ShelfPolicy::FirstFit,
            },
        ),
        (
            "skyline",
            RuntimeStrategy::Skyline {
                heuristic: SkylineHeuristic::MinWaste,
            },
        ),
    ]
}

fn solid(width: u32, height: u32, color: [u8; 4]) -> RgbaImage {
    RgbaImage::from_pixel(width, height, Rgba(color))
}

fn capture(atlas: &RuntimeAtlas) -> RuntimeState {
    let snapshot = atlas.snapshot_atlas().expect("valid runtime snapshot");
    assert_eq!(
        snapshot,
        atlas
            .snapshot_atlas()
            .expect("repeated runtime snapshot must be stable")
    );
    let pixel_pages = (0..16)
        .filter_map(|raw_id| {
            let page_id = PageId::new(raw_id);
            atlas
                .get_page_image(page_id)
                .map(|image| (page_id, image.dimensions(), image.as_raw().to_vec()))
        })
        .collect();
    RuntimeState {
        atlas: snapshot,
        stats: atlas.stats(),
        keys: atlas.keys().into_iter().map(str::to_owned).collect(),
        pixel_page_count: atlas.num_pages(),
        pixel_pages,
    }
}

fn assert_failed_append_is_atomic(
    label: &str,
    atlas: &mut RuntimeAtlas,
    key: &str,
    image: &RgbaImage,
) -> TexPackerError {
    let before = capture(atlas);
    let error = atlas
        .append_with_image(key.to_owned(), image)
        .expect_err("append must fail before commit");
    assert_eq!(
        capture(atlas),
        before,
        "{label}: failed append mutated state"
    );
    error
}

#[test]
fn failed_mutations_and_reuse_preserve_state_for_every_runtime_strategy() {
    for (label, strategy) in strategies() {
        let mut atlas = RuntimeAtlas::new(runtime_config(strategy)).with_outlines(true);
        let first = atlas
            .append_with_image("first".into(), &solid(24, 12, [255, 0, 0, 255]))
            .unwrap_or_else(|error| panic!("{label}: append first: {error}"));
        let second = atlas
            .append_with_image("second".into(), &solid(18, 20, [0, 255, 0, 255]))
            .unwrap_or_else(|error| panic!("{label}: append second: {error}"));

        for update in [&first, &second] {
            let placement = update.placement();
            let content = placement.content();
            let allocation = placement.allocation();
            assert!(
                allocation.contains(&content),
                "{label}: content containment"
            );
            assert_eq!(allocation.w - content.w, 8, "{label}: horizontal reserve");
            assert_eq!(allocation.h - content.h, 8, "{label}: vertical reserve");
            assert_eq!(
                update.dirty_region().width,
                content.w + 4,
                "{label}: extrusion width"
            );
            assert_eq!(
                update.dirty_region().height,
                content.h + 4,
                "{label}: extrusion height"
            );
            let source = placement.frame().source();
            let expected_content = if placement.rotated() {
                (source.h, source.w)
            } else {
                (source.w, source.h)
            };
            assert_eq!(
                (content.w, content.h),
                expected_content,
                "{label}: rotation"
            );
        }
        let second_content = second.placement().content();
        assert_eq!(
            atlas
                .get_page_image(second.placement().page_id())
                .expect("second upload page")
                .get_pixel(second_content.x + 1, second_content.y + 1),
            &Rgba([0, 255, 0, 255]),
            "{label}: staged patch landed at the resolved destination"
        );
        assert_eq!(atlas.keys(), vec!["first", "second"], "{label}: key order");

        let duplicate_error = assert_failed_append_is_atomic(
            label,
            &mut atlas,
            "first",
            &solid(8, 8, [0, 0, 255, 255]),
        );
        assert!(matches!(
            duplicate_error,
            TexPackerError::DuplicateKey { ref key } if key == "first"
        ));
        assert!(matches!(
            assert_failed_append_is_atomic(label, &mut atlas, "zero", &RgbaImage::new(0, 8)),
            TexPackerError::InvalidDimensions {
                width: 0,
                height: 8
            }
        ));
        assert!(!atlas.contains("zero"), "{label}: zero key polluted index");
        assert!(matches!(
            assert_failed_append_is_atomic(
                label,
                &mut atlas,
                "oversized",
                &solid(192, 192, [0, 0, 0, 255]),
            ),
            TexPackerError::TextureTooLarge { .. } | TexPackerError::OutOfSpace { .. }
        ));
        assert!(
            !atlas.contains("oversized"),
            "{label}: oversized key polluted index"
        );

        let first_placement = first.placement();
        assert!(
            atlas
                .evict_with_clear(first_placement.page_id(), "first", true)
                .is_some(),
            "{label}: evict first"
        );
        let third = atlas
            .append_with_image("third".into(), &solid(16, 10, [0, 0, 255, 255]))
            .unwrap_or_else(|error| panic!("{label}: append third: {error}"));
        assert_ne!(
            third.placement().frame_id(),
            first_placement.frame_id(),
            "{label}: frame identity was retargeted"
        );
        assert_ne!(
            third.placement().region_id(),
            first_placement.region_id(),
            "{label}: region identity was retargeted"
        );
        assert_eq!(atlas.keys(), vec!["second", "third"], "{label}: stable IDs");
        let final_state = capture(&atlas);
        let page = final_state
            .atlas
            .page(first_placement.page_id())
            .expect("original page remains stable");
        assert!(page.frame(first_placement.frame_id()).is_none());
        assert!(page.frame(third.placement().frame_id()).is_some());
    }
}

#[test]
fn pixel_staging_failure_does_not_commit_geometry_or_identities() {
    for (label, strategy) in strategies() {
        let mut atlas = RuntimeAtlas::new(runtime_config(strategy));
        let first = atlas
            .append_with_image("first".into(), &solid(16, 16, [255, 0, 0, 255]))
            .unwrap_or_else(|error| panic!("{label}: append first: {error}"));
        let page_id = first.placement().page_id();
        *atlas
            .get_page_image_mut(page_id)
            .expect("first append creates a pixel page") = solid(1, 1, [9, 9, 9, 255]);

        let error = assert_failed_append_is_atomic(
            label,
            &mut atlas,
            "second",
            &solid(12, 12, [0, 255, 0, 255]),
        );
        assert!(matches!(error, TexPackerError::InvariantViolation { .. }));
        assert!(
            !atlas.contains("second"),
            "{label}: failed pixel staging polluted key index"
        );

        *atlas
            .get_page_image_mut(page_id)
            .expect("pixel page remains addressable") = solid(96, 96, [0, 0, 0, 0]);
        let second = atlas
            .append_with_image("second".into(), &solid(12, 12, [0, 255, 0, 255]))
            .unwrap_or_else(|error| panic!("{label}: append after repair: {error}"));
        assert_eq!(second.placement().frame_id().get(), 1, "{label}: frame ID");
        assert_eq!(
            second.placement().region_id().get(),
            1,
            "{label}: region ID"
        );
    }
}

#[test]
fn eviction_with_the_wrong_page_id_is_a_noop() {
    for (label, strategy) in strategies() {
        let mut atlas = RuntimeAtlas::new(runtime_config(strategy));
        let first = atlas
            .append_with_image("first".into(), &solid(40, 40, [255, 0, 0, 255]))
            .unwrap_or_else(|error| panic!("{label}: append first: {error}"));
        let second = atlas
            .append_with_image("second".into(), &solid(40, 40, [0, 255, 0, 255]))
            .unwrap_or_else(|error| panic!("{label}: append second: {error}"));
        assert_ne!(first.placement().page_id(), second.placement().page_id());

        let before = capture(&atlas);
        assert_eq!(
            atlas.evict_with_clear(second.placement().page_id(), "first", true),
            None,
            "{label}: wrong-page eviction must fail"
        );
        assert_eq!(
            capture(&atlas),
            before,
            "{label}: wrong-page eviction mutated state"
        );
        assert!(atlas.contains("first"), "{label}: key index lost first");
    }
}
