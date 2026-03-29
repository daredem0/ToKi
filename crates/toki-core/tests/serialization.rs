use glam::{IVec2, UVec2};
mod support;
use support::{save_test_state, test_entity, test_entity_definition};
use tempfile::NamedTempFile;
use toki_core::entity::*;
use toki_core::game::{InputSystem, RenderQueryService, SceneSystem};
use toki_core::serialization::*;
use toki_core::{FlagValue, GameState, InputKey, Scene};

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
    if let Some(audio) = manager.storage_mut().audio_component_mut(player_id) {
        audio.footstep_trigger_distance = 32.0;
        audio.movement_sound = Some("sfx_step".to_string());
        audio.collision_sound = Some("sfx_hit2".to_string());
    }
    manager
        .storage_mut()
        .components_mut()
        .set_primary_projectile(
            player_id,
            Some(PrimaryProjectileDef {
                sheet: "effects".to_string(),
                object_name: "fireball".to_string(),
                size: [8, 8],
                speed: 6,
                damage: 4,
                lifetime_ticks: 12,
                spawn_offset: [1, -2],
            }),
        );
    manager
        .storage_mut()
        .components_mut()
        .ensure_inventory(player_id)
        .add_item("coin", 3);
    manager.storage_mut().components_mut().set_projectile(
        npc_id,
        Some(ProjectileState {
            sheet: "effects".to_string(),
            object_name: "spark".to_string(),
            size: [4, 4],
            velocity: [2, 0],
            remaining_ticks: 9,
            damage: 2,
            owner_id: Some(player_id),
        }),
    );
    manager.storage_mut().components_mut().set_pickup(
        npc_id,
        Some(PickupDef {
            item_id: "gem".to_string(),
            count: 2,
        }),
    );

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

fn player_position(state: &GameState) -> IVec2 {
    RenderQueryService::new(
        state.world().entity_manager(),
        state.world().player_id(),
        state.runtime().debug_collision_rendering(),
    )
    .player_position()
}

fn player_entity(state: &GameState) -> Option<&Entity> {
    state
        .world()
        .player_id()
        .and_then(|player_id| state.world().entity_manager().get_entity(player_id))
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
    assert_eq!(
        entity.attributes.gameplay.health,
        deserialized.attributes.gameplay.health
    );
    assert_eq!(
        entity.attributes.gameplay.speed,
        deserialized.attributes.gameplay.speed
    );
    assert_eq!(
        entity.attributes.gameplay.solid,
        deserialized.attributes.gameplay.solid
    );
    assert_eq!(
        entity.attributes.rendering.visible,
        deserialized.attributes.rendering.visible
    );
    assert_eq!(
        entity.attributes.behavior.active,
        deserialized.attributes.behavior.active
    );
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
    assert!(deserialized
        .attributes
        .rendering
        .animation_controller
        .is_none());
    assert_eq!(deserialized.attributes.gameplay.health, None);
}

#[test]
fn save_data_capture_persists_only_persistent_scene_entities() {
    let mut game_state = GameState::new_empty();
    let mut scene = Scene::new("main".to_string());
    let persistent = persistent_npc(2, IVec2::new(10, 10));
    let mut transient = persistent_npc(3, IVec2::new(20, 20));
    transient.persistent_across_saves = false;
    scene.add_entity(persistent);
    scene.add_entity(transient);
    SceneSystem::add_scene(&mut game_state, scene);
    SceneSystem::load(&mut game_state, "main").expect("scene should load");

    if let Some(entity) = game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(2)
    {
        entity.position = IVec2::new(99, 88);
    }
    game_state
        .world_mut()
        .entity_manager_mut()
        .despawn_entity(3);
    SceneSystem::sync_persistent_entities_to_active_scene(&mut game_state);

    let save = SaveData::capture(&game_state, 1).expect("save should capture");

    assert_eq!(save.persisted_entities.len(), 1);
    assert_eq!(save.persisted_entities[0].scene_name, "main");
    assert_eq!(save.persisted_entities[0].entity_id, 2);
    assert_eq!(
        save.persisted_entities[0]
            .entity
            .as_ref()
            .expect("persistent entity should be saved")
            .entity
            .position,
        IVec2::new(99, 88)
    );
}

#[test]
fn restore_from_save_data_reapplies_removed_persistent_entities_as_missing() {
    let mut game_state = GameState::new_empty();
    let mut scene = Scene::new("main".to_string());
    scene.add_entity(persistent_npc(2, IVec2::new(10, 10)));
    SceneSystem::add_scene(&mut game_state, scene);
    SceneSystem::load(&mut game_state, "main").expect("scene should load");
    game_state
        .world_mut()
        .entity_manager_mut()
        .despawn_entity(2);
    SceneSystem::sync_persistent_entities_to_active_scene(&mut game_state);

    let save = SaveData::capture(&game_state, 1).expect("save should capture");
    assert_eq!(save.persisted_entities.len(), 1);
    assert!(save.persisted_entities[0].entity.is_none());

    let mut restored = GameState::new_empty();
    let mut restored_scene = Scene::new("main".to_string());
    restored_scene.add_entity(persistent_npc(2, IVec2::new(10, 10)));
    SceneSystem::add_scene(&mut restored, restored_scene);
    SceneSystem::load(&mut restored, "main").expect("scene should load");
    toki_core::game::SceneSystem::restore_from_save_data(&mut restored, &save)
        .expect("save should restore");

    assert!(SceneSystem::active_scene(&restored)
        .and_then(|scene| scene.get_entity(2))
        .is_none());
}

#[test]
fn restore_from_save_data_preserves_saved_player_in_scene_without_player_entry() {
    let mut state = GameState::new_empty();
    state
        .world_mut()
        .insert_entity_definition(player_definition("player"));

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

    SceneSystem::add_scene(&mut state, main_scene.clone());
    SceneSystem::add_scene(&mut state, side_scene.clone());
    SceneSystem::load(&mut state, "main").expect("main scene should load");
    SceneSystem::transition(&mut state, "side", "door")
        .expect("side scene should load through transition");

    let player_id = state
        .world()
        .player_id()
        .expect("player should exist after transition");
    state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player entity should exist")
        .position = IVec2::new(120, 144);

    let save = SaveData::capture(&state, 1).expect("save should capture");

    let mut restored = GameState::new_empty();
    restored
        .world_mut()
        .insert_entity_definition(player_definition("player"));
    SceneSystem::add_scene(&mut restored, main_scene);
    SceneSystem::add_scene(&mut restored, side_scene);
    SceneSystem::load(&mut restored, "main").expect("startup scene should load");
    toki_core::game::SceneSystem::restore_from_save_data(&mut restored, &save)
        .expect("save should restore");

    assert_eq!(
        SceneSystem::active_scene(&restored).map(|scene| scene.name.as_str()),
        Some("side")
    );
    let restored_player = player_entity(&restored).expect("player should restore");
    assert_eq!(restored_player.position, IVec2::new(120, 144));
    assert_eq!(restored_player.entity_kind, EntityKind::Player);
}

#[test]
fn save_slot_round_trip_restores_removed_items_and_entity_health_in_non_main_scene() {
    let mut state = GameState::new_empty();
    state
        .world_mut()
        .insert_entity_definition(player_definition("player"));

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
    let mut wounded_npc = persistent_npc(2, IVec2::new(110, 120));
    wounded_npc.persistent_across_saves = false;
    wounded_npc.attributes.gameplay.health = Some(100);
    let mut dropped_item = create_test_entity();
    dropped_item.id = 3;
    dropped_item.entity_kind = EntityKind::Item;
    dropped_item.control_role = ControlRole::None;
    dropped_item.position = IVec2::new(132, 120);
    side_scene.add_entity(wounded_npc);
    side_scene.add_entity(dropped_item);

    SceneSystem::add_scene(&mut state, main_scene.clone());
    SceneSystem::add_scene(&mut state, side_scene.clone());
    SceneSystem::load(&mut state, "main").expect("main scene should load");
    SceneSystem::transition(&mut state, "side", "door")
        .expect("side scene should load through transition");

    let entity_manager = state.world_mut().entity_manager_mut();
    entity_manager
        .get_entity_mut(2)
        .expect("npc should exist")
        .attributes
        .gameplay
        .health = Some(25);
    entity_manager.despawn_entity(3);

    let temp_dir = tempfile::tempdir().unwrap();
    save_game_to_slot(&mut state, temp_dir.path(), 1).expect("slot save should succeed");
    let save = load_save_data_from_slot(temp_dir.path(), 1).expect("slot load should succeed");

    let mut restored = GameState::new_empty();
    restored
        .world_mut()
        .insert_entity_definition(player_definition("player"));
    SceneSystem::add_scene(&mut restored, main_scene);
    SceneSystem::add_scene(&mut restored, side_scene);
    SceneSystem::load(&mut restored, "main").expect("startup scene should load");
    toki_core::game::SceneSystem::restore_from_save_data(&mut restored, &save)
        .expect("save should restore");

    assert_eq!(
        SceneSystem::active_scene(&restored).map(|scene| scene.name.as_str()),
        Some("side")
    );
    let restored_scene = SceneSystem::active_scene(&restored).expect("side scene should be active");
    assert!(
        restored_scene.get_entity(3).is_none(),
        "despawned ground item should stay gone after load"
    );
    assert_eq!(
        restored_scene
            .get_entity(2)
            .and_then(|entity| entity.attributes.gameplay.health),
        Some(25),
        "damaged npc health should restore from the save"
    );
}

#[test]
fn test_entity_manager_roundtrip() {
    let manager = create_test_entity_manager();
    let original_player_id = manager.get_player_id().unwrap();
    let npc_id = manager
        .entities_of_kind(&EntityKind::Npc)
        .into_iter()
        .next()
        .expect("npc should exist");

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
        .storage()
        .audio_component(original_player_id)
        .expect("player audio component should exist");
    assert_eq!(audio_component.footstep_trigger_distance, 32.0);
    assert_eq!(audio_component.movement_sound.as_deref(), Some("sfx_step"));
    assert_eq!(audio_component.collision_sound.as_deref(), Some("sfx_hit2"));

    let primary_projectile = deserialized
        .storage()
        .components()
        .primary_projectile(original_player_id)
        .expect("player primary projectile should exist");
    assert_eq!(primary_projectile.object_name, "fireball");
    assert_eq!(primary_projectile.spawn_offset, [1, -2]);

    let inventory = deserialized
        .storage()
        .components()
        .inventory(original_player_id)
        .expect("player inventory should exist");
    assert_eq!(inventory.item_count("coin"), 3);

    let projectile = deserialized
        .storage()
        .components()
        .projectile(npc_id)
        .expect("npc projectile state should exist");
    assert_eq!(projectile.object_name, "spark");
    assert_eq!(projectile.owner_id, Some(original_player_id));

    let pickup = deserialized
        .storage()
        .components()
        .pickup(npc_id)
        .expect("npc pickup should exist");
    assert_eq!(pickup.item_id, "gem");
    assert_eq!(pickup.count, 2);

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
    let player_id =
        toki_core::game::SceneSystem::spawn_player_at(&mut game_state, IVec2::new(64, 128));

    // Add some input state (should be reset due to #[serde(default)])
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Up);

    // Test roundtrip
    let json = serde_json::to_string_pretty(&game_state).unwrap();
    let deserialized: GameState = serde_json::from_str(&json).unwrap();

    // Verify entity state preserved
    assert_eq!(deserialized.world().player_id(), Some(player_id));
    let player = player_entity(&deserialized).unwrap();
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
        loaded.player.as_ref().map(|player| player.entity.position),
        Some(IVec2::new(24, 40))
    );
    assert_eq!(loaded.camera.position, Some(IVec2::new(6, 8)));
    assert_eq!(loaded.camera.scale, Some(3));
    assert_eq!(loaded.scene_snapshots.len(), 1);
    assert_eq!(loaded.scene_snapshots[0].name, "main");
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
fn load_save_data_accepts_min_supported_version() {
    let game_state = create_save_test_state();
    let temp_file = NamedTempFile::new().unwrap();

    let mut save_data = SaveData::capture(&game_state, 1).unwrap();
    save_data.version = MIN_SUPPORTED_SAVE_DATA_VERSION;
    save_save_data(&save_data, temp_file.path()).unwrap();

    let loaded = load_save_data(temp_file.path()).expect("min supported version should load");
    assert_eq!(loaded.version, MIN_SUPPORTED_SAVE_DATA_VERSION);
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
        player.entity.position = IVec2::new(80, 96);
    }

    let mut restored = create_save_test_state();
    toki_core::game::SceneSystem::restore_from_save_data(&mut restored, &save_data).unwrap();

    assert_eq!(
        restored.scene().scene_manager().active_scene_name(),
        Some("main")
    );
    assert_eq!(player_position(&restored), IVec2::new(80, 96));
    assert_eq!(
        restored.flag("chapter"),
        Some(&FlagValue::String("intro".to_string()))
    );
    assert_eq!(restored.play_time_ms(), 1_234);
    assert_eq!(
        SceneSystem::active_scene(&restored).and_then(|scene| scene.camera_position),
        Some(IVec2::new(11, 12))
    );
    assert_eq!(
        SceneSystem::active_scene(&restored).and_then(|scene| scene.camera_scale),
        Some(4)
    );
}

#[test]
fn save_slot_metadata_lists_existing_slots_and_empty_slots() {
    let mut state = create_save_test_state();
    let temp_dir = tempfile::tempdir().unwrap();

    let slot_one_path = save_game_to_slot(&mut state, temp_dir.path(), 1).unwrap();
    let slot_three_path = save_game_to_slot(&mut state, temp_dir.path(), 3).unwrap();
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
    let mut state = create_save_test_state();
    let temp_dir = tempfile::tempdir().unwrap();

    let error = save_game_to_slot(&mut state, temp_dir.path(), 0).unwrap_err();
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
fn test_stored_entity_deserializes_legacy_flat_optional_component_shape() {
    let json = serde_json::json!({
        "id": 7,
        "position": [12, 18],
        "size": [16, 16],
        "entity_kind": "Player",
        "category": "human",
        "control_role": "player_character",
        "attributes": {
            "health": 9,
            "stats": { "health": { "base": 9, "current": 5 } },
            "speed": 2.5,
            "solid": true,
            "visible": true,
            "has_shadow": true,
            "animation_controller": null,
            "render_layer": 2,
            "grounding": {},
            "active": true,
            "can_move": true,
            "interactable": false,
            "interaction_reach": 0,
            "ai_config": { "behavior": "none" },
            "movement_profile": "player_wasd",
            "primary_projectile": {
                "sheet": "effects",
                "object_name": "arrow",
                "size": [8, 8],
                "speed": 5,
                "damage": 3,
                "lifetime_ticks": 20,
                "spawn_offset": [0, 0]
            },
            "projectile": {
                "sheet": "effects",
                "object_name": "arrow_flight",
                "size": [8, 8],
                "velocity": [3, 0],
                "remaining_ticks": 6,
                "damage": 3,
                "owner_id": 7
            },
            "pickup": {
                "item_id": "coin",
                "count": 4
            },
            "inventory": {
                "items": {
                    "coin": 8
                }
            },
            "has_inventory": true
        },
        "collision_box": null,
        "tags": ["hero"]
    });

    let stored: StoredEntity =
        serde_json::from_value(json).expect("legacy flat entity json should deserialize");

    assert_eq!(stored.entity.id, 7);
    assert_eq!(stored.entity.attributes.gameplay.health, Some(9));
    assert_eq!(
        stored.entity.attributes.current_stat(HEALTH_STAT_ID),
        Some(9)
    );
    assert!(stored.entity.attributes.behavior.has_inventory);
    assert_eq!(
        stored
            .components
            .primary_projectile
            .as_ref()
            .expect("primary projectile should deserialize")
            .object_name,
        "arrow"
    );
    assert_eq!(
        stored
            .components
            .projectile
            .as_ref()
            .expect("projectile should deserialize")
            .remaining_ticks,
        6
    );
    assert_eq!(
        stored
            .components
            .pickup
            .as_ref()
            .expect("pickup should deserialize")
            .count,
        4
    );
    assert_eq!(
        stored
            .components
            .inventory
            .as_ref()
            .expect("inventory should deserialize")
            .item_count("coin"),
        8
    );
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
