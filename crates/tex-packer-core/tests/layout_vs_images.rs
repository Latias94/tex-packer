use image::{DynamicImage, RgbaImage};
use std::collections::HashMap;
use tex_packer_core::prelude::*;

#[test]
fn layout_and_images_have_same_geometry() {
    // Trimming off to avoid data-dependent changes
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .trim(false)
        .allow_rotation(true)
        .build();

    // Build small set with varied sizes
    let sizes = vec![("a", 40, 20), ("b", 16, 32), ("c", 10, 10), ("d", 8, 48)];
    // layout-only
    let atlas_layout = tex_packer_core::pack_layout(
        sizes.iter().map(|(k, w, h)| (*k, *w, *h)).collect(),
        cfg.clone(),
    )
    .expect("layout");

    // images path
    let mut inputs: Vec<InputImage> = Vec::new();
    for (k, w, h) in &sizes {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(*w, *h));
        inputs.push(InputImage {
            key: (*k).to_string(),
            image: img,
        });
    }
    let out = tex_packer_core::pack_images(inputs, cfg).expect("images");

    // Build key->(page, rect, rotated) maps
    let mut lm: HashMap<String, (usize, Rect, bool)> = HashMap::new();
    for p in &atlas_layout.pages {
        for f in &p.frames {
            lm.insert(f.key.clone(), (p.id, f.frame.clone(), f.rotated));
        }
    }
    let mut im: HashMap<String, (usize, Rect, bool)> = HashMap::new();
    for p in &out.atlas.pages {
        for f in &p.frames {
            im.insert(f.key.clone(), (p.id, f.frame.clone(), f.rotated));
        }
    }

    assert_eq!(lm.len(), im.len());
    for (k, v) in lm {
        let vi = im.get(&k).expect("present");
        // same page id and same frame rectangle + rotation
        assert_eq!(v.0, vi.0, "page id mismatch for key={}", k);
        assert_eq!(v.1, vi.1, "frame rect mismatch for key={}", k);
        assert_eq!(v.2, vi.2, "rotated flag mismatch for key={}", k);
    }
}
