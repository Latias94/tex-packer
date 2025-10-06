use tex_packer_core::prelude::*;

#[test]
fn test_skyline_bottom_left_basic() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut session =
        AtlasSession::new(cfg, RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft));

    // Add some textures
    let result1 = session.append("tex1".into(), 64, 64);
    assert!(result1.is_ok());
    let (page_id, frame) = result1.unwrap();
    assert_eq!(page_id, 0);
    assert_eq!(frame.frame.w, 64);
    assert_eq!(frame.frame.h, 64);

    let result2 = session.append("tex2".into(), 32, 32);
    assert!(result2.is_ok());

    let result3 = session.append("tex3".into(), 48, 48);
    assert!(result3.is_ok());

    // Verify all textures exist
    assert!(session.contains("tex1"));
    assert!(session.contains("tex2"));
    assert!(session.contains("tex3"));
    assert_eq!(session.texture_count(), 3);
}

#[test]
fn test_skyline_min_waste_basic() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut session = AtlasSession::new(cfg, RuntimeStrategy::Skyline(SkylineHeuristic::MinWaste));

    // Add textures
    let result1 = session.append("a".into(), 100, 50);
    assert!(result1.is_ok());

    let result2 = session.append("b".into(), 50, 50);
    assert!(result2.is_ok());

    let result3 = session.append("c".into(), 50, 100);
    assert!(result3.is_ok());

    assert_eq!(session.texture_count(), 3);
}

#[test]
fn test_skyline_with_rotation() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .allow_rotation(true)
        .build();

    let mut session =
        AtlasSession::new(cfg, RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft));

    // Add a wide texture
    let result = session.append("wide".into(), 200, 50);
    assert!(result.is_ok());

    // Add a tall texture (might be rotated)
    let result = session.append("tall".into(), 50, 150);
    assert!(result.is_ok());

    assert_eq!(session.texture_count(), 2);
}

#[test]
fn test_skyline_stats() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut session =
        AtlasSession::new(cfg, RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft));

    // Add textures
    session.append("a".into(), 64, 64).unwrap();
    session.append("b".into(), 64, 64).unwrap();
    session.append("c".into(), 64, 64).unwrap();

    let stats = session.stats();
    assert_eq!(stats.num_pages, 1);
    assert_eq!(stats.num_textures, 3);
    assert!(stats.occupancy > 0.0);
    assert!(stats.total_used_area > 0);

    // Print summary
    println!("{}", stats.summary());
}

#[test]
fn test_skyline_evict_and_reuse() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut session =
        AtlasSession::new(cfg, RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft));

    // Add textures
    session.append("temp1".into(), 64, 64).unwrap();
    session.append("temp2".into(), 64, 64).unwrap();

    assert_eq!(session.texture_count(), 2);

    // Evict one
    assert!(session.evict_by_key("temp1"));
    assert_eq!(session.texture_count(), 1);
    assert!(!session.contains("temp1"));
    assert!(session.contains("temp2"));

    // Add new texture (note: skyline doesn't optimize space reuse like guillotine)
    let result = session.append("new".into(), 32, 32);
    assert!(result.is_ok());
    assert_eq!(session.texture_count(), 2);
}

#[test]
fn test_skyline_multiple_pages() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(128, 128)
        .build();

    let mut session =
        AtlasSession::new(cfg, RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft));

    // Fill first page
    for i in 0..10 {
        let result = session.append(format!("tex{}", i), 40, 40);
        assert!(result.is_ok());
    }

    let stats = session.stats();
    assert!(stats.num_pages >= 1);
    assert_eq!(stats.num_textures, 10);
}

#[test]
fn test_skyline_get_frame() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut session = AtlasSession::new(cfg, RuntimeStrategy::Skyline(SkylineHeuristic::MinWaste));

    session.append("sprite".into(), 64, 64).unwrap();

    // Get frame info
    let frame_info = session.get_frame("sprite");
    assert!(frame_info.is_some());

    let (page_id, frame) = frame_info.unwrap();
    assert_eq!(page_id, 0);
    assert_eq!(frame.frame.w, 64);
    assert_eq!(frame.frame.h, 64);
}

#[test]
fn test_skyline_keys() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut session =
        AtlasSession::new(cfg, RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft));

    session.append("a".into(), 32, 32).unwrap();
    session.append("b".into(), 32, 32).unwrap();
    session.append("c".into(), 32, 32).unwrap();

    let keys = session.keys();
    assert_eq!(keys.len(), 3);
    assert!(keys.contains(&"a"));
    assert!(keys.contains(&"b"));
    assert!(keys.contains(&"c"));
}

#[test]
fn test_skyline_snapshot() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut session =
        AtlasSession::new(cfg, RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft));

    session.append("test1".into(), 64, 64).unwrap();
    session.append("test2".into(), 64, 64).unwrap();

    let atlas = session.snapshot_atlas();
    assert_eq!(atlas.pages.len(), 1);
    assert_eq!(atlas.pages[0].frames.len(), 2);
}

#[test]
fn test_skyline_padding() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .texture_padding(4)
        .build();

    let mut session =
        AtlasSession::new(cfg, RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft));

    let result = session.append("padded".into(), 64, 64);
    assert!(result.is_ok());

    let (_page_id, frame) = result.unwrap();
    // Frame size should be the original size
    assert_eq!(frame.frame.w, 64);
    assert_eq!(frame.frame.h, 64);
}

#[test]
fn test_skyline_border_padding() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .border_padding(8)
        .build();

    let mut session =
        AtlasSession::new(cfg, RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft));

    let result = session.append("bordered".into(), 64, 64);
    assert!(result.is_ok());

    let (_page_id, frame) = result.unwrap();
    // First texture should be placed at border_padding offset
    assert!(frame.frame.x >= 8);
    assert!(frame.frame.y >= 8);
}

#[test]
fn test_skyline_comparison() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    // Test BottomLeft
    let mut session_bl = AtlasSession::new(
        cfg.clone(),
        RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft),
    );
    for i in 0..5 {
        session_bl.append(format!("tex{}", i), 50, 50).unwrap();
    }
    let stats_bl = session_bl.stats();

    // Test MinWaste
    let mut session_mw =
        AtlasSession::new(cfg, RuntimeStrategy::Skyline(SkylineHeuristic::MinWaste));
    for i in 0..5 {
        session_mw.append(format!("tex{}", i), 50, 50).unwrap();
    }
    let stats_mw = session_mw.stats();

    // Both should pack successfully
    assert_eq!(stats_bl.num_textures, 5);
    assert_eq!(stats_mw.num_textures, 5);

    println!("BottomLeft: {}", stats_bl.summary());
    println!("MinWaste: {}", stats_mw.summary());
}

#[test]
fn test_skyline_large_texture() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(512, 512)
        .build();

    let mut session =
        AtlasSession::new(cfg, RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft));

    // Add a large texture
    let result = session.append("large".into(), 400, 400);
    assert!(result.is_ok());

    // Add smaller textures
    session.append("small1".into(), 50, 50).unwrap();
    session.append("small2".into(), 50, 50).unwrap();

    assert_eq!(session.texture_count(), 3);
}

#[test]
fn test_skyline_many_small_textures() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .build();

    let mut session = AtlasSession::new(cfg, RuntimeStrategy::Skyline(SkylineHeuristic::MinWaste));

    // Add many small textures
    for i in 0..20 {
        let result = session.append(format!("small{}", i), 16, 16);
        assert!(result.is_ok());
    }

    assert_eq!(session.texture_count(), 20);

    let stats = session.stats();
    println!("Packed 20 small textures: {}", stats.summary());
}
