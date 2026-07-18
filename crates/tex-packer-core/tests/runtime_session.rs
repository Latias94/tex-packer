use tex_packer_core::config::{
    GuillotineChoice, GuillotineSplit, PageConfig, PageConfigBuilder, RuntimeConfig,
    RuntimeStrategy,
};
use tex_packer_core::model::{PageId, Rect, Region};
use tex_packer_core::runtime::AtlasSession;

fn runtime_config(page: PageConfigBuilder, strategy: RuntimeStrategy) -> RuntimeConfig {
    RuntimeConfig::builder()
        .page_config(page.build().expect("valid page config"))
        .strategy(strategy)
        .build()
        .expect("valid runtime config")
}

fn guillotine_strategy() -> RuntimeStrategy {
    RuntimeStrategy::Guillotine {
        choice: GuillotineChoice::BestAreaFit,
        split: GuillotineSplit::SplitShorterLeftoverAxis,
    }
}

#[test]
fn runtime_append_evict_reuse_space() {
    let cfg = runtime_config(
        PageConfig::builder()
            .max_dimensions(256, 256)
            .allow_rotation(true)
            .texture_padding(2)
            .texture_extrusion(1),
        guillotine_strategy(),
    );
    let mut sess = AtlasSession::new(cfg);

    // Append two items
    let a = sess.append("A".into(), 40, 32).expect("append A");
    let b = sess.append("B".into(), 48, 24).expect("append B");
    assert_eq!(a.page_id(), PageId::new(0));
    assert_eq!(a.content().w, 40);
    assert_frame_size_matches_rotation(&b, 48, 24);

    // Evict A, then insert C with similar size to ensure reuse
    assert!(sess.evict(a.page_id(), "A"));
    let c = sess.append("C".into(), 40, 32).expect("append C");

    // Snapshot and basic sanity: frames should be disjoint
    let snap = sess.snapshot_atlas().expect("valid runtime snapshot");
    let regions: Vec<_> = snap
        .pages()
        .iter()
        .flat_map(|page| page.regions().iter().map(Region::content))
        .collect();
    assert!(disjoint(&regions));

    // C should fit; not asserting exact coords, but w/h preserved
    assert_frame_size_matches_rotation(&c, 40, 32);
}

#[test]
fn runtime_guillotine_reports_rotated_frame_dimensions_in_atlas_orientation() {
    let cfg = runtime_config(
        PageConfig::builder()
            .max_dimensions(256, 256)
            .allow_rotation(true)
            .texture_padding(2)
            .texture_extrusion(1),
        guillotine_strategy(),
    );
    let mut sess = AtlasSession::new(cfg);

    let a = sess.append("A".into(), 40, 32).expect("append A");
    let b = sess.append("B".into(), 48, 24).expect("append B");

    assert_eq!(a.page_id(), PageId::new(0));
    assert!(
        b.rotated(),
        "second item should use the rotated reused slot"
    );
    assert_eq!(b.content().w, 24);
    assert_eq!(b.content().h, 48);
    assert_eq!(b.frame().source(), Rect::new(0, 0, 48, 24));
    assert_eq!(b.frame().source_size(), (48, 24));
}

#[test]
fn runtime_snapshot_resolves_placements_and_matches_domain_stats() {
    let cfg = runtime_config(
        PageConfig::builder()
            .max_dimensions(128, 128)
            .allow_rotation(true)
            .texture_padding(2)
            .texture_extrusion(1),
        guillotine_strategy(),
    );
    let mut session = AtlasSession::new(cfg);
    let first = session
        .append("first".into(), 32, 16)
        .expect("append first");
    let second = session
        .append("second".into(), 24, 20)
        .expect("append second");

    let snapshot = session.snapshot_atlas().expect("valid runtime snapshot");
    assert_eq!(
        snapshot,
        session.snapshot_atlas().expect("repeated runtime snapshot")
    );

    for placement in [&first, &second] {
        let page = snapshot
            .page(placement.page_id())
            .expect("placement page resolves");
        let frame = page
            .frame(placement.frame_id())
            .expect("placement frame resolves");
        let region = page
            .region(placement.region_id())
            .expect("placement region resolves");
        assert_eq!(frame.region_id(), placement.region_id());
        assert_eq!(region.content(), placement.content());
        assert_eq!(region.allocation(), placement.allocation());
        assert_eq!(region.rotated(), placement.rotated());
    }

    let runtime_stats = session.stats();
    let domain_stats = snapshot.stats();
    assert_eq!(runtime_stats.num_pages, domain_stats.num_pages);
    assert_eq!(runtime_stats.num_frames, domain_stats.num_frames);
    assert_eq!(runtime_stats.num_regions, domain_stats.num_regions);
    assert_eq!(runtime_stats.num_aliases, domain_stats.num_aliases);
    assert_eq!(runtime_stats.content_area, domain_stats.content_area);
    assert_eq!(runtime_stats.allocation_area, domain_stats.allocation_area);
    assert_eq!(
        runtime_stats.content_occupancy,
        domain_stats.content_occupancy
    );
    assert_eq!(
        runtime_stats.allocation_occupancy,
        domain_stats.allocation_occupancy
    );
}

fn disjoint(rectangles: &[Rect]) -> bool {
    for i in 0..rectangles.len() {
        for j in (i + 1)..rectangles.len() {
            let a = &rectangles[i];
            let b = &rectangles[j];
            let a_x2 = a.x + a.w;
            let a_y2 = a.y + a.h;
            let b_x2 = b.x + b.w;
            let b_y2 = b.y + b.h;
            let overlap = !(a.x >= b_x2 || b.x >= a_x2 || a.y >= b_y2 || b.y >= a_y2);
            if overlap {
                return false;
            }
        }
    }
    true
}

fn assert_frame_size_matches_rotation(
    placement: &tex_packer_core::runtime::RuntimePlacement,
    source_w: u32,
    source_h: u32,
) {
    let expected = if placement.rotated() {
        (source_h, source_w)
    } else {
        (source_w, source_h)
    };
    let content = placement.content();
    assert_eq!((content.w, content.h), expected);
}
