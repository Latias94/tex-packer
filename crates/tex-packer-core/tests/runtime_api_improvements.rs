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
#[ignore = "U5: rejected oversized appends must not consume runtime page identifiers"]
fn rejected_oversized_append_does_not_consume_page_id() {
    let mut sess = AtlasSession::new(guillotine_config(64, 64));

    assert!(sess.append("too_large".into(), 128, 128).is_err());
    assert_eq!(sess.stats().num_pages, 0);
    assert_eq!(sess.texture_count(), 0);

    let (page_id, _) = sess
        .append("valid".into(), 16, 16)
        .expect("a valid append should still use the first page");
    assert_eq!(page_id, 0);
}

#[test]
#[ignore = "U5: rejected zero-sized appends must not consume runtime page identifiers"]
fn rejected_zero_sized_append_does_not_consume_page_id() {
    let mut sess = AtlasSession::new(guillotine_config(64, 64));

    assert!(sess.append("zero_width".into(), 0, 16).is_err());
    assert_eq!(sess.stats().num_pages, 0);
    assert_eq!(sess.texture_count(), 0);

    let (page_id, _) = sess
        .append("valid".into(), 16, 16)
        .expect("a valid append should still use the first page");
    assert_eq!(page_id, 0);
}

#[test]
#[ignore = "U5: duplicate runtime keys must not leak allocations or change statistics"]
fn duplicate_key_append_preserves_placement_and_stats() {
    let mut sess = AtlasSession::new(guillotine_config(64, 64));

    let (original_page, original_frame) = sess
        .append("duplicate".into(), 16, 16)
        .expect("initial append");
    let stats_before = sess.stats();

    let _duplicate_result = sess.append("duplicate".into(), 16, 16);
    assert_runtime_stats_eq(&sess.stats(), &stats_before);

    let (current_page, current_frame) = sess
        .get_frame("duplicate")
        .expect("the original key must remain present");
    assert_eq!(current_page, original_page);
    assert_eq!(current_frame.frame, original_frame.frame);
    assert_eq!(current_frame.rotated, original_frame.rotated);
}

fn assert_runtime_stats_eq(actual: &RuntimeStats, expected: &RuntimeStats) {
    assert_eq!(actual.num_pages, expected.num_pages);
    assert_eq!(actual.num_textures, expected.num_textures);
    assert_eq!(actual.total_page_area, expected.total_page_area);
    assert_eq!(actual.total_used_area, expected.total_used_area);
    assert_eq!(actual.total_free_area, expected.total_free_area);
    assert_eq!(actual.occupancy, expected.occupancy);
    assert_eq!(actual.num_free_rects, expected.num_free_rects);
}

#[test]
fn test_get_frame() {
    let mut sess = AtlasSession::new(guillotine_config(256, 256));

    // Add some textures
    let (page_a, frame_a) = sess.append("sprite_a".into(), 64, 64).expect("append A");
    let (_page_b, _frame_b) = sess.append("sprite_b".into(), 32, 32).expect("append B");

    // Test get_frame
    let result = sess.get_frame("sprite_a");
    assert!(result.is_some());
    let (found_page, found_frame) = result.unwrap();
    assert_eq!(found_page, page_a);
    assert_eq!(found_frame.key, "sprite_a");
    assert_eq!(found_frame.frame.w, frame_a.frame.w);
    assert_eq!(found_frame.frame.h, frame_a.frame.h);

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
    assert_eq!(stats.num_textures, 0);
    assert_eq!(stats.total_page_area, 0);
    assert_eq!(stats.total_used_area, 0);
    assert_eq!(stats.occupancy, 0.0);

    // Add some textures
    sess.append("a".into(), 64, 64).expect("append A");
    sess.append("b".into(), 32, 32).expect("append B");

    let stats = sess.stats();
    assert_eq!(stats.num_pages, 1);
    assert_eq!(stats.num_textures, 2);
    assert!(stats.total_page_area > 0);
    assert!(stats.total_used_area > 0);
    assert!(stats.occupancy > 0.0);
    assert!(stats.occupancy <= 1.0);

    // Used area should be at least the sum of texture areas (plus padding)
    let min_used = 64 * 64 + 32 * 32;
    assert!(stats.total_used_area >= min_used as u64);

    // Free area should be positive
    assert!(stats.total_free_area > 0);

    // Total should equal used + free (approximately, due to padding)
    let total_accounted = stats.total_used_area + stats.total_free_area;
    assert!(total_accounted <= stats.total_page_area);
}

#[test]
fn test_runtime_stats_summary() {
    let mut sess = AtlasSession::new(guillotine_config(256, 256));

    sess.append("a".into(), 64, 64).expect("append");

    let stats = sess.stats();
    let summary = stats.summary();

    // Summary should contain key information
    assert!(summary.contains("Pages:"));
    assert!(summary.contains("Textures:"));
    assert!(summary.contains("Occupancy:"));
    assert!(summary.contains("Free:"));
    assert!(summary.contains("Used:"));
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
    let (page_a, _) = sess.append("sprite_a".into(), 64, 64).expect("append A");
    assert_eq!(sess.texture_count(), 1);

    // Evict it
    assert!(sess.evict_by_key("sprite_a"));
    assert_eq!(sess.texture_count(), 0);

    // Add new texture with same size - should reuse space
    let (page_b, _) = sess.append("sprite_b".into(), 64, 64).expect("append B");
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
    let (_page_id, frame) = sess.get_frame("a").expect("frame should exist");
    assert_eq!(frame.key, "a");

    // Evict by key
    assert!(sess.evict_by_key("a"));
    assert!(!sess.contains("a"));

    // Stats
    let stats = sess.stats();
    assert_eq!(stats.num_textures, 1);
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
    assert_eq!(stats.num_textures, 10);

    // Total page area should be num_pages * page_size
    let expected_total = (128 * 128) as u64 * stats.num_pages as u64;
    assert_eq!(stats.total_page_area, expected_total);
}
