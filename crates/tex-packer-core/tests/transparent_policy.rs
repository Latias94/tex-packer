use image::{Rgba, RgbaImage};
use tex_packer_core::prelude::*;
use tex_packer_core::TransparentPolicy;

#[test]
fn test_transparent_one_by_one() {
    // Build a fully transparent image
    let mut img = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 0]));
    // Ensure it's truly transparent (default is already transparent)
    img.put_pixel(0, 0, Rgba([0, 0, 0, 0]));

    let inputs = vec![InputImage {
        key: "t.png".into(),
        image: image::DynamicImage::ImageRgba8(img),
    }];

    let cfg = PackerConfig::builder()
        .with_max_dimensions(64, 64)
        .trim(true)
        .transparent_policy(TransparentPolicy::OneByOne)
        .build();

    let out = tex_packer_core::pack_images(inputs, cfg).expect("pack");
    assert_eq!(out.atlas.pages.len(), 1);
    let f = &out.atlas.pages[0].frames[0];
    assert_eq!(f.frame.w, 1);
    assert_eq!(f.frame.h, 1);
}
