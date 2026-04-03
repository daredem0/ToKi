use serde_json::json;

mod common;
use common::{assert_invalid, assert_valid, compile_schema};

fn compile_atlas_schema() -> jsonschema::JSONSchema {
    compile_schema(toki_schemas::ATLAS_SCHEMA, "atlas")
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
