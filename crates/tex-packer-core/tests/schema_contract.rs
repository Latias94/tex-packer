use jsonschema::Validator;
use serde_json::{Value, json};
use tex_packer_core::model::{
    Atlas, AtlasDocument, Frame, FrameId, Meta, Page, PageId, Rect, Region, RegionId,
};

const ATLAS_DOCUMENT_V2_SCHEMA: &str =
    include_str!("../../../schemas/tex-packer-atlas-document-v2.schema.json");

fn schema_validator() -> Validator {
    let schema: Value =
        serde_json::from_str(ATLAS_DOCUMENT_V2_SCHEMA).expect("parse AtlasDocument v2 schema");
    jsonschema::draft7::meta::validate(&schema).expect("schema must conform to Draft 7");
    jsonschema::draft7::new(&schema).expect("compile AtlasDocument v2 schema")
}

fn representative_document_value() -> Value {
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
        vec![Frame::new(
            FrameId::new(400),
            "sprite".to_owned(),
            RegionId::new(91),
            false,
            Rect::new(0, 0, 3, 2),
            (3, 2),
        )],
    )
    .expect("valid representative page");
    let atlas = Atlas::try_new(vec![page], Meta::default()).expect("valid representative atlas");

    serde_json::to_value(AtlasDocument::from_atlas(&atlas)).expect("serialize AtlasDocument")
}

fn assert_schema_rejects(validator: &Validator, value: &Value, case: &str) {
    let errors = validator
        .iter_errors(value)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    assert!(!errors.is_empty(), "schema accepted {case}");
}

#[test]
fn draft7_schema_accepts_the_real_atlas_document_serializer() {
    let validator = schema_validator();
    let value = representative_document_value();

    validator.validate(&value).unwrap_or_else(|error| {
        panic!("serialized AtlasDocument must satisfy the schema: {error}")
    });
}

#[test]
fn draft7_schema_rejects_incompatible_wire_shapes() {
    let validator = schema_validator();
    let valid = representative_document_value();

    let mut wrong_version = valid.clone();
    wrong_version["schema_version"] = json!(1);
    assert_schema_rejects(&validator, &wrong_version, "schema version 1");

    let mut unknown_field = valid.clone();
    unknown_field["pages"][0]["regions"][0]["future"] = json!(true);
    assert_schema_rejects(&validator, &unknown_field, "an unknown nested field");

    let mut malformed_nested_shape = valid;
    malformed_nested_shape["pages"][0]["regions"][0]["content"] = json!({"x": 2, "y": 2, "w": 3});
    assert_schema_rejects(
        &validator,
        &malformed_nested_shape,
        "a nested rectangle without height",
    );
}
