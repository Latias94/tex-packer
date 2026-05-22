# Offline Pipeline And Placement Geometry Refactor — Handoff

Status: Complete
Last updated: 2026-05-22

## Final State

The workstream target state is implemented and verified.

The offline pipeline now has a shared internal placement path in `pipeline.rs`:

- `PreparedItem<T>` represents both image and layout-only prepared items.
- `PackedPage` / `PackedFrame` preserve the item index so rendering can use the original prepared payload without key-based lookup.
- `pack_prepared_pages` handles Auto vs concrete family selection.
- `pack_pages_for_family` owns the page loop, remaining indices, out-of-space handling, frame metadata propagation, and page sizing.
- `build_atlas` owns metadata construction for image and layout-only output.
- `render_output_page` is the image adapter for pixel compositing.

Placement geometry is centralized in crate-private `geometry.rs`:

- Reserved-size calculation lives in `PlacementGeometry`.
- Padding/extrusion offset calculation lives in `PlacementGeometry`.
- Reserved-slot-to-frame conversion lives in `PlacementGeometry`.
- Skyline, MaxRects, Guillotine, and runtime append reuse the helper.

## Completed Tasks

- OPPG-010 — DONE
- OPPG-020 — DONE
- OPPG-030 — DONE
- OPPG-040 — DONE
- OPPG-050 — DONE
- OPPG-060 — DONE
- OPPG-070 — DONE

## Validation

- `cargo fmt --check` — passed.
- `cargo check -p tex-packer-core` — passed.
- `cargo nextest run -p tex-packer-core` — passed 96/96.
- `cargo nextest run --workspace` — passed 96/96.

Targeted evidence is recorded in `EVIDENCE_AND_GATES.md`.

## Decisions

- Kept the public `Packer` trait shape compatible.
- Kept `geometry.rs` crate-private instead of exposing a new public API.
- Preserved public `pack_images`, `pack_layout`, and `pack_layout_items` interfaces.
- Reworked Auto candidate selection to operate on shared placed pages, so image and layout-only APIs share selection logic.
- Chose item-index based render lookup instead of key-based lookup; this avoids duplicate-key ambiguity for future dedupe work.
- Runtime rotated-frame metadata now follows the model contract: `frame.w`/`frame.h` are atlas-orientation dimensions; original input size remains in `source_size`.
- Did not implement identical-content dedupe inside this lane.

## Residual Risks / Follow-ons

- Identical-content dedupe is still a separate follow-on or PR-specific change.
- Consumers that relied on the old runtime rotated-frame dimension bug may notice the corrected metadata. The behaviour is now documented in README and core README.
- GUI dead-code warnings still exist and were observed during workspace testing; they are unrelated to this lane.

## Next Recommended Action

Review the diff and, if acceptable, ask for a conventional commit. Suggested commit subject:

```text
refactor(core): share offline packing pipeline and placement geometry
```
