use image::{DynamicImage, Rgba, RgbaImage};
use std::collections::HashMap;
use tex_packer_core::TexPackerError;
use tex_packer_core::prelude::*;

#[test]
fn layout_and_images_have_same_geometry() {
    // Trimming off to avoid data-dependent changes
    let page = PageConfig::builder()
        .max_dimensions(256, 256)
        .allow_rotation(true)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .trim(false)
        .build()
        .expect("valid offline config");

    // Build small set with varied sizes
    let sizes = vec![("a", 40, 20), ("b", 16, 32), ("c", 10, 10), ("d", 8, 48)];
    // layout-only
    let atlas_layout = tex_packer_core::pack_layout(
        sizes.iter().map(|(k, w, h)| (*k, *w, *h)).collect(),
        cfg.clone(),
    )
    .expect("layout");

    // images path
    let mut inputs: Vec<InputImage> = Vec::new();
    for (k, w, h) in &sizes {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(*w, *h));
        inputs.push(InputImage {
            key: (*k).to_string(),
            image: img,
        });
    }
    let out = tex_packer_core::pack_images(inputs, cfg).expect("images");

    assert_output_pages_match_atlas(&out);
    assert_atlas_geometry_matches(&atlas_layout, &out.atlas);
}

#[test]
fn layout_and_images_match_for_forced_rotation() {
    let page = PageConfig::builder()
        .max_dimensions(16, 12)
        .allow_rotation(true)
        .texture_padding(0)
        .texture_extrusion(0)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .trim(false)
        .sort_order(SortOrder::None)
        .build()
        .expect("valid offline config");

    let atlas_layout =
        tex_packer_core::pack_layout(vec![("rotated", 8, 14)], cfg.clone()).expect("layout");
    let out = tex_packer_core::pack_images(
        vec![InputImage {
            key: "rotated".to_string(),
            image: DynamicImage::ImageRgba8(RgbaImage::new(8, 14)),
        }],
        cfg,
    )
    .expect("images");

    assert_output_pages_match_atlas(&out);
    assert_atlas_geometry_matches(&atlas_layout, &out.atlas);

    let frame = &out.atlas.pages[0].frames[0];
    assert!(frame.rotated);
    assert_eq!(frame.frame.w, 14);
    assert_eq!(frame.frame.h, 8);
}

#[test]
fn layout_items_and_trimmed_images_match_with_padding_extrude_and_page_sizing() {
    let page = PageConfig::builder()
        .max_dimensions(256, 128)
        .allow_rotation(true)
        .texture_padding(4)
        .texture_extrusion(2)
        .border_padding(3)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .trim(true)
        .trim_threshold(0)
        .power_of_two(true)
        .square(true)
        .build()
        .expect("valid offline config");
    let specs = [
        ("a", 40, 30, Rect::new(5, 4, 12, 18)),
        ("b", 38, 34, Rect::new(10, 8, 20, 10)),
        ("c", 32, 22, Rect::new(0, 1, 9, 17)),
    ];

    let image_inputs = specs
        .iter()
        .map(|(key, source_w, source_h, opaque)| InputImage {
            key: (*key).to_string(),
            image: DynamicImage::ImageRgba8(image_with_opaque_rect(*source_w, *source_h, *opaque)),
        })
        .collect();
    let layout_items = specs
        .iter()
        .map(|(key, source_w, source_h, opaque)| LayoutItem {
            key: *key,
            w: opaque.w,
            h: opaque.h,
            source: Some(*opaque),
            source_size: Some((*source_w, *source_h)),
            trimmed: true,
        })
        .collect();

    let atlas_layout = tex_packer_core::pack_layout_items(layout_items, cfg.clone())
        .expect("layout items should pack");
    let out = tex_packer_core::pack_images(image_inputs, cfg).expect("images should pack");

    assert_output_pages_match_atlas(&out);
    assert_atlas_geometry_matches(&atlas_layout, &out.atlas);
    for page in &out.atlas.pages {
        assert_eq!(page.width, page.height);
        assert!(is_pow2(page.width));
        for frame in &page.frames {
            assert!(frame.trimmed);
        }
    }
}

#[test]
fn layout_and_images_match_when_packing_spills_to_multiple_pages() {
    let page = PageConfig::builder()
        .max_dimensions(64, 64)
        .allow_rotation(false)
        .texture_padding(0)
        .texture_extrusion(0)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .trim(false)
        .sort_order(SortOrder::None)
        .build()
        .expect("valid offline config");
    let sizes = [("a", 40, 40), ("b", 40, 40), ("c", 40, 40)];

    let atlas_layout = tex_packer_core::pack_layout(
        sizes.iter().map(|(key, w, h)| (*key, *w, *h)).collect(),
        cfg.clone(),
    )
    .expect("layout should pack");
    let out = tex_packer_core::pack_images(
        sizes
            .iter()
            .enumerate()
            .map(|(index, (key, w, h))| InputImage {
                key: (*key).to_string(),
                image: DynamicImage::ImageRgba8(RgbaImage::from_pixel(
                    *w,
                    *h,
                    Rgba([index as u8, 0, 0, 255]),
                )),
            })
            .collect(),
        cfg,
    )
    .expect("images should pack");

    assert!(out.atlas.pages.len() > 1);
    assert_output_pages_match_atlas(&out);
    assert_atlas_geometry_matches(&atlas_layout, &out.atlas);
}

#[test]
fn layout_and_images_report_same_out_of_space_progress() {
    let page = PageConfig::builder()
        .max_dimensions(32, 32)
        .texture_padding(0)
        .texture_extrusion(0)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .trim(false)
        .build()
        .expect("valid offline config");

    let layout_err =
        tex_packer_core::pack_layout(vec![("too_big", 64, 64)], cfg.clone()).unwrap_err();
    let image_err = match tex_packer_core::pack_images(
        vec![InputImage {
            key: "too_big".to_string(),
            image: DynamicImage::ImageRgba8(RgbaImage::new(64, 64)),
        }],
        cfg,
    ) {
        Ok(_) => panic!("image packing should fail"),
        Err(err) => err,
    };

    assert_eq!(
        out_of_space_progress(layout_err),
        out_of_space_progress(image_err)
    );
}

#[derive(Debug, PartialEq, Eq)]
struct FrameRecord {
    page_id: usize,
    page_size: (u32, u32),
    frame: Rect,
    rotated: bool,
    trimmed: bool,
    source: Rect,
    source_size: (u32, u32),
}

fn assert_atlas_geometry_matches(expected: &Atlas<String>, actual: &Atlas<String>) {
    assert_eq!(expected.pages.len(), actual.pages.len());
    assert_eq!(frame_records(expected), frame_records(actual));
}

fn assert_output_pages_match_atlas(out: &PackOutput) {
    assert_eq!(out.pages.len(), out.atlas.pages.len());
    for (rendered, atlas_page) in out.pages.iter().zip(out.atlas.pages.iter()) {
        assert_eq!(rendered.page.id, atlas_page.id);
        assert_eq!(rendered.page.width, atlas_page.width);
        assert_eq!(rendered.page.height, atlas_page.height);
        assert_eq!(
            rendered.rgba.dimensions(),
            (atlas_page.width, atlas_page.height)
        );
    }
}

fn frame_records(atlas: &Atlas<String>) -> HashMap<String, FrameRecord> {
    let mut records = HashMap::new();
    for page in &atlas.pages {
        for frame in &page.frames {
            records.insert(
                frame.key.clone(),
                FrameRecord {
                    page_id: page.id,
                    page_size: (page.width, page.height),
                    frame: frame.frame,
                    rotated: frame.rotated,
                    trimmed: frame.trimmed,
                    source: frame.source,
                    source_size: frame.source_size,
                },
            );
        }
    }
    records
}

fn image_with_opaque_rect(width: u32, height: u32, opaque: Rect) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    for y in opaque.y..opaque.y + opaque.h {
        for x in opaque.x..opaque.x + opaque.w {
            image.put_pixel(x, y, Rgba([255, 255, 255, 255]));
        }
    }
    image
}

fn is_pow2(v: u32) -> bool {
    v != 0 && (v & (v - 1)) == 0
}

fn out_of_space_progress(err: TexPackerError) -> (usize, usize) {
    match err {
        TexPackerError::OutOfSpaceGeneric { placed, total } => (placed, total),
        other => panic!("expected OutOfSpaceGeneric, got {other:?}"),
    }
}
