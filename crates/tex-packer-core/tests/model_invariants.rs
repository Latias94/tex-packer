use std::fmt::Debug;

use tex_packer_core::error::TexPackerError;
use tex_packer_core::model::{Atlas, Frame, FrameId, Meta, Page, PageId, Rect, Region, RegionId};

fn region(id: u32, content: Rect, allocation: Rect, rotated: bool) -> Region {
    Region::new(RegionId::new(id), content, allocation, rotated)
}

fn frame(
    id: u32,
    key: &str,
    region_id: u32,
    trimmed: bool,
    source: Rect,
    source_size: (u32, u32),
) -> Frame {
    Frame::new(
        FrameId::new(id),
        key.to_owned(),
        RegionId::new(region_id),
        trimmed,
        source,
        source_size,
    )
}

fn assert_invariant<T: Debug>(
    result: Result<T, TexPackerError>,
    expected_context: &str,
    expected_reason: &str,
) {
    let error = result.expect_err("invalid aggregate must be rejected");
    match &error {
        TexPackerError::InvariantViolation { context, reason } => {
            assert!(
                context.contains(expected_context),
                "expected context {expected_context:?}, got {context:?}"
            );
            assert!(
                reason.contains(expected_reason),
                "expected reason {expected_reason:?}, got {reason:?}"
            );
        }
        other => panic!("expected invariant error, got {other:?}"),
    }
}

fn single_region_page(page_id: u32) -> Page {
    Page::try_new(
        PageId::new(page_id),
        16,
        16,
        vec![region(
            9,
            Rect::new(2, 2, 4, 3),
            Rect::new(1, 1, 6, 5),
            false,
        )],
        vec![frame(20, "sprite", 9, false, Rect::new(0, 0, 4, 3), (4, 3))],
    )
    .expect("valid page")
}

#[test]
fn identities_are_opaque_lookups_in_stable_record_order() {
    let page = Page::try_new(
        PageId::new(42),
        12,
        8,
        vec![
            region(50, Rect::new(1, 1, 2, 2), Rect::new(0, 0, 4, 4), false),
            region(2, Rect::new(5, 1, 2, 2), Rect::new(4, 0, 4, 4), false),
        ],
        vec![
            frame(70, "duplicate-key", 2, false, Rect::new(0, 0, 2, 2), (2, 2)),
            frame(4, "duplicate-key", 50, false, Rect::new(0, 0, 2, 2), (2, 2)),
        ],
    )
    .expect("duplicate user keys are valid offline identities");

    assert_eq!(page.region(RegionId::new(2)).expect("region").id().get(), 2);
    assert_eq!(page.frame(FrameId::new(4)).expect("frame").id().get(), 4);
    assert!(page.region(RegionId::new(0)).is_none());
    assert!(page.frame(FrameId::new(0)).is_none());

    let resolved = page
        .resolved_frames()
        .map(|resolved| {
            (
                resolved.page_id().get(),
                resolved.frame().id().get(),
                resolved.frame().key().to_owned(),
                resolved.region().id().get(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        resolved,
        [
            (42, 70, "duplicate-key".to_owned(), 2),
            (42, 4, "duplicate-key".to_owned(), 50),
        ]
    );

    let atlas = Atlas::try_new(vec![page], Meta::default()).expect("valid atlas");
    assert_eq!(atlas.page(PageId::new(42)).expect("page").size(), (12, 8));
    assert!(atlas.page(PageId::new(0)).is_none());
}

#[test]
fn page_and_record_identities_must_be_unique_in_their_scope() {
    let duplicate_regions = Page::try_new(
        PageId::new(3),
        16,
        16,
        vec![
            region(8, Rect::new(1, 1, 2, 2), Rect::new(0, 0, 4, 4), false),
            region(8, Rect::new(5, 1, 2, 2), Rect::new(4, 0, 4, 4), false),
        ],
        vec![],
    );
    assert_invariant(duplicate_regions, "region 8", "duplicate region identity");

    let duplicate_frames = Page::try_new(
        PageId::new(3),
        16,
        16,
        vec![region(
            8,
            Rect::new(1, 1, 2, 2),
            Rect::new(0, 0, 4, 4),
            false,
        )],
        vec![
            frame(12, "first", 8, false, Rect::new(0, 0, 2, 2), (2, 2)),
            frame(12, "second", 8, false, Rect::new(0, 0, 2, 2), (2, 2)),
        ],
    );
    assert_invariant(duplicate_frames, "frame 12", "duplicate frame identity");

    let duplicate_pages = Atlas::try_new(
        vec![single_region_page(21), single_region_page(21)],
        Meta::default(),
    );
    assert_invariant(duplicate_pages, "page 21", "duplicate page identity");
}

#[test]
fn every_frame_resolves_and_every_region_is_referenced() {
    let dangling = Page::try_new(
        PageId::new(4),
        8,
        8,
        vec![],
        vec![frame(
            1,
            "dangling",
            99,
            false,
            Rect::new(0, 0, 1, 1),
            (1, 1),
        )],
    );
    assert_invariant(dangling, "frame 1", "missing region 99");

    let orphan = Page::try_new(
        PageId::new(4),
        8,
        8,
        vec![region(
            6,
            Rect::new(1, 1, 2, 2),
            Rect::new(0, 0, 4, 4),
            false,
        )],
        vec![],
    );
    assert_invariant(orphan, "region 6", "not referenced");
}

#[test]
fn region_geometry_must_be_non_empty_nested_and_inside_the_page() {
    let empty_content = Page::try_new(
        PageId::new(5),
        8,
        8,
        vec![region(
            1,
            Rect::new(1, 1, 0, 2),
            Rect::new(0, 0, 4, 4),
            false,
        )],
        vec![],
    );
    assert_invariant(
        empty_content,
        "region 1",
        "content rectangle must be non-empty",
    );

    let empty_allocation = Page::try_new(
        PageId::new(5),
        8,
        8,
        vec![region(
            1,
            Rect::new(1, 1, 1, 1),
            Rect::new(0, 0, 0, 4),
            false,
        )],
        vec![],
    );
    assert_invariant(
        empty_allocation,
        "region 1",
        "allocation rectangle must be non-empty",
    );

    let content_outside = Page::try_new(
        PageId::new(5),
        8,
        8,
        vec![region(
            1,
            Rect::new(0, 0, 2, 2),
            Rect::new(1, 1, 3, 3),
            false,
        )],
        vec![],
    );
    assert_invariant(content_outside, "region 1", "inside allocation");

    let allocation_outside = Page::try_new(
        PageId::new(5),
        8,
        8,
        vec![region(
            1,
            Rect::new(7, 7, 1, 1),
            Rect::new(7, 7, 2, 2),
            false,
        )],
        vec![],
    );
    assert_invariant(allocation_outside, "region 1", "inside page bounds");

    let overflowed_edge = Page::try_new(
        PageId::new(5),
        u32::MAX,
        u32::MAX,
        vec![region(
            1,
            Rect::new(u32::MAX - 1, 0, 3, 1),
            Rect::new(u32::MAX - 1, 0, 3, 1),
            false,
        )],
        vec![],
    );
    assert_invariant(overflowed_edge, "region 1", "inside page bounds");

    let zero_sized_page = Page::try_new(PageId::new(5), 0, 8, vec![], vec![]);
    assert_invariant(zero_sized_page, "page 5", "dimensions must be positive");
}

#[test]
fn physical_allocations_must_not_overlap_but_may_touch() {
    let overlapping = Page::try_new(
        PageId::new(6),
        16,
        16,
        vec![
            region(1, Rect::new(1, 1, 2, 2), Rect::new(0, 0, 4, 4), false),
            region(2, Rect::new(4, 4, 1, 1), Rect::new(3, 3, 4, 4), false),
        ],
        vec![],
    );
    assert_invariant(overlapping, "page 6", "regions 1 and 2 overlap");

    let touching = Page::try_new(
        PageId::new(6),
        8,
        4,
        vec![
            region(1, Rect::new(1, 1, 2, 2), Rect::new(0, 0, 4, 4), false),
            region(2, Rect::new(5, 1, 2, 2), Rect::new(4, 0, 4, 4), false),
        ],
        vec![
            frame(1, "left", 1, false, Rect::new(0, 0, 2, 2), (2, 2)),
            frame(2, "right", 2, false, Rect::new(0, 0, 2, 2), (2, 2)),
        ],
    );
    assert!(touching.is_ok(), "touching allocation edges do not overlap");
}

#[test]
fn source_geometry_must_be_valid_and_match_physical_rotation() {
    let base_region = || region(1, Rect::new(1, 1, 2, 3), Rect::new(0, 0, 4, 5), false);

    let empty_source = Page::try_new(
        PageId::new(7),
        8,
        8,
        vec![base_region()],
        vec![frame(1, "empty", 1, false, Rect::new(0, 0, 0, 3), (2, 3))],
    );
    assert_invariant(
        empty_source,
        "frame 1",
        "source rectangle must be non-empty",
    );

    let source_outside = Page::try_new(
        PageId::new(7),
        8,
        8,
        vec![base_region()],
        vec![frame(1, "outside", 1, false, Rect::new(1, 0, 2, 3), (2, 3))],
    );
    assert_invariant(source_outside, "frame 1", "inside source size");

    let wrong_unrotated_size = Page::try_new(
        PageId::new(7),
        8,
        8,
        vec![base_region()],
        vec![frame(
            1,
            "wrong-size",
            1,
            false,
            Rect::new(0, 0, 3, 2),
            (3, 2),
        )],
    );
    assert_invariant(
        wrong_unrotated_size,
        "frame 1",
        "does not match region content",
    );

    let wrong_rotated_size = Page::try_new(
        PageId::new(7),
        8,
        8,
        vec![region(
            1,
            Rect::new(1, 1, 2, 3),
            Rect::new(0, 0, 4, 5),
            true,
        )],
        vec![frame(
            1,
            "wrong-rotation",
            1,
            false,
            Rect::new(0, 0, 2, 3),
            (2, 3),
        )],
    );
    assert_invariant(wrong_rotated_size, "frame 1", "rotated=true");
}

#[test]
fn statistics_count_logical_and_physical_facts_once() {
    let page = Page::try_new(
        PageId::new(8),
        10,
        10,
        vec![
            region(10, Rect::new(1, 1, 2, 3), Rect::new(0, 0, 4, 5), true),
            region(20, Rect::new(5, 0, 1, 4), Rect::new(5, 0, 2, 5), false),
        ],
        vec![
            frame(100, "original", 10, false, Rect::new(0, 0, 3, 2), (3, 2)),
            frame(101, "alias", 10, true, Rect::new(1, 1, 3, 2), (5, 4)),
            frame(102, "other", 20, false, Rect::new(0, 0, 1, 4), (1, 4)),
        ],
    )
    .expect("valid numeric fixture");
    let atlas = Atlas::try_new(vec![page], Meta::default()).expect("valid atlas");
    let stats = atlas.stats();

    assert_eq!(stats.num_pages, 1);
    assert_eq!(stats.num_frames, 3);
    assert_eq!(stats.num_regions, 2);
    assert_eq!(stats.num_aliases, 1);
    assert_eq!(stats.num_rotated_regions, 1);
    assert_eq!(stats.num_trimmed_frames, 1);
    assert_eq!(stats.page_area, 100);
    assert_eq!(stats.content_area, 10);
    assert_eq!(stats.allocation_area, 30);
    assert_eq!(stats.content_occupancy, 0.1);
    assert_eq!(stats.allocation_occupancy, 0.3);
    assert_eq!(stats.unallocated_area(), 70);
    assert_eq!(stats.allocation_waste_percentage(), 70.0);
}

#[test]
fn empty_and_extreme_atlases_have_defined_non_overflowing_statistics() {
    let empty = Atlas::try_new(vec![], Meta::default()).expect("empty atlas is valid");
    let empty_stats = empty.stats();
    assert_eq!(empty_stats.page_area, 0);
    assert_eq!(empty_stats.content_occupancy, 0.0);
    assert_eq!(empty_stats.allocation_occupancy, 0.0);
    assert_eq!(empty_stats.allocation_waste_percentage(), 0.0);

    let huge_page = |page_id, frame_id| {
        Page::try_new(
            PageId::new(page_id),
            u32::MAX,
            u32::MAX,
            vec![region(
                1,
                Rect::new(0, 0, u32::MAX, u32::MAX),
                Rect::new(0, 0, u32::MAX, u32::MAX),
                false,
            )],
            vec![frame(
                frame_id,
                "huge",
                1,
                false,
                Rect::new(0, 0, u32::MAX, u32::MAX),
                (u32::MAX, u32::MAX),
            )],
        )
        .expect("maximum coordinate page remains representable")
    };
    let huge = Atlas::try_new(vec![huge_page(1, 1), huge_page(2, 2)], Meta::default())
        .expect("valid huge atlas");
    let expected = u128::from(u32::MAX) * u128::from(u32::MAX) * 2;
    assert_eq!(huge.stats().page_area, expected);
    assert!(huge.stats().page_area > u128::from(u64::MAX));
}
