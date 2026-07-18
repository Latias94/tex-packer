use image::{Rgba, RgbaImage};
use tex_packer_core::config::{OfflineConfig, PageConfig, TransparentPolicy};
use tex_packer_core::error::TexPackerError;
use tex_packer_core::offline::{InputImage, OfflinePacker};

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

    let cfg = OfflineConfig::builder()
        .page_config(page_config())
        .trim(true)
        .transparent_policy(TransparentPolicy::OneByOne)
        .build()
        .expect("valid offline config");

    let packer = OfflinePacker::new(cfg);
    let out = packer.pack_images(inputs).expect("pack");
    let layout = packer
        .layout_images(vec![transparent_input("t.png")])
        .expect("layout");
    assert_eq!(out.atlas(), &layout);
    assert_eq!(out.atlas().pages().len(), 1);
    let resolved = out.atlas().pages()[0]
        .resolved_frames()
        .next()
        .expect("transparent frame");
    assert_eq!(resolved.region().content().w, 1);
    assert_eq!(resolved.region().content().h, 1);
}

fn transparent_input(key: &str) -> InputImage {
    InputImage {
        key: key.into(),
        image: image::DynamicImage::ImageRgba8(RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 0]))),
    }
}

fn page_config() -> PageConfig {
    PageConfig::builder()
        .max_dimensions(64, 64)
        .build()
        .expect("valid page config")
}

fn skip_transparent_config() -> OfflineConfig {
    OfflineConfig::builder()
        .page_config(page_config())
        .trim(true)
        .transparent_policy(TransparentPolicy::Skip)
        .build()
        .expect("valid offline config")
}

#[test]
fn all_skipped_images_are_rejected_with_input_context() {
    let packer = OfflinePacker::new(skip_transparent_config());
    for result in [
        packer
            .pack_images(vec![
                transparent_input("first.png"),
                transparent_input("second.png"),
            ])
            .map(|_| ()),
        packer
            .layout_images(vec![
                transparent_input("first.png"),
                transparent_input("second.png"),
            ])
            .map(|_| ()),
    ] {
        match result {
            Err(TexPackerError::NoPackableInputs { keys }) => {
                assert_eq!(keys, ["first.png", "second.png"]);
            }
            Ok(()) => panic!("all-skipped input should be rejected"),
            Err(err) => panic!("expected NoPackableInputs, got {err:?}"),
        }
    }
}

#[test]
fn skip_policy_has_render_and_layout_parity_for_mixed_inputs() {
    let packer = OfflinePacker::new(skip_transparent_config());
    let inputs = || {
        vec![
            transparent_input("skipped.png"),
            InputImage {
                key: "visible.png".into(),
                image: image::DynamicImage::ImageRgba8(RgbaImage::from_pixel(
                    3,
                    2,
                    Rgba([10, 20, 30, 255]),
                )),
            },
        ]
    };

    let rendered = packer.pack_images(inputs()).expect("rendered pack");
    let layout = packer.layout_images(inputs()).expect("metadata-only pack");

    assert_eq!(rendered.atlas(), &layout);
    let frame = layout.pages()[0].frames().first().expect("visible frame");
    assert_eq!(frame.key(), "visible.png");
    assert_eq!(layout.stats().num_frames, 1);
}
