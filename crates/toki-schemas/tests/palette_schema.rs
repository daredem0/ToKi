use serde_json::json;

mod common;
use common::{assert_invalid, assert_valid, compile_schema};

fn compile_palette_schema() -> jsonschema::JSONSchema {
    compile_schema(toki_schemas::PALETTE_SCHEMA, "palette")
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
