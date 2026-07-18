use image::{DynamicImage, Rgba, RgbaImage};
use tex_packer_core::config::{OfflineConfig, PackingStrategy, PageConfig, SkylineHeuristic};
use tex_packer_core::{InputImage, pack_images};

fn solid_image(w: u32, h: u32, rgba: [u8; 4]) -> DynamicImage {
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            img.put_pixel(x, y, Rgba(rgba));
        }
    }
    DynamicImage::ImageRgba8(img)
}

#[test]
fn extrude_does_not_bleed_across_neighbors() {
    let red = solid_image(32, 32, [255, 0, 0, 255]);
    let green = solid_image(32, 32, [0, 255, 0, 255]);
    let inputs = vec![
        InputImage {
            key: "red".into(),
            image: red,
        },
        InputImage {
            key: "green".into(),
            image: green,
        },
    ];

    let page = PageConfig::builder()
        .max_dimensions(128, 128)
        .allow_rotation(false)
        .texture_padding(4)
        .texture_extrusion(2)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .trim(false)
        .strategy(PackingStrategy::Skyline {
            heuristic: SkylineHeuristic::BottomLeft,
            use_waste_map: false,
        })
        .build()
        .expect("valid offline config");

    let out = pack_images(inputs, cfg).expect("pack");
    assert_eq!(out.pages.len(), 1);
    let page = &out.pages[0];
    let rgba = &page.rgba;
    let atlas_page = out
        .atlas
        .page(page.page_id)
        .expect("rendered page must resolve in atlas");

    // Find frames
    let mut red_f = None;
    let mut green_f = None;
    for resolved in atlas_page.resolved_frames() {
        if resolved.frame().key() == "red" {
            red_f = Some(resolved.region().content());
        }
        if resolved.frame().key() == "green" {
            green_f = Some(resolved.region().content());
        }
    }
    let red_f = red_f.expect("red frame");
    let green_f = green_f.expect("green frame");

    // Ensure there's at least one pixel gap between content frames (due to padding/extrude reservations)
    // and check border pixels adjacent to content are of correct color (i.e., extruded from the same content,
    // not contaminated by the neighbor).
    let _red_edge = (red_f.x + red_f.w - 1, red_f.y + red_f.h - 1);
    let _green_edge = (green_f.x + green_f.w - 1, green_f.y + green_f.h - 1);
    // Sample a few pixels just outside content area (if within bounds) and ensure they match the owner's color
    let sample = |x: u32, y: u32| -> [u8; 4] { rgba.get_pixel(x, y).0 };

    // Right of red content
    if red_f.x + red_f.w + 1 < rgba.width() {
        let p = sample(red_f.x + red_f.w, red_f.y);
        assert_eq!(p, [255, 0, 0, 255]);
    }
    // Below red content
    if red_f.y + red_f.h + 1 < rgba.height() {
        let p = sample(red_f.x, red_f.y + red_f.h);
        assert_eq!(p, [255, 0, 0, 255]);
    }
    // Right of green content
    if green_f.x + green_f.w + 1 < rgba.width() {
        let p = sample(green_f.x + green_f.w, green_f.y);
        assert_eq!(p, [0, 255, 0, 255]);
    }
    // Below green content
    if green_f.y + green_f.h + 1 < rgba.height() {
        let p = sample(green_f.x, green_f.y + green_f.h);
        assert_eq!(p, [0, 255, 0, 255]);
    }
}
