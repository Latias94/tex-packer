# tex-packer-core

[![Crates.io](https://img.shields.io/crates/v/tex-packer-core.svg)](https://crates.io/crates/tex-packer-core)
[![Docs.rs](https://docs.rs/tex-packer-core/badge.svg)](https://docs.rs/tex-packer-core)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/Latias94/tex-packer)

Core library for packing many textures into atlas pages. Provides algorithms, data model, and a pipeline to pack in-memory images and return pages + metadata for export.

## Install

- Cargo.toml

```toml
[dependencies]
tex-packer-core = { git = "https://example.com/your/repo", package = "tex-packer-core" }
image = "0.25"
```

Note: If using crates.io, set the version accordingly after publish.

## Quick Example

```rust
use image::ImageReader;
use tex_packer_core::{InputImage, PackerConfig, pack_images};

fn main() -> anyhow::Result<()> {
    let img1 = ImageReader::open("a.png")?.decode()?;
    let img2 = ImageReader::open("b.png")?.decode()?;
    let inputs = vec![
        InputImage { key: "a".into(), image: img1 },
        InputImage { key: "b".into(), image: img2 },
    ];

    let cfg = PackerConfig { max_width: 1024, max_height: 1024, ..Default::default() };
    let out = pack_images(inputs, cfg)?;
    for page in &out.pages {
        println!("page {} ({}x{}), frames={}", page.page.id, page.page.width, page.page.height, page.page.frames.len());
    }
    Ok(())
}
```

## Configuration

`PackerConfig` controls dimensions, trim, rotation, sorting, and algorithm family/heuristics.

Key fields:
- `max_width`, `max_height`: page limits.
- `allow_rotation`: allow 90° rotation for tighter packing.
- `trim`, `trim_threshold`: trim transparent borders (alpha ≤ threshold).
- `texture_padding`, `border_padding`, `texture_extrusion`.
- `power_of_two`, `square`.
- `family`: `Skyline | MaxRects | Guillotine | Auto`.
- `skyline_heuristic`: `BottomLeft | MinWaste` (+ `use_waste_map`).
- `mr_heuristic`: `BestAreaFit | BestShortSideFit | BestLongSideFit | BottomLeft | ContactPoint`.
- `g_choice` + `g_split`: Guillotine heuristics.
- `sort_order`: stable sorting mode.
- `auto_mode`: `Fast | Quality`.
- `time_budget_ms`, `parallel`: enables time-bounded portfolio and optional parallel evaluation for Auto.
- `mr_reference`: use reference-accurate MaxRects split/prune (higher quality, slower).

Builder and prelude:
- Use `PackerConfig::builder()` for fluent construction and `tex_packer_core::prelude::*` to import common types.

```rust
use tex_packer_core::prelude::*;

let cfg = PackerConfig::builder()
    .with_max_dimensions(1024, 1024)
    .allow_rotation(true)
    .texture_padding(2)
    .auto_mode(AutoMode::Quality)
    .build();
```

## API Surface

- `pack_images(inputs, cfg) -> PackOutput`
  - Inputs: `Vec<InputImage { key: String, image: DynamicImage }>`
  - Output: `PackOutput { atlas: Atlas, pages: Vec<OutputPage> }`
  - `OutputPage { page: Page, rgba: RgbaImage }`
- Data model (serde): `Rect`, `Frame`, `Page`, `Atlas`, `Meta`.
- Export helpers: JSON (hash/array) and Plist string builders are available; the CLI crate covers file writing.

Metadata schema:
- `meta.schema_version` is currently "1" for JSON outputs. Future additive fields may bump this.

## Runtime Usage

When using the core in a game/runtime, prefer layout-only placement (no pixel compositing), then upload subimages to your GPU atlas. The core exposes two runtime‑friendly paths:

- Layout-only, single shot (batch): `pack_layout` / `pack_layout_items`
- Incremental session (append/evict, multi‑shot): `runtime::AtlasSession`

Recommended runtime config
- Algorithm: Skyline MinWaste (good occupancy + steady latency) or BottomLeft for even steadier times
- Trim: off (assume inputs are pre‑trimmed) to avoid alpha scans on hot paths
- Rotation: by engine needs; Padding/Extrude: 2/2 are safe defaults
- Waste map (Skyline): off for steadier perf; on for higher occupancy

Layout-only (sizes only)
```rust
use tex_packer_core::prelude::*;

let items = vec![("a", 32, 20), ("b", 64, 32)];
let cfg = PackerConfig::builder()
    .with_max_dimensions(2048, 2048)
    .allow_rotation(true)
    .square(false)
    .pow2(false)
    .build();

let atlas = pack_layout(items, cfg)?;
for page in &atlas.pages {
    // Upload subimages to your GPU texture using frame.xywh and rotated flag (if you keep it per item)
    // Here, pack_layout returns only geometry; pixel uploads are handled by your renderer.
    for f in &page.frames { println!("{} => {:?}", f.key, f.frame); }
}
```

Layout-only (with source/source_size)
```rust
use tex_packer_core::prelude::*;

let items = vec![
    LayoutItem { key: "a".into(), w: 30, h: 18,
                 source: Some(Rect::new(2, 1, 30, 18)),
                 source_size: Some((32, 20)), trimmed: true },
    LayoutItem { key: "b".into(), w: 64, h: 32, source: None, source_size: None, trimmed: false },
];
let cfg = PackerConfig::builder().with_max_dimensions(2048, 2048).build();
let atlas = pack_layout_items(items, cfg)?;
```

Incremental (append/evict)
```rust
use tex_packer_core::prelude::*;

let cfg = PackerConfig::builder()
    .with_max_dimensions(2048, 2048)
    .allow_rotation(true)
    .build();
let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Guillotine);

let (_page_a, frame_a) = sess.append("spriteA".into(), 64, 32)?; // place
let (_page_b, frame_b) = sess.append("spriteB".into(), 48, 64)?;

// Upload A/B to your GPU atlas using frame.frame (xywh) and frame.rotated

let snap = sess.snapshot_atlas(); // geometry snapshot (no RGBA pages)
assert!(!snap.pages.is_empty());

let removed = sess.evict(_page_a, "spriteA"); // free the slot
```

Shelf runtime strategy
```rust
use tex_packer_core::prelude::*;

let cfg = PackerConfig::builder()
    .with_max_dimensions(2048, 2048)
    .allow_rotation(true)
    .texture_padding(2)
    .texture_extrusion(1)
    .build();

// NextFit keeps filling the last shelf; FirstFit scans from the top.
let mut sess = AtlasSession::new(cfg, RuntimeStrategy::Shelf(ShelfPolicy::NextFit));

let (_page_a, a) = sess.append("A".into(), 64, 32)?;
let (_page_b, b) = sess.append("B".into(), 80, 32)?;
assert!(sess.evict(0, "A"));
let (_page_c, c) = sess.append("C".into(), 64, 32)?; // tends to reuse A's freed segment

let snap = sess.snapshot_atlas(); // geometry only
```

Runtime strategy guidance
- Shelf(NextFit/FirstFit): lower variance, simple and fast; great for online append/evict with many similarly tall items. Use NextFit for fewer scans; FirstFit to reduce top‑area fragmentation.
- Guillotine: higher packing quality under fragmentation; good for heterogeneous sizes; costlier per update.
- Both strategies place content inside reserved slots with offset `extrude + padding/2`, so extrusion never bleeds across neighbors.

Notes
- Frames are positioned inside reserved slots with an offset `extrude + padding/2`, so extrusion stays inside the slot and won’t bleed into neighbors.
- `frame.rotated` specifies a 90° clockwise rotation at placement time; adjust your sampling/upload accordingly.
- For composited PNGs at runtime, you can still call `pack_images` in a background task, but layout-only is preferred for latency.

## Notes

- `mr_reference` (MaxRects split/prune): enables reference-style splitting (SplitFreeNode) with staged pruning; improves packing on large sets at higher CPU cost.
- Skyline Waste Map is aligned with the reference implementation; no-Overlap guarantees are enforced by internal subtractive splitting and pruning.
 - Auto thresholds: `auto_mr_ref_time_ms_threshold` / `auto_mr_ref_input_threshold` let you tune when quality mode auto-enables `mr_reference`.

For CLI usage, templates, and exporters, see `crates/tex-packer-cli/README.md`.

## Wasm

- The core crate is designed to compile to `wasm32-unknown-unknown` (no filesystem, no threads by default).
- Build check:
  - `rustup target add wasm32-unknown-unknown`
  - `cargo build -p tex-packer-core --target wasm32-unknown-unknown`
- Usage model in wasm:
  - Provide decoded RGBA buffers and wrap as `image::DynamicImage` (e.g., `DynamicImage::ImageRgba8(RgbaImage::from_raw(w, h, rgba).unwrap())`).
  - Call `pack_images` with in-memory images and use the returned page RGBA buffers to render to `<canvas>` or export.
- Parallel portfolio is behind the `parallel` feature; keep it disabled for wasm.

## Auto Portfolio & mr_reference

- `family = Auto` tries a small portfolio and picks the best (pages first, then total area).
- `time_budget_ms` can limit evaluation; `parallel` may evaluate candidates in parallel (when feature is enabled).
- In `auto_mode = Quality`, the core auto-enables `mr_reference` for MaxRects candidates when `time_budget_ms >= 200` or the number of inputs `>= 800`.

## Benchmark Summary

- kenney-ui-pack (release, 1024x1024, trim on, rotate on, padding=2):
  - Skyline(MW): 1 page, 88.69%
  - MaxRects(BAF/BL/CP): 1 page, 83.32% / 82.76% / 74.23%
  - Guillotine(BAF+SLAS): 5 pages, 80.58%
- MaxRects split/prune (single 2048x2048, random 4..96 px):
  - N=1000: mr_ref=false → 22.82% (~8ms); mr_ref=true → 58.68% (~304ms)
  - N=5000: mr_ref=false → 24.55% (~9ms);  mr_ref=true → 95.91% (~1241ms)
