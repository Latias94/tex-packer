# v0.3 Core Architecture

**Status:** Accepted and implemented
**Date:** 2026-07-18

## Context

The v0.2 API exposed mutable configuration fields, concrete packing algorithms, aggregate representation, and free functions as one broad surface. Logical sprite metadata and physical atlas placement were both stored on each frame. Identical-content deduplication therefore duplicated physical geometry, statistics had to infer aliases from equal rectangles, and consumers rebuilt relationships independently.

Runtime mutation had a second problem: allocation, identity assignment, metadata insertion, and pixel writes could fail at different points. Without one transaction boundary, failure behavior depended on the strategy and call path.

The v0.3 goal is not source compatibility. It is a smaller API whose types preserve the domain invariants needed by offline packing, incremental runtime packing, persistence, and legacy export.

## Decision

The core is divided into six public responsibilities:

```text
CLI/GUI draft
    -> validated config
    -> offline or runtime facade
    -> private preparation and placement engines
    -> validated Atlas / Page / Region / Frame model
    -> native document or legacy export projection
```

Only `config`, `error`, `export`, `model`, `offline`, and `runtime` are public modules. Placement algorithms and supporting geometry, preparation, compositing, and allocator code are private.

## Configuration Boundary

Configuration is split by invariant ownership:

- `PageConfig` owns dimensions, rotation, border padding, texture padding, extrusion, and derived reservation geometry shared by all workflows.
- `OfflineConfig` owns final page sizing, trimming, transparent policy, sorting, outlines, and `PackingStrategy`.
- `RuntimeConfig` owns only `PageConfig` and `RuntimeStrategy`.

Builders are fallible and consuming. Validated values have private fields and accessors. Strategy enums contain only options relevant to their selected family, so invalid cross-family combinations are not representable after construction.

CLI YAML and GUI controls remain editable adapter-owned drafts. They become core configuration only at the workflow boundary.

## Domain Model

The aggregate separates physical and logical identity:

- `Atlas` owns validated pages and a private `PageId` index.
- `Page` owns physical regions, logical frames, and private page-scoped indexes.
- `Region` owns `RegionId`, content geometry, reserved allocation geometry, and rotation.
- `Frame` owns `FrameId`, an owned string key, `RegionId`, trimming state, and source reconstruction metadata.

`PageId`, `RegionId`, and `FrameId` are opaque `u32` values. They are stable identities, never vector positions. `Page::resolved_frames` joins frames to authoritative regions in stable logical order without repeated lookup.

Aggregate construction validates:

- positive page, source, content, and allocation geometry;
- content contained by allocation and allocation contained by its page;
- no overlapping physical allocations;
- unique identities in their scope;
- every frame reference resolves;
- every region is referenced;
- source geometry matches the physical rotation.

Offline keys are not identities and may repeat. Runtime keys are unique because they address mutable live entries.

## Offline Workflows

`OfflinePacker` is the only public offline entry point:

- `pack_images` performs decoded-image preparation, content deduplication, placement, aggregate construction, and final rendering.
- `layout_images` performs the same preparation, deduplication, placement, and aggregate construction without rendering pages.
- `pack_layout` accepts caller-prepared layout records and intentionally skips pixel preprocessing and content deduplication.

The three operations share placement and aggregate construction, but they do not pretend that decoded pixels and size-only records have the same preparation contract. `PackOutput` contains one `Atlas` plus `RenderedPage` payloads keyed by `PageId`; it does not duplicate page metadata inside output records.

## Placement Boundary

Concrete Skyline, MaxRects, and Guillotine engines implement a crate-private placement contract. A search returns one authoritative physical placement containing both content and allocation geometry. Successful search commits exactly once; unsuccessful search leaves engine state unchanged.

Algorithms do not receive user keys, trimming metadata, exporter policy, or public aggregate types. Auto mode creates concrete placement-policy candidates instead of copying a universal mutable configuration.

## Runtime Transaction

Runtime mutation follows prepare/commit:

1. Validate key and dimensions.
2. Search existing pages or prepare a complete new page.
3. Compute geometry, typed identities, page growth, and index updates without changing live state.
4. For pixel-backed runtime atlases, stage and validate the complete image update.
5. Apply one infallible commit to allocator state, identities, records, indexes, and pixels.

Every failure before step 5 leaves state unchanged. Page and record IDs are monotonic and are not consumed by rejected operations. Snapshots sort by typed identity and reconstruct the same validated domain aggregate used by offline workflows.

## Persistence and Export

The runtime model is not a serde wire type. `AtlasDocument` is the reversible native persistence boundary and currently uses schema version 2. It serializes relationships but never runtime indexes. Deserialization rejects unknown fields; conversion to `Atlas` runs aggregate validation.

Legacy JSON, plist, and template formats are projections through one internal export manifest. Each logical frame resolves its region and emits the established flat `frame` and `rotated` fields. Their schema version remains `"1"`; it is independent from native document version 2.

This separation lets native persistence evolve with the domain without forcing downstream engines to consume internal relationships.

## Statistics

Statistics distinguish three areas:

- `content_area`: unique region content rectangles.
- `allocation_area`: unique reserved rectangles, including padding/extrusion reservation.
- `page_area`: final page rectangles.

Content and allocation occupancy divide their respective area by page area. Logical aliases never inflate physical areas. Runtime snapshots use the same equations, while live runtime statistics expose allocator fragmentation separately.

## Dependency Policy

Risky dependency migrations are serialized behind green architecture gates:

1. `serde_yaml` to `serde_yaml_ng`, with strict structural validation.
2. rand 0.8 to 0.10.2.
3. Criterion 0.7 to 0.8.2.
4. indicatif 0.17 to 0.18.6, with redirected-output tests.

Each migration has an isolated commit and full workspace gate. egui/eframe/egui_extras 0.35 and rfd 0.17 are deferred because they share GUI, renderer, native-dialog, and platform risk that deserves a separate migration gate.

## Consequences

Positive consequences:

- Physical/logical semantics are explicit and validated once.
- Offline deduplication and duplicate keys no longer require inferred identity.
- Export and GUI traversal are linear in logical output size.
- Runtime failures are atomic across every supported strategy.
- Public API changes are concentrated in workflow facades instead of algorithm internals.
- Native persistence can evolve independently from legacy exporters.

Costs and constraints:

- v0.2 Rust consumers must migrate; there are no compatibility shims.
- Callers must choose decoded render, decoded layout, or pure layout explicitly.
- Consumers must resolve a frame to its region to read physical geometry.
- JSON hash and plist dictionaries remain inherently lossy for duplicate keys; lossless consumers must use array-like output or typed native identity.
- `AtlasDocument` version changes require explicit migration rather than permissive deserialization.
