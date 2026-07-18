# Changelog

## Unreleased

## v0.3.0 - 2026-07-18

v0.3 is a deliberate source-breaking release that replaces representation-driven APIs with validated offline and runtime workflows. CLI command families, GUI workflows, and legacy JSON, plist, and template field shapes remain compatible.

### Added

- Added explicit physical `Region` and logical `Frame` records with opaque `PageId`, `RegionId`, and `FrameId` identities.
- Added indexed `Page` and `Atlas` aggregates with validated construction and stable resolved-frame traversal.
- Added `PageConfig`, `OfflineConfig`, and `RuntimeConfig` fallible builders with workflow-specific strategy enums.
- Added `OfflinePacker` operations for decoded image rendering, decoded image layout, and caller-prepared pure layout.
- Added reversible native persistence through `AtlasDocument` schema version 2.
- Added runtime Skyline support and transactional append behavior across all runtime strategies.
- Added explicit content, allocation, and page area statistics.

### Changed

- Identical prepared images now share one physical region while retaining ordered logical frames and source metadata.
- Runtime append, identity assignment, allocator mutation, and pixel staging now commit atomically.
- Exporters resolve logical frames through physical regions while retaining the legacy export schema version `"1"`.
- CLI layout-only mode now uses decoded-image preparation, so trimming, transparent-image policy, and deduplication match rendered packing.
- CLI YAML loading now rejects unknown fields, duplicate keys, tags, and merge keys with structured errors.
- CLI and GUI consume validated core configurations instead of mutating core fields.

### Breaking

- Removed `PackerConfig`, the public `Packer` trait, public concrete packers, public geometry/compositing helpers, free packing wrappers, and `prelude::*`.
- Replaced public aggregate fields with accessors and typed identity lookup.
- Replaced `OutputPage` with `RenderedPage`, which carries `PageId` and an RGBA payload.
- Replaced direct `Atlas` serde with the validated `AtlasDocument` conversion boundary.
- Removed the generic key axis from `Atlas`, `Page`, and `Frame`; logical keys are owned strings and typed IDs provide identity.
- Runtime constructors now receive a validated `RuntimeConfig`, and append returns `RuntimePlacement`.
- Replaced ambiguous used-area/occupancy fields with `content_area`, `allocation_area`, `page_area`, `content_occupancy`, and `allocation_occupancy`.

See [`docs/migrations/v0.3.md`](docs/migrations/v0.3.md) for exact old-to-new API mappings.

### Dependencies

- Replaced archived `serde_yaml 0.9` with `serde_yaml_ng 0.10`.
- Upgraded `rand` from 0.8 to 0.10.2.
- Upgraded Criterion from 0.7 to 0.8.2.
- Upgraded indicatif from 0.17 to 0.18.6.
- Deferred egui/eframe/egui_extras 0.35 and rfd 0.17 to a dedicated GUI/platform migration.

## v0.2.0 - 2026-05-22

This release focused on reliable rotation, padding, extrusion, multi-page output, transparent-image handling, consistent exporters, and stronger regression coverage.

### Notes

- CLI flags and metadata formats were intended to remain compatible.
- Rust consumers used the v0.2 public configuration, algorithm, aggregate-field, and free-function APIs replaced by v0.3.
