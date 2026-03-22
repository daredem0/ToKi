use jsonschema::JSONSchema;
use serde_json::{json, Value};

fn compile_atlas_schema() -> JSONSchema {
    let schema: Value =
        serde_json::from_str(toki_schemas::ATLAS_SCHEMA).expect("atlas schema should parse");
    JSONSchema::compile(&schema).expect("atlas schema should compile")
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
fn atlas_schema_accepts_legacy_truecolor_documents_without_palette_fields() {
    let schema = compile_atlas_schema();
    let doc = json!({
        "image": "players.png",
        "tile_size": [16, 16],
        "tiles": {
            "idle": {
                "position": [0, 0],
                "properties": {
                    "solid": false,
                    "trigger": false
                }
            }
        }
    });

    assert_valid(&schema, &doc);
}

#[test]
fn atlas_schema_accepts_palette_indexed_documents_with_palette_id() {
    let schema = compile_atlas_schema();
    let doc = json!({
        "image": "players.png",
        "tile_size": [16, 16],
        "color_mode": "palette_indexed",
        "palette": "gb_default",
        "tiles": {
            "idle": {
                "position": [0, 0],
                "properties": {
                    "solid": false,
                    "trigger": false
                }
            }
        }
    });

    assert_valid(&schema, &doc);
}

#[test]
fn atlas_schema_rejects_unknown_color_mode() {
    let schema = compile_atlas_schema();
    let doc = json!({
        "image": "players.png",
        "tile_size": [16, 16],
        "color_mode": "indexed",
        "tiles": {
            "idle": {
                "position": [0, 0],
                "properties": {
                    "solid": false,
                    "trigger": false
                }
            }
        }
    });

    assert_invalid(&schema, &doc);
}
