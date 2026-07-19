# tex-packer-cli

[![Crates.io](https://img.shields.io/crates/v/tex-packer-cli.svg)](https://crates.io/crates/tex-packer-cli)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/Latias94/tex-packer)

Command-line atlas packing for local builds and CI. The CLI reads image files, validates CLI/YAML configuration, runs `tex-packer-core`, and writes PNG plus JSON, plist, or template metadata.

## Install

```bash
cargo install tex-packer-cli
```

From this repository:

```bash
cargo install --path crates/tex-packer-cli
```

## First Pack

```bash
tex-packer pack ./assets --out ./out --name atlas
```

Outputs:

- One page: `out/atlas.png` and `out/atlas.json`.
- Multiple pages: `out/atlas_0.png`, `out/atlas_1.png`, and one `out/atlas.json`.

Higher-quality offline packing:

```bash
tex-packer pack ./assets \
  --out ./out \
  --name atlas \
  --algorithm auto \
  --auto-mode quality \
  --time-budget 500 \
  --allow-rotation \
  --texture-padding 2 \
  --texture-extrusion 2
```

## Commands

- `tex-packer pack <input>` writes rendered page PNGs plus metadata.
- `tex-packer template <input>` selects template metadata.
- `tex-packer layout <input>` runs decoded-image preparation and writes metadata without PNG pages.
- `tex-packer bench <input>` performs one timed pack and prints content/allocation occupancy.

Global UX flags are `-q`/`--quiet`, `-v`/`--verbose`, and `--progress true|false`. Progress defaults to enabled, is suppressed by quiet, and is automatically hidden when stderr is redirected.

Run `tex-packer --help` or `tex-packer pack --help` for the full option list.

## Metadata

| Format | Selection | Notes |
| --- | --- | --- |
| JSON array | `--metadata json-array` or `json` | Preserves page order, frame order, and duplicate keys. |
| JSON hash | `--metadata json-hash` | Provides object lookup; the last frame wins for a duplicate key. |
| Plist | `--metadata plist` | TexturePacker-style metadata with page names. |
| Template | `--metadata template` or `template` command | Built-in or caller-supplied Handlebars output. |

Built-in engine names are `unity`, `godot`, `phaser3`, `phaser3_single`, `spine`, `cocos`, and `unreal`:

```bash
tex-packer template ./assets --engine unity --out ./out
tex-packer template ./assets --engine phaser3 --out ./out
tex-packer template ./assets --template ./custom.hbs --out ./out
```

The template context contains ordered pages, page size/name, ordered logical sprites, resolved region geometry, rotation, source reconstruction metadata, and pivot. Built-in templates live under [`src/templates`](src/templates).

Legacy JSON output keeps `meta.schema_version = "1"`. This is independent from the core library's reversible native `AtlasDocument` version 2. The legacy JSON schemas are in the repository [schemas directory](https://github.com/Latias94/tex-packer/tree/main/schemas).

## Statistics

`--export-stats <path>` writes:

```text
pages, frames, regions, aliases,
page_area, content_area, allocation_area,
content_occupancy, allocation_occupancy
```

`frames` counts logical entries, while `regions` and physical areas count deduplicated allocations once. With `--dry-run`, the same values are printed instead of written.

## YAML Configuration

Pass `--config <path>` to overlay a flat YAML configuration on the CLI draft:

```yaml
family: auto
auto_mode: quality
time_budget_ms: 500
parallel: true
max_width: 1024
max_height: 1024
allow_rotation: true
border_padding: 0
texture_padding: 2
texture_extrusion: 0
trim: true
trim_threshold: 0
transparent_policy: keep
power_of_two: false
square: false
sort_order: area_desc
mr_reference: false
```

The accepted family-specific values are shown by `tex-packer pack --help`. `--print-config --print-config-format yaml` prints the merged flat projection and exits.

Configuration loading is strict:

- Unknown fields are rejected with a source location.
- Duplicate mapping keys are rejected.
- YAML tags and merge keys (`<<`) are rejected instead of being silently discarded or expanded.
- YAML 1.1-only boolean words such as `yes`, `no`, `on`, and `off` remain strings and fail when a boolean is required.

## Parallel Auto Packing

Build with the `parallel` feature before using `--parallel` for concurrent Auto candidate evaluation:

```bash
cargo run -p tex-packer-cli --features parallel -- \
  pack ./assets --algorithm auto --auto-mode quality --parallel
```

Without the feature, configuration and output remain valid but candidate evaluation is sequential.

## Common Recipes

```bash
# Include and exclude globs
tex-packer pack ./assets --include "**/*.png" --exclude "**/draft/**"

# Layout-only JSON hash
tex-packer layout ./assets --out ./layout --metadata json-hash

# Plist output
tex-packer pack ./assets --out ./out --metadata plist

# Validate configuration without packing
tex-packer pack ./assets --print-config --print-config-format yaml

# Validate packing and report stats without writing atlas files
tex-packer pack ./assets --dry-run --export-stats ./out/stats.json
```

For source-level v0.2 to v0.3 changes in the core library, see the [v0.3 migration guide](https://github.com/Latias94/tex-packer/blob/main/docs/migrations/v0.3.md).
