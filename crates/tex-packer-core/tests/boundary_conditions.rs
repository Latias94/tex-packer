use image::{DynamicImage, RgbaImage};
use tex_packer_core::config::{AlgorithmFamily, PackerConfig};
use tex_packer_core::error::TexPackerError;
use tex_packer_core::{InputImage, pack_images, pack_layout};

/// Test zero-sized atlas dimensions
#[test]
fn test_zero_width() {
    let cfg = PackerConfig {
        max_width: 0,
        max_height: 1024,
        ..Default::default()
    };

    let result = cfg.validate();
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
    let cfg = PackerConfig {
        max_width: 1024,
        max_height: 0,
        ..Default::default()
    };

    let result = cfg.validate();
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
    let cfg = PackerConfig {
        max_width: 0,
        max_height: 0,
        ..Default::default()
    };

    let result = cfg.validate();
    assert!(result.is_err());
}

/// Test border padding that exceeds dimensions
#[test]
fn test_border_padding_exceeds_width() {
    let cfg = PackerConfig {
        max_width: 100,
        max_height: 100,
        border_padding: 50,
        ..Default::default()
    };

    let result = cfg.validate();
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
    let cfg = PackerConfig {
        max_width: 100,
        max_height: 100,
        border_padding: 50, // 50 * 2 = 100, leaves 0 space
        ..Default::default()
    };

    let result = cfg.validate();
    assert!(result.is_err());
}

/// Test empty input
#[test]
fn test_empty_input_pack_images() {
    let cfg = PackerConfig::default();
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
    let cfg = PackerConfig::default();
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
    let cfg = PackerConfig {
        max_width: 100,
        max_height: 100,
        border_padding: 0,
        texture_padding: 0,
        texture_extrusion: 0,
        trim: false,
        ..Default::default()
    };

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
    let cfg = PackerConfig {
        max_width: 100,
        max_height: 100,
        border_padding: 0,
        texture_padding: 0,
        texture_extrusion: 0,
        trim: false,
        ..Default::default()
    };

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
    let cfg = PackerConfig {
        max_width: 1,
        max_height: 1,
        border_padding: 0,
        texture_padding: 0,
        texture_extrusion: 0,
        ..Default::default()
    };

    assert!(cfg.validate().is_ok());
}

/// Test 1x1 texture in 1x1 atlas
#[test]
fn test_single_pixel_texture() {
    let cfg = PackerConfig {
        max_width: 1,
        max_height: 1,
        border_padding: 0,
        texture_padding: 0,
        texture_extrusion: 0,
        trim: false,
        family: AlgorithmFamily::Skyline,
        ..Default::default()
    };

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
    let cfg = PackerConfig {
        max_width: 16384,
        max_height: 16384,
        ..Default::default()
    };

    assert!(cfg.validate().is_ok());
}

/// Test configuration with all algorithms
#[test]
fn test_all_algorithms_with_valid_config() {
    let algorithms = vec![
        AlgorithmFamily::Skyline,
        AlgorithmFamily::MaxRects,
        AlgorithmFamily::Guillotine,
    ];

    for algo in algorithms {
        let cfg = PackerConfig {
            max_width: 256,
            max_height: 256,
            family: algo.clone(),
            ..Default::default()
        };

        assert!(cfg.validate().is_ok());

        // Test with a simple texture
        let img = DynamicImage::ImageRgba8(RgbaImage::new(32, 32));
        let inputs = vec![InputImage {
            key: "test".to_string(),
            image: img,
        }];

        let result = pack_images(inputs, cfg);
        assert!(result.is_ok(), "Algorithm {:?} should work", algo);
    }
}

/// Test extreme padding configuration
#[test]
fn test_extreme_padding() {
    let cfg = PackerConfig {
        max_width: 1000,
        max_height: 1000,
        border_padding: 10,
        texture_padding: 100,
        texture_extrusion: 50,
        ..Default::default()
    };

    // Should be valid (though impractical)
    assert!(cfg.validate().is_ok());
}

/// Test zero-sized texture in layout
#[test]
fn test_zero_sized_texture_layout() {
    let cfg = PackerConfig::default();
    let inputs = vec![
        ("normal".to_string(), 32, 32),
        ("zero_width".to_string(), 0, 32),
        ("zero_height".to_string(), 32, 0),
    ];

    // Should handle gracefully (likely skip zero-sized textures)
    let result = pack_layout(inputs, cfg);
    // Depending on implementation, this might succeed or fail
    // The important thing is it doesn't panic
    let _ = result;
}

/// Test many small textures
#[test]
fn test_many_small_textures() {
    let cfg = PackerConfig {
        max_width: 512,
        max_height: 512,
        ..Default::default()
    };

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
