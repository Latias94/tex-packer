# Remaining Architecture Deepening — Design

Status: Complete
Last updated: 2026-05-22

## Why This Lane Exists

The previous architecture-deepening lane closed the biggest core packing seams: geometry, free-space, runtime placement facade, validated context, offline pipeline, configuration adapters, and invariant tests. A follow-up architecture scan found four remaining high-value seams that are still worth addressing while the context is fresh.

This lane exists to finish those remaining candidates without reopening the completed lane or mixing unrelated cleanups.

## Relevant Authority

- ADRs: none found in `docs/adr/` as of 2026-05-22.
- Domain context: `CONTEXT.md` does not exist yet.
- Prior completed lane: `docs/workstreams/architecture-deepening-v1/`.
- Review artifact: `%TEMP%/architecture-review-20260522-124721.html`.

## Problem

Four Modules remain shallower than ideal:

1. Export format Adapters duplicate Atlas-to-export field projection.
2. Runtime strategy Implementations still share one large `runtime_placement.rs` file.
3. CLI `run_pack` still combines input discovery, image loading, core packing, output writing, template rendering, and reporting.
4. Image/layout preparation semantics still live inside `pipeline.rs` instead of a dedicated preparation Module.

Each issue leaks knowledge across a Seam and increases the cost of future correctness changes.

## Target State

- Export functions render from one internal export manifest projection Module.
- Runtime placement strategy Adapters have separate Modules behind one internal Interface.
- CLI pack command flow is split into deeper Modules while preserving flags and output behavior.
- Image preparation owns trimming, transparent-policy, source rects, layout item preparation, and deterministic sorting.
- Public core APIs, CLI flags, metadata shapes, and GUI behavior remain compatible unless an explicit task records otherwise.

## In Scope

- Crate-private/core-internal refactors in `crates/tex-packer-core/src/`.
- CLI-internal refactors in `crates/tex-packer-cli/src/`.
- Snapshot or regression tests that prove export and pipeline behavior.
- Workstream docs, evidence, journal, and closeout.

## Out Of Scope

- New packing algorithms.
- New public export schema versions.
- GUI redesign.
- Performance micro-optimization unrelated to these seams.
- Breaking public API or CLI flags.

## Architecture Direction

1. Start with export manifest projection because it is low risk and provides a clear test surface.
2. Split runtime strategy Modules while runtime invariant tests remain green.
3. Extract image preparation before further CLI work so core preparation semantics are settled.
4. Extract CLI pack command pipeline after the core seams stop moving.
5. Close with full workspace verification.

## Shipped Architecture

- Export functions now render from a shared internal `export_manifest.rs` projection for JSON, plist, and template context.
- Runtime placement strategy logic now lives in `runtime_placement/guillotine.rs`, `runtime_placement/shelf.rs`, and `runtime_placement/skyline.rs` behind the existing runtime placement facade.
- Image/layout preparation now lives in `preparation.rs`, including trim rectangle calculation, transparent-policy handling, prepared item shape, source rects, and deterministic sorting.
- CLI pack orchestration now lives in `pack_command.rs`, `input_loader.rs`, and `output_writer.rs`; `main.rs` is back to CLI definitions, subcommand routing, bench wiring, and tracing setup.
- Public core APIs, CLI flags, metadata shapes, and GUI behavior were preserved.

## Closeout Condition

This lane is closed because:

- RAD-010 through RAD-060 are complete or explicitly split,
- `cargo fmt --check` passes,
- targeted export/runtime/pipeline/CLI gates pass,
- `cargo nextest run -p tex-packer-core` passes,
- `cargo nextest run --workspace` passes,
- workstream evidence is current,
- and `WORKSTREAM.json` status is `complete`.
