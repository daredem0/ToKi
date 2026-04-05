use serde_json::json;

mod common;
use common::{assert_invalid, assert_valid, compile_schema};

fn compile_tileset_schema() -> jsonschema::JSONSchema {
    compile_schema(toki_schemas::TILESET_SCHEMA, "tileset")
}

#[test]
fn tileset_schema_accepts_source_linked_entries() {
    let schema = compile_tileset_schema();
    let doc = json!({
        "tile_size": [16, 16],
        "entries": {
            "terrain/tile/grass": {
                "atlas_name": "terrain.json",
                "kind": "tile",
                "source_name": "grass"
            },
            "AutoTile_Grass/autotile/terrain": {
                "atlas_name": "AutoTile_Grass.json",
                "kind": "autotile",
                "source_name": "terrain",
                "display_name": "AutoTile_Grass"
            }
        }
    });

    assert_valid(&schema, &doc);
}

#[test]
fn tileset_schema_rejects_missing_source_name() {
    let schema = compile_tileset_schema();
    let doc = json!({
        "tile_size": [16, 16],
        "entries": {
            "terrain/tile/grass": {
                "atlas_name": "terrain.json",
                "kind": "tile"
            }
        }
    });

    assert_invalid(&schema, &doc);
}
