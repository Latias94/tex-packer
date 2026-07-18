use image::{DynamicImage, Rgba, RgbaImage};
use tex_packer_core::{
    AlgorithmFamily, Frame, InputImage, PackOutput, PackerConfig, Rect, SortOrder, pack_images,
    to_json_hash,
};

fn input(key: &str, image: RgbaImage) -> InputImage {
    InputImage {
        key: key.to_string(),
        image: DynamicImage::ImageRgba8(image),
    }
}

fn solid(width: u32, height: u32, color: [u8; 4]) -> RgbaImage {
    RgbaImage::from_pixel(width, height, Rgba(color))
}

fn config(width: u32, height: u32) -> PackerConfig {
    PackerConfig {
        max_width: width,
        max_height: height,
        allow_rotation: false,
        border_padding: 0,
        texture_padding: 0,
        texture_extrusion: 0,
        trim: false,
        family: AlgorithmFamily::Skyline,
        ..Default::default()
    }
}

fn frame<'a>(output: &'a PackOutput, key: &str) -> (usize, &'a Frame) {
    output
        .atlas
        .pages
        .iter()
        .find_map(|page| {
            page.frames
                .iter()
                .find(|frame| frame.key == key)
                .map(|frame| (page.id, frame))
        })
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

    let (hero_page, hero) = frame(&output, "hero");
    let (copy_page, copy) = frame(&output, "hero_copy");
    assert_eq!(hero_page, copy_page);
    assert_eq!(hero.frame, copy.frame);
    assert_eq!(hero.rotated, copy.rotated);

    let stats = output.stats();
    assert_eq!(stats.num_frames, 2);
    assert_eq!(stats.num_regions, 1);
    assert_eq!(stats.num_deduplicated, 1);
    assert_eq!(stats.used_region_area, 12);

    let exported = to_json_hash(&output.atlas);
    let frames = exported["frames"].as_object().expect("frame map");
    assert!(frames.contains_key("hero"));
    assert!(frames.contains_key("hero_copy"));
    assert_eq!(frames["hero"]["frame"], frames["hero_copy"]["frame"]);
}

#[test]
fn logical_frames_keep_prepared_order() {
    let duplicate = solid(2, 2, [255, 0, 0, 255]);
    let mut cfg = config(16, 16);
    cfg.sort_order = SortOrder::None;
    let output = pack_images(
        vec![
            input("original", duplicate.clone()),
            input("middle", solid(1, 1, [0, 0, 255, 255])),
            input("alias", duplicate),
        ],
        cfg,
    )
    .expect("pack frames in input order");

    let keys = output.atlas.pages[0]
        .frames
        .iter()
        .map(|frame| frame.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(keys, ["original", "middle", "alias"]);
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

    let (_, vertical) = frame(&output, "vertical");
    let (_, horizontal) = frame(&output, "horizontal");
    assert_eq!((vertical.frame.w, vertical.frame.h), (1, 2));
    assert_eq!((horizontal.frame.w, horizontal.frame.h), (2, 1));
    assert_ne!(vertical.frame, horizontal.frame);

    let stats = output.stats();
    assert_eq!(stats.num_regions, 2);
    assert_eq!(stats.num_deduplicated, 0);
}

#[test]
fn trimmed_aliases_share_pixels_and_keep_source_metadata() {
    let mut first = RgbaImage::new(4, 5);
    first.put_pixel(0, 1, Rgba([255, 0, 0, 255]));
    first.put_pixel(0, 2, Rgba([0, 255, 0, 255]));

    let mut second = RgbaImage::new(7, 6);
    second.put_pixel(5, 3, Rgba([255, 0, 0, 255]));
    second.put_pixel(5, 4, Rgba([0, 255, 0, 255]));

    let mut cfg = config(16, 16);
    cfg.trim = true;
    let output = pack_images(vec![input("first", first), input("second", second)], cfg)
        .expect("pack trimmed aliases");

    let (_, first) = frame(&output, "first");
    let (_, second) = frame(&output, "second");
    assert_eq!(first.frame, second.frame);
    assert_eq!(first.source, Rect::new(0, 1, 1, 2));
    assert_eq!(first.source_size, (4, 5));
    assert_eq!(second.source, Rect::new(5, 3, 1, 2));
    assert_eq!(second.source_size, (7, 6));
    assert!(first.trimmed);
    assert!(second.trimmed);
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

    assert_eq!(output.pages.len(), 2);
    let (first_page, first) = frame(&output, "copy_a");
    let (second_page, second) = frame(&output, "copy_b");
    assert_eq!(first_page, second_page);
    assert_eq!(first.frame, second.frame);

    let stats = output.stats();
    assert_eq!(stats.num_frames, 3);
    assert_eq!(stats.num_regions, 2);
    assert_eq!(stats.num_deduplicated, 1);
    assert!(stats.occupancy <= 1.0);

    let page = &output.pages[first_page];
    assert_eq!(
        page.rgba.get_pixel(first.frame.x, first.frame.y),
        &Rgba([0, 0, 255, 255])
    );
}

#[test]
fn shared_region_respects_rotation_padding_and_extrusion() {
    let image = solid(4, 2, [90, 120, 150, 255]);
    let mut cfg = config(8, 10);
    cfg.allow_rotation = true;
    cfg.border_padding = 1;
    cfg.texture_padding = 2;
    cfg.texture_extrusion = 1;
    let output = pack_images(
        vec![input("original", image.clone()), input("alias", image)],
        cfg,
    )
    .expect("pack rotated aliases with reserved spacing");

    let (_, original) = frame(&output, "original");
    let (_, alias) = frame(&output, "alias");
    assert!(original.rotated);
    assert_eq!(original.frame, alias.frame);
    assert_eq!((original.frame.w, original.frame.h), (2, 4));
    assert_eq!(output.stats().used_region_area, 8);

    let page = &output.pages[0];
    assert_eq!((page.rgba.width(), page.rgba.height()), (8, 10));
    assert_eq!(
        page.rgba.get_pixel(original.frame.x - 1, original.frame.y),
        &Rgba([90, 120, 150, 255])
    );
}

#[test]
fn every_offline_algorithm_reuses_identical_content() {
    for family in [
        AlgorithmFamily::Skyline,
        AlgorithmFamily::MaxRects,
        AlgorithmFamily::Guillotine,
        AlgorithmFamily::Auto,
    ] {
        let image = solid(3, 2, [17, 29, 43, 255]);
        let mut cfg = config(16, 16);
        cfg.family = family;
        let output = pack_images(
            vec![input("original", image.clone()), input("alias", image)],
            cfg,
        )
        .expect("pack with algorithm");

        assert_eq!(
            frame(&output, "original").1.frame,
            frame(&output, "alias").1.frame
        );
        assert_eq!(output.stats().num_regions, 1);
    }
}
