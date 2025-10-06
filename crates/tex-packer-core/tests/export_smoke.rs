use tex_packer_core::prelude::*;

#[test]
fn export_json_and_plist_smoke() {
    let cfg = PackerConfig::builder()
        .with_max_dimensions(256, 256)
        .allow_rotation(true)
        .build();
    let items = vec![("a", 32, 16), ("b", 10, 10)];
    let atlas = tex_packer_core::pack_layout(items, cfg).expect("pack");

    // json-array
    let ja = tex_packer_core::to_json_array(&atlas);
    let obj = ja.as_object().expect("object");
    assert!(obj.contains_key("pages"));
    assert!(obj.contains_key("meta"));

    // json-hash
    let jh = tex_packer_core::to_json_hash(&atlas);
    let obj = jh.as_object().expect("object");
    assert!(obj.contains_key("frames"));
    assert!(obj.contains_key("meta"));

    // plist (with filenames)
    let names: Vec<String> = atlas
        .pages
        .iter()
        .map(|p| format!("page_{}.png", p.id))
        .collect();
    let plist = tex_packer_core::to_plist_hash_with_pages(&atlas, &names);
    assert!(plist.contains("<key>frames</key>"));
    assert!(plist.contains("<key>meta</key>"));
    assert!(plist.contains("textureFile")); // textureFileName or textureFileNames
}
