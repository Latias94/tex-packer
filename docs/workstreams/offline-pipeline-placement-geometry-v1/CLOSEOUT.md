# Offline Pipeline And Placement Geometry Refactor — Closeout

Date: 2026-05-22
Status: Complete

## Workstream Compliance

- Blocking findings: none.
- Important findings: none.
- Scope check: satisfied. The lane deepened the offline pipeline and extracted placement geometry; it did not implement identical-content dedupe.
- Task ledger: OPPG-010 through OPPG-070 are complete.
- Documentation: README, core README, TODO, milestones, evidence, handoff, and WORKSTREAM.json are updated.

## Code Quality

- Blocking findings: none.
- `pipeline.rs` now keeps the page loop, Auto candidate selection, atlas construction, and image rendering adapter in one module.
- `geometry.rs` is crate-private and keeps reserved-size and reserved-slot-to-frame logic behind a small internal interface.
- Skyline, MaxRects, Guillotine, and runtime append no longer duplicate frame-offset geometry.
- New tests exercise public seams rather than private helpers.

## Verification

- `cargo fmt --check` — passed.
- `cargo check -p tex-packer-core` — passed.
- `cargo nextest run -p tex-packer-core` — passed 96/96.
- `cargo nextest run --workspace` — passed 96/96.

## Residual Risk

- Runtime rotated-frame metadata is corrected to the model contract. Consumers relying on the old runtime-only original-dimension behaviour may need to adapt; this is documented.
- Workspace testing still emits pre-existing GUI dead-code warnings; they are unrelated to this refactor.
- Identical-content dedupe is not implemented in this lane and should remain a separate follow-on.

## Suggested Follow-ons

1. Implement/review identical-content dedupe against the new shared offline pipeline.
2. Consider a separate GUI warning cleanup if desired.
