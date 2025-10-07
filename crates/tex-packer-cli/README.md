# tex-packer-cli

[![Crates.io](https://img.shields.io/crates/v/tex-packer-cli.svg)](https://crates.io/crates/tex-packer-cli)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/Latias94/tex-packer)

Command-line tool for tex-packer. Packs images from disk into atlas pages, writes PNGs and metadata (JSON/Plist/templates).

## Install

- From repo: `cargo install --path crates/tex-packer-cli`
- From crates.io: use `cargo install tex-packer-cli` after publish.
- Parallel portfolio (optional): build the CLI with the `parallel` feature so `--parallel` takes effect.
  - Example: `cargo run -p tex-packer-cli --features parallel -- <args>`

## Usage

Subcommands:

- Pack: `tex-packer pack <input> [options]` (writes PNGs + metadata)
- Template: `tex-packer template <input> [options]` (forces `--metadata template`)
- Layout: `tex-packer layout <input> [options]` (layout-only: no PNGs; exports JSON/Plist)
- Bench: `tex-packer bench <input> [--algorithm auto] [--auto-mode quality] [--time-budget MS]`

Global flags: `[-q|--quiet] [-v|--verbose] [--progress|--no-progress]`

Metadata formats:

- `--metadata json-array` (alias: `json`) — JSON array layout
- `--metadata json-hash` — JSON hash layout
- `--metadata plist` — TexturePacker-style Plist
- `--metadata template` — Handlebars template (use `--engine unity|godot|phaser3|phaser3_single|spine|cocos|unreal` or `--template <file.hbs>`) 

Examples:
- Pack basic: `tex-packer pack assets/kenney-ui-pack --out out --name atlas`
- Auto (quality): `tex-packer pack assets/kenney-ui-pack --algorithm auto --auto-mode quality --time-budget 500 --parallel --metadata plist`
  - Note: `--parallel` requires building with `--features parallel`.
- Template export: `tex-packer template assets/kenney-ui-pack --engine unity --out out`
- Plist export: `tex-packer pack assets/kenney-ui-pack --metadata plist --out out`
- Layout-only (JSON-Hash): `tex-packer layout assets/generated --out-dir out_layout --name atlas_layout --metadata json-hash`
- Layout-only (Plist): `tex-packer layout assets/generated/basic --out-dir out_layout --name basic_layout --metadata plist`
- Stats: `--export-stats out/stats.json` writes `{ pages, used_area, total_area, occupancy }`
- MaxRects reference split/prune: add `--mr-reference` (quality better on large sets; slower)
- Print merged config and exit: `--print-config` (useful to inspect YAML+CLI result)
- Include/Exclude: `--include "**/*.png" --exclude "**/ui/**"` (multiple allowed)
- Verbosity: `-q/--quiet` suppresses logs; `-v`/`-vv` increases verbosity
- Progress: `--progress/--no-progress` toggles progress bars (default on; disabled by quiet)
- Auto thresholds: override quality mode thresholds via `--auto-mr-ref-time-threshold 500` or `--auto-mr-ref-input-threshold 1000`

## YAML Configuration

You can provide a YAML file via `--config` to set options together. CLI flags still override where noted.

```yaml
family: auto            # skyline|maxrects|guillotine|auto
skyline: minwaste
heuristic: baf          # for MaxRects
use_waste_map: false
max_width: 1024
max_height: 1024
allow_rotation: true
border_padding: 0
texture_padding: 2
texture_extrusion: 0
trim: true
trim_threshold: 0
power_of_two: false
square: false
sort_order: area_desc
auto_mode: quality
# Portfolio controls
time_budget_ms: 500
parallel: true
# MaxRects split/prune path (reference-accurate)
mr_reference: true
```

## Templates

Built-in engines: `unity`, `godot`, `phaser3` (multi-atlas), `phaser3_single` (single-page json), `spine` (.atlas text), `cocos`, `unreal`.
- Custom template: `--metadata template --template my.tpl.hbs`
- Template context: see docs/TEMPLATES.md or inspect built-ins under `src/templates/`.

Compact context shape:
- `pages: [ { image: String, size: { w, h }, sprites: [ { name, frame:{x,y,w,h}, rotated, trimmed, sprite_source_size:{x,y,w,h}, source_size:{w,h}, pivot:{x,y} } ] } ]`
- `meta: { app, version, format, scale }` (JSON exports include full atlas meta with `schema_version`)

## Notes

- Sorting is stable; repeated runs with same inputs/config yield the same atlas.
- `--parallel` requires enabling the `parallel` feature in the core crate when building from source.
- For large sets, use `--release` to improve performance.
- JSON metadata includes `meta.schema_version = "1"`.
- JSON Schema (optional): see `schemas/tex-packer-atlas-hash.schema.json` and `schemas/tex-packer-atlas-array.schema.json`.

## Auto Presets & mr_reference

- `--algorithm auto --auto-mode fast|quality` tries a small portfolio (quality tries more MaxRects/Guillotine variants).
- Selection: minimize pages, then total area (sum of page areas).
- Time budget: `--time-budget <ms>` limits candidate evaluation time; `--parallel` can evaluate candidates in parallel.
- MaxRects `--mr-reference` toggles reference-accurate split/prune. In quality mode, the core auto-enables `mr_reference` for MaxRects candidates when `time_budget_ms >= 200` or inputs `>= 800`.

## Benchmark (Summary)

- kenney-ui-pack (release): Skyline(MW)=1p/88.69%; MaxRects(BAF/BL/CP)=1p/83.32%/82.76%/74.23%; Guillotine(BAF+SLAS)=5p/80.58%.
- MaxRects split/prune (single 2048x2048)
  - N=1000: mr_ref=false → 22.82% (~8ms); mr_ref=true → 58.68% (~304ms)
  - N=5000: mr_ref=false → 24.55% (~9ms);  mr_ref=true → 95.91% (~1241ms)
