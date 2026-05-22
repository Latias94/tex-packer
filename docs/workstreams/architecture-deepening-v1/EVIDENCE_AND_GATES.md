# Architecture Deepening Refactor — Evidence And Gates

Status: Complete
Last updated: 2026-05-22

## Smallest Current Repro

The first active slice is the invariant test harness:

```bash
cargo nextest run -p tex-packer-core --test algorithm_invariants
```

## Gate Set

### Targeted Iteration Gates

```bash
cargo nextest run -p tex-packer-core --test algorithm_invariants
cargo nextest run -p tex-packer-core --test boundary_conditions --test skyline_no_rotation --test runtime_skyline
cargo nextest run -p tex-packer-core --test maxrects_determinism --test skyline_waste_map --test runtime_session
cargo nextest run -p tex-packer-core --test runtime_session --test runtime_skyline --test runtime_shelf --test runtime_api_improvements --test runtime_atlas_tests --test algorithm_invariants
cargo check -p tex-packer-cli -p tex-packer-gui
```

### Package Gate

```bash
cargo check -p tex-packer-core
cargo nextest run -p tex-packer-core
```

### Broader Closeout Gate

```bash
cargo fmt --check
cargo nextest run --workspace
```

### Review Gate

Before marking the lane complete, review against:

- workstream compliance: TODO statuses, evidence anchors, docs updated;
- code quality: deep Modules have smaller Interfaces and better locality;
- compatibility: public types and CLI flags remain stable unless explicitly documented.

## Evidence Anchors

- `docs/workstreams/architecture-deepening-v1/DESIGN.md`
- `docs/workstreams/architecture-deepening-v1/TODO.md`
- `docs/workstreams/architecture-deepening-v1/MILESTONES.md`
- `crates/tex-packer-core/tests/algorithm_invariants.rs`
- `crates/tex-packer-core/src/geometry.rs`
- `crates/tex-packer-core/src/free_space.rs`
- `crates/tex-packer-core/src/runtime_placement.rs`
- `crates/tex-packer-cli/src/config_adapter.rs` or equivalent

## Evidence Log

- 2026-05-22: Prior lane closed with `cargo nextest run -p tex-packer-core` passing 96/96 and workspace passing 96/96.
- 2026-05-22: Core algorithm hardening committed as `3c1bdfb fix(core): harden skyline placement and score calculations` after `cargo nextest run -p tex-packer-core` passed 99/99 and `cargo fmt --check` passed.

- 2026-05-22 ADP-020: `cargo nextest run -p tex-packer-core --test algorithm_invariants` passed 4/4. Proves the new invariant test Module covers offline algorithms, runtime strategies, multi-page page-local disjointness, within-page bounds, source/frame dimension preservation, rotation-size consistency, and deterministic repeatability.
- 2026-05-22 ADP-020: `cargo fmt --check` passed. Proves the new test Module is rustfmt-clean.

- 2026-05-22 ADP-030: `cargo nextest run -p tex-packer-core --test algorithm_invariants --test boundary_conditions --test skyline_no_rotation --test runtime_skyline` passed 38/38. Proves centralized geometry helpers preserve offline/runtime invariants, boundary conditions, and Skyline regression behavior.
- 2026-05-22 ADP-030: `cargo check -p tex-packer-core` passed. Proves the core crate compiles after geometry deepening.

- 2026-05-22 ADP-040: `cargo fmt --check` passed after extracting `free_space.rs`.
- 2026-05-22 ADP-040: `cargo check -p tex-packer-core` passed. Proves the core crate compiles after shared free-space extraction.
- 2026-05-22 ADP-040: `cargo nextest run -p tex-packer-core --test algorithm_invariants --test maxrects_determinism --test skyline_waste_map --test runtime_session` passed 8/8. Proves shared free-space helpers preserve algorithm invariants, MaxRects determinism, Skyline waste-map behavior, and runtime guillotine reuse/rotation behavior.

- 2026-05-22 ADP-050: `cargo check -p tex-packer-core` passed after moving runtime placement into `runtime_placement.rs`.
- 2026-05-22 ADP-050: `cargo nextest run -p tex-packer-core --test runtime_session --test runtime_skyline --test runtime_shelf --test runtime_api_improvements --test runtime_atlas_tests --test algorithm_invariants` passed 51/51. Proves runtime public behavior, runtime atlas pixel behavior, and shared invariants survived the runtime placement split.

- 2026-05-22 ADP-060: `cargo fmt --check` passed.
- 2026-05-22 ADP-060: `cargo check -p tex-packer-core` passed. Proves the core crate compiles after adding `PackingContext`.
- 2026-05-22 ADP-060: `cargo nextest run -p tex-packer-core --test boundary_conditions --test force_max_and_border --test padding_extrude_offsets --test algorithm_invariants` passed 26/26. Proves validated/derived context preserves boundary validation, border/force-max behavior, padding/extrude offsets, and shared invariants.

- 2026-05-22 ADP-070: `cargo check -p tex-packer-core` passed after introducing internal `OfflinePipeline`.
- 2026-05-22 ADP-070: `cargo nextest run -p tex-packer-core --test layout_vs_images --test pack_stats --test compose_extrude_no_bleed --test algorithm_invariants` passed 18/18. Proves image/layout alignment, stats, extrusion compositing, and shared invariants survived the pipeline deepening.
- 2026-05-22 ADP-070: `cargo fmt --check` passed after removing the unused helper warning.

- 2026-05-22 ADP-080: `cargo fmt --check` passed.
- 2026-05-22 ADP-080: `cargo check -p tex-packer-cli -p tex-packer-gui` passed. GUI crate reported pre-existing dead-code warnings in presets/state/stats; CLI crate reported no warnings after import cleanup.
- 2026-05-22 ADP-080: `cargo nextest run -p tex-packer-core --test boundary_conditions` passed 17/17. Proves strict configuration parsing and validation adapter changes preserve core boundary behavior.

- 2026-05-22 ADP-090: `cargo fmt --check` passed. Proves all changed Rust code is rustfmt-clean.
- 2026-05-22 ADP-090: `cargo check -p tex-packer-core` passed. Proves core crate compiles after the full architecture-deepening lane.
- 2026-05-22 ADP-090: `cargo nextest run -p tex-packer-core` passed 103/103. Proves core package behavior, including new invariant tests, existing algorithm tests, runtime tests, export smoke, and boundary tests.
- 2026-05-22 ADP-090: `cargo nextest run --workspace` passed 103/103. Proves workspace test binaries compile and all workspace tests pass. GUI crate emitted dead-code warnings for existing unused functions/fields; these are non-failing and unchanged by this lane.

## Review Findings

### Workstream Compliance

- No blocking findings. ADP-010 through ADP-090 are marked complete, and each task has evidence recorded in this file plus a journal entry.
- No ADR conflicts were found; `docs/adr/` does not exist in this repo.
- No public metadata schema or public `Rect` shape change was introduced.

### Code Quality

- No blocking findings. The lane deepened internal Modules without widening public Interfaces:
  - `geometry.rs` owns exclusive-edge and derived packing context semantics.
  - `free_space.rs` owns shared free-list split/subtract/prune/merge/scoring helpers.
  - `runtime_placement.rs` owns runtime strategy state while `runtime.rs` stays a facade.
  - `pipeline.rs` owns offline flow through `OfflinePipeline`.
  - `config_adapter.rs` owns CLI/YAML/bench config parsing and validation.

### Residual Risk

- GUI still has pre-existing dead-code warnings in `presets.rs`, `state.rs`, and `stats.rs`. They do not fail current gates and are unrelated to this architecture lane; they are suitable for a separate cleanup issue if desired.

## Notes

Record fresh command output after each task. Do not mark a task DONE based only on stale evidence.
