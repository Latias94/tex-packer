use image::{DynamicImage, Rgba, RgbaImage};
use tex_packer_core::config::{OfflineConfig, PackingStrategy, PageConfig, SkylineHeuristic};
use tex_packer_core::model::{Atlas, Meta};
use tex_packer_core::offline::{InputImage, LayoutItem, OfflinePacker, PackOutput};

fn pack_images(
    inputs: Vec<InputImage>,
    config: OfflineConfig,
) -> tex_packer_core::error::Result<PackOutput> {
    OfflinePacker::new(config).pack_images(inputs)
}

fn pack_layout<K: Into<String>>(
    inputs: Vec<(K, u32, u32)>,
    config: OfflineConfig,
) -> tex_packer_core::error::Result<Atlas> {
    OfflinePacker::new(config).pack_layout(
        inputs
            .into_iter()
            .map(|(key, w, h)| LayoutItem {
                key: key.into(),
                w,
                h,
                source: None,
                source_size: None,
                trimmed: false,
            })
            .collect(),
    )
}

#[test]
fn test_pack_stats_basic() {
    let cfg = offline_config(page_config(256, 256, true, 0, 0), false, false);

    // Create 4 unique textures of 64x64 each
    let mut inputs = Vec::new();
    for i in 0..4 {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(64, 64, Rgba([i as u8, 0, 0, 255])));
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
    assert_eq!(stats.num_regions, 4);
    assert_eq!(stats.num_aliases, 0);
    assert_eq!(stats.content_area, 4 * 64 * 64);
    assert_eq!(stats.allocation_area, stats.content_area);

    // Occupancy should be reasonable (at least 25% for this simple case)
    assert!(
        stats.content_occupancy > 0.25,
        "Content occupancy: {}",
        stats.content_occupancy
    );
    assert!(stats.content_occupancy <= 1.0);

    // Total page area should be >= used area
    assert!(stats.page_area >= stats.content_area);

    // No rotations or trimming in this test
    assert_eq!(stats.num_rotated_regions, 0);
    assert_eq!(stats.num_trimmed_frames, 0);
}

#[test]
fn test_pack_stats_with_rotation() {
    let cfg = offline_config(page_config(128, 128, true, 0, 0), false, false);

    // Create some rectangular textures that might benefit from rotation
    let mut inputs = Vec::new();
    for i in 0..3 {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(80, 40, Rgba([i as u8, 0, 0, 255])));
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
    assert!(stats.num_rotated_regions <= 3);
}

#[test]
fn test_pack_stats_with_trimming() {
    let cfg = offline_config(page_config(256, 256, true, 0, 0), true, false);

    // Create images with transparent borders
    let mut inputs = Vec::new();
    for i in 0..2 {
        let mut img = RgbaImage::new(64, 64);
        // Fill center 32x32 with opaque pixels, leave borders transparent
        for y in 16..48 {
            for x in 16..48 {
                img.put_pixel(x, y, Rgba([255, i as u8, 0, 255]));
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
    assert_eq!(stats.num_trimmed_frames, 2);

    // Used area should be less than 2 * 64 * 64 due to trimming
    assert_eq!(stats.num_regions, 2);
    assert!(stats.content_area < 2 * 64 * 64);
}

#[test]
fn test_pack_stats_summary() {
    let page = PageConfig::builder()
        .max_dimensions(128, 128)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .build()
        .expect("valid offline config");

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
    assert!(summary.contains("Content occupancy:"));
    assert!(summary.contains("Allocation occupancy:"));
    assert!(summary.contains("1")); // 1 frame
}

#[test]
fn test_pack_stats_wasted_area() {
    let cfg = offline_config(page_config(256, 256, true, 0, 0), false, true);

    // Single small texture in large atlas
    let img = DynamicImage::ImageRgba8(RgbaImage::new(32, 32));
    let inputs = vec![InputImage {
        key: "small".to_string(),
        image: img,
    }];

    let result = pack_images(inputs, cfg).expect("packing should succeed");
    let stats = result.stats();

    let wasted = stats.unallocated_area();
    let waste_pct = stats.allocation_waste_percentage();

    // With force_max_dimensions, should have significant wasted space
    // 32x32 texture in 256x256 page = 1024 used, 65536 total
    assert!(
        wasted > 0,
        "Wasted: {}, Total: {}, Used: {}",
        wasted,
        stats.page_area,
        stats.allocation_area
    );
    assert!(waste_pct > 0.0);
    assert!(waste_pct < 100.0);

    // Wasted + used should equal total
    assert_eq!(wasted + stats.allocation_area, stats.page_area);
}

#[test]
fn test_pack_stats_layout_only() {
    let page = PageConfig::builder()
        .max_dimensions(256, 256)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .build()
        .expect("valid offline config");

    let inputs = vec![("a", 32, 32), ("b", 64, 64), ("c", 48, 48)];

    let atlas = pack_layout(inputs, cfg).expect("packing should succeed");
    let stats = atlas.stats();

    assert_eq!(stats.num_frames, 3);
    assert!(stats.num_pages >= 1);

    // Calculate expected used area
    let expected_used = 32 * 32 + 64 * 64 + 48 * 48;
    assert_eq!(stats.content_area, expected_used);
    let expected_allocated = 34 * 34 + 66 * 66 + 50 * 50;
    assert_eq!(stats.allocation_area, expected_allocated);

    assert!(stats.content_occupancy > 0.0);
    assert!(stats.content_occupancy <= 1.0);
}

#[test]
fn test_pack_stats_multiple_pages() {
    let cfg = offline_config(page_config(128, 128, true, 0, 0), false, false);

    // Create many textures to force multiple pages
    let mut inputs = Vec::new();
    for i in 0..20 {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(40, 40, Rgba([i as u8, 0, 0, 255])));
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
        .atlas()
        .pages()
        .iter()
        .map(|p| u64::from(p.width()) * u64::from(p.height()))
        .sum();
    assert_eq!(stats.page_area, u128::from(expected_total));

    // Max page dimensions should be <= configured max
    assert!(
        result
            .atlas()
            .pages()
            .iter()
            .all(|page| page.width() <= 128)
    );
    assert!(
        result
            .atlas()
            .pages()
            .iter()
            .all(|page| page.height() <= 128)
    );
}

#[test]
fn test_pack_stats_empty_atlas() {
    let atlas = Atlas::try_new(vec![], Meta::default()).expect("valid empty atlas");

    let stats = atlas.stats();

    assert_eq!(stats.num_pages, 0);
    assert_eq!(stats.num_frames, 0);
    assert_eq!(stats.num_regions, 0);
    assert_eq!(stats.num_aliases, 0);
    assert_eq!(stats.page_area, 0);
    assert_eq!(stats.content_area, 0);
    assert_eq!(stats.allocation_area, 0);
    assert_eq!(stats.content_occupancy, 0.0);
    assert_eq!(stats.allocation_occupancy, 0.0);
    assert_eq!(stats.unallocated_area(), 0);
}

#[test]
fn content_and_allocation_metrics_count_aliases_once() {
    let page = PageConfig::builder()
        .max_dimensions(32, 32)
        .allow_rotation(false)
        .texture_padding(2)
        .texture_extrusion(1)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .trim(false)
        .force_max_dimensions(true)
        .strategy(PackingStrategy::Skyline {
            heuristic: SkylineHeuristic::BottomLeft,
            use_waste_map: false,
        })
        .build()
        .expect("valid offline config");
    let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 3, Rgba([1, 2, 3, 255])));
    let result = pack_images(
        vec![
            InputImage {
                key: "original".into(),
                image: image.clone(),
            },
            InputImage {
                key: "alias".into(),
                image,
            },
        ],
        cfg,
    )
    .expect("packing should succeed");

    let stats = result.stats();
    assert_eq!(stats.num_frames, 2);
    assert_eq!(stats.num_regions, 1);
    assert_eq!(stats.num_aliases, 1);
    assert_eq!(stats.page_area, 32 * 32);
    assert_eq!(stats.content_area, 4 * 3);
    assert_eq!(stats.allocation_area, 8 * 7);
    assert_eq!(stats.content_occupancy, 12.0 / 1024.0);
    assert_eq!(stats.allocation_occupancy, 56.0 / 1024.0);
}

fn page_config(
    width: u32,
    height: u32,
    allow_rotation: bool,
    texture_padding: u32,
    texture_extrusion: u32,
) -> PageConfig {
    PageConfig::builder()
        .max_dimensions(width, height)
        .allow_rotation(allow_rotation)
        .texture_padding(texture_padding)
        .texture_extrusion(texture_extrusion)
        .build()
        .expect("valid page config")
}

fn offline_config(page: PageConfig, trim: bool, force_max_dimensions: bool) -> OfflineConfig {
    OfflineConfig::builder()
        .page_config(page)
        .trim(trim)
        .force_max_dimensions(force_max_dimensions)
        .strategy(PackingStrategy::Skyline {
            heuristic: SkylineHeuristic::BottomLeft,
            use_waste_map: false,
        })
        .build()
        .expect("valid offline config")
}
