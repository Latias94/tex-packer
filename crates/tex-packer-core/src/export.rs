use crate::export_manifest::ExportManifest;
use crate::model::Atlas;
use serde::Serialize;
use serde_json::{Value, json};

/// Serialize the whole `Atlas` as a JSON object `{ pages, meta }` (array-of-pages style).
/// Suitable for generic tooling and simple consumption.
pub fn to_json_array<K: ToString + Clone + Serialize>(atlas: &Atlas<K>) -> Value {
    let manifest = ExportManifest::from_atlas(atlas);
    let pages_val = manifest
        .pages
        .iter()
        .map(|page| {
            let frames_val: Vec<Value> = page
                .frames
                .iter()
                .map(|frame| {
                    json!({
                        "key": frame.key,
                        "frame": frame.frame_value(),
                        "rotated": frame.rotated,
                        "trimmed": frame.trimmed,
                        "spriteSourceSize": frame.sprite_source_size_value(),
                        "sourceSize": frame.source_size_value(),
                        "pivot": frame.pivot_value()
                    })
                })
                .collect();
            json!({
                "id": page.id,
                "width": page.width,
                "height": page.height,
                "frames": frames_val,
            })
        })
        .collect::<Vec<_>>();
    json!({"pages": pages_val, "meta": &manifest.meta})
}

/// Flatten frames keyed by name, include page id/size hints.
/// Shape: `{ frames: { name: { frame, rotated, trimmed, spriteSourceSize, sourceSize, pivot, page, pageSize } }, meta }`.
/// Compatible with many engine pipelines expecting TexturePacker-like JSON hash.
pub fn to_json_hash<K: ToString + Clone>(atlas: &Atlas<K>) -> Value {
    let manifest = ExportManifest::from_atlas(atlas);
    let mut frames = serde_json::Map::new();
    for page in &manifest.pages {
        for frame in &page.frames {
            frames.insert(
                frame.key.clone(),
                json!({
                    "frame": frame.frame_value(),
                    "rotated": frame.rotated,
                    "trimmed": frame.trimmed,
                    "spriteSourceSize": frame.sprite_source_size_value(),
                    "sourceSize": frame.source_size_value(),
                    "pivot": frame.pivot_value(),
                    "page": frame.page,
                    "pageSize": frame.page_size_value(),
                }),
            );
        }
    }
    json!({ "frames": frames, "meta": &manifest.meta })
}
