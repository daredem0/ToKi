
use super::GameManager;
use std::collections::HashMap;
use std::path::PathBuf;
use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
use toki_core::assets::tilemap::TileMap;
use toki_core::GameState;
use winit::keyboard::KeyCode;

fn sample_atlas() -> AtlasMeta {
    let mut tiles = HashMap::new();
    tiles.insert(
        "slime/idle_0".to_string(),
        TileInfo {
            position: glam::UVec2::new(0, 0),
            properties: TileProperties::default(),
        },
    );
    tiles.insert(
        "slime/idle_1".to_string(),
        TileInfo {
            position: glam::UVec2::new(1, 0),
            properties: TileProperties::default(),
        },
    );
    tiles.insert(
        "solid".to_string(),
        TileInfo {
            position: glam::UVec2::new(0, 1),
            properties: TileProperties {
                solid: true,
                trigger: false,
            },
        },
    );
    tiles.insert(
        "trigger".to_string(),
        TileInfo {
            position: glam::UVec2::new(1, 1),
            properties: TileProperties {
                solid: false,
                trigger: true,
            },
        },
    );

    AtlasMeta {
        image: PathBuf::from("creatures.png"),
        tile_size: glam::UVec2::new(16, 16),
        tiles,
    }
}

fn sample_tilemap() -> TileMap {
    TileMap {
        size: glam::UVec2::new(2, 1),
        tile_size: glam::UVec2::new(16, 16),
        atlas: PathBuf::from("terrain.json"),
        tiles: vec!["solid".to_string(), "trigger".to_string()],
        objects: vec![],
    }
}

fn walkable_tilemap() -> TileMap {
    TileMap {
        size: glam::UVec2::new(2, 1),
        tile_size: glam::UVec2::new(16, 16),
        atlas: PathBuf::from("terrain.json"),
        tiles: vec!["trigger".to_string(), "trigger".to_string()],
        objects: vec![],
    }
}

#[test]
fn debug_toggle_key_is_forwarded_to_core_input() {
    let mut game_state = GameState::new_empty();
    game_state.spawn_player_at(glam::IVec2::new(0, 0));
    let mut manager = GameManager::new(game_state);

    assert!(!manager.is_debug_collision_rendering_enabled());
    manager.handle_keyboard_input(KeyCode::F4, true);
    assert!(manager.is_debug_collision_rendering_enabled());
}

#[test]
fn unsupported_key_does_not_change_debug_state() {
    let mut game_state = GameState::new_empty();
    game_state.spawn_player_at(glam::IVec2::new(0, 0));
    let mut manager = GameManager::new(game_state);

    manager.handle_keyboard_input(KeyCode::Space, true);
    assert!(!manager.is_debug_collision_rendering_enabled());
}

#[test]
fn wrapper_methods_expose_core_entity_state() {
    let mut game_state = GameState::new_empty();
    let player_id = game_state.spawn_player_at(glam::IVec2::new(10, 12));
    let mut manager = GameManager::new(game_state);

    let npc_id = manager.spawn_player_like_npc(glam::IVec2::new(20, 12));
    let renderable = manager.get_renderable_entities();
    let entities_for_camera = manager.entities_for_camera();

    assert_eq!(manager.player_id(), Some(player_id));
    assert_eq!(manager.player_position(), glam::IVec2::new(10, 12));
    assert_eq!(manager.sprite_size(), 16);
    assert_eq!(renderable.len(), 2);
    assert_eq!(entities_for_camera.len(), 2);
    assert!(entities_for_camera
        .iter()
        .any(|entity| entity.id == player_id));
    assert!(entities_for_camera.iter().any(|entity| entity.id == npc_id));
}

#[test]
fn sprite_frame_wrappers_resolve_from_atlas() {
    let mut game_state = GameState::new_empty();
    let player_id = game_state.spawn_player_at(glam::IVec2::new(0, 0));
    let manager = GameManager::new(game_state);
    let atlas = sample_atlas();
    let texture_size = atlas.image_size().expect("atlas image size should exist");

    let entity_frame = manager.get_entity_sprite_frame(player_id, &atlas, texture_size);
    let entity_frame = entity_frame.expect("player frame should resolve from atlas");

    let current_frame = manager.current_sprite_frame(&atlas, texture_size);
    assert_eq!(current_frame.u0, entity_frame.u0);
    assert_eq!(current_frame.v0, entity_frame.v0);
    assert_eq!(current_frame.u1, entity_frame.u1);
    assert_eq!(current_frame.v1, entity_frame.v1);
}

#[test]
fn debug_collision_wrappers_return_tiles_and_boxes_when_enabled() {
    let mut game_state = GameState::new_empty();
    game_state.spawn_player_at(glam::IVec2::new(0, 0));
    let mut manager = GameManager::new(game_state);
    let atlas = sample_atlas();
    let tilemap = sample_tilemap();

    assert!(manager.get_entity_collision_boxes().is_empty());
    assert!(manager
        .get_solid_tile_positions(&tilemap, &atlas)
        .is_empty());
    assert!(manager
        .get_trigger_tile_positions(&tilemap, &atlas)
        .is_empty());

    manager.handle_keyboard_input(KeyCode::F4, true);

    assert!(!manager.get_entity_collision_boxes().is_empty());
    assert_eq!(
        manager.get_solid_tile_positions(&tilemap, &atlas),
        vec![(0, 0)]
    );
    assert_eq!(
        manager.get_trigger_tile_positions(&tilemap, &atlas),
        vec![(1, 0)]
    );
}

#[test]
fn player_wasd_profile_ignores_arrow_keys_for_movement() {
    let mut game_state = GameState::new_empty();
    let player_id = game_state.spawn_player_at(glam::IVec2::new(0, 0));
    let mut manager = GameManager::new(game_state);
    let atlas = sample_atlas();
    let tilemap = walkable_tilemap();

    manager.handle_keyboard_input(KeyCode::ArrowRight, true);
    let result = manager.update(glam::UVec2::new(128, 128), &tilemap, &atlas);
    manager.handle_keyboard_input(KeyCode::ArrowRight, false);

    assert!(!result.player_moved);
    assert_eq!(
        manager
            .game_state
            .entity_manager()
            .get_entity(player_id)
            .expect("player should exist")
            .position,
        glam::IVec2::new(0, 0)
    );
}

#[test]
fn player_wasd_profile_moves_from_wasd_keys() {
    let mut game_state = GameState::new_empty();
    let player_id = game_state.spawn_player_at(glam::IVec2::new(0, 0));
    let mut manager = GameManager::new(game_state);
    let atlas = sample_atlas();
    let tilemap = walkable_tilemap();

    manager.handle_keyboard_input(KeyCode::KeyD, true);
    let result = manager.update(glam::UVec2::new(128, 128), &tilemap, &atlas);
    manager.handle_keyboard_input(KeyCode::KeyD, false);

    assert!(result.player_moved);
    assert_eq!(
        manager
            .game_state
            .entity_manager()
            .get_entity(player_id)
            .expect("player should exist")
            .position,
        glam::IVec2::new(1, 0)
    );
}

#[test]
fn player_wasd_space_triggers_primary_action_attack_when_clip_exists() {
    let mut game_state = GameState::new_empty();
    let player_id = game_state.spawn_player_at(glam::IVec2::new(0, 0));
    let player = game_state
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: toki_core::animation::AnimationState::IdleDown,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_down_a".to_string()],
        frame_duration_ms: 180.0,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: toki_core::animation::AnimationState::AttackDown,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_down_a".to_string()],
        frame_duration_ms: 120.0,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(toki_core::animation::AnimationState::IdleDown);

    let mut manager = GameManager::new(game_state);
    let atlas = sample_atlas();
    let tilemap = walkable_tilemap();

    manager.handle_keyboard_input(KeyCode::Space, true);
    manager.update(glam::UVec2::new(128, 128), &tilemap, &atlas);
    manager.handle_keyboard_input(KeyCode::Space, false);

    let current_state = manager
        .game_state
        .entity_manager()
        .get_entity(player_id)
        .and_then(|entity| entity.attributes.animation_controller.as_ref())
        .map(|controller| controller.current_clip_state);
    assert_eq!(
        current_state,
        Some(toki_core::animation::AnimationState::AttackDown)
    );
}
