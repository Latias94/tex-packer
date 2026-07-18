use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use tex_packer_core::export::to_json_array;
use tex_packer_core::model::{Atlas, Frame, FrameId, Meta, Page, PageId, Rect, Region, RegionId};

fn alias_atlas(alias_count: u32) -> Atlas {
    let region_id = RegionId::new(37);
    let frames = (0..alias_count)
        .map(|frame_id| {
            Frame::new(
                FrameId::new(frame_id),
                format!("alias-{frame_id}"),
                region_id,
                false,
                Rect::new(0, 0, 1, 1),
                (1, 1),
            )
        })
        .collect();
    let page = Page::try_new(
        PageId::new(11),
        16,
        16,
        vec![Region::new(
            region_id,
            Rect::new(1, 1, 1, 1),
            Rect::new(1, 1, 1, 1),
            false,
        )],
        frames,
    )
    .expect("valid alias benchmark page");
    Atlas::try_new(vec![page], Meta::default()).expect("valid alias benchmark atlas")
}

fn atlas_resolution(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("atlas_resolution");
    for alias_count in [1_000, 10_000] {
        let atlas = alias_atlas(alias_count);
        group.throughput(Throughput::Elements(u64::from(alias_count)));
        group.bench_with_input(
            BenchmarkId::from_parameter(alias_count),
            &atlas,
            |bencher, atlas| {
                bencher.iter(|| {
                    let checksum = atlas.pages().iter().flat_map(Page::resolved_frames).fold(
                        0u64,
                        |checksum, resolved| {
                            checksum
                                .wrapping_add(u64::from(resolved.frame().id().get()))
                                .wrapping_add(u64::from(resolved.region().id().get()))
                        },
                    );
                    black_box(checksum)
                });
            },
        );
    }
    group.finish();
}

fn atlas_export(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("atlas_export");
    for alias_count in [1_000, 10_000] {
        let atlas = alias_atlas(alias_count);
        group.throughput(Throughput::Elements(u64::from(alias_count)));
        group.bench_with_input(
            BenchmarkId::from_parameter(alias_count),
            &atlas,
            |bencher, atlas| {
                bencher.iter(|| black_box(to_json_array(black_box(atlas))));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, atlas_resolution, atlas_export);
criterion_main!(benches);
