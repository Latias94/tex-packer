# tex-packer

A deterministic texture atlas packer for Rust. Use it as a command-line tool, a desktop GUI, or a pure Rust library.

`tex-packer` supports Skyline, MaxRects, and Guillotine packing; multi-page atlases; identical-content deduplication; trimming; rotation; padding; extrusion; layout-only packing; and JSON, Plist, and engine-template metadata exporters.

![GUI Overview](https://raw.githubusercontent.com/Latias94/tex-packer/main/screenshots/gui-overview.png)

## Which package should I use?

| Need | Package | What it does |
| --- | --- | --- |
| Pack image folders from scripts or CI | `tex-packer-cli` | Reads images from disk and writes atlas PNG pages plus metadata. |
| Pack visually on desktop | `tex-packer-gui` | Lets you choose folders, tune settings, preview pages, and export. |
| Integrate packing in your Rust app/tool | `tex-packer-core` | Pure in-memory API with no filesystem side effects. |

## Installation

From this repository:

```bash
# CLI
cargo install --path crates/tex-packer-cli

# GUI
cargo run -p tex-packer-gui --release
```

After the crates are published to crates.io:

```bash
cargo install tex-packer-cli
cargo install tex-packer-gui
```

For library usage:

```toml
[dependencies]
tex-packer-core = "0.2"
image = "0.25"
```

Before crates.io publication, depend on the repository directly:

```toml
[dependencies]
tex-packer-core = { git = "https://github.com/Latias94/tex-packer", package = "tex-packer-core" }
image = "0.25"
```

## CLI quickstart

Pack all images in a folder:

```bash
tex-packer pack ./assets --out ./out --name atlas
```

Outputs:

- Single page: `out/atlas.png` and `out/atlas.json`
- Multiple pages: `out/atlas_0.png`, `out/atlas_1.png`, ... and `out/atlas.json`

Common recipes:

```bash
# Higher-quality offline packing
tex-packer pack ./assets \
  --out ./out \
  --name atlas \
  --algorithm auto \
  --auto-mode quality \
  --time-budget 500 \
  --allow-rotation \
  --texture-padding 2 \
  --texture-extrusion 2

# TexturePacker-style plist metadata
tex-packer pack ./assets --out ./out --name atlas --metadata plist

# Engine template export
tex-packer template ./assets --out ./out --name atlas --engine unity
tex-packer template ./assets --out ./out --name atlas --engine godot
tex-packer template ./assets --out ./out --name atlas --engine phaser3
tex-packer template ./assets --out ./out --name atlas --engine spine

# Layout-only metadata, no PNG compositing
tex-packer layout ./assets --out ./out --name atlas --metadata json-hash

# Include/exclude files
tex-packer pack ./assets --include "**/*.png" --exclude "**/draft/**"

# Inspect the final merged config without packing
tex-packer pack ./assets --print-config --print-config-format yaml

# Dry run: compute layout and stats without writing atlas files
tex-packer pack ./assets --dry-run --export-stats ./out/stats.json
```

Metadata formats:

| Format | Flag | Use when |
| --- | --- | --- |
| JSON array | `--metadata json-array` or `json` | You want a simple ordered frame list. |
| JSON hash | `--metadata json-hash` | You want lookup by sprite name. |
| Plist | `--metadata plist` | You need TexturePacker-style metadata. |
| Template | `--metadata template` or `tex-packer template` | You need Unity, Godot, Phaser, Spine, Cocos, Unreal, or a custom Handlebars export. |

Run `tex-packer --help` or `tex-packer pack --help` for all options.

### Parallel auto packing

`--parallel` only takes effect when the CLI is built with the `parallel` feature:

```bash
cargo run -p tex-packer-cli --features parallel -- \
  pack ./assets --algorithm auto --auto-mode quality --parallel
```

## GUI quickstart

Run the desktop GUI:

```bash
cargo run -p tex-packer-gui --release
```

Basic workflow:

1. Pick an input folder containing images.
2. Pick an output folder.
3. Adjust atlas size, algorithm, padding, rotation, trimming, and auto settings.
4. Click **Pack** to preview atlas pages.
5. Click **Export** to write PNG pages and JSON metadata.

The GUI is the easiest way to tune settings interactively. The CLI is better for repeatable build scripts and CI.

## Library quickstart

Use `tex-packer-core` when you already have images in memory or want to embed atlas packing in another Rust tool.

```rust
use std::fs;

use image::ImageReader;
use tex_packer_core::{InputImage, PackerConfig, pack_images, to_json_hash};

fn main() -> anyhow::Result<()> {
    let images = vec![
        ("hero".to_string(), ImageReader::open("assets/hero.png")?.decode()?),
        ("enemy".to_string(), ImageReader::open("assets/enemy.png")?.decode()?),
    ];

    let inputs = images
        .into_iter()
        .map(|(key, image)| InputImage { key, image })
        .collect();

    let config = PackerConfig::builder()
        .with_max_dimensions(1024, 1024)
        .allow_rotation(true)
        .trim(true)
        .texture_padding(2)
        .texture_extrusion(2)
        .build();

    let output = pack_images(inputs, config)?;

    for page in &output.pages {
        let path = format!("out/atlas_{}.png", page.page.id);
        page.rgba.save(path)?;
    }

    fs::write(
        "out/atlas.json",
        serde_json::to_string_pretty(&to_json_hash(&output.atlas))?,
    )?;

    Ok(())
}
```

Layout-only packing is useful for engines that upload pixels themselves:

```rust
use tex_packer_core::prelude::*;

let items = vec![("hero", 64, 48), ("enemy", 32, 32), ("coin", 16, 16)];
let config = PackerConfig::builder()
    .with_max_dimensions(2048, 2048)
    .allow_rotation(true)
    .texture_padding(2)
    .build();

let atlas = pack_layout(items, config)?;

for page in &atlas.pages {
    for frame in &page.frames {
        println!("{} => page {}, {:?}", frame.key, page.id, frame.frame);
    }
}
# Ok::<(), tex_packer_core::PackError>(())
```

For streaming or runtime use, `tex_packer_core::runtime::AtlasSession` supports append/evict workflows with runtime strategies such as Guillotine and Shelf.

See:

- [`crates/tex-packer-core/README.md`](crates/tex-packer-core/README.md) for library API details.
- [`crates/tex-packer-cli/README.md`](crates/tex-packer-cli/README.md) for CLI options, YAML config, and template exporters.
- [`crates/tex-packer-gui/README.md`](crates/tex-packer-gui/README.md) for GUI usage.

## Recommended defaults

| Scenario | Suggested settings |
| --- | --- |
| Offline/build-time quality | `--algorithm auto --auto-mode quality --time-budget 500 --allow-rotation --texture-padding 2 --texture-extrusion 2` |
| Fast repeatable builds | `--algorithm skyline --skyline minwaste --texture-padding 2` |
| Runtime/layout-only | Use `pack_layout` or `tex-packer layout`; keep trimming off if assets are already prepared. |
| Engines that need power-of-two or square textures | Add `--pow2`, `--square`, or both. |
| Avoid texture bleeding | Use `--texture-padding 2 --texture-extrusion 2`. |

Rule of thumb:

- Choose **Auto Quality** for final offline atlas generation.
- Choose **Skyline MinWaste** for predictable speed.
- Choose **layout-only** when your renderer handles pixel uploads.

## Algorithms

- Skyline: BottomLeft, MinWaste, optional Waste Map.
- MaxRects: BestAreaFit, BestShortSideFit, BestLongSideFit, BottomLeft, ContactPoint.
- Guillotine: Best/Worst area or side choices plus several split strategies.
- Auto: evaluates a portfolio and chooses fewer pages first, then smaller total page area.

For large offline packs, `mr_reference` can improve MaxRects placement quality at higher CPU cost. In Auto Quality mode, the core can enable it automatically for larger or more generous time-budgeted jobs.

## Wasm

`tex-packer-core` is designed to build for `wasm32-unknown-unknown` because it has no filesystem side effects:

```bash
rustup target add wasm32-unknown-unknown
cargo build -p tex-packer-core --target wasm32-unknown-unknown
```

In browser/wasm usage, decode images yourself, pass `DynamicImage` values to the core, then consume returned RGBA pages or layout metadata.

## Development

Useful local checks:

```bash
cargo fmt --check
cargo check -p tex-packer-core -p tex-packer-cli -p tex-packer-gui
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
```

## Status

Active development. The v0.2 series focuses on correctness, deterministic output, safer placement geometry, stronger regression coverage, and more practical CLI/GUI workflows.
