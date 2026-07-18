use serde_json::{Value, json};
use tex_packer_core::prelude::*;

fn representative_atlas() -> Atlas<&'static str> {
    let shared_region = Rect::new(1, 2, 8, 16);

    Atlas {
        pages: vec![
            Page {
                id: 3,
                width: 64,
                height: 32,
                frames: vec![
                    Frame {
                        key: "hero",
                        frame: shared_region,
                        rotated: true,
                        trimmed: true,
                        source: Rect::new(2, 3, 16, 8),
                        source_size: (20, 12),
                    },
                    Frame {
                        key: "hero&alias",
                        frame: shared_region,
                        rotated: true,
                        trimmed: true,
                        source: Rect::new(2, 3, 16, 8),
                        source_size: (20, 12),
                    },
                ],
            },
            Page {
                id: 9,
                width: 32,
                height: 64,
                frames: vec![Frame {
                    key: "enemy",
                    frame: Rect::new(4, 5, 12, 6),
                    rotated: false,
                    trimmed: false,
                    source: Rect::new(0, 0, 12, 6),
                    source_size: (12, 6),
                }],
            },
        ],
        meta: Meta {
            schema_version: "1".into(),
            app: "tex-packer-test".into(),
            version: "0.2.0".into(),
            format: "RGBA8888".into(),
            scale: 1.0,
            power_of_two: false,
            square: false,
            max_dim: (64, 64),
            padding: (1, 2),
            extrude: 1,
            allow_rotation: true,
            trim_mode: "alpha".into(),
            background_color: Some([1, 2, 3, 4]),
        },
    }
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

    let actual = tex_packer_core::to_json_array(&atlas);

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

    let actual = tex_packer_core::to_json_hash(&atlas);

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
fn plist_preserves_frame_and_custom_page_name_order() {
    let atlas = representative_atlas();
    let page_names = vec!["atlas&3.png".to_owned(), "atlas_9.png".to_owned()];

    let plist = tex_packer_core::to_plist_hash_with_pages(&atlas, &page_names);

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

    let plist = tex_packer_core::to_plist_hash(&atlas);

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
