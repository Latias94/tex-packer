use tex_packer_core::prelude::*;

fn reserved_slot(f: &Rect, cfg: &PageConfig) -> Rect {
    let pad_half = cfg.texture_padding() / 2;
    let off = cfg.texture_extrusion() + pad_half;
    Rect::new(
        f.x.saturating_sub(off),
        f.y.saturating_sub(off),
        f.w + cfg.texture_extrusion() * 2 + cfg.texture_padding(),
        f.h + cfg.texture_extrusion() * 2 + cfg.texture_padding(),
    )
}

#[test]
fn force_max_ignores_pow2_and_square() {
    let page = PageConfig::builder()
        .max_dimensions(300, 180)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .force_max_dimensions(true)
        .power_of_two(true)
        .square(true)
        .build()
        .expect("valid offline config");
    let inputs = vec![("a", 10, 10)];
    let atlas = tex_packer_core::pack_layout(inputs, cfg).expect("pack");
    let p = &atlas.pages[0];
    assert_eq!(p.width, 300);
    assert_eq!(p.height, 180);
}

#[test]
fn border_padding_is_respected_in_pack_images() {
    // Use RGBA path to validate composition path, with non-zero border/padding/extrude
    let page = PageConfig::builder()
        .max_dimensions(256, 256)
        .border_padding(8)
        .texture_padding(4)
        .texture_extrusion(2)
        .build()
        .expect("valid page config");
    let cfg = OfflineConfig::builder()
        .page_config(page)
        .build()
        .expect("valid offline config");
    let mut inputs: Vec<InputImage> = Vec::new();
    for i in 0..4u32 {
        let img = image::DynamicImage::ImageRgba8(image::RgbaImage::new(32, 16));
        inputs.push(InputImage {
            key: format!("t{}", i),
            image: img,
        });
    }
    let out = tex_packer_core::pack_images(inputs, cfg.clone()).expect("pack");
    let page_config = cfg.page_config();
    for page in &out.atlas.pages {
        // Logical border rectangle
        let border_rect = Rect::new(
            page_config.border_padding(),
            page_config.border_padding(),
            page_config.max_width() - page_config.border_padding() * 2,
            page_config.max_height() - page_config.border_padding() * 2,
        );
        for f in &page.frames {
            let slot = reserved_slot(&f.frame, page_config);
            assert!(
                border_rect.contains(&slot),
                "reserved slot must stay within border: border={:?} slot={:?}",
                border_rect,
                slot
            );
        }
    }
}
