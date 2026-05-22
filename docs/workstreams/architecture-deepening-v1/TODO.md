# Architecture Deepening Refactor — TODO

Status: Complete
Last updated: 2026-05-22

## M0 — Scope And Evidence Freeze

- [x] ADP-010 [owner=planner] [deps=none] [scope=docs/workstreams/architecture-deepening-v1]
  Goal: Freeze problem, target state, non-goals, task order, and evidence gates.
  Validation: DESIGN.md, MILESTONES.md, EVIDENCE_AND_GATES.md, WORKSTREAM.json exist and agree.
  Evidence: `docs/workstreams/architecture-deepening-v1/DESIGN.md`
  Handoff: Use this ledger as the authority for the active goal.

## M1 — Safety Harness Before Refactor

- [x] ADP-020 [owner=codex] [deps=ADP-010] [scope=crates/tex-packer-core/tests]
  Goal: Add a reusable algorithm invariant test Module covering offline algorithms and runtime strategies.
  Validation: `cargo nextest run -p tex-packer-core --test algorithm_invariants`
  Review: Invariants must assert no overlap, within-page placement, source/frame dimensions, multi-page isolation, and deterministic repeatability.
  Evidence: `crates/tex-packer-core/tests/algorithm_invariants.rs`
  Handoff: DONE 2026-05-22. Added deterministic invariant harness before invasive code movement.

## M2 — Deepen Core Geometry And Free-space

- [x] ADP-030 [owner=codex] [deps=ADP-020] [scope=crates/tex-packer-core/src/geometry.rs,crates/tex-packer-core/src/model.rs,crates/tex-packer-core/src/packer/*.rs,crates/tex-packer-core/src/runtime.rs]
  Goal: Deepen pixel geometry into the single internal Interface for edge math, containment, intersection, area scoring, and slot/frame conversion.
  Validation: `cargo nextest run -p tex-packer-core --test algorithm_invariants --test boundary_conditions --test skyline_no_rotation --test runtime_skyline`
  Review: Public `Rect` serialized shape remains compatible; algorithms stop defining ad hoc edge helpers where practical.
  Evidence: `crates/tex-packer-core/src/geometry.rs`
  Handoff: DONE 2026-05-22. Public `Rect` shape and inclusive `right()`/`bottom()` semantics unchanged; internal exclusive-edge helpers centralized in `geometry.rs`.

- [x] ADP-040 [owner=codex] [deps=ADP-030] [scope=crates/tex-packer-core/src/free_space.rs,crates/tex-packer-core/src/packer/*.rs,crates/tex-packer-core/src/runtime.rs]
  Goal: Extract free-space subtract/prune/merge/score operations into a deep crate-private Module reused by real adapters.
  Validation: `cargo nextest run -p tex-packer-core --test algorithm_invariants --test maxrects_determinism --test skyline_waste_map --test runtime_session`
  Review: Guillotine, MaxRects, Skyline waste map, and runtime must not keep divergent copies of identical free-list rules unless documented.
  Evidence: `crates/tex-packer-core/src/free_space.rs`
  Handoff: DONE 2026-05-22. Added `free_space.rs` with shared scoring, split, subtract, prune, and merge helpers; algorithm-specific policy remains at call sites.

## M3 — Runtime Placement Deepening

- [x] ADP-050 [owner=codex] [deps=ADP-040] [scope=crates/tex-packer-core/src/runtime.rs,crates/tex-packer-core/src/runtime_placement.rs,crates/tex-packer-core/src/runtime_atlas.rs]
  Goal: Move runtime placement strategy state and helpers out of the public `AtlasSession` facade into an internal Runtime placement Module.
  Validation: `cargo nextest run -p tex-packer-core --test runtime_session --test runtime_skyline --test runtime_shelf --test runtime_api_improvements --test runtime_atlas_tests --test algorithm_invariants`
  Review: `AtlasSession` owns session-level concerns; placement adapters own choose/place/free-area behavior.
  Evidence: `crates/tex-packer-core/src/runtime_placement.rs`
  Handoff: DONE 2026-05-22. `AtlasSession` is now a facade over internal `RuntimePage`/`runtime_placement.rs`; public runtime types stayed stable.

## M4 — Validated Context And Offline Pipeline Deepening

- [x] ADP-060 [owner=codex] [deps=ADP-050] [scope=crates/tex-packer-core/src/config.rs,crates/tex-packer-core/src/geometry.rs,crates/tex-packer-core/src/pipeline.rs,crates/tex-packer-core/src/runtime*.rs]
  Goal: Introduce a validated packing context or equivalent deep Module so callers use precomputed usable area, padding/extrude totals, page sizing policy, and overflow policy.
  Validation: `cargo nextest run -p tex-packer-core --test boundary_conditions --test force_max_and_border --test padding_extrude_offsets --test algorithm_invariants`
  Review: Raw `PackerConfig` remains public; internal code should not recompute derived invariants ad hoc.
  Evidence: `crates/tex-packer-core/src/config.rs`, `crates/tex-packer-core/src/geometry.rs`
  Handoff: DONE 2026-05-22. Added crate-private `PackingContext` for usable area, reserved geometry, and page-size policy; public `PackerConfig` stayed unchanged.

- [x] ADP-070 [owner=codex] [deps=ADP-060] [scope=crates/tex-packer-core/src/pipeline.rs,crates/tex-packer-core/src/compositing.rs]
  Goal: Deepen the offline pipeline implementation around preparation, page packing, page sizing, rendering, and metadata without widening the public Interface.
  Validation: `cargo nextest run -p tex-packer-core --test layout_vs_images --test pack_stats --test compose_extrude_no_bleed --test algorithm_invariants`
  Review: `pack_images`, `pack_layout`, and `pack_layout_items` remain behaviorally aligned.
  Evidence: `crates/tex-packer-core/src/pipeline.rs`
  Handoff: DONE 2026-05-22. Added internal `OfflinePipeline` Module to concentrate image/layout pack phases without public Interface changes.

## M5 — Configuration Adapter Convergence

- [x] ADP-080 [owner=codex] [deps=ADP-060] [scope=crates/tex-packer-cli/src,crates/tex-packer-gui/src/presets.rs,crates/tex-packer-core/src/config.rs,schemas]
  Goal: Converge CLI/YAML/GUI configuration adapters through consistent parse and validation behavior.
  Validation: `cargo check -p tex-packer-cli -p tex-packer-gui && cargo nextest run -p tex-packer-core --test boundary_conditions`
  Review: Avoid silent enum fallback in YAML unless intentionally documented; preserve existing CLI flags.
  Evidence: `crates/tex-packer-cli/src/config_adapter.rs` or equivalent.
  Handoff: DONE 2026-05-22. Added CLI `config_adapter.rs` so CLI, YAML, and bench configs share strict enum parsing plus `PackerConfig::validate()`; GUI presets stayed unchanged because they already construct typed `PackerConfig` values.

## M6 — Documentation, Verification, Closeout

- [x] ADP-090 [owner=codex] [deps=ADP-020,ADP-030,ADP-040,ADP-050,ADP-060,ADP-070,ADP-080] [scope=README.md,crates/tex-packer-core/README.md,docs/workstreams/architecture-deepening-v1]
  Goal: Update docs/evidence and close the lane after fresh verification.
  Validation: `cargo fmt --check && cargo check -p tex-packer-core && cargo nextest run -p tex-packer-core && cargo nextest run --workspace`
  Review: `review-workstream` and `verify-rust-workstream` expectations satisfied by recorded evidence.
  Evidence: `EVIDENCE_AND_GATES.md`, `WORKSTREAM.json`, optional `CLOSEOUT.md`
  Handoff: DONE 2026-05-22. Fresh closeout gates passed and lane was closed with residual GUI dead-code warnings recorded as follow-on risk, not blocker.
