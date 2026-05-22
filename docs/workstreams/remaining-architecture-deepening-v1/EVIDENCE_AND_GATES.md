# Remaining Architecture Deepening — Evidence And Gates

Status: Complete
Last updated: 2026-05-22

## Gate Set

### Targeted Gates

```bash
cargo nextest run -p tex-packer-core --test export_smoke
cargo check -p tex-packer-cli
cargo nextest run -p tex-packer-core --test runtime_session --test runtime_shelf --test runtime_skyline --test runtime_api_improvements --test algorithm_invariants
cargo nextest run -p tex-packer-core --test transparent_policy --test layout_vs_images --test padding_extrude_offsets --test algorithm_invariants
```

### Closeout Gates

```bash
cargo fmt --check
cargo check -p tex-packer-core -p tex-packer-cli -p tex-packer-gui
cargo nextest run -p tex-packer-core
cargo nextest run --workspace
```

## Evidence Anchors

- `docs/workstreams/remaining-architecture-deepening-v1/DESIGN.md`
- `docs/workstreams/remaining-architecture-deepening-v1/TODO.md`
- `docs/workstreams/remaining-architecture-deepening-v1/MILESTONES.md`
- `crates/tex-packer-core/src/export*.rs`
- `crates/tex-packer-core/src/runtime_placement*`
- `crates/tex-packer-core/src/pipeline.rs`
- `crates/tex-packer-cli/src`

## Evidence Log

- 2026-05-22 RAD-010: Workstream opened for four remaining architecture-deepening candidates from `%TEMP%/architecture-review-20260522-124721.html`.
- 2026-05-22 RAD-020: `cargo nextest run -p tex-packer-core --test export_smoke` passed 1/1. Proves JSON/plist export smoke behavior remains intact after introducing export manifest projection.
- 2026-05-22 RAD-020: `cargo check -p tex-packer-cli` passed. Proves CLI template context now compiles against the shared core template projection.
- 2026-05-22 RAD-020: `cargo nextest run -p tex-packer-core --test layout_vs_images --test export_smoke` passed 6/6. Provides an additional layout/export alignment check after projection refactor.
- 2026-05-22 RAD-030: `cargo nextest run -p tex-packer-core --test runtime_session --test runtime_shelf --test runtime_skyline --test runtime_api_improvements --test algorithm_invariants` passed 35/35. Proves runtime strategy split preserved public runtime behavior and shared invariants.
- 2026-05-22 RAD-030: `cargo nextest run -p tex-packer-core --test export_smoke --test layout_vs_images` passed 6/6. Confirms export and layout behavior remained aligned after runtime split.
- 2026-05-22 RAD-030: `cargo check -p tex-packer-cli` passed after runtime placement modularization.
- 2026-05-22 RAD-040: `cargo fmt --check` passed.
- 2026-05-22 RAD-040: `cargo nextest run -p tex-packer-core --test transparent_policy --test layout_vs_images --test padding_extrude_offsets --test algorithm_invariants` passed 13/13. Proves preparation extraction preserved transparent-policy behavior, image/layout alignment, padding/extrude offsets, and shared invariants.
- 2026-05-22 RAD-040: `cargo check -p tex-packer-core` passed with no warnings after re-exporting `compute_trim_rect` from the preparation Module.
- 2026-05-22 RAD-050: `cargo fmt --check` passed after CLI pack pipeline extraction.
- 2026-05-22 RAD-050: `cargo check -p tex-packer-cli` passed. Proves `main.rs` delegates to `pack_command.rs`, `input_loader.rs`, and `output_writer.rs` without warnings.
- 2026-05-22 RAD-050: `cargo nextest run -p tex-packer-core --test export_smoke --test layout_vs_images --test transparent_policy --test runtime_session --test runtime_shelf --test runtime_skyline --test runtime_api_improvements --test algorithm_invariants` passed 42/42. Proves core export, preparation, layout/image, and runtime paths remain aligned after the CLI pipeline split.
- 2026-05-22 RAD-050: `cargo run -p tex-packer-cli -- pack assets/generated/basic/basic_000.png --out-dir %TEMP%/tex-packer-rad050-cli --name smoke --metadata json --max-width 256 --max-height 256 --progress false` passed and wrote `smoke.png` plus `smoke.json`. Proves the refactored CLI writes the normal PNG + JSON output path.
- 2026-05-22 RAD-050: `cargo run -p tex-packer-cli -- pack assets/generated/basic/basic_000.png --out-dir %TEMP%/tex-packer-rad050-dry-template --name smoke --metadata template --engine unity --dry-run --max-width 256 --max-height 256 --progress false` passed and wrote no files. Proves dry-run template rendering validates in memory without output side effects.
- 2026-05-22 RAD-050: `cargo nextest run --workspace` passed 103/103. Proves the extracted CLI pipeline did not break workspace tests.
- 2026-05-22 RAD-060: `cargo fmt --check` passed. Proves formatting is clean at closeout.
- 2026-05-22 RAD-060: `cargo check -p tex-packer-core -p tex-packer-cli -p tex-packer-gui` passed. Proves all workspace packages compile after the refactors.
- 2026-05-22 RAD-060: `cargo nextest run -p tex-packer-core` passed 103/103. Proves the complete core regression suite is green after export, runtime, and preparation refactors.
- 2026-05-22 RAD-060: `cargo nextest run --workspace` passed 103/103. Proves workspace test gates are green at closeout.

## Notes

Closeout complete. No broader gate was skipped.
