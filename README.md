# tex-packer

A modern, deterministic texture atlas packer for Rust. Ships both a core library and a CLI, supporting multiple packing algorithms (Skyline, MaxRects, Guillotine), multipage packing, trimming, rotation, extrusion, and engine-friendly exporters (JSON, Plist, and template-based for Unity/Godot/Phaser/Spine/Cocos/Unreal).

- Crates
  - `tex-packer-core`: pure library (no fs side effects). Packs in-memory images into atlases and returns pages + metadata. wasm-friendly design.
  - `tex-packer-cli`: command-line tool built on the core. Handles I/O, encoding, logging, and exporters.
- `tex-packer-gui`: desktop GUI built with egui/eframe (wgpu); load folder, configure, preview, and export.

- Algorithms
  - Skyline: BottomLeft, MinWaste (+ optional Waste Map)
  - MaxRects: BestArea/ShortSide/LongSide/BottomLeft/ContactPoint
  - Guillotine: Choice (Best/Worst Area/Side) + Split (Short/Long axis + Min/Max area)

- Highlights
  - Multipage packing, stable sorting, auto presets (fast/quality)
  - Rotation-safe rendering, trim, padding/extrude, debug outlines
- Exporters: JSON (hash/array), Plist (TexturePacker style), templates (Unity/Godot/Phaser/Spine/Cocos/Unreal)

## Best Practices (Algorithm & Settings)

- Offline/build-time (quality)
  - Algorithm: `--algorithm auto --auto-mode quality` (optionally `--features parallel` + `--parallel`)
  - Trim: on (threshold 0), Rotation: on (unless your runtime disallows it)
  - Padding/Extrude: `--texture-padding 2`, `--texture-extrusion 2`
  - POW2/Square: only if required by target engine (`--pow2`, `--square`)

- Runtime/load-time (latency)
  - Use layout-only APIs, then upload subimages to GPU
    - Core: `pack_layout` / `pack_layout_items`, or `runtime::AtlasSession` for Append/Grow/Evict
    - CLI: `layout` subcommand to export JSON/Plist (no PNG)
  - Algorithm: Skyline MinWaste; consider BottomLeft for steadier times
  - Trim: off (pre-trim assets); Waste Map: off for steadier latency, on for higher occupancy
  - Keep padding/extrude = 2 for safe sampling

Rule of thumb
- “Highest quality with a time budget” → Auto Quality
- “Fast & predictable runtime” → Skyline MinWaste + layout-only
- “Incremental/streaming” → `runtime::AtlasSession` (Guillotine)

## Quickstart

CLI:

- Install: `cargo install --path crates/tex-packer-cli`
- Pack: `tex-packer pack <input_dir> --out out --name atlas`
- Metadata formats: `--metadata json-array` (alias: `json`) | `json-hash` | `plist` | `template`
  - For `template`: use `--engine unity|godot|phaser3|phaser3_single|spine|cocos|unreal` or provide `--template <file.hbs>`
- Quality preset: `tex-packer pack assets/kenney-ui-pack --algorithm auto --auto-mode quality --time-budget 500 --parallel --metadata plist`
  - Note: For `--parallel` to take effect, build the CLI with `--features parallel` (e.g., `cargo run -p tex-packer-cli --features parallel -- ...`).
- Templates: `tex-packer template assets/kenney-ui-pack --engine unity --out out`
- Bench (quick): `tex-packer bench assets/kenney-ui-pack --algorithm auto`
- Export stats: `--export-stats out/stats.json`
  - Include/Exclude globs: `--include "**/*.png" --exclude "**/ui/**"`
  - Progress/verbosity: `--progress/--no-progress`, `-q`, `-v/-vv`

GUI:

- Run from repo: `cargo run -p tex-packer-gui`
- Pick input/output folders, adjust config, Pack, and Export.

![GUI Overview](https://raw.githubusercontent.com/Latias94/tex-packer/main/screenshots/gui-overview.png)

Library:

```rust
use image::DynamicImage;
use tex_packer_core::{pack_images, InputImage, PackerConfig};

fn pack_in_memory(images: Vec<(String, DynamicImage)>) -> anyhow::Result<()> {
    let inputs: Vec<InputImage> = images.into_iter()
        .map(|(key, image)| InputImage { key, image })
        .collect();
    let cfg = PackerConfig { max_width: 1024, max_height: 1024, ..Default::default() };
    let out = pack_images(inputs, cfg)?;
    for page in &out.pages { println!("page {}: {}x{}", page.page.id, page.page.width, page.page.height); }
    Ok(())
}
```

## Crates

- See `crates/tex-packer-core/README.md` for library API and configuration reference.
- See `crates/tex-packer-cli/README.md` for CLI usage and template exporters.

## Status

Active development. Algorithms and exporters are in good shape; auto presets and templates cover common engine workflows. See crate READMEs for details and examples.

- Ergonomics: `PackerConfig::builder()` and a `prelude` are available in the core crate.
- JSON meta: now includes `schema_version = "1"` for forward compatibility.

## Wasm

- The core builds for `wasm32-unknown-unknown` (no fs side effects). Check with:
  - `rustup target add wasm32-unknown-unknown`
  - `cargo build -p tex-packer-core --target wasm32-unknown-unknown`
- In browser/wasm, pass in-memory RGBA as `DynamicImage` and consume RGBA pages for rendering.

## Auto Presets

- `--algorithm auto --auto-mode fast|quality`
  - Selection rule: minimize pages first, then total area (sum of page areas)
  - Time budget: `--time-budget <ms>` limits candidate evaluation time
  - Parallel: `--parallel` evaluates candidates in parallel when the core is built with the `parallel` feature
- MaxRects reference path (mr_reference)
  - When on, uses reference-accurate SplitFreeNode + staged prune (higher quality on large sets; slower)
  - Auto(quality) auto-enables mr_reference for MaxRects candidates when `time_budget_ms >= 200` or inputs `>= 800`

## Benchmarks (Summary)

- kenney-ui-pack (release): max 1024x1024, trim on, rotate on, padding=2
  - Skyline(MW): 1 page, 88.69%
  - MaxRects(BAF/BL/CP): 1 page, 83.32% / 82.76% / 74.23%
  - Guillotine(BAF+SLAS): 5 pages, 80.58%
- MaxRects split/prune path (single-page synthetic 2048x2048, 4..96 px random)
  - N=1000: mr_ref=false → placed 377 (22.82%), ~8ms; mr_ref=true → placed 1000 (58.68%), ~304ms
  - N=5000: mr_ref=false → placed 405 (24.55%), ~9ms;  mr_ref=true → placed 1611 (95.91%), ~1241ms
  - Interpretation: mr_reference greatly improves single-page occupancy/placements at higher CPU cost. Prefer on for offline quality; off for runtime/latency.

### Benchmarks (Generated Test Sets)

- Config: max 2048x2048; trim on; rotation on; padding=2; extrude=2; border=0; pow2/square off.
- Dataset: `assets/generated` (subsets: `basic`, `thin`, `trim`).
- Method: portfolio comparison (Skyline MW / MaxRects BAF, BL, CP / Guillotine BAF+SLAS). Selection favors fewer pages, then lower total area. Times below are wall‑clock from release builds on a single machine; results are deterministic.

- Mixed (assets/generated): pages=3 (release)
  - Skyline(MW): occ=83.69%, time=32ms
  - MaxRects(BL): occ=70.55%, time=34ms
  - MaxRects(BAF): occ=67.73%, time=33ms
  - Guillotine(BAF+SLAS): occ=62.73%, time=38ms
  - MaxRects(CP): occ=56.15%, time=36ms
  - Raw JSON: `out/bench_generated_release/bench_portfolio.json`

- basic (assets/generated/basic): pages=1 (release)
  - Skyline(MW): occ=78.13%, time=4.6ms
  - MaxRects(BL): occ=46.78%, time=5.3ms
  - MaxRects(BAF): occ=46.65%, time=5.3ms
  - MaxRects(CP): occ=25.45%, time=6.1ms
  - Guillotine(BAF+SLAS): occ=45.81%, time=5.0ms

- thin (assets/generated/thin): pages=1 (release)
  - Skyline(MW): occ=48.08%, time=709µs
  - MaxRects(BL): occ=32.08%, time=660µs
  - MaxRects(BAF): occ=22.01%, time=805µs
  - MaxRects(CP): occ=2.54%, time=2.5ms
  - Guillotine(BAF+SLAS): occ=11.15%, time=915µs

- trim (assets/generated/trim): pages=1 (release)
  - Skyline(MW): occ=67.52%, time=3.7ms
  - MaxRects(BAF): occ=26.58%, time=4.0ms
  - MaxRects(BL): occ=26.06%, time=4.5ms
  - MaxRects(CP): occ=4.75%, time=6.2ms
  - Guillotine(BAF+SLAS): occ=23.67%, time=4.1ms

Notes:
- Times are for relative comparison; use your target hardware to calibrate. The ranking trends hold between debug and release.
- The generated sets include a 1‑px border and large, centered numeric labels to simplify visual inspection of atlases (overlaps/bleeding/rotation).


