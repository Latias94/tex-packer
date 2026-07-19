# Changelog

## v0.3.0 - 2026-07-19

v0.3.0 is a deliberate source-breaking release that replaces representation-oriented Rust APIs with validated offline and runtime workflows. CLI command families, GUI workflows, and the established JSON, plist, and template export shapes remain compatible. The release is tracked in [PR #2](https://github.com/Latias94/tex-packer/pull/2).

### Added

- Offline image packing now stores identical prepared pixel content in one physical atlas region while preserving every logical frame, input order, and source reconstruction record. Thanks to [@phamtuan993](https://github.com/phamtuan993) for the original deduplication contribution in [PR #1](https://github.com/Latias94/tex-packer/pull/1).
- `AtlasDocument` schema version 2 provides reversible native persistence with validated identities, references, geometry, overlap, metadata, and a published [JSON Schema](schemas/tex-packer-atlas-document-v2.schema.json).

### Changed

- `tex-packer-core` now exposes validated `PageConfig`, `OfflineConfig`, and `RuntimeConfig` values through the `OfflinePacker`, `AtlasSession`, and `RuntimeAtlas` workflow facades; placement engines and geometry/compositing helpers are implementation details.
- CLI `layout` now performs the same trimming, transparent-image handling, and content deduplication as rendered image packing before omitting page pixels.
- CLI YAML configuration now uses `serde_yaml_ng 0.10` and rejects unknown fields, duplicate keys, tags, and merge keys instead of accepting ambiguous input.
- Core, CLI, and GUI statistics now distinguish logical frames, physical regions, content area, reserved allocation area, final page area, and their corresponding occupancy values.

### Fixed

- Failed runtime appends no longer consume identities, mutate allocator state, publish partial model records, or partially update page pixels.
- Runtime append and release operations now use indexed lookup and commit allocator changes in place, avoiding work proportional to every existing region on the normal append path.

### Breaking

- Rust consumers must upgrade `tex-packer-core`, `tex-packer-cli`, and `tex-packer-gui` together and replace `PackerConfig`, free packing functions, public concrete packers, and `prelude::*` imports with validated workflow configuration and facade methods.
- `Atlas`, `Page`, and `Frame` now use private aggregate state, owned string keys, and opaque `PageId`, `RegionId`, and `FrameId` values; use accessors and resolved-frame traversal rather than public fields or positional ID assumptions.
- `OutputPage` is replaced by `RenderedPage`, runtime append returns `RuntimePlacement`, and direct `Atlas` serde is replaced by `AtlasDocument::from_atlas` and `AtlasDocument::try_into_atlas`.
- Statistics and CLI `--export-stats` output replace ambiguous used/total-area fields with `content_area`, `allocation_area`, `page_area`, `content_occupancy`, and `allocation_occupancy`.

See the [v0.3 migration guide](docs/migrations/v0.3.md) for complete API mappings and upgrade examples.

## v0.2.0 - 2026-05-22

This release focuses on making tex-packer more reliable for real projects and easier to use from both the CLI and GUI.

### Highlights

- Improved packing correctness for rotation, padding, extrusion, multi-page atlases, and edge-case image sizes.
- More consistent JSON, plist, and template metadata output.
- Better handling of transparent images and trimmed sprites.
- Cleaner CLI pack flow with the same familiar flags and output filenames.
- GUI preview/export behavior is more stable, with cleaner input handling and selection updates.
- Added stronger regression coverage and a stricter lint gate for future releases.

### Notes

- Existing CLI flags and metadata formats are intended to remain compatible.
- If you use the Rust crates directly, update `tex-packer-core`, `tex-packer-cli`, and `tex-packer-gui` together to `0.2.0`.
