# tex-packer-core

[![Crates.io](https://img.shields.io/crates/v/tex-packer-core.svg)](https://crates.io/crates/tex-packer-core)
[![Docs.rs](https://docs.rs/tex-packer-core/badge.svg)](https://docs.rs/tex-packer-core)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/Latias94/tex-packer)

The in-memory packing engine for tex-packer. The crate exposes validated offline and runtime workflows while keeping image preparation, placement engines, and compositing private.

## Install

```toml
[dependencies]
tex-packer-core = "0.3"
image = "0.25"
```

## Public Boundary

The supported module boundary is intentionally small:

| Module | Responsibility |
| --- | --- |
| `config` | Fallible builders and workflow-specific strategy values. |
| `model` | Validated atlas aggregates, typed identities, resolution, statistics, and native persistence. |
| `offline` | Decoded render, decoded layout, and pure layout workflows. |
| `runtime` | Incremental layout and pixel-backed atlas workflows. |
| `export` | Legacy JSON, plist, and template projections. |
| `error` | `Result` and `TexPackerError`. |

Concrete Skyline, MaxRects, Guillotine, geometry, preparation, and compositing modules are implementation details. There is no public algorithm trait or prelude.

## Validated Configuration

`PageConfig` validates geometry shared by every workflow. `OfflineConfig` and `RuntimeConfig` then add only policy that applies to their workflow.

```rust
use std::time::Duration;

use tex_packer_core::config::{
    AutoMode, OfflineConfig, PackingStrategy, PageConfig,
};

# fn example() -> tex_packer_core::Result<()> {
let page = PageConfig::builder()
    .max_dimensions(2048, 2048)
    .allow_rotation(true)
    .border_padding(2)
    .texture_padding(2)
    .texture_extrusion(1)
    .build()?;

let config = OfflineConfig::builder()
    .page_config(page)
    .trim(true)
    .trim_threshold(0)
    .strategy(PackingStrategy::Auto {
        mode: AutoMode::Quality,
        time_budget: Some(Duration::from_millis(500)),
        parallel: false,
        reference_time_threshold: None,
        reference_input_threshold: None,
    })
    .build()?;
# let _ = config;
# Ok(())
# }
```

All builders consume themselves and return `Result`. Once built, configuration fields are private and exposed through accessors, so invalid geometry and unrelated strategy settings cannot be introduced later.

## Offline Workflows

One `OfflinePacker` provides three distinct operations:

| Operation | Input | Preparation | Pixel output |
| --- | --- | --- | --- |
| `pack_images` | Decoded `InputImage` values | Trimming, transparent policy, content deduplication | `PackOutput` with `RenderedPage` values |
| `layout_images` | Decoded `InputImage` values | Same preparation and deduplication | `Atlas` only |
| `pack_layout` | Caller-prepared `LayoutItem` values | Size/source validation only | `Atlas` only |

```rust
use image::{DynamicImage, RgbaImage};
use tex_packer_core::config::OfflineConfig;
use tex_packer_core::offline::{InputImage, OfflinePacker};

# fn example() -> tex_packer_core::Result<()> {
let packer = OfflinePacker::new(OfflineConfig::default());
let output = packer.pack_images(vec![InputImage {
    key: "hero".into(),
    image: DynamicImage::ImageRgba8(RgbaImage::new(16, 16)),
}])?;

for rendered in output.pages() {
    let page = output
        .atlas()
        .page(rendered.page_id())
        .expect("rendered page identity must resolve");
    assert_eq!(rendered.rgba().dimensions(), page.size());
}
# Ok(())
# }
```

`PackOutput::into_parts` transfers ownership of the validated atlas and rendered pages when a caller needs to store them separately.

## Regions, Frames, and Resolution

A `Region` is physical: it owns the content rectangle, full reserved allocation, and rotation. A `Frame` is logical: it owns a `FrameId`, user key, source rectangle, source size, and a reference to a region on the same page.

```rust
# use tex_packer_core::model::Atlas;
# fn inspect(atlas: &Atlas) {
for page in atlas.pages() {
    for resolved in page.resolved_frames() {
        let frame = resolved.frame();
        let region = resolved.region();
        println!(
            "frame {} ({}) -> region {} at {:?}, allocation {:?}, rotated={}",
            frame.id(),
            frame.key(),
            region.id(),
            region.content(),
            region.allocation(),
            region.rotated()
        );
    }
}
# }
```

`PageId`, `RegionId`, and `FrameId` are opaque `u32` identities, not vector indexes. Use `Atlas::page`, `Page::region`, and `Page::frame` for direct lookup. Use `Page::resolved_frames` for stable-order traversal without repeated searches.

Decoded offline packing may place identical prepared pixels once while retaining a frame for each input. Duplicate offline keys are valid and retain distinct `FrameId` values. Native v2 documents, JSON arrays, and template projections preserve ordered logical entries. JSON hash keeps the last frame for a repeated key; plist retains its legacy dictionary shape, so parsed duplicate-key behavior is lossy and parser-dependent.

## Runtime Workflows

`AtlasSession` manages incremental geometry. `RuntimeAtlas` wraps the same session semantics with page pixel buffers.

```rust
use tex_packer_core::config::{
    PageConfig, RuntimeConfig, RuntimeStrategy, ShelfPolicy,
};
use tex_packer_core::runtime::AtlasSession;

# fn example() -> tex_packer_core::Result<()> {
let page = PageConfig::builder()
    .max_dimensions(2048, 2048)
    .allow_rotation(true)
    .build()?;
let config = RuntimeConfig::builder()
    .page_config(page)
    .strategy(RuntimeStrategy::Shelf {
        policy: ShelfPolicy::FirstFit,
    })
    .build()?;

let mut session = AtlasSession::new(config);
let placement = session.append("hero".into(), 64, 32)?;
println!(
    "page {}, frame {}, content {:?}",
    placement.page_id(),
    placement.frame_id(),
    placement.content()
);

let snapshot = session.snapshot_atlas()?;
assert_eq!(snapshot.stats().num_frames, 1);
assert!(session.evict_by_key("hero"));
# Ok(())
# }
```

Runtime keys are unique. Each append uses a prepare/commit transaction: validation, placement, identity allocation, and pixel staging complete before live state changes. Failed appends leave allocator state, IDs, metadata, and page pixels unchanged.

## Persistence and Export

`Atlas` does not implement serde. Use the explicit native document boundary:

```rust
# use tex_packer_core::model::Atlas;
use tex_packer_core::model::AtlasDocument;

# fn round_trip(atlas: &Atlas) -> Result<(), Box<dyn std::error::Error>> {
let document = AtlasDocument::from_atlas(atlas);
let json = serde_json::to_string_pretty(&document)?;
let decoded: AtlasDocument = serde_json::from_str(&json)?;
let restored = decoded.try_into_atlas()?;
assert_eq!(&restored, atlas);
# Ok(())
# }
```

The native document schema is version 2 and is described by the [AtlasDocument v2 JSON Schema](https://github.com/Latias94/tex-packer/blob/main/schemas/tex-packer-atlas-document-v2.schema.json). Deserialization rejects unknown fields; `try_into_atlas` additionally validates identities, references, geometry, overlap, and metadata.

Legacy interoperability remains separate in `export`:

- `to_json_array` preserves page and logical frame order.
- `to_json_hash` provides key lookup and last-value behavior for duplicate keys.
- `to_plist_hash` and `to_plist_hash_with_pages` produce TexturePacker-style plist text; their key-addressed frame dictionary is not a lossless duplicate-key identity format.
- `to_template_context` builds the stable Handlebars projection used by the CLI.

Legacy JSON export metadata retains schema version `"1"`; that version is independent from native `AtlasDocument` schema version `2`.

## Statistics

`Atlas::stats` and `PackOutput::stats` distinguish logical and physical facts:

- `num_frames`: logical entries.
- `num_regions`: physical allocations.
- `num_aliases`: frames beyond the unique region count.
- `content_area`: sum of unique region content rectangles.
- `allocation_area`: sum of unique reserved allocations.
- `page_area`: sum of final page areas.
- `content_occupancy` and `allocation_occupancy`: the corresponding area divided by `page_area`.

Aliases are counted once in physical area metrics. Runtime statistics use the same equations and report allocator fragmentation separately.

## Wasm

The core has no filesystem dependency and can target `wasm32-unknown-unknown`. Decode images in the host, pass `DynamicImage` values to an offline workflow, and consume RGBA pages or geometry. The optional `parallel` feature should remain disabled unless the target provides the required threading support.

## Migration

v0.3 intentionally removes the v0.2 public algorithms, `PackerConfig`, free wrappers, prelude, public aggregate fields, generic key axis, and direct `Atlas` serde contract. See the [v0.3 migration guide](https://github.com/Latias94/tex-packer/blob/main/docs/migrations/v0.3.md) for exact replacements.
