use serde_json::json;

mod common;
use common::{assert_invalid, assert_valid, compile_schema};

fn compile_object_sheet_schema() -> jsonschema::JSONSchema {
    compile_schema(toki_schemas::OBJECT_SHEET_SCHEMA, "object sheet")
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
