use serde_json::{Value, json};
use tex_packer_core::export::{
    to_json_array, to_json_hash, to_plist_hash, to_plist_hash_with_pages, to_template_context,
};
use tex_packer_core::model::{Atlas, AtlasDocument};

fn representative_atlas() -> Atlas {
    let document: AtlasDocument = serde_json::from_value(json!({
        "schema_version": 2,
        "meta": {
            "app": "tex-packer-test",
            "version": "0.2.0",
            "format": "RGBA8888",
            "scale": 1.0,
            "power_of_two": false,
            "square": false,
            "max_dimensions": [64, 64],
            "padding": [1, 2],
            "extrude": 1,
            "allow_rotation": true,
            "trim_mode": "alpha",
            "background_color": [1, 2, 3, 4]
        },
        "pages": [
            {
                "id": 3,
                "width": 64,
                "height": 32,
                "regions": [{
                    "id": 7,
                    "content": {"x": 1, "y": 2, "w": 8, "h": 16},
                    "allocation": {"x": 1, "y": 2, "w": 8, "h": 16},
                    "rotated": true
                }],
                "frames": [
                    {
                        "id": 11,
                        "key": "hero",
                        "region_id": 7,
                        "trimmed": true,
                        "source": {"x": 2, "y": 3, "w": 16, "h": 8},
                        "source_size": [20, 12]
                    },
                    {
                        "id": 12,
                        "key": "hero&alias",
                        "region_id": 7,
                        "trimmed": true,
                        "source": {"x": 2, "y": 3, "w": 16, "h": 8},
                        "source_size": [20, 12]
                    }
                ]
            },
            {
                "id": 9,
                "width": 32,
                "height": 64,
                "regions": [{
                    "id": 4,
                    "content": {"x": 4, "y": 5, "w": 12, "h": 6},
                    "allocation": {"x": 4, "y": 5, "w": 12, "h": 6},
                    "rotated": false
                }],
                "frames": [{
                    "id": 2,
                    "key": "enemy",
                    "region_id": 4,
                    "trimmed": false,
                    "source": {"x": 0, "y": 0, "w": 12, "h": 6},
                    "source_size": [12, 6]
                }]
            }
        ]
    }))
    .expect("representative native atlas document should deserialize");

    document
        .try_into_atlas()
        .expect("representative native atlas document should be valid")
}

fn atlas_with_duplicate_keys() -> Atlas {
    let mut document_value =
        serde_json::to_value(AtlasDocument::from_atlas(&representative_atlas()))
            .expect("native atlas document should serialize");
    let first_page = document_value["pages"][0]
        .as_object_mut()
        .expect("first page should be an object");
    first_page["regions"]
        .as_array_mut()
        .expect("regions should be an array")
        .push(json!({
            "id": 8,
            "content": {"x": 20, "y": 4, "w": 5, "h": 7},
            "allocation": {"x": 20, "y": 4, "w": 5, "h": 7},
            "rotated": false
        }));
    let alias = first_page["frames"]
        .as_array_mut()
        .expect("frames should be an array")
        .get_mut(1)
        .expect("representative atlas should contain an alias");
    alias["key"] = json!("hero");
    alias["region_id"] = json!(8);
    alias["trimmed"] = json!(false);
    alias["source"] = json!({"x": 0, "y": 0, "w": 5, "h": 7});
    alias["source_size"] = json!([5, 7]);

    serde_json::from_value::<AtlasDocument>(document_value)
        .expect("duplicate-key native atlas document should deserialize")
        .try_into_atlas()
        .expect("duplicate user keys should remain valid logical identities")
}

fn expected_meta() -> Value {
    json!({
        "schema_version": "1",
        "app": "tex-packer-test",
        "version": "0.2.0",
        "format": "RGBA8888",
        "scale": 1.0,
        "power_of_two": false,
        "square": false,
        "max_dim": [64, 64],
        "padding": [1, 2],
        "extrude": 1,
        "allow_rotation": true,
        "trim_mode": "alpha",
        "background_color": [1, 2, 3, 4],
    })
}

fn assert_fragments_in_order(document: &str, fragments: &[&str]) {
    let mut offset = 0;

    for fragment in fragments {
        let relative = document[offset..]
            .find(fragment)
            .unwrap_or_else(|| panic!("missing ordered fragment: {fragment:?}"));
        offset += relative + fragment.len();
    }
}

fn plist_frame_dict<'a>(plist: &'a str, escaped_key: &str) -> &'a str {
    let marker = format!("    <key>{escaped_key}</key>\n    <dict>\n");
    let start = plist
        .find(&marker)
        .unwrap_or_else(|| panic!("missing plist frame: {escaped_key}"))
        + marker.len();
    let remainder = &plist[start..];
    let end = remainder
        .find("    </dict>\n")
        .unwrap_or_else(|| panic!("unterminated plist frame: {escaped_key}"));
    &remainder[..end]
}

#[test]
fn json_array_preserves_page_and_logical_frame_order() {
    let atlas = representative_atlas();

    let actual = to_json_array(&atlas);

    assert_eq!(
        actual,
        json!({
            "pages": [
                {
                    "id": 3,
                    "width": 64,
                    "height": 32,
                    "frames": [
                        {
                            "key": "hero",
                            "frame": {"x": 1, "y": 2, "w": 8, "h": 16},
                            "rotated": true,
                            "trimmed": true,
                            "spriteSourceSize": {"x": 2, "y": 3, "w": 16, "h": 8},
                            "sourceSize": {"w": 20, "h": 12},
                            "pivot": {"x": 0.5, "y": 0.5},
                        },
                        {
                            "key": "hero&alias",
                            "frame": {"x": 1, "y": 2, "w": 8, "h": 16},
                            "rotated": true,
                            "trimmed": true,
                            "spriteSourceSize": {"x": 2, "y": 3, "w": 16, "h": 8},
                            "sourceSize": {"w": 20, "h": 12},
                            "pivot": {"x": 0.5, "y": 0.5},
                        },
                    ],
                },
                {
                    "id": 9,
                    "width": 32,
                    "height": 64,
                    "frames": [{
                        "key": "enemy",
                        "frame": {"x": 4, "y": 5, "w": 12, "h": 6},
                        "rotated": false,
                        "trimmed": false,
                        "spriteSourceSize": {"x": 0, "y": 0, "w": 12, "h": 6},
                        "sourceSize": {"w": 12, "h": 6},
                        "pivot": {"x": 0.5, "y": 0.5},
                    }],
                },
            ],
            "meta": expected_meta(),
        })
    );
}

#[test]
fn json_hash_preserves_alias_geometry_and_page_metadata() {
    let atlas = representative_atlas();

    let actual = to_json_hash(&atlas);

    assert_eq!(
        actual,
        json!({
            "frames": {
                "hero": {
                    "frame": {"x": 1, "y": 2, "w": 8, "h": 16},
                    "rotated": true,
                    "trimmed": true,
                    "spriteSourceSize": {"x": 2, "y": 3, "w": 16, "h": 8},
                    "sourceSize": {"w": 20, "h": 12},
                    "pivot": {"x": 0.5, "y": 0.5},
                    "page": 3,
                    "pageSize": {"w": 64, "h": 32},
                },
                "hero&alias": {
                    "frame": {"x": 1, "y": 2, "w": 8, "h": 16},
                    "rotated": true,
                    "trimmed": true,
                    "spriteSourceSize": {"x": 2, "y": 3, "w": 16, "h": 8},
                    "sourceSize": {"w": 20, "h": 12},
                    "pivot": {"x": 0.5, "y": 0.5},
                    "page": 3,
                    "pageSize": {"w": 64, "h": 32},
                },
                "enemy": {
                    "frame": {"x": 4, "y": 5, "w": 12, "h": 6},
                    "rotated": false,
                    "trimmed": false,
                    "spriteSourceSize": {"x": 0, "y": 0, "w": 12, "h": 6},
                    "sourceSize": {"w": 12, "h": 6},
                    "pivot": {"x": 0.5, "y": 0.5},
                    "page": 9,
                    "pageSize": {"w": 32, "h": 64},
                },
            },
            "meta": expected_meta(),
        })
    );

    let frame_keys = actual["frames"]
        .as_object()
        .expect("frames object")
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(frame_keys, ["enemy", "hero", "hero&alias"]);
}

#[test]
fn duplicate_keys_are_preserved_in_arrays_and_later_values_win_in_hashes() {
    let atlas = atlas_with_duplicate_keys();

    let array = to_json_array(&atlas);
    let duplicate_frames = array["pages"][0]["frames"]
        .as_array()
        .expect("array export should contain logical frames");
    assert_eq!(duplicate_frames[0]["key"], "hero");
    assert_eq!(duplicate_frames[1]["key"], "hero");
    assert_eq!(duplicate_frames[0]["frame"]["x"], 1);
    assert_eq!(duplicate_frames[1]["frame"]["x"], 20);

    let hash = to_json_hash(&atlas);
    assert_eq!(hash["frames"]["hero"]["frame"]["x"], 20);
    assert_eq!(
        hash["frames"]["hero"]["sourceSize"],
        json!({"w": 5, "h": 7})
    );
    assert_eq!(
        hash["frames"]
            .as_object()
            .expect("hash export should contain a frame object")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["enemy", "hero"]
    );
}

#[test]
fn template_context_preserves_page_sprite_and_meta_shapes() {
    let atlas = representative_atlas();
    let page_names = vec!["atlas_3.png".to_owned(), "atlas_9.png".to_owned()];

    let context = to_template_context(&atlas, &page_names);
    let actual = serde_json::to_value(context).expect("template context should serialize");

    assert_eq!(
        actual,
        json!({
            "pages": [
                {
                    "image": "atlas_3.png",
                    "size": {"w": 64, "h": 32},
                    "sprites": [
                        {
                            "name": "hero",
                            "frame": {"x": 1, "y": 2, "w": 8, "h": 16},
                            "rotated": true,
                            "trimmed": true,
                            "sprite_source_size": {"x": 2, "y": 3, "w": 16, "h": 8},
                            "source_size": {"w": 20, "h": 12},
                            "pivot": {"x": 0.5, "y": 0.5}
                        },
                        {
                            "name": "hero&alias",
                            "frame": {"x": 1, "y": 2, "w": 8, "h": 16},
                            "rotated": true,
                            "trimmed": true,
                            "sprite_source_size": {"x": 2, "y": 3, "w": 16, "h": 8},
                            "source_size": {"w": 20, "h": 12},
                            "pivot": {"x": 0.5, "y": 0.5}
                        }
                    ]
                },
                {
                    "image": "atlas_9.png",
                    "size": {"w": 32, "h": 64},
                    "sprites": [{
                        "name": "enemy",
                        "frame": {"x": 4, "y": 5, "w": 12, "h": 6},
                        "rotated": false,
                        "trimmed": false,
                        "sprite_source_size": {"x": 0, "y": 0, "w": 12, "h": 6},
                        "source_size": {"w": 12, "h": 6},
                        "pivot": {"x": 0.5, "y": 0.5}
                    }]
                }
            ],
            "meta": {
                "app": "tex-packer-test",
                "version": "0.2.0",
                "format": "RGBA8888",
                "scale": 1.0
            }
        })
    );
}

#[test]
fn plist_preserves_frame_and_custom_page_name_order() {
    let atlas = representative_atlas();
    let page_names = vec!["atlas&3.png".to_owned(), "atlas_9.png".to_owned()];

    let plist = to_plist_hash_with_pages(&atlas, &page_names);

    assert_fragments_in_order(
        &plist,
        &[
            "<key>frames</key>",
            "<key>hero</key>",
            "<key>hero&amp;alias</key>",
            "<key>enemy</key>",
            "<key>meta</key>",
            "<key>textureFileNames</key><array>",
            "<string>atlas&amp;3.png</string>",
            "<string>atlas_9.png</string>",
            "</array>",
            "<key>app</key><string>tex-packer-test</string>",
            "<key>version</key><string>0.2.0</string>",
        ],
    );

    for key in ["hero", "hero&amp;alias"] {
        let frame = plist_frame_dict(&plist, key);
        assert_fragments_in_order(
            frame,
            &[
                "<key>page</key><integer>3</integer>",
                "<key>pageSize</key><string>{64, 32}</string>",
                "<key>frame</key><string>{{1,2},{8,16}}</string>",
                "<key>rotated</key><true />",
                "<key>trimmed</key><true />",
                "<key>spriteSourceSize</key><string>{{2,3},{16,8}}</string>",
                "<key>sourceSize</key><string>{20, 12}</string>",
                "<key>pivot</key><string>{0.50, 0.50}</string>",
            ],
        );
    }

    let enemy = plist_frame_dict(&plist, "enemy");
    assert_fragments_in_order(
        enemy,
        &[
            "<key>page</key><integer>9</integer>",
            "<key>pageSize</key><string>{32, 64}</string>",
            "<key>frame</key><string>{{4,5},{12,6}}</string>",
            "<key>rotated</key><false />",
            "<key>trimmed</key><false />",
            "<key>spriteSourceSize</key><string>{{0,0},{12,6}}</string>",
            "<key>sourceSize</key><string>{12, 6}</string>",
        ],
    );

    assert!(!plist.contains("schema_version"));
    assert!(!plist.contains("<key>pages</key>"));
}

#[test]
fn plist_without_page_names_preserves_multi_page_size_order() {
    let atlas = representative_atlas();

    let plist = to_plist_hash(&atlas);

    assert_fragments_in_order(
        &plist,
        &[
            "<key>pages</key><array>",
            "<string>{64, 32}</string>",
            "<string>{32, 64}</string>",
            "</array>",
        ],
    );
    assert!(!plist.contains("textureFileName"));
}
