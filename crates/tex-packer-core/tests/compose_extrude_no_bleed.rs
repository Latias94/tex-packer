use image::{DynamicImage, Rgba, RgbaImage};
use tex_packer_core::config::{AlgorithmFamily, AutoMode, SortOrder};
use tex_packer_core::{pack_images, InputImage, PackerConfig};

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

    let cfg = PackerConfig {
        max_width: 128,
        max_height: 128,
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
        mr_heuristic: tex_packer_core::config::MaxRectsHeuristic::BestAreaFit,
        skyline_heuristic: tex_packer_core::config::SkylineHeuristic::BottomLeft,
        g_choice: tex_packer_core::config::GuillotineChoice::BestAreaFit,
        g_split: tex_packer_core::config::GuillotineSplit::SplitShorterLeftoverAxis,
        auto_mode: AutoMode::Quality,
        sort_order: SortOrder::AreaDesc,
        time_budget_ms: None,
        parallel: false,
        mr_reference: false,
        auto_mr_ref_time_ms_threshold: None,
        auto_mr_ref_input_threshold: None,
    };

    let out = pack_images(inputs, cfg).expect("pack");
    assert_eq!(out.pages.len(), 1);
    let page = &out.pages[0];
    let rgba = &page.rgba;

    // Find frames
    let mut red_f = None;
    let mut green_f = None;
    for f in &page.page.frames {
        if f.key == "red" {
            red_f = Some(f);
        }
        if f.key == "green" {
            green_f = Some(f);
        }
    }
    let red_f = red_f.expect("red frame");
    let green_f = green_f.expect("green frame");

    // Ensure there's at least one pixel gap between content frames (due to padding/extrude reservations)
    // and check border pixels adjacent to content are of correct color (i.e., extruded from the same content,
    // not contaminated by the neighbor).
    let _red_edge = (
        red_f.frame.x + red_f.frame.w - 1,
        red_f.frame.y + red_f.frame.h - 1,
    );
    let _green_edge = (
        green_f.frame.x + green_f.frame.w - 1,
        green_f.frame.y + green_f.frame.h - 1,
    );
    // Sample a few pixels just outside content area (if within bounds) and ensure they match the owner's color
    let sample = |x: u32, y: u32| -> [u8; 4] { rgba.get_pixel(x, y).0 };

    // Right of red content
    if red_f.frame.x + red_f.frame.w + 1 < rgba.width() {
        let p = sample(red_f.frame.x + red_f.frame.w, red_f.frame.y);
        assert_eq!(p, [255, 0, 0, 255]);
    }
    // Below red content
    if red_f.frame.y + red_f.frame.h + 1 < rgba.height() {
        let p = sample(red_f.frame.x, red_f.frame.y + red_f.frame.h);
        assert_eq!(p, [255, 0, 0, 255]);
    }
    // Right of green content
    if green_f.frame.x + green_f.frame.w + 1 < rgba.width() {
        let p = sample(green_f.frame.x + green_f.frame.w, green_f.frame.y);
        assert_eq!(p, [0, 255, 0, 255]);
    }
    // Below green content
    if green_f.frame.y + green_f.frame.h + 1 < rgba.height() {
        let p = sample(green_f.frame.x, green_f.frame.y + green_f.frame.h);
        assert_eq!(p, [0, 255, 0, 255]);
    }
}
