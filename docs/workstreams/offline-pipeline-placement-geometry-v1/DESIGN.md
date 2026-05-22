# Offline Pipeline And Placement Geometry Refactor

Status: Complete
Last updated: 2026-05-22

## Why This Lane Exists

The offline atlas packing path has become too shallow. pack_images, pack_layout, and pack_layout_items each know about sorting, placement loops, page sizing, frame metadata, and meta construction. The algorithm Modules also know how to convert reserved slots into atlas frames. That leaked geometry makes new behaviours, such as identical-content dedupe, touch too many places and fail in subtle ways.

This workstream deepens two Modules:

1. the offline packing pipeline Module;
2. the placement geometry Module.

## Relevant Authority

- ADRs: none found in this repository.
- Existing docs:
  - README.md
  - crates/tex-packer-core/README.md
- Related workstreams: none found.
- Architecture review artifact:
  - C:\Users\Frankorz\AppData\Local\Temp\architecture-review-20260522-092759.html

## Problem

The current offline pipeline has low locality. Adding a cross-cutting packing behaviour requires editing preparation, layout-only packing, image packing, rendering, page sizing, stats, and sometimes CLI logic. Placement geometry also repeats inside Skyline, MaxRects, Guillotine, and runtime code.

## Target State

When this workstream closes:

- pack_images, pack_layout, and pack_layout_items share one offline placement pipeline implementation.
- Sorting, remaining-item iteration, page creation, page sizing, atlas meta construction, and out-of-space errors live behind one smaller Interface.
- Algorithms return or operate on reserved slots without owning frame-offset geometry.
- A placement geometry Module owns reserved-size calculation and reserved-slot-to-frame conversion.
- Existing public behaviour remains compatible unless explicitly documented.
- Tests prove image packing, layout-only packing, rotation, padding, extrusion, page sizing, and stats still agree.

## In Scope

- Refactor crates/tex-packer-core/src/pipeline.rs to concentrate offline placement logic.
- Refactor crates/tex-packer-core/src/packer/*.rs so common geometry is not repeated per algorithm.
- Add focused tests for shared pipeline invariants and placement geometry invariants.
- Update docs if public behaviour or public helper names change.
- Preserve current CLI and GUI behaviour.

## Out Of Scope

- Implementing the identical-content dedupe PR itself.
- Changing metadata export formats.
- Rewriting runtime eviction behaviour.
- Adding a new public configuration surface unless needed to preserve compatibility.
- Large GUI redesign.

## Starting Assumptions

| Assumption | Confidence | Evidence | Consequence if wrong |
| --- | --- | --- | --- |
| Public callers mainly use pack_images, pack_layout, pack_layout_items, and PackerConfig. | High | README and core prelude exports. | Need compatibility shims if internal names change. |
| The placement algorithms can keep their external Packer trait while sharing geometry helpers first. | Medium | Current trait returns Frame<K> and tests depend on frame metadata. | May need an intermediate internal type before changing the trait. |
| Runtime placement can reuse geometry helpers without being forced into the offline pipeline. | Medium | untime.rs already has separate append/evict needs. | Keep runtime changes limited to shared geometry helpers only. |
| Current tests are a strong regression net for compatibility. | High | cargo nextest run -p tex-packer-core currently passes 91/91. | Add missing tests before changing behaviour. |

## Architecture Direction

Deepen the offline packing pipeline first. The new Module should expose a small Interface for packing prepared items and return placed pages plus enough render instructions for image compositing. This improves locality: the page loop, remaining set, meta construction, and out-of-space handling stop being copied.

Then deepen placement geometry. The geometry Module should hide padding, extrusion, half-padding, rotation-size, reserved-size, and frame construction behind one Interface. Algorithm Modules should focus on choosing and consuming reserved slots.

The runtime atlas session should not be collapsed into the offline pipeline in this lane. It has append/evict locality that differs from batch packing. It may consume the shared placement geometry Module once that Interface exists.

## Closeout Condition

This lane can close when:

- the target state is implemented,
- cargo fmt --check passes,
- cargo check -p tex-packer-core passes,
- cargo nextest run -p tex-packer-core passes,
- any broader workspace gate chosen in EVIDENCE_AND_GATES.md passes or has a recorded reason for narrowing,
- docs reflect the shipped structure,
- and follow-on work, including dedupe, is either split or explicitly deferred.

Final status on 2026-05-22: complete. The shared offline pipeline and crate-private placement geometry module are implemented, documented, and verified. Identical-content dedupe is deferred as a follow-on.
