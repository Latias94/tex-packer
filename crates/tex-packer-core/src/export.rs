use crate::model::Atlas;
use serde::Serialize;
use serde_json::{json, Value};

/// Serialize the whole `Atlas` as a JSON object `{ pages, meta }` (array-of-pages style).
/// Suitable for generic tooling and simple consumption.
pub fn to_json_array<K: ToString + Clone + Serialize>(atlas: &Atlas<K>) -> Value {
    // Build array-of-pages with per-frame fields using camelCase for source metadata,
    // consistent with the hash schema naming.
    let pages_val = atlas
        .pages
        .iter()
        .map(|p| {
            let frames_val: Vec<Value> = p
                .frames
                .iter()
                .map(|fr| {
                    let frame = json!({"x": fr.frame.x, "y": fr.frame.y, "w": fr.frame.w, "h": fr.frame.h});
                    let sprite_source_size = json!({"x": fr.source.x, "y": fr.source.y, "w": fr.source.w, "h": fr.source.h});
                    let source_size = json!({"w": fr.source_size.0, "h": fr.source_size.1});
                    let pivot = json!({"x": 0.5, "y": 0.5});
                    json!({
                        "key": fr.key.to_string(),
                        "frame": frame,
                        "rotated": fr.rotated,
                        "trimmed": fr.trimmed,
                        "spriteSourceSize": sprite_source_size,
                        "sourceSize": source_size,
                        "pivot": pivot
                    })
                })
                .collect();
            json!({
                "id": p.id,
                "width": p.width,
                "height": p.height,
                "frames": frames_val,
            })
        })
        .collect::<Vec<_>>();
    json!({"pages": pages_val, "meta": &atlas.meta})
}

/// Flatten frames keyed by name, include page id/size hints.
/// Shape: `{ frames: { name: { frame, rotated, trimmed, spriteSourceSize, sourceSize, pivot, page, pageSize } }, meta }`.
/// Compatible with many engine pipelines expecting TexturePacker-like JSON hash.
pub fn to_json_hash<K: ToString + Clone>(atlas: &Atlas<K>) -> Value {
    // Flatten frames keyed by name, include page info
    let mut frames = serde_json::Map::new();
    for page in &atlas.pages {
        for fr in &page.frames {
            let key = fr.key.to_string();
            let frame = json!({"x": fr.frame.x, "y": fr.frame.y, "w": fr.frame.w, "h": fr.frame.h});
            let sprite_source_size =
                json!({"x": fr.source.x, "y": fr.source.y, "w": fr.source.w, "h": fr.source.h});
            let source_size = json!({"w": fr.source_size.0, "h": fr.source_size.1});
            let pivot = json!({"x": 0.5, "y": 0.5});
            frames.insert(
                key,
                json!({
                    "frame": frame,
                    "rotated": fr.rotated,
                    "trimmed": fr.trimmed,
                    "spriteSourceSize": sprite_source_size,
                    "sourceSize": source_size,
                    "pivot": pivot,
                    "page": page.id,
                    "pageSize": {"w": page.width, "h": page.height},
                }),
            );
        }
    }
    json!({ "frames": frames, "meta": &atlas.meta })
}
