use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tex_packer_core::prelude::*;

fn generate_textures(count: usize, min_size: u32, max_size: u32) -> Vec<(String, u32, u32)> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..count)
        .map(|i| {
            let w = rng.gen_range(min_size..=max_size);
            let h = rng.gen_range(min_size..=max_size);
            (format!("tex_{}", i), w, h)
        })
        .collect()
}

fn bench_runtime_strategy(c: &mut Criterion) {
    let mut group = c.benchmark_group("runtime_strategies");

    let texture_counts = vec![50, 100, 200];

    for count in texture_counts {
        let textures = generate_textures(count, 16, 64);

        group.throughput(Throughput::Elements(count as u64));

        // Benchmark Guillotine
        group.bench_with_input(
            BenchmarkId::new("Guillotine", count),
            &textures,
            |b, textures| {
                b.iter(|| {
                    let cfg = PackerConfig::builder()
                        .with_max_dimensions(2048, 2048)
                        .build();
                    let mut session = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);
                    for (key, w, h) in textures {
                        let _ = session.append(key.clone(), *w, *h);
                    }
                    black_box(session)
                });
            },
        );

        // Benchmark Shelf NextFit
        group.bench_with_input(
            BenchmarkId::new("Shelf_NextFit", count),
            &textures,
            |b, textures| {
                b.iter(|| {
                    let cfg = PackerConfig::builder()
                        .with_max_dimensions(2048, 2048)
                        .build();
                    let mut session =
                        AtlasSession::new(cfg, RuntimeStrategy::Shelf(ShelfPolicy::NextFit));
                    for (key, w, h) in textures {
                        let _ = session.append(key.clone(), *w, *h);
                    }
                    black_box(session)
                });
            },
        );

        // Benchmark Shelf FirstFit
        group.bench_with_input(
            BenchmarkId::new("Shelf_FirstFit", count),
            &textures,
            |b, textures| {
                b.iter(|| {
                    let cfg = PackerConfig::builder()
                        .with_max_dimensions(2048, 2048)
                        .build();
                    let mut session =
                        AtlasSession::new(cfg, RuntimeStrategy::Shelf(ShelfPolicy::FirstFit));
                    for (key, w, h) in textures {
                        let _ = session.append(key.clone(), *w, *h);
                    }
                    black_box(session)
                });
            },
        );

        // Benchmark Skyline BottomLeft
        group.bench_with_input(
            BenchmarkId::new("Skyline_BottomLeft", count),
            &textures,
            |b, textures| {
                b.iter(|| {
                    let cfg = PackerConfig::builder()
                        .with_max_dimensions(2048, 2048)
                        .build();
                    let mut session = AtlasSession::new(
                        cfg,
                        RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft),
                    );
                    for (key, w, h) in textures {
                        let _ = session.append(key.clone(), *w, *h);
                    }
                    black_box(session)
                });
            },
        );

        // Benchmark Skyline MinWaste
        group.bench_with_input(
            BenchmarkId::new("Skyline_MinWaste", count),
            &textures,
            |b, textures| {
                b.iter(|| {
                    let cfg = PackerConfig::builder()
                        .with_max_dimensions(2048, 2048)
                        .build();
                    let mut session = AtlasSession::new(
                        cfg,
                        RuntimeStrategy::Skyline(SkylineHeuristic::MinWaste),
                    );
                    for (key, w, h) in textures {
                        let _ = session.append(key.clone(), *w, *h);
                    }
                    black_box(session)
                });
            },
        );
    }

    group.finish();
}

fn bench_append_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("append_operations");

    let cfg = PackerConfig::builder()
        .with_max_dimensions(2048, 2048)
        .build();

    // Benchmark single append for each strategy
    group.bench_function("Guillotine_single_append", |b| {
        b.iter(|| {
            let mut session = AtlasSession::new(cfg.clone(), RuntimeStrategy::Guillotine);
            black_box(session.append("test".into(), 64, 64))
        });
    });

    group.bench_function("Shelf_single_append", |b| {
        b.iter(|| {
            let mut session =
                AtlasSession::new(cfg.clone(), RuntimeStrategy::Shelf(ShelfPolicy::NextFit));
            black_box(session.append("test".into(), 64, 64))
        });
    });

    group.bench_function("Skyline_single_append", |b| {
        b.iter(|| {
            let mut session = AtlasSession::new(
                cfg.clone(),
                RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft),
            );
            black_box(session.append("test".into(), 64, 64))
        });
    });

    group.finish();
}

fn bench_query_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_operations");

    let cfg = PackerConfig::builder()
        .with_max_dimensions(2048, 2048)
        .build();

    // Setup: Create session with 100 textures
    let mut session = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);
    for i in 0..100 {
        let _ = session.append(format!("tex_{}", i), 32, 32);
    }

    group.bench_function("get_frame", |b| {
        b.iter(|| black_box(session.get_frame("tex_50")));
    });

    group.bench_function("contains", |b| {
        b.iter(|| black_box(session.contains("tex_50")));
    });

    group.bench_function("keys", |b| {
        b.iter(|| black_box(session.keys()));
    });

    group.bench_function("texture_count", |b| {
        b.iter(|| black_box(session.texture_count()));
    });

    group.bench_function("stats", |b| {
        b.iter(|| black_box(session.stats()));
    });

    group.finish();
}

fn bench_evict_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("evict_operations");

    let cfg = PackerConfig::builder()
        .with_max_dimensions(2048, 2048)
        .build();

    group.bench_function("evict_by_key", |b| {
        b.iter_batched(
            || {
                let mut session = AtlasSession::new(cfg.clone(), RuntimeStrategy::Guillotine);
                for i in 0..50 {
                    let _ = session.append(format!("tex_{}", i), 32, 32);
                }
                session
            },
            |mut session| black_box(session.evict_by_key("tex_25")),
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_space_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("space_efficiency");

    // Test with uniform sizes
    let uniform_textures: Vec<(String, u32, u32)> =
        (0..100).map(|i| (format!("tex_{}", i), 64, 64)).collect();

    // Test with varied sizes
    let varied_textures = generate_textures(100, 16, 128);

    for (name, textures) in [("uniform", &uniform_textures), ("varied", &varied_textures)] {
        for strategy_name in ["Guillotine", "Shelf_NextFit", "Skyline_BottomLeft"] {
            group.bench_with_input(
                BenchmarkId::new(format!("{}_{}", strategy_name, name), textures.len()),
                textures,
                |b, textures| {
                    b.iter(|| {
                        let cfg = PackerConfig::builder()
                            .with_max_dimensions(1024, 1024)
                            .build();

                        let strategy = match strategy_name {
                            "Guillotine" => RuntimeStrategy::Guillotine,
                            "Shelf_NextFit" => RuntimeStrategy::Shelf(ShelfPolicy::NextFit),
                            "Skyline_BottomLeft" => {
                                RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft)
                            }
                            _ => unreachable!(),
                        };

                        let mut session = AtlasSession::new(cfg, strategy);
                        for (key, w, h) in textures {
                            let _ = session.append(key.clone(), *w, *h);
                        }

                        let stats = session.stats();
                        black_box(stats.occupancy)
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_with_rotation(c: &mut Criterion) {
    let mut group = c.benchmark_group("with_rotation");

    let textures = generate_textures(100, 32, 128);

    for allow_rotation in [false, true] {
        let rotation_str = if allow_rotation {
            "enabled"
        } else {
            "disabled"
        };

        group.bench_with_input(
            BenchmarkId::new(format!("Skyline_rotation_{}", rotation_str), textures.len()),
            &textures,
            |b, textures| {
                b.iter(|| {
                    let cfg = PackerConfig::builder()
                        .with_max_dimensions(2048, 2048)
                        .allow_rotation(allow_rotation)
                        .build();

                    let mut session = AtlasSession::new(
                        cfg,
                        RuntimeStrategy::Skyline(SkylineHeuristic::BottomLeft),
                    );
                    for (key, w, h) in textures {
                        let _ = session.append(key.clone(), *w, *h);
                    }
                    black_box(session)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_runtime_strategy,
    bench_append_operations,
    bench_query_operations,
    bench_evict_operations,
    bench_space_efficiency,
    bench_with_rotation,
);
criterion_main!(benches);
