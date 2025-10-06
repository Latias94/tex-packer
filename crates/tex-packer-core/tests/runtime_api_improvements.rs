use tex_packer_core::prelude::*;

#[test]
fn test_get_frame() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);

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
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);

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
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);

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
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);

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
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);

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
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);

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
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);

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
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);

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
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);

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
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);

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
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Shelf(ShelfPolicy::FirstFit));

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
    let cfg = PackerConfig::builder()
        .with_max_dimensions(128, 128)
        .build();
    let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);

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

