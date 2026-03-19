use jsonschema::JSONSchema;
use serde_json::{json, Value};

fn compile_scene_schema() -> JSONSchema {
    let schema: Value =
        serde_json::from_str(toki_schemas::SCENE_SCHEMA).expect("scene schema should parse");
    JSONSchema::compile(&schema).expect("scene schema should compile")
}

fn scene_with_actions(actions: Vec<Value>) -> Value {
    json!({
        "name": "SchemaActionTest",
        "maps": [],
        "entities": [],
        "anchors": [],
        "rules": {
            "rules": [
                {
                    "id": "rule_1",
                    "enabled": true,
                    "priority": 0,
                    "once": false,
                    "trigger": "OnStart",
                    "conditions": ["Always"],
                    "actions": actions
                }
            ]
        }
    })
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
fn scene_schema_accepts_all_rule_action_payload_variants() {
    let schema = compile_scene_schema();
    let doc = scene_with_actions(vec![
        json!({"PlaySound": {"channel": "Movement", "sound_id": "sfx_step"}}),
        json!({"PlayMusic": {"track_id": "lavandia"}}),
        json!({"PlayAnimation": {"target": "Player", "state": "Walk"}}),
        json!({"SetVelocity": {"target": {"Entity": 3}, "velocity": [2, -1]}}),
        json!({"Spawn": {"entity_type": "Npc", "position": [64, 32]}}),
        json!({"DestroySelf": {"target": {"Entity": 3}}}),
        json!({"SwitchScene": {"scene_name": "Main Scene", "spawn_point_id": "from_forest"}}),
    ]);
    assert_valid(&schema, &doc);
}

#[test]
fn scene_schema_rejects_invalid_rule_action_payload_combinations() {
    let schema = compile_scene_schema();
    let invalid_actions = vec![
        json!({"PlaySound": {"channel": "Ambience", "sound_id": "sfx_step"}}),
        json!({"PlaySound": {"channel": "Movement"}}),
        json!({"PlayMusic": {"track_id": ""}}),
        json!({"PlayAnimation": {"target": "Player", "state": "Run"}}),
        json!({"SetVelocity": {"target": {"Entity": 0}, "velocity": [1, 2]}}),
        json!({"SetVelocity": {"target": "Player", "velocity": [1, 2, 3]}}),
        json!({"Spawn": {"entity_type": "Enemy", "position": [1, 2]}}),
        json!({"DestroySelf": {}}),
        json!({"SwitchScene": {"scene_name": ""}}),
        json!({"SwitchScene": {"scene_name": "Main Scene"}}),
        json!({"SwitchScene": {"scene_name": "Main Scene", "spawn_point_id": ""}}),
        json!({"UnknownAction": {"foo": "bar"}}),
        json!({"PlayMusic": {"track_id": "a"}, "PlaySound": {"channel": "Movement", "sound_id": "b"}}),
    ];

    for action in invalid_actions {
        let doc = scene_with_actions(vec![action]);
        assert_invalid(&schema, &doc);
    }
}

#[test]
fn scene_schema_accepts_scene_anchors_and_background_music() {
    let schema = compile_scene_schema();
    let doc = json!({
        "name": "AnchorScene",
        "maps": [],
        "entities": [],
        "background_music_track_id": "lavandia",
        "anchors": [
            {
                "id": "from_forest",
                "kind": "SpawnPoint",
                "position": [128, 96],
                "facing": "Right"
            }
        ]
    });

    assert_valid(&schema, &doc);
}

#[test]
fn scene_schema_accepts_optional_scene_player_entry() {
    let schema = compile_scene_schema();
    let doc = json!({
        "name": "PlayerEntryScene",
        "maps": [],
        "entities": [],
        "anchors": [
            {
                "id": "spawn_1",
                "kind": "SpawnPoint",
                "position": [0, 0]
            }
        ],
        "player_entry": {
            "entity_definition_name": "player",
            "spawn_point_id": "spawn_1"
        }
    });

    assert_valid(&schema, &doc);
}

#[test]
fn scene_schema_rejects_invalid_scene_anchor_payloads() {
    let schema = compile_scene_schema();
    let invalid_docs = vec![
        json!({
            "name": "InvalidScene",
            "maps": [],
            "entities": [],
            "anchors": [{"id": "", "kind": "SpawnPoint", "position": [0, 0]}]
        }),
        json!({
            "name": "InvalidScene",
            "maps": [],
            "entities": [],
            "anchors": [{"id": "spawn", "kind": "SpawnPoint"}]
        }),
        json!({
            "name": "InvalidScene",
            "maps": [],
            "entities": [],
            "anchors": [{"id": "spawn", "kind": "Unknown", "position": [0, 0]}]
        }),
        json!({
            "name": "InvalidScene",
            "maps": [],
            "entities": [],
            "player_entry": {"entity_definition_name": "", "spawn_point_id": "spawn"}
        }),
        json!({
            "name": "InvalidScene",
            "maps": [],
            "entities": [],
            "player_entry": {"entity_definition_name": "player"}
        }),
    ];

    for doc in invalid_docs {
        assert_invalid(&schema, &doc);
    }
}
