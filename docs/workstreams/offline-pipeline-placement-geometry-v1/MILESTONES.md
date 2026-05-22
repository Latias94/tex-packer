# Offline Pipeline And Placement Geometry Refactor — Milestones

Status: Complete
Last updated: 2026-05-22

## M0 — Scope And Evidence Freeze

Exit criteria:

- Problem and target state are explicit.
- Non-goals are explicit.
- Architecture review candidates 1 and 2 are mapped into tasks.
- First proof target is chosen.

Status: Complete.

Primary evidence:

- docs/workstreams/offline-pipeline-placement-geometry-v1/DESIGN.md
- docs/workstreams/offline-pipeline-placement-geometry-v1/TODO.md

## M1 — Shared Offline Pipeline Proof

Exit criteria:

- pack_images, pack_layout, and pack_layout_items share one placement implementation.
- Existing public behaviour remains compatible.
- Tests prove image/layout geometry agreement.

Status: Complete.

Primary gates:

- `cargo nextest run -p tex-packer-core --test layout_vs_images --test pack_stats --test boundary_conditions --test padding_extrude_offsets`
- `cargo nextest run -p tex-packer-core --test layout_vs_images --test padding_extrude_offsets --test force_max_and_border --test pow2_square`

## M2 — Placement Geometry Ownership

Exit criteria:

- Shared geometry helpers own reserved-size and frame-offset invariants.
- Skyline, MaxRects, Guillotine no longer duplicate placement geometry formulas.
- Runtime uses the shared geometry only where semantics match.

Status: Complete.

Primary gates:

- `cargo nextest run -p tex-packer-core --test skyline_rotation_fit --test maxrects_rotation_fit --test guillotine_rotation_fit --test padding_extrude_offsets`
- `cargo nextest run -p tex-packer-core --test runtime_api_improvements --test runtime_atlas_tests --test runtime_session --test runtime_skyline --test runtime_shelf`

## M3 — Closeout

Exit criteria:

- cargo fmt --check passes.
- cargo check -p tex-packer-core passes.
- cargo nextest run -p tex-packer-core passes.
- Broader workspace gate is run or intentionally narrowed with a recorded reason.
- Remaining dedupe work is either implemented, deferred, or split into a follow-on.
- WORKSTREAM.json status is updated.

Status: Complete.

Primary gates:

- `cargo fmt --check`
- `cargo check -p tex-packer-core`
- `cargo nextest run -p tex-packer-core`
- `cargo nextest run --workspace`
