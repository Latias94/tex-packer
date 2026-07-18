use image::{Rgba, RgbaImage};
use tex_packer_core::TransparentPolicy;
use tex_packer_core::prelude::*;

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

fn transparent_input(key: &str) -> InputImage {
    InputImage {
        key: key.into(),
        image: image::DynamicImage::ImageRgba8(RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 0]))),
    }
}

fn skip_transparent_config() -> PackerConfig {
    PackerConfig::builder()
        .with_max_dimensions(64, 64)
        .trim(true)
        .transparent_policy(TransparentPolicy::Skip)
        .build()
}

#[test]
fn all_skipped_images_currently_produce_an_empty_output() {
    let out = tex_packer_core::pack_images(
        vec![
            transparent_input("first.png"),
            transparent_input("second.png"),
        ],
        skip_transparent_config(),
    )
    .expect("v0.2 returns an empty output after all inputs are skipped");

    assert!(out.atlas.pages.is_empty());
    assert!(out.pages.is_empty());
}

#[test]
#[ignore = "U4: report no packable inputs after TransparentPolicy::Skip"]
fn all_skipped_images_are_rejected_with_input_context() {
    let result = tex_packer_core::pack_images(
        vec![
            transparent_input("first.png"),
            transparent_input("second.png"),
        ],
        skip_transparent_config(),
    );

    match result {
        Ok(_) => panic!("all-skipped input should be rejected"),
        Err(err) => {
            let message = err.to_string();
            assert!(message.contains("first.png") || message.contains("second.png"));
        }
    }
}
