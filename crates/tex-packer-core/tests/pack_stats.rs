use image::{DynamicImage, RgbaImage};
use tex_packer_core::prelude::*;

#[test]
fn test_pack_stats_basic() {
    let cfg = PackerConfig {
        max_width: 256,
        max_height: 256,
        border_padding: 0,
        texture_padding: 0,
        texture_extrusion: 0,
        trim: false,
        family: AlgorithmFamily::Skyline,
        ..Default::default()
    };

    // Create 4 textures of 64x64 each
    let mut inputs = Vec::new();
    for i in 0..4 {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(64, 64));
        inputs.push(InputImage {
            key: format!("tex_{}", i),
            image: img,
        });
    }

    let result = pack_images(inputs, cfg).expect("packing should succeed");
    let stats = result.stats();

    // Verify basic stats
    assert_eq!(stats.num_frames, 4);
    assert!(stats.num_pages >= 1);

    // Each frame is 64x64 = 4096 pixels
    assert_eq!(stats.used_frame_area, 4 * 64 * 64);

    // Occupancy should be reasonable (at least 25% for this simple case)
    assert!(stats.occupancy > 0.25, "Occupancy: {}", stats.occupancy);
    assert!(stats.occupancy <= 1.0);

    // Total page area should be >= used area
    assert!(stats.total_page_area >= stats.used_frame_area);

    // No rotations or trimming in this test
    assert_eq!(stats.num_rotated, 0);
    assert_eq!(stats.num_trimmed, 0);
}

#[test]
fn test_pack_stats_with_rotation() {
    let cfg = PackerConfig {
        max_width: 128,
        max_height: 128,
        allow_rotation: true,
        border_padding: 0,
        texture_padding: 0,
        texture_extrusion: 0,
        trim: false,
        family: AlgorithmFamily::Skyline,
        ..Default::default()
    };

    // Create some rectangular textures that might benefit from rotation
    let mut inputs = Vec::new();
    for i in 0..3 {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(80, 40));
        inputs.push(InputImage {
            key: format!("rect_{}", i),
            image: img,
        });
    }

    let result = pack_images(inputs, cfg).expect("packing should succeed");
    let stats = result.stats();

    assert_eq!(stats.num_frames, 3);
    // Some frames might be rotated
    // (exact count depends on algorithm, so we just check it's valid)
    assert!(stats.num_rotated <= 3);
}

#[test]
fn test_pack_stats_with_trimming() {
    let cfg = PackerConfig {
        max_width: 256,
        max_height: 256,
        trim: true,
        trim_threshold: 0,
        border_padding: 0,
        texture_padding: 0,
        texture_extrusion: 0,
        family: AlgorithmFamily::Skyline,
        ..Default::default()
    };

    // Create images with transparent borders
    let mut inputs = Vec::new();
    for i in 0..2 {
        let mut img = RgbaImage::new(64, 64);
        // Fill center 32x32 with opaque pixels, leave borders transparent
        for y in 16..48 {
            for x in 16..48 {
                img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        inputs.push(InputImage {
            key: format!("trimmed_{}", i),
            image: DynamicImage::ImageRgba8(img),
        });
    }

    let result = pack_images(inputs, cfg).expect("packing should succeed");
    let stats = result.stats();

    assert_eq!(stats.num_frames, 2);
    // Both should be trimmed
    assert_eq!(stats.num_trimmed, 2);

    // Used area should be less than 2 * 64 * 64 due to trimming
    assert!(stats.used_frame_area < 2 * 64 * 64);
}

#[test]
fn test_pack_stats_summary() {
    let cfg = PackerConfig {
        max_width: 128,
        max_height: 128,
        ..Default::default()
    };

    let img = DynamicImage::ImageRgba8(RgbaImage::new(32, 32));
    let inputs = vec![InputImage {
        key: "test".to_string(),
        image: img,
    }];

    let result = pack_images(inputs, cfg).expect("packing should succeed");
    let stats = result.stats();

    let summary = stats.summary();

    // Summary should contain key information
    assert!(summary.contains("Pages:"));
    assert!(summary.contains("Frames:"));
    assert!(summary.contains("Occupancy:"));
    assert!(summary.contains("1")); // 1 frame
}

#[test]
fn test_pack_stats_wasted_area() {
    let cfg = PackerConfig {
        max_width: 256,
        max_height: 256,
        border_padding: 0,
        texture_padding: 0,
        texture_extrusion: 0,
        trim: false,
        force_max_dimensions: true, // Force full page size
        family: AlgorithmFamily::Skyline,
        ..Default::default()
    };

    // Single small texture in large atlas
    let img = DynamicImage::ImageRgba8(RgbaImage::new(32, 32));
    let inputs = vec![InputImage {
        key: "small".to_string(),
        image: img,
    }];

    let result = pack_images(inputs, cfg).expect("packing should succeed");
    let stats = result.stats();

    let wasted = stats.wasted_area();
    let waste_pct = stats.waste_percentage();

    // With force_max_dimensions, should have significant wasted space
    // 32x32 texture in 256x256 page = 1024 used, 65536 total
    assert!(
        wasted > 0,
        "Wasted: {}, Total: {}, Used: {}",
        wasted,
        stats.total_page_area,
        stats.used_frame_area
    );
    assert!(waste_pct > 0.0);
    assert!(waste_pct < 100.0);

    // Wasted + used should equal total
    assert_eq!(wasted + stats.used_frame_area, stats.total_page_area);
}

#[test]
fn test_pack_stats_layout_only() {
    let cfg = PackerConfig {
        max_width: 256,
        max_height: 256,
        ..Default::default()
    };

    let inputs = vec![("a", 32, 32), ("b", 64, 64), ("c", 48, 48)];

    let atlas = pack_layout(inputs, cfg).expect("packing should succeed");
    let stats = atlas.stats();

    assert_eq!(stats.num_frames, 3);
    assert!(stats.num_pages >= 1);

    // Calculate expected used area
    let expected_used = 32 * 32 + 64 * 64 + 48 * 48;
    assert_eq!(stats.used_frame_area, expected_used);

    assert!(stats.occupancy > 0.0);
    assert!(stats.occupancy <= 1.0);
}

#[test]
fn test_pack_stats_multiple_pages() {
    let cfg = PackerConfig {
        max_width: 128,
        max_height: 128,
        border_padding: 0,
        texture_padding: 0,
        texture_extrusion: 0,
        trim: false,
        family: AlgorithmFamily::Skyline,
        ..Default::default()
    };

    // Create many textures to force multiple pages
    let mut inputs = Vec::new();
    for i in 0..20 {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(40, 40));
        inputs.push(InputImage {
            key: format!("tex_{}", i),
            image: img,
        });
    }

    let result = pack_images(inputs, cfg).expect("packing should succeed");
    let stats = result.stats();

    assert_eq!(stats.num_frames, 20);
    assert!(stats.num_pages > 1, "Should require multiple pages");

    // Total area should be sum of all pages
    let expected_total: u64 = result
        .atlas
        .pages
        .iter()
        .map(|p| (p.width as u64) * (p.height as u64))
        .sum();
    assert_eq!(stats.total_page_area, expected_total);

    // Max page dimensions should be <= configured max
    assert!(stats.max_page_width <= 128);
    assert!(stats.max_page_height <= 128);
}

#[test]
fn test_pack_stats_empty_atlas() {
    // Create an empty atlas manually
    let atlas: Atlas<String> = Atlas {
        pages: vec![],
        meta: Meta {
            schema_version: "1".into(),
            app: "test".into(),
            version: "0.1.0".into(),
            format: "RGBA8888".into(),
            scale: 1.0,
            power_of_two: false,
            square: false,
            max_dim: (256, 256),
            padding: (0, 0),
            extrude: 0,
            allow_rotation: false,
            trim_mode: "none".into(),
            background_color: None,
        },
    };

    let stats = atlas.stats();

    assert_eq!(stats.num_pages, 0);
    assert_eq!(stats.num_frames, 0);
    assert_eq!(stats.total_page_area, 0);
    assert_eq!(stats.used_frame_area, 0);
    assert_eq!(stats.occupancy, 0.0);
    assert_eq!(stats.wasted_area(), 0);
}
