use serde_json::json;

mod common;
use common::{assert_invalid, assert_valid, compile_schema};

fn compile_ui_layout_schema() -> jsonschema::JSONSchema {
    compile_schema(toki_schemas::UI_LAYOUT_SCHEMA, "ui layout")
}

#[test]
fn ui_layout_schema_accepts_valid_layout_documents() {
    let schema = compile_ui_layout_schema();
    let doc = json!({
        "id": "hud",
        "title": "HUD",
        "startup_visible": true,
        "z_order": 5,
        "root": {
            "id": "root",
            "title": "Root",
            "layout": {
                "anchor": "stretch",
                "size": [320.0, 180.0]
            },
            "kind": {
                "kind": "grid_container",
                "columns": 2,
                "spacing": { "left": 4, "top": 4 }
            },
            "children": [
                {
                    "id": "health",
                    "title": "Health",
                    "kind": {
                        "kind": "progress_bar",
                        "value": {
                            "mode": "percent",
                            "percent": {
                                "kind": "expression",
                                "expression": "75"
                            }
                        }
                    }
                },
                {
                    "id": "inventory",
                    "title": "Inventory",
                    "kind": {
                        "kind": "scroll_list",
                        "collection": { "kind": "player_inventory" },
                        "row_height": 24,
                        "row_spacing": 4,
                        "row_template": {
                            "segments": [
                                { "kind": "item_id" },
                                { "kind": "literal", "text": " x" },
                                { "kind": "item_count" }
                            ]
                        }
                    }
                },
                {
                    "id": "label",
                    "title": "Coins",
                    "style": {
                        "typography": {
                            "font_family": "Monospace",
                            "font_size_px": 18,
                            "weight": "Bold",
                            "slant": "Italic",
                            "anchor": "Center"
                        }
                    },
                    "kind": {
                        "kind": "label",
                        "content": {
                            "segments": [
                                { "kind": "literal", "text": "Coins: " },
                                { "kind": "binding", "binding": {
                                    "kind": "value_path",
                                    "path": "flags.coins"
                                }}
                            ]
                        }
                    }
                }
            ]
        }
    });

    assert_valid(&schema, &doc);
}

#[test]
fn ui_layout_schema_rejects_invalid_layout_documents() {
    let schema = compile_ui_layout_schema();
    let invalid_docs = vec![
        json!({}),
        json!({"id": "", "root": {"id": "root"}}),
        json!({
            "id": "hud",
            "root": {
                "id": "",
                "kind": {"kind": "grid_container", "columns": 0, "spacing": {}}
            }
        }),
        json!({
            "id": "hud",
            "root": {
                "id": "root",
                "kind": {"kind": "image", "image_id": ""}
            }
        }),
        json!({
            "id": "hud",
            "root": {
                "id": "root",
                "kind": {
                    "kind": "progress_bar",
                    "value": {
                        "mode": "percent",
                        "percent": { "kind": "expression", "expression": "" }
                    }
                }
            }
        }),
    ];

    for doc in invalid_docs {
        assert_invalid(&schema, &doc);
    }
}
