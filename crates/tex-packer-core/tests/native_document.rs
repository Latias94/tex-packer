use serde_json::{Value, json};
use tex_packer_core::error::TexPackerError;
use tex_packer_core::model::{
    Atlas, AtlasDocument, Frame, FrameId, Meta, Page, PageId, Rect, Region, RegionId,
};

fn representative_atlas() -> Atlas {
    let page = Page::try_new(
        PageId::new(17),
        16,
        16,
        vec![Region::new(
            RegionId::new(91),
            Rect::new(2, 2, 3, 2),
            Rect::new(1, 1, 5, 4),
            false,
        )],
        vec![
            Frame::new(
                FrameId::new(400),
                "same-key".to_owned(),
                RegionId::new(91),
                false,
                Rect::new(0, 0, 3, 2),
                (3, 2),
            ),
            Frame::new(
                FrameId::new(7),
                "same-key".to_owned(),
                RegionId::new(91),
                true,
                Rect::new(4, 5, 3, 2),
                (10, 9),
            ),
        ],
    )
    .expect("valid representative page");
    Atlas::try_new(vec![page], Meta::default()).expect("valid representative atlas")
}

fn valid_document_value() -> Value {
    serde_json::to_value(AtlasDocument::from_atlas(&representative_atlas()))
        .expect("serialize document")
}

fn assert_invalid_document(value: Value, expected_reason: &str) {
    let document: AtlasDocument = serde_json::from_value(value).expect("valid wire shape");
    let error = document
        .try_into_atlas()
        .expect_err("invalid document must not enter the model");
    match error {
        TexPackerError::InvalidDocument { reason } => assert!(
            reason.contains(expected_reason),
            "expected {expected_reason:?}, got {reason:?}"
        ),
        other => panic!("expected document error, got {other:?}"),
    }
}

#[test]
fn native_v2_round_trip_rebuilds_indexes_and_preserves_order() {
    let atlas = representative_atlas();
    let document = AtlasDocument::from_atlas(&atlas);
    assert_eq!(document.schema_version(), 2);

    let encoded = serde_json::to_string_pretty(&document).expect("serialize native document");
    let decoded: AtlasDocument = serde_json::from_str(&encoded).expect("deserialize document");
    let restored = decoded.try_into_atlas().expect("validate restored atlas");

    assert_eq!(restored, atlas);
    let page = restored.page(PageId::new(17)).expect("page index rebuilt");
    assert!(restored.page(PageId::new(0)).is_none());
    assert_eq!(
        page.region(RegionId::new(91)).expect("region").content(),
        Rect::new(2, 2, 3, 2)
    );
    assert_eq!(
        page.frame(FrameId::new(7)).expect("frame").key(),
        "same-key"
    );
    assert_eq!(
        page.resolved_frames()
            .map(|resolved| resolved.frame().id().get())
            .collect::<Vec<_>>(),
        [400, 7]
    );
}

#[test]
fn wire_shape_contains_relationships_but_never_runtime_indexes() {
    let value = valid_document_value();
    let root = value.as_object().expect("document object");
    assert_eq!(
        root.keys().map(String::as_str).collect::<Vec<_>>(),
        ["meta", "pages", "schema_version"]
    );
    assert_eq!(value["schema_version"], 2);
    assert!(value["meta"].get("schema_version").is_none());
    assert_eq!(value["meta"]["max_dimensions"], json!([1024, 1024]));

    let page = value["pages"][0].as_object().expect("page object");
    assert_eq!(
        page.keys().map(String::as_str).collect::<Vec<_>>(),
        ["frames", "height", "id", "regions", "width"]
    );
    assert!(page.get("index").is_none());
    assert!(page.get("region_by_id").is_none());
    assert_eq!(value["pages"][0]["regions"][0]["id"], 91);
    assert_eq!(value["pages"][0]["frames"][0]["region_id"], 91);
    assert_eq!(value["pages"][0]["frames"][1]["key"], "same-key");
}

#[test]
fn unsupported_schema_versions_fail_before_aggregate_construction() {
    let mut value = valid_document_value();
    value["schema_version"] = json!(1);
    assert_invalid_document(value, "unsupported schema version 1; expected 2");
}

#[test]
fn unknown_fields_are_rejected_at_every_wire_level() {
    let mut root_unknown = valid_document_value();
    root_unknown["future"] = json!(true);
    assert!(serde_json::from_value::<AtlasDocument>(root_unknown).is_err());

    let mut page_unknown = valid_document_value();
    page_unknown["pages"][0]["future"] = json!(true);
    assert!(serde_json::from_value::<AtlasDocument>(page_unknown).is_err());

    let mut region_unknown = valid_document_value();
    region_unknown["pages"][0]["regions"][0]["future"] = json!(true);
    assert!(serde_json::from_value::<AtlasDocument>(region_unknown).is_err());

    let mut frame_unknown = valid_document_value();
    frame_unknown["pages"][0]["frames"][0]["future"] = json!(true);
    assert!(serde_json::from_value::<AtlasDocument>(frame_unknown).is_err());

    let mut rect_unknown = valid_document_value();
    rect_unknown["pages"][0]["regions"][0]["content"]["future"] = json!(true);
    assert!(serde_json::from_value::<AtlasDocument>(rect_unknown).is_err());

    let mut meta_unknown = valid_document_value();
    meta_unknown["meta"]["future"] = json!(true);
    assert!(serde_json::from_value::<AtlasDocument>(meta_unknown).is_err());
}

#[test]
fn document_loading_validates_metadata_before_accepting_the_atlas() {
    for (field, invalid_value, expected_reason) in [
        ("app", json!("  "), "app must not be empty"),
        ("version", json!(""), "version must not be empty"),
        ("format", json!(""), "format must not be empty"),
        ("trim_mode", json!(""), "trim_mode must not be empty"),
        ("scale", json!(0.0), "scale must be finite and positive"),
        (
            "max_dimensions",
            json!([0, 1024]),
            "maximum dimensions must be positive",
        ),
        (
            "padding",
            json!([u32::MAX, 0]),
            "border padding 4294967295 overflows",
        ),
        ("extrude", json!(u32::MAX), "overflow allocation geometry"),
    ] {
        let mut value = valid_document_value();
        value["meta"][field] = invalid_value;
        assert_invalid_document(value, expected_reason);
    }

    let mut no_usable_area = valid_document_value();
    no_usable_area["meta"]["max_dimensions"] = json!([16, 16]);
    no_usable_area["meta"]["padding"] = json!([8, 0]);
    assert_invalid_document(no_usable_area, "leaves no usable page area");

    let mut impossible_reservation = valid_document_value();
    impossible_reservation["meta"]["max_dimensions"] = json!([16, 16]);
    impossible_reservation["meta"]["padding"] = json!([0, 16]);
    assert_invalid_document(
        impossible_reservation,
        "require at least 17x17 usable pixels",
    );
}

#[test]
fn document_loading_rejects_invalid_identities_and_references() {
    let mut duplicate_page = valid_document_value();
    let cloned_page = duplicate_page["pages"][0].clone();
    duplicate_page["pages"]
        .as_array_mut()
        .expect("pages array")
        .push(cloned_page);
    assert_invalid_document(duplicate_page, "duplicate page identity");

    let mut duplicate_region = valid_document_value();
    let cloned_region = duplicate_region["pages"][0]["regions"][0].clone();
    duplicate_region["pages"][0]["regions"]
        .as_array_mut()
        .expect("regions array")
        .push(cloned_region);
    assert_invalid_document(duplicate_region, "duplicate region identity");

    let mut duplicate_frame = valid_document_value();
    duplicate_frame["pages"][0]["frames"][1]["id"] = json!(400);
    assert_invalid_document(duplicate_frame, "duplicate frame identity");

    let mut dangling_frame = valid_document_value();
    dangling_frame["pages"][0]["frames"][0]["region_id"] = json!(999);
    assert_invalid_document(dangling_frame, "missing region 999");

    let mut orphan_region = valid_document_value();
    orphan_region["pages"][0]["regions"]
        .as_array_mut()
        .expect("regions array")
        .push(json!({
            "id": 92,
            "content": {"x": 9, "y": 1, "w": 1, "h": 1},
            "allocation": {"x": 8, "y": 0, "w": 3, "h": 3},
            "rotated": false
        }));
    assert_invalid_document(orphan_region, "not referenced by any frame");
}

#[test]
fn document_loading_rejects_invalid_physical_and_source_geometry() {
    let mut outside_page = valid_document_value();
    outside_page["pages"][0]["regions"][0]["allocation"] =
        json!({"x": 15, "y": 15, "w": 2, "h": 2});
    outside_page["pages"][0]["regions"][0]["content"] = json!({"x": 15, "y": 15, "w": 1, "h": 1});
    outside_page["pages"][0]["frames"][0]["source"] = json!({"x": 0, "y": 0, "w": 1, "h": 1});
    outside_page["pages"][0]["frames"][1]["source"] = json!({"x": 4, "y": 5, "w": 1, "h": 1});
    assert_invalid_document(outside_page, "inside page bounds");

    let mut overlap = valid_document_value();
    overlap["pages"][0]["regions"]
        .as_array_mut()
        .expect("regions array")
        .push(json!({
            "id": 92,
            "content": {"x": 5, "y": 3, "w": 1, "h": 1},
            "allocation": {"x": 5, "y": 3, "w": 3, "h": 3},
            "rotated": false
        }));
    overlap["pages"][0]["frames"]
        .as_array_mut()
        .expect("frames array")
        .push(json!({
            "id": 8,
            "key": "overlap",
            "region_id": 92,
            "trimmed": false,
            "source": {"x": 0, "y": 0, "w": 1, "h": 1},
            "source_size": [1, 1]
        }));
    assert_invalid_document(overlap, "regions 91 and 92 overlap");

    let mut invalid_source = valid_document_value();
    invalid_source["pages"][0]["frames"][0]["source"] = json!({"x": 2, "y": 0, "w": 3, "h": 2});
    invalid_source["pages"][0]["frames"][0]["source_size"] = json!([4, 2]);
    assert_invalid_document(invalid_source, "inside source size");

    let mut contradictory_rotation = valid_document_value();
    contradictory_rotation["pages"][0]["regions"][0]["rotated"] = json!(true);
    assert_invalid_document(contradictory_rotation, "rotated=true");
}
