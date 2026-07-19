use image::{DynamicImage, Rgba, RgbaImage};
use tex_packer_core::config::{OfflineConfig, PageConfig, RuntimeConfig};
use tex_packer_core::export::{
    to_json_array, to_json_hash, to_plist_hash, to_plist_hash_with_pages, to_template_context,
};
use tex_packer_core::model::{AtlasDocument, PageId};
use tex_packer_core::offline::{InputImage, LayoutItem, OfflinePacker};
use tex_packer_core::runtime::{AtlasSession, RuntimeAtlas};

fn decoded_input(key: &str, color: [u8; 4]) -> InputImage {
    InputImage {
        key: key.to_owned(),
        image: DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 3, Rgba(color))),
    }
}

#[test]
fn curated_public_api_supports_every_promised_workflow() {
    let page_config = PageConfig::builder()
        .max_dimensions(32, 32)
        .build()
        .expect("valid page config");
    let offline_config = OfflineConfig::builder()
        .page_config(page_config.clone())
        .build()
        .expect("valid offline config");
    let offline = OfflinePacker::new(offline_config);

    let rendered = offline
        .pack_images(vec![decoded_input("hero", [255, 0, 0, 255])])
        .expect("decoded image render");
    let decoded_layout = offline
        .layout_images(vec![decoded_input("hero", [255, 0, 0, 255])])
        .expect("decoded image layout");
    assert_eq!(rendered.atlas(), &decoded_layout);
    assert_eq!(rendered.pages().len(), 1);
    let rendered_page = &rendered.pages()[0];
    let atlas_page = rendered
        .atlas()
        .page(rendered_page.page_id())
        .expect("rendered page identity resolves");
    assert_eq!(rendered_page.rgba().dimensions(), atlas_page.size());

    let pure_layout = offline
        .pack_layout(vec![LayoutItem {
            key: "button".into(),
            w: 6,
            h: 5,
            source: None,
            source_size: None,
            trimmed: false,
        }])
        .expect("pure layout");
    assert_eq!(pure_layout.stats().num_frames, 1);

    let runtime_config = RuntimeConfig::builder()
        .page_config(page_config)
        .build()
        .expect("valid runtime config");
    let mut session = AtlasSession::new(runtime_config.clone());
    let placement = session
        .append("runtime-layout".into(), 5, 4)
        .expect("runtime layout append");
    assert_eq!(placement.page_id(), PageId::new(0));
    let runtime_snapshot = session.snapshot_atlas().expect("runtime snapshot");
    assert_eq!(runtime_snapshot.stats().num_frames, 1);

    let mut runtime_atlas = RuntimeAtlas::new(runtime_config);
    let image = RgbaImage::from_pixel(5, 4, Rgba([0, 255, 0, 255]));
    let update = runtime_atlas
        .append_with_image("runtime-image".into(), &image)
        .expect("runtime image append");
    assert!(
        runtime_atlas
            .get_page_image(update.placement().page_id())
            .is_some()
    );
    assert_eq!(
        runtime_atlas
            .snapshot_atlas()
            .expect("runtime image snapshot")
            .stats()
            .num_frames,
        1
    );

    let document = AtlasDocument::from_atlas(rendered.atlas());
    let encoded = serde_json::to_string(&document).expect("serialize native document");
    assert!(encoded.contains("\"schema_version\":2"));
    let decoded: AtlasDocument = serde_json::from_str(&encoded).expect("parse native document");
    assert_eq!(
        decoded.try_into_atlas().expect("validate native document"),
        *rendered.atlas()
    );

    let page_names = vec!["atlas.png".to_owned()];
    assert_eq!(
        to_json_array(rendered.atlas())["pages"][0]["frames"][0]["key"],
        "hero"
    );
    assert!(to_json_hash(rendered.atlas())["frames"]["hero"].is_object());
    assert!(to_plist_hash(rendered.atlas()).contains("<key>hero</key>"));
    assert!(to_plist_hash_with_pages(rendered.atlas(), &page_names).contains("atlas.png"));
    assert_eq!(
        to_template_context(rendered.atlas(), &page_names).pages[0].sprites[0].name,
        "hero"
    );
}
