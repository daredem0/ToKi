use jsonschema::JSONSchema;
use serde_json::{json, Value};

fn compile_entity_schema() -> JSONSchema {
    let schema: Value =
        serde_json::from_str(toki_schemas::ENTITY_SCHEMA).expect("entity schema should parse");
    JSONSchema::compile(&schema).expect("entity schema should compile")
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
fn entity_schema_accepts_authored_health_and_attack_power_stats() {
    let schema = compile_entity_schema();
    let doc = json!({
        "name": "fighter",
        "display_name": "Fighter",
        "description": "Stat-authored fighter",
        "rendering": {
            "size": [16, 16],
            "render_layer": 0,
            "visible": true
        },
        "attributes": {
            "health": 30,
            "stats": {
                "health": 30,
                "attack_power": 17
            },
            "speed": 2,
            "solid": true,
            "active": true,
            "can_move": true,
            "has_inventory": false
        },
        "collision": {
            "enabled": true,
            "offset": [0, 0],
            "size": [16, 16],
            "trigger": false
        },
        "audio": {
            "footstep_trigger_distance": 16.0,
            "movement_sound": "step"
        },
        "animations": {
            "atlas_name": "players",
            "clips": [
                {
                    "state": "idle",
                    "frame_tiles": ["fighter/idle_0"],
                    "frame_duration_ms": 150.0,
                    "loop_mode": "loop"
                }
            ],
            "default_state": "idle"
        },
        "category": "human",
        "tags": []
    });

    assert_valid(&schema, &doc);
}

#[test]
fn entity_schema_rejects_negative_authored_stats() {
    let schema = compile_entity_schema();
    let doc = json!({
        "name": "fighter",
        "display_name": "Fighter",
        "description": "Invalid fighter",
        "rendering": {
            "size": [16, 16],
            "render_layer": 0,
            "visible": true
        },
        "attributes": {
            "health": 30,
            "stats": {
                "attack_power": -1
            },
            "speed": 2,
            "solid": true,
            "active": true,
            "can_move": true,
            "has_inventory": false
        },
        "collision": {
            "enabled": true,
            "offset": [0, 0],
            "size": [16, 16],
            "trigger": false
        },
        "audio": {
            "footstep_trigger_distance": 16.0,
            "movement_sound": "step"
        },
        "animations": {
            "atlas_name": "players",
            "clips": [
                {
                    "state": "idle",
                    "frame_tiles": ["fighter/idle_0"],
                    "frame_duration_ms": 150.0,
                    "loop_mode": "loop"
                }
            ],
            "default_state": "idle"
        },
        "category": "human",
        "tags": []
    });

    assert_invalid(&schema, &doc);
}

#[test]
fn entity_schema_accepts_primary_projectile_authoring() {
    let schema = compile_entity_schema();
    let doc = json!({
        "name": "ranger",
        "display_name": "Ranger",
        "description": "Projectile-capable ranger",
        "rendering": {
            "size": [16, 16],
            "render_layer": 0,
            "visible": true
        },
        "attributes": {
            "health": 30,
            "stats": {
                "attack_power": 8
            },
            "primary_projectile": {
                "sheet": "fauna",
                "object_name": "rock",
                "size": [16, 16],
                "speed": 4,
                "damage": 8,
                "lifetime_ticks": 20,
                "spawn_offset": [0, 0]
            },
            "speed": 2,
            "solid": true,
            "active": true,
            "can_move": true,
            "has_inventory": false
        },
        "collision": {
            "enabled": true,
            "offset": [0, 0],
            "size": [16, 16],
            "trigger": false
        },
        "audio": {
            "footstep_trigger_distance": 16.0,
            "movement_sound": "step"
        },
        "animations": {
            "atlas_name": "players",
            "clips": [
                {
                    "state": "idle",
                    "frame_tiles": ["ranger/idle_0"],
                    "frame_duration_ms": 150.0,
                    "loop_mode": "loop"
                }
            ],
            "default_state": "idle"
        },
        "category": "human",
        "tags": []
    });

    assert_valid(&schema, &doc);
}

#[test]
fn entity_schema_rejects_invalid_primary_projectile_lifetime() {
    let schema = compile_entity_schema();
    let doc = json!({
        "name": "ranger",
        "display_name": "Ranger",
        "description": "Projectile-capable ranger",
        "rendering": {
            "size": [16, 16],
            "render_layer": 0,
            "visible": true
        },
        "attributes": {
            "health": 30,
            "primary_projectile": {
                "sheet": "fauna",
                "object_name": "rock",
                "size": [16, 16],
                "speed": 4,
                "damage": 8,
                "lifetime_ticks": 0
            },
            "speed": 2,
            "solid": true,
            "active": true,
            "can_move": true,
            "has_inventory": false
        },
        "collision": {
            "enabled": true,
            "offset": [0, 0],
            "size": [16, 16],
            "trigger": false
        },
        "audio": {
            "footstep_trigger_distance": 16.0,
            "movement_sound": "step"
        },
        "animations": {
            "atlas_name": "players",
            "clips": [
                {
                    "state": "idle",
                    "frame_tiles": ["ranger/idle_0"],
                    "frame_duration_ms": 150.0,
                    "loop_mode": "loop"
                }
            ],
            "default_state": "idle"
        },
        "category": "human",
        "tags": []
    });

    assert_invalid(&schema, &doc);
}
