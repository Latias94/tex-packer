# Remaining Architecture Deepening — Closeout

Date: 2026-05-22
Status: Complete

## Scope Completed

This workstream completed the four remaining architecture-deepening items selected from the architecture review:

1. Export Manifest Projection Module
2. Runtime Strategy Adapter Modules
3. Image Preparation Module
4. CLI Pack Command Pipeline

## Shipped Changes

- Added `crates/tex-packer-core/src/export_manifest.rs` so JSON, plist, and template output use one shared Atlas-to-export projection.
- Split runtime placement internals into `runtime_placement/guillotine.rs`, `runtime_placement/shelf.rs`, and `runtime_placement/skyline.rs` while keeping the public runtime APIs unchanged.
- Added `crates/tex-packer-core/src/preparation.rs` for trim, transparent policy, prepared item shape, layout item preparation, and deterministic sorting. `compute_trim_rect` remains available from both the top-level core exports and `tex_packer_core::pipeline`.
- Added `crates/tex-packer-cli/src/input_loader.rs`, `pack_command.rs`, and `output_writer.rs`; reduced `main.rs` back to argument definitions, subcommand routing, bench wiring, and tracing.
- Preserved existing CLI flags, output filenames, metadata formats, dry-run behavior, template rendering behavior, and layout-only behavior.

## Verification

Fresh closeout gates:

- `cargo fmt --check` — passed.
- `cargo check -p tex-packer-core -p tex-packer-cli -p tex-packer-gui` — passed.
- `cargo nextest run -p tex-packer-core` — passed 103/103.
- `cargo nextest run --workspace` — passed 103/103.

Additional targeted gates and CLI smoke checks are recorded in `EVIDENCE_AND_GATES.md`.

## Risks And Follow-ons

- No required follow-on work remains for this lane.
- GUI config editor simplification was intentionally kept out of scope because it is speculative and lower value than the four completed seams.
- Any future public export schema changes should be handled as a new workstream because this lane intentionally preserved metadata shape compatibility.
