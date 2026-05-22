# Offline Pipeline And Placement Geometry Refactor — Evidence And Gates

Status: Complete
Last updated: 2026-05-22

## Smallest Current Repro

The current core regression net is:

```bash
cargo nextest run -p tex-packer-core
```

Fresh final result: 96/96 tests passed on 2026-05-22.

## Gate Set

### Targeted Iteration Gate — offline pipeline

```bash
cargo nextest run -p tex-packer-core --test layout_vs_images --test pack_stats --test boundary_conditions --test padding_extrude_offsets
```

Covers image/layout agreement, stats semantics, edge dimensions, and padding/extrusion offsets.

### Targeted Iteration Gate — placement geometry

```bash
cargo nextest run -p tex-packer-core --test skyline_rotation_fit --test maxrects_rotation_fit --test guillotine_rotation_fit --test padding_extrude_offsets
```

Covers rotation and frame offset invariants across the three offline algorithms.

### Targeted Iteration Gate — runtime geometry reuse

```bash
cargo nextest run -p tex-packer-core --test runtime_api_improvements --test runtime_atlas_tests --test runtime_session --test runtime_skyline --test runtime_shelf
```

Covers append, evict, update region, and runtime packing semantics for runtime use of shared placement geometry.

### Package Gate

```bash
cargo fmt --check
cargo check -p tex-packer-core
cargo nextest run -p tex-packer-core
```

### Broader Closeout Gate

```bash
cargo nextest run --workspace
```

### Review Gate

Self-review was recorded in `CLOSEOUT.md` before marking the lane complete. No blocking findings remain.

## Evidence Anchors

- docs/workstreams/offline-pipeline-placement-geometry-v1/DESIGN.md
- docs/workstreams/offline-pipeline-placement-geometry-v1/TODO.md
- docs/workstreams/offline-pipeline-placement-geometry-v1/MILESTONES.md
- docs/workstreams/offline-pipeline-placement-geometry-v1/CLOSEOUT.md
- crates/tex-packer-core/src/pipeline.rs
- crates/tex-packer-core/src/geometry.rs
- crates/tex-packer-core/src/packer/*.rs
- crates/tex-packer-core/src/runtime.rs
- crates/tex-packer-core/tests/layout_vs_images.rs
- crates/tex-packer-core/tests/padding_extrude_offsets.rs
- crates/tex-packer-core/tests/runtime_session.rs
- crates/tex-packer-core/tests/runtime_atlas_tests.rs

## Evidence Log

- 2026-05-22: Workstream opened. Baseline from earlier session: `cargo nextest run -p tex-packer-core` passed 91/91 on main before refactor.
- 2026-05-22: OPPG-020 completed. Extracted shared `PreparedItem<T>`, `PackedPage`, `pack_prepared_pages`, `pack_pages_for_family`, `build_atlas`, and render adapter path in `crates/tex-packer-core/src/pipeline.rs`.
- 2026-05-22: OPPG-020 targeted gate passed: `cargo nextest run -p tex-packer-core --test layout_vs_images --test pack_stats --test boundary_conditions --test padding_extrude_offsets` — 8/8 passed.
- 2026-05-22: OPPG-020 package checks passed: `cargo check -p tex-packer-core`; `cargo fmt --check`; `cargo nextest run -p tex-packer-core` — 91/91 passed.
- 2026-05-22: Documented placement-geometry filter command using bare test names produced 0 tests; reran with `--test` selectors as the authoritative targeted gate.
- 2026-05-22: OPPG-040 targeted gate passed: `cargo nextest run -p tex-packer-core --test skyline_rotation_fit --test maxrects_rotation_fit --test guillotine_rotation_fit --test padding_extrude_offsets` — 6/6 passed.
- 2026-05-22: OPPG-050 first runtime gate exposed a runtime rotated-frame metadata mismatch: old runtime `make_frame` kept original dimensions even when rotated. The shared geometry path reports frame dimensions in atlas orientation, matching the `Frame` model and offline algorithms. Added `runtime_guillotine_reports_rotated_frame_dimensions_in_atlas_orientation`.
- 2026-05-22: OPPG-050 runtime gate passed: `cargo nextest run -p tex-packer-core --test runtime_api_improvements --test runtime_atlas_tests --test runtime_session --test runtime_skyline --test runtime_shelf` — 46/46 passed.
- 2026-05-22: OPPG-030 regression gate passed: `cargo nextest run -p tex-packer-core --test layout_vs_images --test padding_extrude_offsets --test force_max_and_border --test pow2_square` — 15/15 passed.
- 2026-05-22: Final package gate passed: `cargo fmt --check`; `cargo check -p tex-packer-core`; `cargo nextest run -p tex-packer-core` — 96/96 passed.
- 2026-05-22: Broader closeout gate passed: `cargo nextest run --workspace` — 96/96 passed. Existing GUI dead-code warnings were emitted, with no test failures.

## Notes

- `crates/tex-packer-core/src/geometry.rs` is crate-private, so this lane does not add a new public helper surface.
- Public packing functions remain `pack_images`, `pack_layout`, and `pack_layout_items`.
- Runtime rotated-frame metadata now follows the existing `Frame` contract: `frame.w`/`frame.h` are atlas-orientation dimensions, and original dimensions remain in `source_size`.
- Identical-content dedupe remains intentionally out of scope for this lane.
