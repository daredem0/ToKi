use jsonschema::JSONSchema;
use serde_json::{json, Value};

fn compile_object_sheet_schema() -> JSONSchema {
    let schema: Value = serde_json::from_str(toki_schemas::OBJECT_SHEET_SCHEMA)
        .expect("object sheet schema should parse");
    JSONSchema::compile(&schema).expect("object sheet schema should compile")
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
fn object_sheet_schema_accepts_named_placeable_objects() {
    let schema = compile_object_sheet_schema();
    let doc = json!({
        "sheet_type": "objects",
        "image": "fauna.png",
        "tile_size": [16, 16],
        "objects": {
            "fauna_a": {
                "position": [0, 0],
                "size_tiles": [1, 1]
            },
            "fauna_b": {
                "position": [1, 0],
                "size_tiles": [1, 1]
            }
        }
    });

    assert_valid(&schema, &doc);
}

#[test]
fn object_sheet_schema_rejects_invalid_object_entries() {
    let schema = compile_object_sheet_schema();
    let invalid_docs = [
        json!({
            "image": "fauna.png",
            "tile_size": [16, 16],
            "objects": {
                "fauna_a": {
                    "position": [0, 0],
                    "size_tiles": [1, 1]
                }
            }
        }),
        json!({
            "sheet_type": "tiles",
            "image": "fauna.png",
            "tile_size": [16, 16],
            "objects": {
                "fauna_a": {
                    "position": [0, 0],
                    "size_tiles": [1, 1]
                }
            }
        }),
        json!({
            "sheet_type": "objects",
            "image": "",
            "tile_size": [16, 16],
            "objects": {
                "fauna_a": {
                    "position": [0, 0],
                    "size_tiles": [1, 1]
                }
            }
        }),
        json!({
            "sheet_type": "objects",
            "image": "fauna.png",
            "tile_size": [16, 16],
            "objects": {
                "fauna_a": {
                    "position": [0, 0],
                    "size_tiles": [0, 1]
                }
            }
        }),
        json!({
            "sheet_type": "objects",
            "image": "fauna.png",
            "tile_size": [16, 16],
            "objects": {}
        }),
    ];

    for doc in invalid_docs {
        assert_invalid(&schema, &doc);
    }
}
