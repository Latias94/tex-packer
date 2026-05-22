# Remaining Architecture Deepening — TODO

Status: Complete
Last updated: 2026-05-22

## M0 — Workstream Setup

- [x] RAD-010 [owner=planner] [deps=none] [scope=docs/workstreams/remaining-architecture-deepening-v1]
  Goal: Freeze scope, task order, non-goals, validation gates, and handoff state.
  Validation: workstream docs exist and agree.
  Review: Do not reopen the completed `architecture-deepening-v1` lane.
  Evidence: `DESIGN.md`, `MILESTONES.md`, `EVIDENCE_AND_GATES.md`, `WORKSTREAM.json`.
  Handoff: This ledger is authoritative for remaining architecture deepening.

## M1 — Export Manifest Projection

- [x] RAD-020 [owner=codex] [deps=RAD-010] [scope=crates/tex-packer-core/src/export*.rs,crates/tex-packer-core/src/model.rs,crates/tex-packer-cli/src/main.rs,schemas,tests]
  Goal: Introduce an internal export manifest projection Module so JSON, plist, and CLI template rendering share one Atlas-to-export projection.
  Validation: `cargo nextest run -p tex-packer-core --test export_smoke && cargo check -p tex-packer-cli`
  Review: Preserve public export function names and current metadata shapes; add/adjust tests around duplicate projection risk.
  Evidence: new `export_manifest.rs` or equivalent plus export tests.
  Handoff: DONE 2026-05-22. Added `export_manifest.rs`; JSON, plist, and template context now render from one projection while preserving public export functions and current shapes.

## M2 — Runtime Strategy Adapter Modules

- [x] RAD-030 [owner=codex] [deps=RAD-010] [scope=crates/tex-packer-core/src/runtime_placement*.rs,crates/tex-packer-core/src/runtime.rs,crates/tex-packer-core/tests/runtime*]
  Goal: Split Guillotine, Shelf, and Skyline runtime strategy Implementations into separate internal Adapter Modules behind a small runtime placement Interface.
  Validation: `cargo nextest run -p tex-packer-core --test runtime_session --test runtime_shelf --test runtime_skyline --test runtime_api_improvements --test algorithm_invariants`
  Review: `AtlasSession` public Interface remains unchanged; strategy-specific logic should gain Locality.
  Evidence: `runtime_placement/guillotine.rs`, `runtime_placement/shelf.rs`, `runtime_placement/skyline.rs` or equivalent.
  Handoff: DONE 2026-05-22. Runtime placement now delegates to separate internal Guillotine/Shelf/Skyline Adapter Modules behind one runtime placement Interface.

## M3 — Image Preparation Module

- [x] RAD-040 [owner=codex] [deps=RAD-020] [scope=crates/tex-packer-core/src/pipeline.rs,crates/tex-packer-core/src/preparation.rs,crates/tex-packer-core/tests]
  Goal: Extract image/layout preparation into a crate-private Module owning trim, transparent policy, prepared item shape, source rects, and sorting.
  Validation: `cargo nextest run -p tex-packer-core --test transparent_policy --test layout_vs_images --test padding_extrude_offsets --test algorithm_invariants`
  Review: `pack_images`, `pack_layout`, and `pack_layout_items` behavior remains aligned.
  Evidence: new `preparation.rs` or equivalent.
  Handoff: DONE 2026-05-22. Added `preparation.rs` for image/layout preparation, transparent-policy handling, trim rect calculation, prepared item shape, and deterministic sorting.

## M4 — CLI Pack Command Pipeline

- [x] RAD-050 [owner=codex] [deps=RAD-020,RAD-040] [scope=crates/tex-packer-cli/src]
  Goal: Split CLI pack command orchestration into deeper internal Modules for input loading, pack execution, metadata/template output, and reporting.
  Validation: `cargo check -p tex-packer-cli && cargo nextest run --workspace`
  Review: Preserve existing flags, output filenames, print-config behavior, layout-only mode, template mode, dry-run, and progress behavior.
  Evidence: `pack_command.rs`, `input_loader.rs`, `output_writer.rs`, or equivalent.
  Handoff: DONE 2026-05-22. CLI pack orchestration now delegates input discovery/loading, pack/layout execution, metadata/template writing, and stats reporting to dedicated internal Modules. Existing filenames, layout-only behavior, dry-run template validation, and progress switch behavior were preserved.

## M5 — Verification And Closeout

- [x] RAD-060 [owner=codex] [deps=RAD-020,RAD-030,RAD-040,RAD-050] [scope=docs/workstreams/remaining-architecture-deepening-v1]
  Goal: Run fresh verification, review the lane, record evidence, and close the workstream.
  Validation: `cargo fmt --check && cargo check -p tex-packer-core -p tex-packer-cli -p tex-packer-gui && cargo nextest run -p tex-packer-core && cargo nextest run --workspace`
  Review: `verify-rust-workstream` and `close-workstream` expectations satisfied by recorded evidence.
  Evidence: `EVIDENCE_AND_GATES.md`, `CLOSEOUT.md`, `WORKSTREAM.json`.
  Handoff: DONE 2026-05-22. Closeout gates passed, `CLOSEOUT.md` records final status, and no follow-on work is required for this lane.
