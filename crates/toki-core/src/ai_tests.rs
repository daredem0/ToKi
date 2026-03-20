use super::*;
use crate::animation::{AnimationClip, AnimationController, LoopMode};
use crate::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
use crate::assets::tilemap::TileMap;
use crate::entity::{AiConfig, ControlRole, Entity, EntityAttributes, EntityKind};
use std::collections::HashMap;
use std::path::PathBuf;

/// Creates a minimal TileMap for AI testing with all passable tiles.
fn create_test_tilemap() -> TileMap {
    TileMap {
        size: UVec2::new(16, 16),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test_atlas.json"),
        tiles: vec!["grass".to_string(); 256], // 16x16 passable tiles
        objects: Vec::new(),
    }
}

/// Creates a minimal AtlasMeta for AI testing with non-solid tiles.
fn create_test_atlas() -> AtlasMeta {
    let mut tiles = HashMap::new();
    tiles.insert(
        "grass".to_string(),
        TileInfo {
            position: UVec2::ZERO,
            properties: TileProperties {
                solid: false,
                trigger: false,
            },
        },
    );
    AtlasMeta {
        image: PathBuf::from("test.png"),
        tile_size: UVec2::new(16, 16),
        tiles,
    }
}

fn create_test_entity(id: EntityId, position: IVec2, behavior: AiBehavior) -> Entity {
    let mut controller = AnimationController::new();
    controller.add_clip(AnimationClip {
        state: AnimationState::Idle,
        atlas_name: "test".to_string(),
        frame_tile_names: vec!["idle_0".to_string()],
        frame_duration_ms: 100.0,
        loop_mode: LoopMode::Loop,
    });
    controller.add_clip(AnimationClip {
        state: AnimationState::Walk,
        atlas_name: "test".to_string(),
        frame_tile_names: vec!["walk_0".to_string()],
        frame_duration_ms: 100.0,
        loop_mode: LoopMode::Loop,
    });
    controller.play(AnimationState::Idle);

    Entity {
        id,
        position,
        size: UVec2::new(16, 16),
        entity_kind: EntityKind::Npc,
        category: "creature".to_string(),
        definition_name: Some("test_npc".to_string()),
        control_role: ControlRole::None,
        audio: Default::default(),
        attributes: EntityAttributes {
            ai_config: AiConfig {
                behavior,
                detection_radius: 64,
            },
            animation_controller: Some(controller),
            speed: 2.0,
            ..EntityAttributes::default()
        },
        collision_box: None,
        movement_accumulator: glam::Vec2::ZERO,
        tags: Vec::new(),
    }
}

#[test]
fn ai_system_creates_with_empty_state() {
    let ai_system = AiSystem::new();
    assert_eq!(ai_system.frame_counter, 0);
    assert!(ai_system.entity_states.is_empty());
}

#[test]
fn ai_system_reset_clears_all_state() {
    let mut ai_system = AiSystem::new();
    ai_system.frame_counter = 100;
    ai_system.get_or_create_state(1);
    ai_system.get_or_create_state(2);

    ai_system.reset();

    assert_eq!(ai_system.frame_counter, 0);
    assert!(ai_system.entity_states.is_empty());
}

#[test]
fn ai_system_only_updates_every_60_frames() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    let npc = create_test_entity(1, IVec2::new(32, 32), AiBehavior::Wander);
    entity_manager.add_existing_entity(npc);

    // First 59 updates should return no results
    for _ in 0..59 {
        let results = ai_system.update(
            &entity_manager,
            None,
            UVec2::new(256, 256),
            &tilemap,
            &atlas,
        );
        assert!(results.is_empty(), "Should not update before 60 frames");
    }

    // 60th update should return results
    let results = ai_system.update(
        &entity_manager,
        None,
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );
    assert_eq!(results.len(), 1, "Should update on 60th frame");
}

#[test]
fn ai_system_skips_player_entity() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    let mut player = create_test_entity(1, IVec2::new(32, 32), AiBehavior::Wander);
    player.control_role = ControlRole::PlayerCharacter;
    // player_id is set automatically when adding entity with PlayerCharacter role
    entity_manager.add_existing_entity(player);

    // Fast forward to update frame
    for _ in 0..60 {
        ai_system.update(
            &entity_manager,
            entity_manager.get_player_id(),
            UVec2::new(256, 256),
            &tilemap,
            &atlas,
        );
    }

    // Player should not be in results
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );
    // No wander entities since player is skipped
    assert!(results.is_empty());
}

#[test]
fn ai_system_only_updates_wander_entities() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    let wander_npc = create_test_entity(1, IVec2::new(32, 32), AiBehavior::Wander);
    let idle_npc = create_test_entity(2, IVec2::new(64, 64), AiBehavior::None);

    entity_manager.add_existing_entity(wander_npc);
    entity_manager.add_existing_entity(idle_npc);

    // Fast forward to update frame
    ai_system.frame_counter = 59;
    let results = ai_system.update(
        &entity_manager,
        None,
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    // Only wander entity should be in results
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entity_id, 1);
}

#[test]
fn ai_system_wander_entity_moves_or_stays() {
    fastrand::seed(42);

    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    let npc = create_test_entity(1, IVec2::new(64, 64), AiBehavior::Wander);
    entity_manager.add_existing_entity(npc);

    // Fast forward to update frame
    ai_system.frame_counter = 59;
    let results = ai_system.update(
        &entity_manager,
        None,
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(result.entity_id, 1);

    // Either moved or stayed (random)
    assert!(result.new_animation.is_some());
}
