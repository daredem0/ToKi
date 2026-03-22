use jsonschema::JSONSchema;
use serde_json::{json, Value};

fn compile_palette_schema() -> JSONSchema {
    let schema: Value =
        serde_json::from_str(toki_schemas::PALETTE_SCHEMA).expect("palette schema should parse");
    JSONSchema::compile(&schema).expect("palette schema should compile")
}

fn assert_valid(schema: &JSONSchema, doc: &Value) {
    if let Err(errors) = schema.validate(doc) {
        let details = errors.map(|error| error.to_string()).collect::<Vec<_>>();
        panic!(
            "expected schema-valid document, got: {}",
            details.join(" | ")
        );
    }
}

fn assert_invalid(schema: &JSONSchema, doc: &Value) {
    assert!(
        schema.validate(doc).is_err(),
        "expected schema-invalid document"
    );
}

#[test]
fn palette_schema_accepts_four_rgba_colors() {
    let schema = compile_palette_schema();
    let doc = json!({
        "colors": [
            [15, 56, 15, 255],
            [48, 98, 48, 255],
            [139, 172, 15, 255],
            [155, 188, 15, 255]
        ]
    });

    assert_valid(&schema, &doc);
}

#[test]
fn palette_schema_rejects_wrong_palette_length() {
    let schema = compile_palette_schema();
    let doc = json!({
        "colors": [
            [15, 56, 15, 255],
            [48, 98, 48, 255],
            [139, 172, 15, 255]
        ]
    });

    assert_invalid(&schema, &doc);
}

#[test]
fn palette_schema_rejects_color_channels_out_of_range() {
    let schema = compile_palette_schema();
    let doc = json!({
        "colors": [
            [15, 56, 15, 255],
            [48, 98, 48, 255],
            [139, 172, 15, 255],
            [155, 188, 15, 300]
        ]
    });

    assert_invalid(&schema, &doc);
}
