use glam::{IVec2, UVec2};
use tempfile::NamedTempFile;
use toki_core::animation::{AnimationClip, AnimationController, AnimationState, LoopMode};
use toki_core::collision::CollisionBox;
use toki_core::entity::*;
use toki_core::serialization::*;
use toki_core::{GameState, InputKey};

fn test_definition(name: &str, category: &str) -> EntityDefinition {
    EntityDefinition {
        name: name.to_string(),
        display_name: format!("Display {name}"),
        description: format!("Definition for {name}"),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
        },
        attributes: AttributesDef {
            health: Some(100),
            speed: 2,
            solid: true,
            active: true,
            can_move: true,
            ai_behavior: if category == "creature" {
                AiBehavior::Wander
            } else {
                AiBehavior::None
            },
            movement_profile: if category == "human" {
                MovementProfile::PlayerWasd
            } else {
                MovementProfile::None
            },
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
            movement_sound: "sfx_step".to_string(),
            collision_sound: Some("sfx_hit2".to_string()),
        },
        animations: AnimationsDef {
            atlas_name: "creatures".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["slime/idle_0".to_string()],
                frame_duration_ms: 150.0,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle".to_string(),
        },
        category: category.to_string(),
        tags: vec!["test".to_string()],
    }
}

fn create_test_entity() -> Entity {
    let mut controller = AnimationController::new();
    let clip = AnimationClip {
        state: AnimationState::Walk,
        atlas_name: "test_atlas".to_string(),
        frame_tile_names: vec!["frame1".to_string(), "frame2".to_string()],
        frame_duration_ms: 100.0,
        loop_mode: LoopMode::Loop,
    };
    controller.add_clip(clip);

    let attributes = EntityAttributes {
        health: Some(100),
        speed: 5,
        solid: true,
        visible: true,
        animation_controller: Some(controller),
        render_layer: 2,
        active: true,
        can_move: true,
        ai_behavior: AiBehavior::None,
        movement_profile: MovementProfile::PlayerWasd,
        has_inventory: true,
    };

    Entity {
        id: 42,
        position: IVec2::new(10, 20),
        size: UVec2::new(16, 16),
        entity_kind: EntityKind::Player,
        category: "human".to_string(),
        definition_name: Some("player".to_string()),
        control_role: ControlRole::PlayerCharacter,
        attributes,
        collision_box: Some(CollisionBox::solid_box(UVec2::new(16, 16))),
    }
}

fn create_test_entity_manager() -> EntityManager {
    let mut manager = EntityManager::new();
    let npc_def = test_definition("npc", "creature");
    let player_id = manager.add_existing_entity(create_test_entity());
    let npc_id = manager
        .spawn_from_definition(&npc_def, IVec2::new(50, 75))
        .expect("npc spawn from definition should succeed");

    if let Some(player) = manager.get_entity_mut(player_id) {
        player.position = IVec2::new(100, 200);
    }
    if let Some(audio) = manager.audio_component_mut(player_id) {
        audio.footstep_trigger_distance = 32.0;
        audio.movement_sound = Some("sfx_step".to_string());
        audio.collision_sound = Some("sfx_hit2".to_string());
    }

    // Modify some state to test preservation
    manager.set_entity_active(npc_id, false);

    manager
}

#[test]
fn test_entity_roundtrip_serialization() {
    let entity = create_test_entity();

    // Test JSON roundtrip
    let json = serde_json::to_string_pretty(&entity).unwrap();
    let deserialized: Entity = serde_json::from_str(&json).unwrap();

    // Verify core fields
    assert_eq!(entity.id, deserialized.id);
    assert_eq!(entity.position, deserialized.position);
    assert_eq!(entity.size, deserialized.size);
    assert_eq!(entity.entity_kind, deserialized.entity_kind);
    assert_eq!(entity.definition_name, deserialized.definition_name);

    // Verify attributes
    assert_eq!(entity.attributes.health, deserialized.attributes.health);
    assert_eq!(entity.attributes.speed, deserialized.attributes.speed);
    assert_eq!(entity.attributes.solid, deserialized.attributes.solid);
    assert_eq!(entity.attributes.visible, deserialized.attributes.visible);
    assert_eq!(entity.attributes.active, deserialized.attributes.active);
}

#[test]
fn test_entity_minimal_fields() {
    let entity = Entity {
        id: 1,
        position: IVec2::ZERO,
        size: UVec2::new(1, 1),
        entity_kind: EntityKind::Item,
        category: "item".to_string(),
        definition_name: None,
        control_role: ControlRole::None,
        attributes: EntityAttributes::default(),
        collision_box: None,
    };

    let json = serde_json::to_string_pretty(&entity).unwrap();
    let deserialized: Entity = serde_json::from_str(&json).unwrap();

    assert_eq!(entity.id, deserialized.id);
    assert_eq!(entity.position, deserialized.position);
    assert_eq!(entity.entity_kind, deserialized.entity_kind);
    assert_eq!(deserialized.definition_name, None);
    assert!(deserialized.collision_box.is_none());
    assert!(deserialized.attributes.animation_controller.is_none());
    assert_eq!(deserialized.attributes.health, None);
}

#[test]
fn test_entity_manager_roundtrip() {
    let manager = create_test_entity_manager();
    let original_player_id = manager.get_player_id().unwrap();

    // Test roundtrip
    let json = serde_json::to_string_pretty(&manager).unwrap();
    let deserialized: EntityManager = serde_json::from_str(&json).unwrap();

    // Verify entities were preserved
    assert_eq!(deserialized.get_player_id(), Some(original_player_id));
    assert!(deserialized.get_entity(original_player_id).is_some());

    // Verify lookup tables were preserved
    let player_entities = deserialized.entities_of_kind(&EntityKind::Player);
    assert_eq!(player_entities.len(), 1);
    assert_eq!(player_entities[0], original_player_id);

    let npc_entities = deserialized.entities_of_kind(&EntityKind::Npc);
    assert_eq!(npc_entities.len(), 1);

    // Verify audio components were preserved
    let audio_component = deserialized
        .audio_component(original_player_id)
        .expect("player audio component should exist");
    assert_eq!(audio_component.footstep_trigger_distance, 32.0);
    assert_eq!(audio_component.movement_sound.as_deref(), Some("sfx_step"));
    assert_eq!(audio_component.collision_sound.as_deref(), Some("sfx_hit2"));

    // Verify active status was preserved
    let active_entities = deserialized.active_entities();
    assert!(active_entities.contains(&original_player_id));
    // NPC should be inactive as we set it that way
    assert_eq!(active_entities.len(), 1); // Only player active
}

#[test]
fn test_empty_entity_manager() {
    let manager = EntityManager::new();

    let json = serde_json::to_string_pretty(&manager).unwrap();
    let deserialized: EntityManager = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.get_player_id(), None);
    assert_eq!(deserialized.active_entities().len(), 0);
    assert_eq!(deserialized.entities_of_kind(&EntityKind::Player).len(), 0);
    assert_eq!(deserialized.entities_of_kind(&EntityKind::Npc).len(), 0);
}

#[test]
fn test_game_state_roundtrip() {
    let mut game_state = GameState::new_empty();
    let player_id = game_state.spawn_player_at(IVec2::new(64, 128));

    // Add some input state (should be reset due to #[serde(default)])
    game_state.handle_key_press(InputKey::Up);

    // Test roundtrip
    let json = serde_json::to_string_pretty(&game_state).unwrap();
    let deserialized: GameState = serde_json::from_str(&json).unwrap();

    // Verify entity state preserved
    assert_eq!(deserialized.player_id(), Some(player_id));
    let player = deserialized.player_entity().unwrap();
    assert_eq!(player.position, IVec2::new(64, 128));
    assert_eq!(player.entity_kind, EntityKind::Player);
}

#[test]
fn test_save_load_entity_to_file() {
    let entity = create_test_entity();
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_str().unwrap();

    // Test save
    save_entity_to_file(&entity, file_path).unwrap();

    // Test load
    let loaded_entity = load_entity_from_file(file_path).unwrap();

    // Verify
    assert_eq!(entity.id, loaded_entity.id);
    assert_eq!(entity.position, loaded_entity.position);
    assert_eq!(entity.entity_kind, loaded_entity.entity_kind);
}

#[test]
fn test_save_load_scene() {
    let manager = create_test_entity_manager();
    let original_player_id = manager.get_player_id().unwrap();
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_str().unwrap();

    // Test save
    save_scene(&manager, file_path).unwrap();

    // Test load
    let loaded_manager = load_scene(file_path).unwrap();

    // Verify
    assert_eq!(loaded_manager.get_player_id(), Some(original_player_id));
    let loaded_player = loaded_manager.get_player().unwrap();
    assert_eq!(loaded_player.position, IVec2::new(100, 200));

    let npc_entities = loaded_manager.entities_of_kind(&EntityKind::Npc);
    assert_eq!(npc_entities.len(), 1);
}

#[test]
fn test_save_load_game_state() {
    let mut game_state = GameState::new_empty();
    let player_id = game_state.spawn_player_at(IVec2::new(100, 200));
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path().to_str().unwrap();

    // Test save
    save_game(&game_state, file_path).unwrap();

    // Test load
    let loaded_game_state = load_game(file_path).unwrap();

    // Verify
    assert_eq!(loaded_game_state.player_id(), Some(player_id));
    let loaded_player = loaded_game_state.player_entity().unwrap();
    assert_eq!(loaded_player.position, IVec2::new(100, 200));
}

#[test]
fn test_json_structure() {
    let entity = create_test_entity();
    let json = serde_json::to_string_pretty(&entity).unwrap();

    // Verify JSON contains expected fields for debugging
    assert!(json.contains("\"id\": 42"));
    assert!(json.contains("\"position\""));
    assert!(json.contains("\"entity_kind\": \"Player\""));
    assert!(!json.contains("\"footstep_distance_accumulator\""));
    assert!(!json.contains("\"last_collision_state\""));
    assert!(!json.contains("\"footstep_trigger_distance\""));
}

#[test]
fn test_entity_deserialization_ignores_legacy_audio_fields() {
    let mut json_value = serde_json::to_value(create_test_entity()).unwrap();
    let object = json_value
        .as_object_mut()
        .expect("serialized entity should be a JSON object");
    object.insert(
        "footstep_distance_accumulator".to_string(),
        serde_json::json!(15.5),
    );
    object.insert(
        "footstep_trigger_distance".to_string(),
        serde_json::json!(32.0),
    );
    object.insert("last_collision_state".to_string(), serde_json::json!(true));
    object.insert(
        "movement_sound".to_string(),
        serde_json::json!("legacy_step"),
    );

    let json = serde_json::to_string(&json_value).unwrap();
    let parsed: Entity = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.id, 42);
    assert_eq!(parsed.position, IVec2::new(10, 20));
    assert_eq!(parsed.entity_kind, EntityKind::Player);
}

#[test]
fn test_invalid_json_handling() {
    let invalid_json = r#"{"invalid": "json"}"#;

    let result = serde_json::from_str::<Entity>(invalid_json);
    assert!(result.is_err());

    let result = serde_json::from_str::<EntityManager>(invalid_json);
    assert!(result.is_err());

    let result = serde_json::from_str::<GameState>(invalid_json);
    assert!(result.is_err());
}

#[test]
fn test_file_error_handling() {
    // Test loading from non-existent file
    let result = load_entity_from_file("/non/existent/path.json");
    assert!(result.is_err());

    let result = load_scene("/non/existent/path.json");
    assert!(result.is_err());

    let result = load_game("/non/existent/path.json");
    assert!(result.is_err());
}

#[test]
fn test_entity_kind_serialization() {
    let entity_types = vec![
        EntityKind::Player,
        EntityKind::Npc,
        EntityKind::Item,
        EntityKind::Decoration,
        EntityKind::Trigger,
    ];

    for entity_type in entity_types {
        let json = serde_json::to_string(&entity_type).unwrap();
        let deserialized: EntityKind = serde_json::from_str(&json).unwrap();
        assert_eq!(entity_type, deserialized);
    }
}
