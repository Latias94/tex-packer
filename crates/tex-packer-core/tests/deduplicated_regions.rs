use image::{DynamicImage, Rgba, RgbaImage};
use tex_packer_core::config::{
    AutoMode, GuillotineChoice, GuillotineSplit, MaxRectsHeuristic, OfflineConfig,
    OfflineConfigBuilder, PackingStrategy, PageConfig, SkylineHeuristic, SortOrder,
};
use tex_packer_core::export::to_json_hash;
use tex_packer_core::model::{Rect, ResolvedFrame};
use tex_packer_core::offline::{InputImage, OfflinePacker, PackOutput};

fn pack_images(
    inputs: Vec<InputImage>,
    config: OfflineConfig,
) -> tex_packer_core::error::Result<PackOutput> {
    OfflinePacker::new(config).pack_images(inputs)
}

fn input(key: &str, image: RgbaImage) -> InputImage {
    InputImage {
        key: key.to_string(),
        image: DynamicImage::ImageRgba8(image),
    }
}

fn solid(width: u32, height: u32, color: [u8; 4]) -> RgbaImage {
    RgbaImage::from_pixel(width, height, Rgba(color))
}

fn page_config(
    width: u32,
    height: u32,
    allow_rotation: bool,
    border_padding: u32,
    texture_padding: u32,
    texture_extrusion: u32,
) -> PageConfig {
    PageConfig::builder()
        .max_dimensions(width, height)
        .allow_rotation(allow_rotation)
        .border_padding(border_padding)
        .texture_padding(texture_padding)
        .texture_extrusion(texture_extrusion)
        .build()
        .expect("valid page config")
}

fn config_builder(width: u32, height: u32) -> OfflineConfigBuilder {
    OfflineConfig::builder()
        .page_config(page_config(width, height, false, 0, 0, 0))
        .trim(false)
        .strategy(PackingStrategy::Skyline {
            heuristic: SkylineHeuristic::BottomLeft,
            use_waste_map: false,
        })
}

fn config(width: u32, height: u32) -> OfflineConfig {
    config_builder(width, height)
        .build()
        .expect("valid offline config")
}

fn resolved_frame<'a>(output: &'a PackOutput, key: &str) -> ResolvedFrame<'a> {
    output
        .atlas()
        .pages()
        .iter()
        .flat_map(|page| page.resolved_frames())
        .find(|resolved| resolved.frame().key() == key)
        .unwrap_or_else(|| panic!("missing frame {key}"))
}

#[test]
fn identical_images_share_region_and_keep_logical_keys() {
    let image = solid(4, 3, [12, 34, 56, 255]);
    let output = pack_images(
        vec![input("hero", image.clone()), input("hero_copy", image)],
        config(16, 16),
    )
    .expect("pack identical images");

    let hero = resolved_frame(&output, "hero");
    let copy = resolved_frame(&output, "hero_copy");
    assert_eq!(hero.page_id(), copy.page_id());
    assert_eq!(hero.region().id(), copy.region().id());
    assert_eq!(hero.region().content(), copy.region().content());
    assert_eq!(hero.region().rotated(), copy.region().rotated());

    let stats = output.stats();
    assert_eq!(stats.num_frames, 2);
    assert_eq!(stats.num_regions, 1);
    assert_eq!(stats.num_aliases, 1);
    assert_eq!(stats.content_area, 12);

    let exported = to_json_hash(output.atlas());
    let frames = exported["frames"].as_object().expect("frame map");
    assert!(frames.contains_key("hero"));
    assert!(frames.contains_key("hero_copy"));
    assert_eq!(frames["hero"]["frame"], frames["hero_copy"]["frame"]);
}

#[test]
fn logical_frames_keep_prepared_order() {
    let duplicate = solid(2, 2, [255, 0, 0, 255]);
    let cfg = config_builder(16, 16)
        .sort_order(SortOrder::None)
        .build()
        .expect("valid offline config");
    let output = pack_images(
        vec![
            input("original", duplicate.clone()),
            input("middle", solid(1, 1, [0, 0, 255, 255])),
            input("alias", duplicate),
        ],
        cfg,
    )
    .expect("pack frames in input order");

    let keys = output.atlas().pages()[0]
        .frames()
        .iter()
        .map(|frame| frame.key())
        .collect::<Vec<_>>();
    assert_eq!(keys, ["original", "middle", "alias"]);
}

#[test]
fn duplicate_keys_keep_distinct_logical_identities() {
    let image = solid(2, 2, [255, 0, 0, 255]);
    let cfg = config_builder(16, 16)
        .sort_order(SortOrder::None)
        .build()
        .expect("valid offline config");
    let output = pack_images(
        vec![input("duplicate", image.clone()), input("duplicate", image)],
        cfg,
    )
    .expect("pack duplicate keys");

    let page = &output.atlas().pages()[0];
    assert_eq!(page.frames().len(), 2);
    assert_eq!(page.frames()[0].key(), "duplicate");
    assert_eq!(page.frames()[1].key(), "duplicate");
    assert_ne!(page.frames()[0].id(), page.frames()[1].id());
    assert_eq!(page.frames()[0].region_id(), page.frames()[1].region_id());
}

#[test]
fn duplicate_keys_with_distinct_pixels_render_by_frame_identity() {
    let cfg = config_builder(16, 16)
        .sort_order(SortOrder::None)
        .build()
        .expect("valid offline config");
    let output = pack_images(
        vec![
            input("duplicate", solid(2, 2, [255, 0, 0, 255])),
            input("duplicate", solid(2, 2, [0, 0, 255, 255])),
        ],
        cfg,
    )
    .expect("pack duplicate keys with distinct pixels");

    let page = &output.atlas().pages()[0];
    let resolved = page.resolved_frames().collect::<Vec<_>>();
    assert_eq!(resolved.len(), 2);
    assert_ne!(resolved[0].frame().id(), resolved[1].frame().id());
    assert_ne!(resolved[0].region().id(), resolved[1].region().id());

    let rendered = output
        .pages()
        .iter()
        .find(|rendered| rendered.page_id() == page.id())
        .expect("rendered page");
    let first = resolved[0].region().content();
    let second = resolved[1].region().content();
    assert_eq!(
        rendered.rgba().get_pixel(first.x, first.y),
        &Rgba([255, 0, 0, 255])
    );
    assert_eq!(
        rendered.rgba().get_pixel(second.x, second.y),
        &Rgba([0, 0, 255, 255])
    );
}

#[test]
fn matching_bytes_with_different_dimensions_are_distinct_regions() {
    let pixels = vec![1, 2, 3, 255, 5, 6, 7, 255];
    let vertical = RgbaImage::from_raw(1, 2, pixels.clone()).expect("vertical image");
    let horizontal = RgbaImage::from_raw(2, 1, pixels).expect("horizontal image");

    let output = pack_images(
        vec![input("vertical", vertical), input("horizontal", horizontal)],
        config(8, 8),
    )
    .expect("pack differently shaped images");

    let vertical = resolved_frame(&output, "vertical");
    let horizontal = resolved_frame(&output, "horizontal");
    assert_eq!(
        (vertical.region().content().w, vertical.region().content().h),
        (1, 2)
    );
    assert_eq!(
        (
            horizontal.region().content().w,
            horizontal.region().content().h
        ),
        (2, 1)
    );
    assert_ne!(vertical.region().content(), horizontal.region().content());

    let stats = output.stats();
    assert_eq!(stats.num_regions, 2);
    assert_eq!(stats.num_aliases, 0);
}

#[test]
fn trimmed_aliases_share_pixels_and_keep_source_metadata() {
    let mut first = RgbaImage::new(4, 5);
    first.put_pixel(0, 1, Rgba([255, 0, 0, 255]));
    first.put_pixel(0, 2, Rgba([0, 255, 0, 255]));

    let mut second = RgbaImage::new(7, 6);
    second.put_pixel(5, 3, Rgba([255, 0, 0, 255]));
    second.put_pixel(5, 4, Rgba([0, 255, 0, 255]));

    let cfg = config_builder(16, 16)
        .trim(true)
        .build()
        .expect("valid offline config");
    let output = pack_images(vec![input("first", first), input("second", second)], cfg)
        .expect("pack trimmed aliases");

    let first = resolved_frame(&output, "first");
    let second = resolved_frame(&output, "second");
    assert_eq!(first.region().id(), second.region().id());
    assert_eq!(first.frame().source(), Rect::new(0, 1, 1, 2));
    assert_eq!(first.frame().source_size(), (4, 5));
    assert_eq!(second.frame().source(), Rect::new(5, 3, 1, 2));
    assert_eq!(second.frame().source_size(), (7, 6));
    assert!(first.frame().trimmed());
    assert!(second.frame().trimmed());
}

#[test]
fn duplicate_region_on_later_page_is_placed_once_without_panicking() {
    let full = solid(4, 4, [255, 0, 0, 255]);
    let duplicate = solid(2, 2, [0, 0, 255, 255]);
    let output = pack_images(
        vec![
            input("full", full),
            input("copy_a", duplicate.clone()),
            input("copy_b", duplicate),
        ],
        config(4, 4),
    )
    .expect("pack duplicate on later page");

    assert_eq!(output.pages().len(), 2);
    let first = resolved_frame(&output, "copy_a");
    let second = resolved_frame(&output, "copy_b");
    assert_eq!(first.page_id(), second.page_id());
    assert_eq!(first.region().id(), second.region().id());

    let stats = output.stats();
    assert_eq!(stats.num_frames, 3);
    assert_eq!(stats.num_regions, 2);
    assert_eq!(stats.num_aliases, 1);
    assert!(stats.content_occupancy <= 1.0);

    let page = output
        .pages()
        .iter()
        .find(|page| page.page_id() == first.page_id())
        .expect("rendered page for alias");
    let content = first.region().content();
    assert_eq!(
        page.rgba().get_pixel(content.x, content.y),
        &Rgba([0, 0, 255, 255])
    );
}

#[test]
fn shared_region_respects_rotation_padding_and_extrusion() {
    let image = solid(4, 2, [90, 120, 150, 255]);
    let cfg = OfflineConfig::builder()
        .page_config(page_config(8, 10, true, 1, 2, 1))
        .trim(false)
        .strategy(PackingStrategy::Skyline {
            heuristic: SkylineHeuristic::BottomLeft,
            use_waste_map: false,
        })
        .build()
        .expect("valid offline config");
    let output = pack_images(
        vec![input("original", image.clone()), input("alias", image)],
        cfg,
    )
    .expect("pack rotated aliases with reserved spacing");

    let original = resolved_frame(&output, "original");
    let alias = resolved_frame(&output, "alias");
    assert!(original.region().rotated());
    assert_eq!(original.region().id(), alias.region().id());
    assert_eq!(
        (original.region().content().w, original.region().content().h),
        (2, 4)
    );
    assert_eq!(output.stats().content_area, 8);

    let page = &output.pages()[0];
    assert_eq!(page.rgba().dimensions(), (8, 10));
    let content = original.region().content();
    assert_eq!(
        page.rgba().get_pixel(content.x - 1, content.y),
        &Rgba([90, 120, 150, 255])
    );
}

#[test]
fn every_offline_algorithm_reuses_identical_content() {
    for strategy in [
        PackingStrategy::Skyline {
            heuristic: SkylineHeuristic::BottomLeft,
            use_waste_map: false,
        },
        PackingStrategy::MaxRects {
            heuristic: MaxRectsHeuristic::BestAreaFit,
            reference: false,
        },
        PackingStrategy::Guillotine {
            choice: GuillotineChoice::BestAreaFit,
            split: GuillotineSplit::SplitShorterLeftoverAxis,
        },
        PackingStrategy::Auto {
            mode: AutoMode::Quality,
            time_budget: None,
            parallel: false,
            reference_time_threshold: None,
            reference_input_threshold: None,
        },
    ] {
        let image = solid(3, 2, [17, 29, 43, 255]);
        let cfg = config_builder(16, 16)
            .strategy(strategy)
            .build()
            .expect("valid offline config");
        let output = pack_images(
            vec![input("original", image.clone()), input("alias", image)],
            cfg,
        )
        .expect("pack with algorithm");

        assert_eq!(
            resolved_frame(&output, "original").region().id(),
            resolved_frame(&output, "alias").region().id()
        );
        assert_eq!(output.stats().num_regions, 1);
    }
}
