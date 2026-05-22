# Offline Pipeline And Placement Geometry Refactor — TODO

Status: Complete
Last updated: 2026-05-22

## M0 — Scope And Evidence Freeze

- [x] OPPG-010 [owner=planner] [deps=none] [scope=docs/workstreams/offline-pipeline-placement-geometry-v1]
  Goal: Freeze problem, target state, non-goals, and evidence anchors for candidates 1 and 2.
  Validation: DESIGN.md, MILESTONES.md, EVIDENCE_AND_GATES.md, WORKSTREAM.json exist and agree.
  Evidence: docs/workstreams/offline-pipeline-placement-geometry-v1/DESIGN.md
  Handoff: DONE. Planner created this workstream from the architecture review.

## M1 — First Vertical Proof: shared offline placement pipeline

- [x] OPPG-020 [owner=codex] [deps=OPPG-010] [scope=crates/tex-packer-core/src/pipeline.rs,crates/tex-packer-core/tests]
  Goal: Replace duplicated pack_images / pack_layout / pack_layout_items placement loops with one shared offline pipeline module while preserving public behaviour.
  Validation: cargo nextest run -p tex-packer-core --test layout_vs_images --test pack_stats --test boundary_conditions --test padding_extrude_offsets
  Review: No blocking findings. The public APIs remain pack_images, pack_layout, and pack_layout_items; their placement path is shared internally.
  Evidence: crates/tex-packer-core/src/pipeline.rs; crates/tex-packer-core/tests/layout_vs_images.rs; docs/workstreams/offline-pipeline-placement-geometry-v1/EVIDENCE_AND_GATES.md
  Handoff: DONE.

- [x] OPPG-030 [owner=codex] [deps=OPPG-020] [scope=crates/tex-packer-core/src/pipeline.rs,crates/tex-packer-core/tests]
  Goal: Add regression tests that prove image packing and layout-only packing share page/frame geometry for trim, rotation, padding, extrusion, page sizing, and out-of-space cases.
  Validation: cargo nextest run -p tex-packer-core --test layout_vs_images --test padding_extrude_offsets --test force_max_and_border --test pow2_square
  Review: No blocking findings. Tests exercise public APIs and compare observable atlas geometry rather than private helpers.
  Evidence: crates/tex-packer-core/tests/layout_vs_images.rs
  Handoff: DONE.

## M2 — Placement Geometry Module

- [x] OPPG-040 [owner=codex] [deps=OPPG-020] [scope=crates/tex-packer-core/src/geometry.rs,crates/tex-packer-core/src/packer]
  Goal: Introduce a placement geometry module that owns reserved-size calculation and reserved-slot-to-frame conversion for Skyline, MaxRects, and Guillotine.
  Validation: cargo nextest run -p tex-packer-core --test skyline_rotation_fit --test maxrects_rotation_fit --test guillotine_rotation_fit --test padding_extrude_offsets
  Review: No blocking findings. The Packer trait stayed compatible; geometry is a crate-private module.
  Evidence: crates/tex-packer-core/src/geometry.rs; crates/tex-packer-core/src/packer/*.rs; crates/tex-packer-core/tests/padding_extrude_offsets.rs
  Handoff: DONE.

- [x] OPPG-050 [owner=codex] [deps=OPPG-040] [scope=crates/tex-packer-core/src/runtime.rs,crates/tex-packer-core/tests]
  Goal: Reuse placement geometry in runtime append/update paths only where it improves locality without changing runtime append/evict semantics.
  Validation: cargo nextest run -p tex-packer-core --test runtime_api_improvements --test runtime_atlas_tests --test runtime_session --test runtime_skyline --test runtime_shelf
  Review: No blocking findings. Runtime append/evict behaviour remains covered; rotated runtime frame dimensions now match the Frame model contract and are documented.
  Evidence: crates/tex-packer-core/src/runtime.rs; crates/tex-packer-core/tests/runtime_session.rs; crates/tex-packer-core/tests/runtime_atlas_tests.rs
  Handoff: DONE.

## M3 — Integration, docs, and closeout

- [x] OPPG-060 [owner=codex] [deps=OPPG-030,OPPG-050] [scope=README.md,crates/tex-packer-core/README.md,docs/workstreams/offline-pipeline-placement-geometry-v1]
  Goal: Update docs and evidence to describe the refactored offline pipeline and placement geometry ownership.
  Validation: cargo check -p tex-packer-core; cargo nextest run -p tex-packer-core
  Review: No blocking findings. README and core README describe shared image/layout placement and rotated frame metadata.
  Evidence: README.md; crates/tex-packer-core/README.md; EVIDENCE_AND_GATES.md
  Handoff: DONE. Identical-content dedupe remains intentionally out of scope and should be handled as a follow-on or PR-specific fix.

- [x] OPPG-070 [owner=planner] [deps=OPPG-060] [scope=docs/workstreams/offline-pipeline-placement-geometry-v1]
  Goal: Close the lane or create narrower follow-ons.
  Validation: cargo fmt --check; cargo check -p tex-packer-core; cargo nextest run -p tex-packer-core; cargo nextest run --workspace
  Review: No blocking findings recorded in CLOSEOUT.md.
  Evidence: EVIDENCE_AND_GATES.md; WORKSTREAM.json; CLOSEOUT.md
  Handoff: DONE.
