use super::GameManager;
use std::collections::HashMap;
use std::path::PathBuf;
use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
use toki_core::assets::tilemap::TileMap;
use toki_core::sprite_render::{SpriteRenderOrigin, SpriteVisualRef};
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
    let renderable = manager.get_sprite_render_requests();
    let entities_for_camera = manager.entities_for_camera();

    assert_eq!(manager.player_id(), Some(player_id));
    assert_eq!(manager.player_position(), glam::IVec2::new(10, 12));
    assert_eq!(renderable.len(), 2);
    assert_eq!(entities_for_camera.len(), 2);
    assert!(entities_for_camera
        .iter()
        .any(|entity| entity.id == player_id));
    assert!(entities_for_camera.iter().any(|entity| entity.id == npc_id));
    assert!(renderable
        .iter()
        .any(|request| request.origin == SpriteRenderOrigin::AnimatedEntity(player_id)));
    assert!(renderable
        .iter()
        .any(|request| request.origin == SpriteRenderOrigin::AnimatedEntity(npc_id)));
}

#[test]
fn sprite_render_request_wrapper_exposes_object_sheet_backed_entities() {
    let mut game_state = GameState::new_empty();
    let pickup_definition = toki_core::entity::EntityDefinition {
        name: "coin_pickup_render".to_string(),
        display_name: "Coin Pickup Render".to_string(),
        description: "Static object-sheet-backed pickup".to_string(),
        rendering: toki_core::entity::RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            static_object: Some(toki_core::entity::StaticObjectRenderDef {
                sheet: "items".to_string(),
                object_name: "coin".to_string(),
            }),
        },
        attributes: toki_core::entity::AttributesDef {
            health: None,
            stats: HashMap::new(),
            speed: 0.0,
            solid: false,
            active: true,
            can_move: false,
            interactable: false,
            interaction_reach: 0,
            ai_config: toki_core::entity::AiConfig::default(),
            movement_profile: toki_core::entity::MovementProfile::None,
            primary_projectile: None,
            pickup: Some(toki_core::entity::PickupDef {
                item_id: "coin".to_string(),
                count: 1,
            }),
            has_inventory: false,
        },
        collision: toki_core::entity::CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: true,
        },
        audio: toki_core::entity::AudioDef {
            footstep_trigger_distance: 16.0,
            hearing_radius: 64,
            movement_sound_trigger: toki_core::entity::MovementSoundTrigger::Distance,
            movement_sound: "".to_string(),
            collision_sound: None,
        },
        animations: toki_core::entity::AnimationsDef {
            atlas_name: "".to_string(),
            clips: vec![],
            default_state: "".to_string(),
        },
        category: "item".to_string(),
        tags: vec!["pickup".to_string()],
    };
    let pickup_id = game_state
        .entity_manager_mut()
        .spawn_from_definition(&pickup_definition, glam::IVec2::new(24, 12))
        .expect("pickup should spawn");
    let manager = GameManager::new(game_state);

    let renderable = manager.get_sprite_render_requests();
    assert_eq!(renderable.len(), 1);
    assert_eq!(
        renderable[0].origin,
        SpriteRenderOrigin::StaticEntity(pickup_id)
    );
    assert_eq!(
        renderable[0].visual,
        SpriteVisualRef::ObjectSheetObject {
            sheet_name: "items".to_string(),
            object_name: "coin".to_string(),
        }
    );
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
        glam::IVec2::new(2, 0) // Moved 2 pixels right (default speed)
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
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: toki_core::animation::AnimationState::AttackDown,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_down_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
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
