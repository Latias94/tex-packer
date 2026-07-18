use tex_packer_core::PageId;
use tex_packer_core::TexPackerError;
use tex_packer_core::prelude::*;

fn runtime_config(width: u32, height: u32, strategy: RuntimeStrategy) -> RuntimeConfig {
    let page = PageConfig::builder()
        .max_dimensions(width, height)
        .build()
        .expect("valid page config");
    RuntimeConfig::builder()
        .page_config(page)
        .strategy(strategy)
        .build()
        .expect("valid runtime config")
}

fn guillotine_config(width: u32, height: u32) -> RuntimeConfig {
    runtime_config(
        width,
        height,
        RuntimeStrategy::Guillotine {
            choice: GuillotineChoice::BestAreaFit,
            split: GuillotineSplit::SplitShorterLeftoverAxis,
        },
    )
}

#[test]
fn rejected_oversized_append_does_not_consume_page_id() {
    let mut sess = AtlasSession::new(guillotine_config(64, 64));

    assert!(sess.append("too_large".into(), 128, 128).is_err());
    assert_eq!(sess.stats().num_pages, 0);
    assert_eq!(sess.texture_count(), 0);

    let placement = sess
        .append("valid".into(), 16, 16)
        .expect("a valid append should still use the first page");
    assert_eq!(placement.page_id(), PageId::new(0));
}

#[test]
fn rejected_zero_sized_append_does_not_consume_page_id() {
    let mut sess = AtlasSession::new(guillotine_config(64, 64));

    assert!(sess.append("zero_width".into(), 0, 16).is_err());
    assert_eq!(sess.stats().num_pages, 0);
    assert_eq!(sess.texture_count(), 0);

    let placement = sess
        .append("valid".into(), 16, 16)
        .expect("a valid append should still use the first page");
    assert_eq!(placement.page_id(), PageId::new(0));
}

#[test]
fn duplicate_key_append_preserves_placement_and_stats() {
    let mut sess = AtlasSession::new(guillotine_config(64, 64));

    let original = sess
        .append("duplicate".into(), 16, 16)
        .expect("initial append");
    let stats_before = sess.stats();

    let duplicate_result = sess.append("duplicate".into(), 16, 16);
    assert!(matches!(
        duplicate_result,
        Err(TexPackerError::DuplicateKey { ref key }) if key == "duplicate"
    ));
    assert_runtime_stats_eq(&sess.stats(), &stats_before);

    let current = sess
        .get_frame("duplicate")
        .expect("the original key must remain present");
    assert_eq!(current.page_id(), original.page_id());
    assert_eq!(current.content(), original.content());
    assert_eq!(current.rotated(), original.rotated());
}

fn assert_runtime_stats_eq(actual: &RuntimeStats, expected: &RuntimeStats) {
    assert_eq!(actual, expected);
}

#[test]
fn test_get_frame() {
    let mut sess = AtlasSession::new(guillotine_config(256, 256));

    // Add some textures
    let placement_a = sess.append("sprite_a".into(), 64, 64).expect("append A");
    sess.append("sprite_b".into(), 32, 32).expect("append B");

    // Test get_frame
    let result = sess.get_frame("sprite_a");
    assert!(result.is_some());
    let found = result.expect("sprite_a placement");
    assert_eq!(found.page_id(), placement_a.page_id());
    assert_eq!(found.frame().key(), "sprite_a");
    assert_eq!(found.content(), placement_a.content());

    // Test non-existent key
    assert!(sess.get_frame("non_existent").is_none());
}

#[test]
fn test_evict_by_key() {
    let mut sess = AtlasSession::new(guillotine_config(256, 256));

    // Add textures
    sess.append("sprite_a".into(), 64, 64).expect("append A");
    sess.append("sprite_b".into(), 32, 32).expect("append B");

    // Verify they exist
    assert!(sess.contains("sprite_a"));
    assert!(sess.contains("sprite_b"));
    assert_eq!(sess.texture_count(), 2);

    // Evict by key (no need to know page_id)
    assert!(sess.evict_by_key("sprite_a"));
    assert!(!sess.contains("sprite_a"));
    assert!(sess.contains("sprite_b"));
    assert_eq!(sess.texture_count(), 1);

    // Try to evict non-existent key
    assert!(!sess.evict_by_key("non_existent"));
    assert_eq!(sess.texture_count(), 1);

    // Evict remaining texture
    assert!(sess.evict_by_key("sprite_b"));
    assert_eq!(sess.texture_count(), 0);
}

#[test]
fn test_contains() {
    let mut sess = AtlasSession::new(guillotine_config(256, 256));

    // Initially empty
    assert!(!sess.contains("sprite_a"));

    // Add texture
    sess.append("sprite_a".into(), 64, 64).expect("append A");
    assert!(sess.contains("sprite_a"));
    assert!(!sess.contains("sprite_b"));

    // Add another
    sess.append("sprite_b".into(), 32, 32).expect("append B");
    assert!(sess.contains("sprite_a"));
    assert!(sess.contains("sprite_b"));

    // Evict one
    sess.evict_by_key("sprite_a");
    assert!(!sess.contains("sprite_a"));
    assert!(sess.contains("sprite_b"));
}

#[test]
fn test_keys() {
    let mut sess = AtlasSession::new(guillotine_config(256, 256));

    // Initially empty
    assert_eq!(sess.keys().len(), 0);

    // Add textures
    sess.append("sprite_a".into(), 64, 64).expect("append A");
    sess.append("sprite_b".into(), 32, 32).expect("append B");
    sess.append("sprite_c".into(), 48, 48).expect("append C");

    let keys = sess.keys();
    assert_eq!(keys.len(), 3);
    assert!(keys.contains(&"sprite_a"));
    assert!(keys.contains(&"sprite_b"));
    assert!(keys.contains(&"sprite_c"));

    // Evict one
    sess.evict_by_key("sprite_b");
    let keys = sess.keys();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&"sprite_a"));
    assert!(!keys.contains(&"sprite_b"));
    assert!(keys.contains(&"sprite_c"));
}

#[test]
fn test_texture_count() {
    let mut sess = AtlasSession::new(guillotine_config(256, 256));

    assert_eq!(sess.texture_count(), 0);

    sess.append("a".into(), 32, 32).expect("append");
    assert_eq!(sess.texture_count(), 1);

    sess.append("b".into(), 32, 32).expect("append");
    assert_eq!(sess.texture_count(), 2);

    sess.append("c".into(), 32, 32).expect("append");
    assert_eq!(sess.texture_count(), 3);

    sess.evict_by_key("b");
    assert_eq!(sess.texture_count(), 2);

    sess.evict_by_key("a");
    sess.evict_by_key("c");
    assert_eq!(sess.texture_count(), 0);
}

#[test]
fn test_runtime_stats() {
    let mut sess = AtlasSession::new(guillotine_config(256, 256));

    // Empty session
    let stats = sess.stats();
    assert_eq!(stats.num_pages, 0);
    assert_eq!(stats.num_frames, 0);
    assert_eq!(stats.num_regions, 0);
    assert_eq!(stats.page_area, 0);
    assert_eq!(stats.allocation_area, 0);
    assert_eq!(stats.allocation_occupancy, 0.0);

    // Add some textures
    sess.append("a".into(), 64, 64).expect("append A");
    sess.append("b".into(), 32, 32).expect("append B");

    let stats = sess.stats();
    assert_eq!(stats.num_pages, 1);
    assert_eq!(stats.num_frames, 2);
    assert_eq!(stats.num_regions, 2);
    assert!(stats.page_area > 0);
    assert!(stats.allocation_area > 0);
    assert!(stats.allocation_occupancy > 0.0);
    assert!(stats.allocation_occupancy <= 1.0);

    // Used area should be at least the sum of texture areas (plus padding)
    let min_used = 64 * 64 + 32 * 32;
    assert!(stats.allocation_area >= min_used as u128);

    // Free area should be positive
    assert!(stats.allocator_free_area > 0);

    // Total should equal used + free (approximately, due to padding)
    let total_accounted = stats.allocation_area + stats.allocator_free_area;
    assert!(total_accounted <= stats.page_area);
}

#[test]
fn test_runtime_stats_summary() {
    let mut sess = AtlasSession::new(guillotine_config(256, 256));

    sess.append("a".into(), 64, 64).expect("append");

    let stats = sess.stats();
    let summary = stats.summary();

    // Summary should contain key information
    assert!(summary.contains("Pages:"));
    assert!(summary.contains("Frames:"));
    assert!(summary.contains("Content occupancy:"));
    assert!(summary.contains("Allocation occupancy:"));
    assert!(summary.contains("Allocator free:"));
}

#[test]
fn test_runtime_stats_fragmentation() {
    let mut sess = AtlasSession::new(guillotine_config(256, 256));

    // Add and remove textures to create fragmentation
    sess.append("a".into(), 64, 64).expect("append A");
    sess.append("b".into(), 32, 32).expect("append B");
    sess.append("c".into(), 48, 48).expect("append C");

    let stats_before = sess.stats();
    let frag_before = stats_before.fragmentation();

    // Evict middle texture to create fragmentation
    sess.evict_by_key("b");

    let stats_after = sess.stats();
    let frag_after = stats_after.fragmentation();

    // Fragmentation should be non-negative
    assert!(frag_before >= 0.0);
    assert!(frag_after >= 0.0);
}

#[test]
fn test_runtime_stats_waste_percentage() {
    let mut sess = AtlasSession::new(guillotine_config(256, 256));

    sess.append("a".into(), 32, 32).expect("append");

    let stats = sess.stats();
    let waste = stats.waste_percentage();

    // Waste should be between 0 and 100
    assert!(waste >= 0.0);
    assert!(waste <= 100.0);

    // With a small texture in a large atlas, waste should be significant
    assert!(waste > 50.0);
}

#[test]
fn test_evict_by_key_with_reuse() {
    let mut sess = AtlasSession::new(guillotine_config(256, 256));

    // Add texture
    let page_a = sess
        .append("sprite_a".into(), 64, 64)
        .expect("append A")
        .page_id();
    assert_eq!(sess.texture_count(), 1);

    // Evict it
    assert!(sess.evict_by_key("sprite_a"));
    assert_eq!(sess.texture_count(), 0);

    // Add new texture with same size - should reuse space
    let page_b = sess
        .append("sprite_b".into(), 64, 64)
        .expect("append B")
        .page_id();
    assert_eq!(sess.texture_count(), 1);

    // Should be on the same page
    assert_eq!(page_a, page_b);
}

#[test]
fn test_shelf_strategy_with_new_api() {
    let cfg = runtime_config(
        256,
        256,
        RuntimeStrategy::Shelf {
            policy: ShelfPolicy::FirstFit,
        },
    );
    let mut sess = AtlasSession::new(cfg);

    // Add textures
    sess.append("a".into(), 64, 32).expect("append A");
    sess.append("b".into(), 48, 32).expect("append B");

    // Test new API methods
    assert!(sess.contains("a"));
    assert!(sess.contains("b"));
    assert_eq!(sess.texture_count(), 2);

    let keys = sess.keys();
    assert_eq!(keys.len(), 2);

    // Get frame
    let placement = sess.get_frame("a").expect("frame should exist");
    assert_eq!(placement.frame().key(), "a");

    // Evict by key
    assert!(sess.evict_by_key("a"));
    assert!(!sess.contains("a"));

    // Stats
    let stats = sess.stats();
    assert_eq!(stats.num_frames, 1);
}

#[test]
fn test_multiple_pages_stats() {
    let mut sess = AtlasSession::new(guillotine_config(128, 128));

    // Add many textures to force multiple pages
    for i in 0..10 {
        sess.append(format!("tex_{}", i), 50, 50)
            .expect("append should succeed");
    }

    let stats = sess.stats();
    assert!(stats.num_pages > 1, "Should have multiple pages");
    assert_eq!(stats.num_frames, 10);

    // Total page area should be num_pages * page_size
    let expected_total = (128 * 128) as u128 * stats.num_pages as u128;
    assert_eq!(stats.page_area, expected_total);
}
