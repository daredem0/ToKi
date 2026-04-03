use serde_json::json;

mod common;
use common::{assert_invalid, assert_valid, compile_schema};

fn compile_map_schema() -> jsonschema::JSONSchema {
    compile_schema(toki_schemas::MAP_SCHEMA, "map")
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
