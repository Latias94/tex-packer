use image::{DynamicImage, RgbaImage};
use tex_packer_core::config::{
    MaxRectsHeuristic, OfflineConfig, PackingStrategy, PageConfig, SkylineHeuristic,
};
use tex_packer_core::error::TexPackerError;
use tex_packer_core::{InputImage, pack_images, pack_layout};

/// Test zero-sized atlas dimensions
#[test]
fn test_zero_width() {
    let result = PageConfig::builder().max_dimensions(0, 1024).build();
    assert!(result.is_err());
    match result {
        Err(TexPackerError::InvalidDimensions { width, height }) => {
            assert_eq!(width, 0);
            assert_eq!(height, 1024);
        }
        _ => panic!("Expected InvalidDimensions error"),
    }
}

#[test]
fn test_zero_height() {
    let result = PageConfig::builder().max_dimensions(1024, 0).build();
    assert!(result.is_err());
    match result {
        Err(TexPackerError::InvalidDimensions { width, height }) => {
            assert_eq!(width, 1024);
            assert_eq!(height, 0);
        }
        _ => panic!("Expected InvalidDimensions error"),
    }
}

#[test]
fn test_both_dimensions_zero() {
    let result = PageConfig::builder().max_dimensions(0, 0).build();
    assert!(result.is_err());
}

/// Test border padding that exceeds dimensions
#[test]
fn test_border_padding_exceeds_width() {
    let result = PageConfig::builder()
        .max_dimensions(100, 100)
        .border_padding(50)
        .build();
    assert!(result.is_err());
    match result {
        Err(TexPackerError::InvalidConfig(msg)) => {
            assert!(msg.contains("border_padding"));
        }
        _ => panic!("Expected InvalidConfig error"),
    }
}

#[test]
fn test_border_padding_leaves_no_space() {
    let result = PageConfig::builder()
        .max_dimensions(100, 100)
        .border_padding(50)
        .build();
    assert!(result.is_err());
}

/// Test empty input
#[test]
fn test_empty_input_pack_images() {
    let cfg = OfflineConfig::default();
    let inputs: Vec<InputImage> = vec![];

    let result = pack_images(inputs, cfg);
    assert!(result.is_err());
    match result {
        Err(TexPackerError::Empty) => {}
        _ => panic!("Expected Empty error"),
    }
}

#[test]
fn test_empty_input_pack_layout() {
    let cfg = OfflineConfig::default();
    let inputs: Vec<(String, u32, u32)> = vec![];

    let result = pack_layout(inputs, cfg);
    assert!(result.is_err());
    match result {
        Err(TexPackerError::Empty) => {}
        _ => panic!("Expected Empty error"),
    }
}

/// Test texture larger than atlas
#[test]
fn test_texture_too_large_width() {
    let cfg = offline_config(100, 100, PackingStrategy::default());

    // Create a 200x50 image (width exceeds atlas)
    let img = DynamicImage::ImageRgba8(RgbaImage::new(200, 50));
    let inputs = vec![InputImage {
        key: "large".to_string(),
        image: img,
    }];

    let result = pack_images(inputs, cfg);
    assert!(result.is_err());
    // Should fail to pack
}

#[test]
fn test_texture_too_large_height() {
    let cfg = offline_config(100, 100, PackingStrategy::default());

    // Create a 50x200 image (height exceeds atlas)
    let img = DynamicImage::ImageRgba8(RgbaImage::new(50, 200));
    let inputs = vec![InputImage {
        key: "tall".to_string(),
        image: img,
    }];

    let result = pack_images(inputs, cfg);
    assert!(result.is_err());
}

/// Test 1x1 minimum valid configuration
#[test]
fn test_minimum_valid_config() {
    let result = PageConfig::builder()
        .max_dimensions(1, 1)
        .texture_padding(0)
        .texture_extrusion(0)
        .build();

    assert!(result.is_ok());
}

/// Test 1x1 texture in 1x1 atlas
#[test]
fn test_single_pixel_texture() {
    let cfg = offline_config(1, 1, PackingStrategy::default());

    let img = DynamicImage::ImageRgba8(RgbaImage::new(1, 1));
    let inputs = vec![InputImage {
        key: "pixel".to_string(),
        image: img,
    }];

    let result = pack_images(inputs, cfg);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.pages.len(), 1);
    assert_eq!(output.atlas.pages[0].frames.len(), 1);
}

/// Test very large atlas dimensions (stress test)
#[test]
fn test_very_large_dimensions() {
    let result = PageConfig::builder().max_dimensions(16_384, 16_384).build();

    assert!(result.is_ok());
}

/// Test configuration with all algorithms
#[test]
fn test_all_algorithms_with_valid_config() {
    let strategies = [
        PackingStrategy::Skyline {
            heuristic: SkylineHeuristic::BottomLeft,
            use_waste_map: false,
        },
        PackingStrategy::MaxRects {
            heuristic: MaxRectsHeuristic::BestAreaFit,
            reference: false,
        },
        PackingStrategy::Guillotine {
            choice: Default::default(),
            split: Default::default(),
        },
    ];

    for strategy in strategies {
        let cfg = offline_config(256, 256, strategy);

        // Test with a simple texture
        let img = DynamicImage::ImageRgba8(RgbaImage::new(32, 32));
        let inputs = vec![InputImage {
            key: "test".to_string(),
            image: img,
        }];

        let result = pack_images(inputs, cfg);
        assert!(result.is_ok(), "Strategy {strategy:?} should work");
    }
}

/// Test extreme padding configuration
#[test]
fn test_extreme_padding() {
    let result = PageConfig::builder()
        .max_dimensions(1_000, 1_000)
        .border_padding(10)
        .texture_padding(100)
        .texture_extrusion(50)
        .build();

    // Should be valid (though impractical)
    assert!(result.is_ok());
}

/// Test zero-sized texture in layout
#[test]
fn test_zero_sized_texture_layout() {
    let cfg = OfflineConfig::default();
    let inputs = vec![
        ("normal".to_string(), 32, 32),
        ("zero_width".to_string(), 0, 32),
        ("zero_height".to_string(), 32, 0),
    ];

    let atlas = pack_layout(inputs, cfg).expect("v0.2 accepts zero-sized layout items");
    assert_eq!(
        atlas
            .pages
            .iter()
            .map(|page| page.frames.len())
            .sum::<usize>(),
        3
    );
}

#[test]
#[ignore = "U4: reject zero-sized layout items with key-aware errors"]
fn zero_sized_layout_item_is_rejected_with_key_context() {
    let result = pack_layout(
        vec![("zero_width".to_string(), 0, 32)],
        OfflineConfig::default(),
    );

    match result {
        Ok(_) => panic!("zero-sized layout item should be rejected"),
        Err(err) => assert!(err.to_string().contains("zero_width")),
    }
}

/// Test many small textures
#[test]
fn test_many_small_textures() {
    let cfg = offline_config(512, 512, PackingStrategy::default());

    let mut inputs = Vec::new();
    for i in 0..100 {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(8, 8));
        inputs.push(InputImage {
            key: format!("small_{}", i),
            image: img,
        });
    }

    let result = pack_images(inputs, cfg);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(!output.atlas.pages.is_empty());
}

#[test]
fn test_large_layout_dimensions_do_not_overflow_algorithm_scores() {
    let cfg = offline_config(
        70_000,
        70_000,
        PackingStrategy::MaxRects {
            heuristic: MaxRectsHeuristic::BestAreaFit,
            reference: false,
        },
    );

    let atlas = pack_layout(vec![("large", 65_000, 65_000)], cfg)
        .expect("large layout-only rectangle should not overflow score calculations");
    assert_eq!(atlas.pages.len(), 1);
    assert_eq!(atlas.pages[0].frames[0].frame.w, 65_000);
    assert_eq!(atlas.pages[0].frames[0].frame.h, 65_000);
}

fn offline_config(width: u32, height: u32, strategy: PackingStrategy) -> OfflineConfig {
    let page = PageConfig::builder()
        .max_dimensions(width, height)
        .texture_padding(0)
        .texture_extrusion(0)
        .build()
        .expect("valid page config");

    OfflineConfig::builder()
        .page_config(page)
        .trim(false)
        .strategy(strategy)
        .build()
        .expect("valid offline config")
}
