use image::{Rgba, RgbaImage};
use tex_packer_core::prelude::*;

#[test]
fn test_runtime_atlas_basic() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut atlas = RuntimeAtlas::new(cfg, RuntimeStrategy::Guillotine);

    // Create a test image
    let img = RgbaImage::from_pixel(64, 64, Rgba([255, 0, 0, 255]));

    // Append with image
    let result = atlas.append_with_image("red_square".into(), &img);
    assert!(result.is_ok());

    let (page_id, frame, region) = result.unwrap();
    assert_eq!(page_id, 0);
    assert_eq!(frame.frame.w, 64);
    assert_eq!(frame.frame.h, 64);
    assert_eq!(region.page_id, 0);
    assert_eq!(region.width, 64);
    assert_eq!(region.height, 64);
    assert!(!region.is_empty());
}

#[test]
fn test_runtime_atlas_get_page_image() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut atlas = RuntimeAtlas::new(cfg, RuntimeStrategy::Guillotine);

    // Initially no pages
    assert_eq!(atlas.num_pages(), 0);
    assert!(atlas.get_page_image(0).is_none());

    // Add an image
    let img = RgbaImage::from_pixel(32, 32, Rgba([0, 255, 0, 255]));
    atlas.append_with_image("green".into(), &img).unwrap();

    // Now page 0 should exist
    assert_eq!(atlas.num_pages(), 1);
    assert!(atlas.get_page_image(0).is_some());

    let page = atlas.get_page_image(0).unwrap();
    assert_eq!(page.width(), 256);
    assert_eq!(page.height(), 256);
}

#[test]
fn test_runtime_atlas_pixel_data() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut atlas = RuntimeAtlas::new(cfg, RuntimeStrategy::Guillotine);

    // Create a red image
    let red_img = RgbaImage::from_pixel(32, 32, Rgba([255, 0, 0, 255]));
    let (page_id, frame, _) = atlas.append_with_image("red".into(), &red_img).unwrap();

    // Verify pixel data was copied
    let page = atlas.get_page_image(page_id).unwrap();
    let pixel = page.get_pixel(frame.frame.x, frame.frame.y);
    assert_eq!(pixel, &Rgba([255, 0, 0, 255]));
}

#[test]
fn test_runtime_atlas_evict_with_clear() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut atlas = RuntimeAtlas::new(cfg, RuntimeStrategy::Guillotine);

    // Add an image
    let img = RgbaImage::from_pixel(32, 32, Rgba([255, 0, 0, 255]));
    let (page_id, frame, _) = atlas.append_with_image("test".into(), &img).unwrap();

    // Verify pixel is red
    let page = atlas.get_page_image(page_id).unwrap();
    let pixel = page.get_pixel(frame.frame.x, frame.frame.y);
    assert_eq!(pixel, &Rgba([255, 0, 0, 255]));

    // Evict with clear
    let region = atlas.evict_with_clear(page_id, "test", true);
    assert!(region.is_some());
    let region = region.unwrap();
    assert!(!region.is_empty());

    // Verify pixel is now transparent (cleared)
    let page = atlas.get_page_image(page_id).unwrap();
    let pixel = page.get_pixel(frame.frame.x, frame.frame.y);
    assert_eq!(pixel, &Rgba([0, 0, 0, 0]));
}

#[test]
fn test_runtime_atlas_evict_by_key_with_clear() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut atlas = RuntimeAtlas::new(cfg, RuntimeStrategy::Guillotine);

    // Add an image
    let img = RgbaImage::from_pixel(32, 32, Rgba([0, 255, 0, 255]));
    let (page_id, frame, _) = atlas.append_with_image("green".into(), &img).unwrap();

    // Evict by key with clear
    let region = atlas.evict_by_key_with_clear("green", true);
    assert!(region.is_some());

    // Verify cleared
    let page = atlas.get_page_image(page_id).unwrap();
    let pixel = page.get_pixel(frame.frame.x, frame.frame.y);
    assert_eq!(pixel, &Rgba([0, 0, 0, 0]));
}

#[test]
fn test_runtime_atlas_background_color() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(128, 128)
        .build();

    let bg_color = Rgba([100, 100, 100, 255]);
    let mut atlas =
        RuntimeAtlas::new(cfg, RuntimeStrategy::Guillotine).with_background_color(bg_color);

    // Add an image to create a page
    let img = RgbaImage::from_pixel(16, 16, Rgba([255, 255, 255, 255]));
    atlas.append_with_image("white".into(), &img).unwrap();

    // Check that background is the specified color
    let page = atlas.get_page_image(0).unwrap();
    // Check a pixel that should be background
    let bg_pixel = page.get_pixel(100, 100);
    assert_eq!(bg_pixel, &bg_color);
}

#[test]
fn test_runtime_atlas_multiple_images() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut atlas = RuntimeAtlas::new(cfg, RuntimeStrategy::Guillotine);

    // Add multiple images
    let red = RgbaImage::from_pixel(32, 32, Rgba([255, 0, 0, 255]));
    let green = RgbaImage::from_pixel(32, 32, Rgba([0, 255, 0, 255]));
    let blue = RgbaImage::from_pixel(32, 32, Rgba([0, 0, 255, 255]));

    atlas.append_with_image("red".into(), &red).unwrap();
    atlas.append_with_image("green".into(), &green).unwrap();
    atlas.append_with_image("blue".into(), &blue).unwrap();

    // Verify all exist
    assert!(atlas.contains("red"));
    assert!(atlas.contains("green"));
    assert!(atlas.contains("blue"));
    assert_eq!(atlas.texture_count(), 3);
}

#[test]
fn test_runtime_atlas_update_region() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut atlas = RuntimeAtlas::new(cfg, RuntimeStrategy::Guillotine);

    let img = RgbaImage::from_pixel(64, 48, Rgba([255, 255, 0, 255]));
    let (page_id, frame, region) = atlas.append_with_image("yellow".into(), &img).unwrap();

    // Verify update region matches frame
    assert_eq!(region.page_id, page_id);
    assert_eq!(region.x, frame.frame.x);
    assert_eq!(region.y, frame.frame.y);
    assert_eq!(region.width, frame.frame.w);
    assert_eq!(region.height, frame.frame.h);
    assert_eq!(region.area(), 64 * 48);
}

#[test]
fn test_runtime_atlas_append_without_image() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut atlas = RuntimeAtlas::new(cfg, RuntimeStrategy::Guillotine);

    // Append without image data (geometry only)
    let result = atlas.append("geometry_only".into(), 64, 64);
    assert!(result.is_ok());

    let (page_id, frame) = result.unwrap();
    assert_eq!(page_id, 0);
    assert_eq!(frame.frame.w, 64);
    assert_eq!(frame.frame.h, 64);

    // No pages should be created (no pixel data)
    assert_eq!(atlas.num_pages(), 0);
}

#[test]
fn test_runtime_atlas_mixed_usage() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut atlas = RuntimeAtlas::new(cfg, RuntimeStrategy::Guillotine);

    // Mix geometry-only and with-image appends
    atlas.append("geo1".into(), 32, 32).unwrap();

    let img = RgbaImage::from_pixel(32, 32, Rgba([255, 0, 0, 255]));
    atlas.append_with_image("img1".into(), &img).unwrap();

    atlas.append("geo2".into(), 32, 32).unwrap();

    // All should be tracked
    assert_eq!(atlas.texture_count(), 3);
    assert!(atlas.contains("geo1"));
    assert!(atlas.contains("img1"));
    assert!(atlas.contains("geo2"));

    // Only one page created (for img1)
    assert_eq!(atlas.num_pages(), 1);
}

#[test]
fn test_runtime_atlas_stats() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut atlas = RuntimeAtlas::new(cfg, RuntimeStrategy::Guillotine);

    let img = RgbaImage::from_pixel(64, 64, Rgba([255, 0, 0, 255]));
    atlas.append_with_image("test".into(), &img).unwrap();

    let stats = atlas.stats();
    assert_eq!(stats.num_textures, 1);
    assert!(stats.occupancy > 0.0);
}

#[test]
fn test_runtime_atlas_get_page_image_mut() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut atlas = RuntimeAtlas::new(cfg, RuntimeStrategy::Guillotine);

    let img = RgbaImage::from_pixel(32, 32, Rgba([255, 0, 0, 255]));
    atlas.append_with_image("test".into(), &img).unwrap();

    // Get mutable reference and modify
    if let Some(page) = atlas.get_page_image_mut(0) {
        page.put_pixel(0, 0, Rgba([0, 0, 255, 255]));
    }

    // Verify modification
    let page = atlas.get_page_image(0).unwrap();
    assert_eq!(page.get_pixel(0, 0), &Rgba([0, 0, 255, 255]));
}

#[test]
fn test_runtime_atlas_rotation() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .allow_rotation(true)
        .build();

    let mut atlas = RuntimeAtlas::new(cfg, RuntimeStrategy::Guillotine);

    // Create a non-square image
    let mut img = RgbaImage::new(64, 32);
    // Fill with a pattern to verify rotation
    for y in 0..32 {
        for x in 0..64 {
            img.put_pixel(x, y, Rgba([x as u8, y as u8, 0, 255]));
        }
    }

    let (page_id, frame, _) = atlas.append_with_image("rect".into(), &img).unwrap();

    // If rotated, frame dimensions should be swapped
    if frame.rotated {
        assert_eq!(frame.frame.w, 32);
        assert_eq!(frame.frame.h, 64);
    } else {
        assert_eq!(frame.frame.w, 64);
        assert_eq!(frame.frame.h, 32);
    }

    // Verify page was created
    assert!(atlas.get_page_image(page_id).is_some());
}

#[test]
fn test_update_region_empty() {
    let region = UpdateRegion::empty();
    assert!(region.is_empty());
    assert_eq!(region.area(), 0);
}

#[test]
fn test_update_region_area() {
    let region = UpdateRegion {
        page_id: 0,
        x: 10,
        y: 20,
        width: 64,
        height: 48,
    };

    assert!(!region.is_empty());
    assert_eq!(region.area(), 64 * 48);
}
