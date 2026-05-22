# Changelog

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
