use jsonschema::JSONSchema;
use serde_json::{json, Value};

fn compile_map_schema() -> JSONSchema {
    let schema: Value =
        serde_json::from_str(toki_schemas::MAP_SCHEMA).expect("map schema should parse");
    JSONSchema::compile(&schema).expect("map schema should compile")
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
fn map_schema_accepts_map_objects_with_visibility_and_solidity() {
    let schema = compile_map_schema();
    let doc = json!({
        "size": [2, 2],
        "tile_size": [16, 16],
        "atlas": "terrain.json",
        "tiles": ["grass", "grass", "grass", "grass"],
        "objects": [
            {
                "sheet": "fauna.json",
                "object_name": "bush",
                "position": [16, 32],
                "size_px": [16, 16],
                "visible": false,
                "solid": true
            }
        ]
    });

    assert_valid(&schema, &doc);
}

#[test]
fn map_schema_rejects_invalid_map_object_size() {
    let schema = compile_map_schema();
    let doc = json!({
        "size": [1, 1],
        "tile_size": [16, 16],
        "atlas": "terrain.json",
        "tiles": ["grass"],
        "objects": [
            {
                "sheet": "fauna.json",
                "object_name": "bush",
                "position": [0, 0],
                "size_px": [0, 16]
            }
        ]
    });

    assert_invalid(&schema, &doc);
}
