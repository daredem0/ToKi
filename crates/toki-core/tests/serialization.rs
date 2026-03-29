use glam::{IVec2, UVec2};
use tempfile::NamedTempFile;
#[path = "support/game_state_compat.rs"]
mod game_state_compat;
use toki_core::entity::*;
use toki_core::serialization::*;
use toki_core::{FlagValue, GameState, InputKey, Scene};
use game_state_compat::GameStateCompatExt;
use toki_test_fixtures::{save_test_state, test_entity, test_entity_definition};

fn test_definition(name: &str, category: &str) -> EntityDefinition {
    test_entity_definition(name, category)
}

fn create_test_entity() -> Entity {
    test_entity()
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

fn create_save_test_state() -> GameState {
    save_test_state()
}

fn player_definition(name: &str) -> EntityDefinition {
    let mut definition = test_definition(name, "human");
    definition.attributes.can_move = true;
    definition.attributes.has_inventory = true;
    definition
}

fn persistent_npc(id: u32, position: IVec2) -> Entity {
    let mut entity = create_test_entity();
    entity.id = id;
    entity.entity_kind = EntityKind::Npc;
    entity.control_role = ControlRole::None;
    entity.position = position;
    entity.persistent_across_saves = true;
    entity
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
        persistent_across_saves: false,
        control_role: ControlRole::None,
        audio: EntityAudioSettings::default(),
        attributes: EntityAttributes::default(),
        collision_box: None,
        movement_accumulator: glam::Vec2::ZERO,
        tags: Vec::new(),
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
fn save_data_capture_persists_only_persistent_scene_entities() {
    let mut game_state = GameState::new_empty();
    let mut scene = Scene::new("main".to_string());
    let persistent = persistent_npc(2, IVec2::new(10, 10));
    let mut transient = persistent_npc(3, IVec2::new(20, 20));
    transient.persistent_across_saves = false;
    scene.entities.push(persistent);
    scene.entities.push(transient);
    game_state.add_scene(scene);
    game_state.load_scene("main").expect("scene should load");

    if let Some(entity) = game_state.entity_manager_mut().get_entity_mut(2) {
        entity.position = IVec2::new(99, 88);
    }
    game_state
        .entity_manager_mut()
        .despawn_entity(3);
    game_state.sync_persistent_entities_to_active_scene();

    let save = SaveData::capture(&game_state, 1).expect("save should capture");

    assert_eq!(save.persisted_entities.len(), 1);
    assert_eq!(save.persisted_entities[0].scene_name, "main");
    assert_eq!(save.persisted_entities[0].entity_id, 2);
    assert_eq!(
        save.persisted_entities[0]
            .entity
            .as_ref()
            .expect("persistent entity should be saved")
            .position,
        IVec2::new(99, 88)
    );
}

#[test]
fn restore_from_save_data_reapplies_removed_persistent_entities_as_missing() {
    let mut game_state = GameState::new_empty();
    let mut scene = Scene::new("main".to_string());
    scene.entities.push(persistent_npc(2, IVec2::new(10, 10)));
    game_state.add_scene(scene);
    game_state.load_scene("main").expect("scene should load");
    game_state.entity_manager_mut().despawn_entity(2);
    game_state.sync_persistent_entities_to_active_scene();

    let save = SaveData::capture(&game_state, 1).expect("save should capture");
    assert_eq!(save.persisted_entities.len(), 1);
    assert!(save.persisted_entities[0].entity.is_none());

    let mut restored = GameState::new_empty();
    let mut restored_scene = Scene::new("main".to_string());
    restored_scene.entities.push(persistent_npc(2, IVec2::new(10, 10)));
    restored.add_scene(restored_scene);
    restored.load_scene("main").expect("scene should load");
    restored
        .restore_from_save_data(&save)
        .expect("save should restore");

    assert!(restored.active_scene().and_then(|scene| scene.get_entity(2)).is_none());
}

#[test]
fn restore_from_save_data_preserves_saved_player_in_scene_without_player_entry() {
    let mut state = GameState::new_empty();
    state.add_entity_definition(player_definition("player"));

    let mut main_scene = Scene::new("main".to_string());
    main_scene.anchors.push(toki_core::scene::SceneAnchor {
        id: "main_spawn".to_string(),
        kind: toki_core::scene::SceneAnchorKind::SpawnPoint,
        position: IVec2::new(16, 16),
        facing: None,
    });
    main_scene.player_entry = Some(toki_core::scene::ScenePlayerEntry {
        entity_definition_name: "player".into(),
        spawn_point_id: "main_spawn".to_string(),
    });

    let mut side_scene = Scene::new("side".to_string());
    side_scene.anchors.push(toki_core::scene::SceneAnchor {
        id: "door".to_string(),
        kind: toki_core::scene::SceneAnchorKind::SpawnPoint,
        position: IVec2::new(96, 112),
        facing: None,
    });

    state.add_scene(main_scene.clone());
    state.add_scene(side_scene.clone());
    state.load_scene("main").expect("main scene should load");
    state
        .transition_to_scene("side", "door")
        .expect("side scene should load through transition");

    let player_id = state.player_id().expect("player should exist after transition");
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player entity should exist")
        .position = IVec2::new(120, 144);

    let save = SaveData::capture(&state, 1).expect("save should capture");

    let mut restored = GameState::new_empty();
    restored.add_entity_definition(player_definition("player"));
    restored.add_scene(main_scene);
    restored.add_scene(side_scene);
    restored.load_scene("main").expect("startup scene should load");
    restored
        .restore_from_save_data(&save)
        .expect("save should restore");

    assert_eq!(
        restored.active_scene().map(|scene| scene.name.as_str()),
        Some("side")
    );
    let restored_player = restored.player_entity().expect("player should restore");
    assert_eq!(restored_player.position, IVec2::new(120, 144));
    assert_eq!(restored_player.entity_kind, EntityKind::Player);
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
fn save_data_round_trips_with_versioned_metadata() {
    let game_state = create_save_test_state();
    let temp_file = NamedTempFile::new().unwrap();

    let save_data = SaveData::capture(&game_state, 2).unwrap();
    assert_eq!(save_data.version, SAVE_DATA_VERSION);
    assert_eq!(save_data.metadata.slot, 2);
    assert_eq!(save_data.metadata.scene_name, "main");

    save_save_data(&save_data, temp_file.path()).unwrap();
    let loaded = load_save_data(temp_file.path()).unwrap();

    assert_eq!(loaded.version, SAVE_DATA_VERSION);
    assert_eq!(loaded.metadata.slot, 2);
    assert_eq!(loaded.metadata.scene_name, "main");
    assert_eq!(loaded.flags.get("coins"), Some(&FlagValue::Int(7)));
    assert_eq!(
        loaded.player.as_ref().map(|player| player.position),
        Some(IVec2::new(24, 40))
    );
    assert_eq!(loaded.camera.position, Some(IVec2::new(6, 8)));
    assert_eq!(loaded.camera.scale, Some(3));
}

#[test]
fn load_save_data_rejects_unsupported_version() {
    let game_state = create_save_test_state();
    let temp_file = NamedTempFile::new().unwrap();

    let mut save_data = SaveData::capture(&game_state, 1).unwrap();
    save_data.version = SAVE_DATA_VERSION + 1;
    save_save_data(&save_data, temp_file.path()).unwrap();

    let error = load_save_data(temp_file.path()).unwrap_err();
    assert!(matches!(
        error,
        SerializationError::InvalidSaveVersion {
            expected: SAVE_DATA_VERSION,
            actual
        } if actual == SAVE_DATA_VERSION + 1
    ));
}

#[test]
fn load_save_data_rejects_oversized_files() {
    let temp_file = NamedTempFile::new().unwrap();
    std::fs::write(
        temp_file.path(),
        vec![b'x'; (MAX_SAVE_FILE_SIZE as usize).saturating_add(1)],
    )
    .unwrap();

    let error = load_save_data(temp_file.path()).unwrap_err();
    assert!(matches!(
        error,
        SerializationError::FileTooLarge {
            max_bytes: MAX_SAVE_FILE_SIZE,
            ..
        }
    ));
}

#[test]
fn restore_from_save_data_rehydrates_existing_project_state() {
    let source_state = create_save_test_state();
    let mut save_data = SaveData::capture(&source_state, 1).unwrap();
    save_data.metadata.play_time_ms = 1_234;
    save_data.camera.position = Some(IVec2::new(11, 12));
    save_data.camera.scale = Some(4);
    save_data
        .flags
        .set("chapter", FlagValue::String("intro".to_string()));
    if let Some(player) = &mut save_data.player {
        player.position = IVec2::new(80, 96);
    }

    let mut restored = create_save_test_state();
    restored.restore_from_save_data(&save_data).unwrap();

    assert_eq!(restored.scene_manager().active_scene_name(), Some("main"));
    assert_eq!(restored.player_position(), IVec2::new(80, 96));
    assert_eq!(
        restored.flag("chapter"),
        Some(&FlagValue::String("intro".to_string()))
    );
    assert_eq!(restored.play_time_ms(), 1_234);
    assert_eq!(
        restored
            .active_scene()
            .and_then(|scene| scene.camera_position),
        Some(IVec2::new(11, 12))
    );
    assert_eq!(
        restored.active_scene().and_then(|scene| scene.camera_scale),
        Some(4)
    );
}

#[test]
fn save_slot_metadata_lists_existing_slots_and_empty_slots() {
    let state = create_save_test_state();
    let temp_dir = tempfile::tempdir().unwrap();

    let slot_one_path = save_game_to_slot(&state, temp_dir.path(), 1).unwrap();
    let slot_three_path = save_game_to_slot(&state, temp_dir.path(), 3).unwrap();
    assert!(slot_one_path.ends_with("slot_1.json"));
    assert!(slot_three_path.ends_with("slot_3.json"));

    let slots = list_save_slot_metadata(temp_dir.path()).unwrap();
    assert_eq!(slots.len(), MAX_SAVE_SLOTS as usize);
    assert_eq!(slots[0].as_ref().map(|metadata| metadata.slot), Some(1));
    assert!(slots[1].is_none());
    assert_eq!(slots[2].as_ref().map(|metadata| metadata.slot), Some(3));
}

#[test]
fn save_slot_helpers_reject_invalid_slot_numbers() {
    let state = create_save_test_state();
    let temp_dir = tempfile::tempdir().unwrap();

    let error = save_game_to_slot(&state, temp_dir.path(), 0).unwrap_err();
    assert!(matches!(error, SerializationError::InvalidSaveSlot(0)));
}

#[test]
fn test_json_structure() {
    let entity = create_test_entity();
    let json = serde_json::to_string_pretty(&entity).unwrap();

    // Verify JSON contains expected fields for debugging
    assert!(json.contains("\"id\": 42"));
    assert!(json.contains("\"position\""));
    assert!(json.contains("\"entity_kind\": \"Player\""));
    assert!(json.contains("\"footstep_trigger_distance\": 32.0"));
    assert!(json.contains("\"movement_sound\": \"sfx_step\""));
    assert!(!json.contains("\"footstep_distance_accumulator\""));
    assert!(!json.contains("\"last_collision_state\""));
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

    let result = load_save_data("/non/existent/path.json");
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
