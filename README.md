# tex-packer

A deterministic texture atlas packer for Rust. Use it as a command-line tool, a desktop GUI, or an in-memory Rust library.

tex-packer supports Skyline, MaxRects, and Guillotine placement; multi-page atlases; identical-content deduplication; trimming; rotation; padding; extrusion; layout-only workflows; and JSON, plist, and engine-template exporters.

![GUI Overview](https://raw.githubusercontent.com/Latias94/tex-packer/main/screenshots/gui-overview.png)

## Packages

| Need | Package | Responsibility |
| --- | --- | --- |
| Pack folders in scripts or CI | `tex-packer-cli` | Reads image files and writes atlas pages plus metadata. |
| Tune and preview packs on desktop | `tex-packer-gui` | Provides folder selection, validated controls, preview, and export. |
| Embed packing in a Rust application | `tex-packer-core` | Provides side-effect-free offline and runtime workflows over in-memory data. |

## Installation

From crates.io:

```bash
cargo install tex-packer-cli
cargo install tex-packer-gui
```

From this repository:

```bash
cargo install --path crates/tex-packer-cli
cargo run -p tex-packer-gui --release
```

Library dependency:

```toml
[dependencies]
tex-packer-core = "0.3"
image = "0.25"
serde_json = "1"
```

## CLI Quickstart

```bash
tex-packer pack ./assets --out ./out --name atlas
```

The CLI writes `out/atlas.png` and `out/atlas.json` for one page, or numbered PNG files plus one metadata file for multiple pages.

Common workflows:

```bash
# Higher-quality offline packing
tex-packer pack ./assets \
  --out ./out \
  --algorithm auto \
  --auto-mode quality \
  --time-budget 500 \
  --allow-rotation \
  --texture-padding 2 \
  --texture-extrusion 2

# TexturePacker-style plist metadata
tex-packer pack ./assets --out ./out --metadata plist

# Built-in engine template
tex-packer template ./assets --out ./out --engine unity

# Metadata without page rendering
tex-packer layout ./assets --out ./out --metadata json-hash

# Inspect the validated CLI/YAML projection
tex-packer pack ./assets --print-config --print-config-format yaml
```

`--parallel` takes effect when the CLI is built with its `parallel` feature:

```bash
cargo run -p tex-packer-cli --features parallel -- \
  pack ./assets --algorithm auto --auto-mode quality --parallel
```

See [the CLI guide](crates/tex-packer-cli/README.md) for configuration, output formats, and template details.

## GUI Quickstart

```bash
cargo run -p tex-packer-gui --release
```

Pick input and output folders, adjust the packing configuration, select **Pack** to preview, then select **Export** to write PNG and JSON output. See [the GUI guide](crates/tex-packer-gui/README.md) for the complete workflow.

## Library Quickstart

v0.3 exposes validated workflow facades instead of public placement algorithms and free packing functions.

```rust
use std::fs;

use image::ImageReader;
use tex_packer_core::config::{OfflineConfig, PageConfig};
use tex_packer_core::export::to_json_hash;
use tex_packer_core::offline::{InputImage, OfflinePacker};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let page = PageConfig::builder()
        .max_dimensions(1024, 1024)
        .allow_rotation(true)
        .texture_padding(2)
        .texture_extrusion(2)
        .build()?;
    let config = OfflineConfig::builder().page_config(page).build()?;
    let packer = OfflinePacker::new(config);

    let output = packer.pack_images(vec![
        InputImage {
            key: "hero".into(),
            image: ImageReader::open("assets/hero.png")?.decode()?,
        },
        InputImage {
            key: "enemy".into(),
            image: ImageReader::open("assets/enemy.png")?.decode()?,
        },
    ])?;

    fs::create_dir_all("out")?;
    for rendered in output.pages() {
        let page = output
            .atlas()
            .page(rendered.page_id())
            .expect("rendered page identity must resolve");
        rendered.rgba().save(format!("out/atlas_{}.png", page.id()))?;
    }
    fs::write(
        "out/atlas.json",
        serde_json::to_string_pretty(&to_json_hash(output.atlas()))?,
    )?;
    Ok(())
}
```

For size-only packing, construct `LayoutItem` values and call `OfflinePacker::pack_layout`. For decoded images that need trimming and deduplication but not rendered page pixels, call `OfflinePacker::layout_images`.

```rust
use tex_packer_core::config::OfflineConfig;
use tex_packer_core::offline::{LayoutItem, OfflinePacker};

# fn example() -> tex_packer_core::Result<()> {
let packer = OfflinePacker::new(OfflineConfig::default());
let atlas = packer.pack_layout(vec![
    LayoutItem {
        key: "hero".into(),
        w: 64,
        h: 48,
        source: None,
        source_size: None,
        trimmed: false,
    },
    LayoutItem {
        key: "coin".into(),
        w: 16,
        h: 16,
        source: None,
        source_size: None,
        trimmed: false,
    },
])?;

for page in atlas.pages() {
    for resolved in page.resolved_frames() {
        println!(
            "{} => page {}, {:?}",
            resolved.frame().key(),
            resolved.page_id(),
            resolved.region().content()
        );
    }
}
# Ok(())
# }
```

## Core Model

- `Region` is one physical allocation and owns content geometry, reserved allocation geometry, and rotation.
- `Frame` is one logical sprite and owns its stable `FrameId`, key, source reconstruction metadata, and `RegionId` reference.
- `Page::resolved_frames` is the canonical traversal when both logical and physical facts are needed.
- Offline workflows allow duplicate keys because `FrameId` is authoritative. Native v2 documents, JSON arrays, and templates preserve duplicates; JSON hash keeps the last value, while plist dictionaries are parser-dependent and lossy for repeated keys.
- Runtime workflows require unique keys and return owned `RuntimePlacement` values.
- `Atlas` is an invariant-bearing runtime model. Serialize `AtlasDocument` for reversible native persistence; use the `export` module for legacy JSON, plist, and template formats.

See [the core crate guide](crates/tex-packer-core/README.md) for configuration, runtime packing, persistence, exporters, and statistics.

## v0.3 Migration

v0.3 intentionally removes the v0.2 `PackerConfig`, public algorithm hierarchy, free packing wrappers, broad prelude, public fields, and direct `Atlas` serde contract. CLI command families and legacy exporter shapes remain stable.

Read [the v0.3 migration guide](docs/migrations/v0.3.md) before upgrading a Rust consumer. The architectural rationale is recorded in [the v0.3 core architecture note](docs/solutions/v0-3-core-architecture.md).

## Development

```bash
cargo fmt --all -- --check
cargo nextest run --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check --workspace --all-targets --all-features --locked
```

Maintainers should follow [the release procedure](docs/releasing.md) for CLI
artifacts, crates.io trusted publishing, and partial-release recovery.

## Status

v0.3 is a deliberate breaking release focused on explicit domain identity, validated configuration, transactional runtime mutation, and a smaller public API.
