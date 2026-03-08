use glam::{IVec2, UVec2};
use std::collections::HashMap;
use std::path::PathBuf;
use toki_core::assets::{
    atlas::{AtlasMeta, TileInfo, TileProperties},
    tilemap::TileMap,
};
use toki_core::entity::{
    AnimationClipDef, AnimationsDef, AttributesDef, AudioDef, CollisionDef, EntityDefinition,
    RenderingDef,
};
use toki_core::sprite::{Animation, Frame, SpriteInstance, SpriteSheetMeta};
use toki_core::{GameState, InputKey};

fn create_test_sprite() -> SpriteInstance {
    let animation = Animation {
        name: "test_anim".into(),
        looped: true,
        frames: vec![
            Frame {
                index: 0,
                duration_ms: 100,
            },
            Frame {
                index: 1,
                duration_ms: 100,
            },
        ],
    };
    let sprite_sheet = SpriteSheetMeta {
        frame_size: (16, 16),
        frame_count: 2,
        sheet_size: (32, 16),
    };
    SpriteInstance::new(IVec2::new(50, 60), animation, sprite_sheet)
}

fn create_test_tilemap() -> TileMap {
    TileMap {
        size: UVec2::new(10, 10),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test_atlas.json"),
        tiles: vec!["floor".to_string(); 100], // 10x10 grid of floor tiles
    }
}

fn create_test_atlas() -> AtlasMeta {
    let mut tiles = HashMap::new();
    tiles.insert(
        "floor".to_string(),
        TileInfo {
            position: UVec2::new(0, 0),
            properties: TileProperties {
                solid: false,
                trigger: false,
            },
        },
    );

    AtlasMeta {
        image: PathBuf::from("test_atlas.png"),
        tile_size: UVec2::new(16, 16),
        tiles,
    }
}

fn test_definition(name: &str, entity_type: &str) -> EntityDefinition {
    EntityDefinition {
        name: name.to_string(),
        display_name: format!("Display {name}"),
        description: format!("Definition for {name}"),
        entity_type: entity_type.to_string(),
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
        category: "test".to_string(),
        tags: vec!["test".to_string()],
    }
}

#[test]
fn game_state_new_initializes_correctly() {
    let sprite = create_test_sprite();
    let initial_position = sprite.position;
    let game_state = GameState::new(sprite);

    assert_eq!(game_state.player_position(), initial_position);
    // Test EntityManager integration
    assert_eq!(game_state.entities().len(), 1);
    assert_eq!(game_state.player_id(), Some(1));
    assert!(game_state.player_entity().is_some());
    assert_eq!(
        game_state.player_entity().unwrap().position,
        initial_position
    );
    assert_eq!(game_state.sprite_size(), 16);
}

#[test]
fn input_key_enum_properties() {
    // Test that InputKey implements required traits
    let key1 = InputKey::Up;
    let key2 = InputKey::Up;
    let key3 = InputKey::Down;

    // Test Debug
    assert_eq!(format!("{:?}", key1), "Up");

    // Test Clone and Copy
    let key_clone = key1;
    assert_eq!(key1, key_clone);

    // Test PartialEq and Eq
    assert_eq!(key1, key2);
    assert_ne!(key1, key3);

    // Test Hash (through HashSet usage)
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(key1);
    assert!(set.contains(&key2));
    assert!(!set.contains(&key3));
}

#[test]
fn game_state_key_press_and_release() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let initial_position = game_state.player_position();

    // Press a key
    game_state.handle_key_press(InputKey::Right);

    // Should not move yet (requires update)
    assert_eq!(game_state.player_position(), initial_position);

    // Update should process the input
    let world_bounds = UVec2::new(1000, 1000);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();
    let result = game_state.update(world_bounds, &tilemap, &atlas);

    assert!(result.player_moved);
    assert!(game_state.player_position().x > initial_position.x);

    // Release the key
    game_state.handle_key_release(InputKey::Right);

    // Another update should not move further
    let position_after_release = game_state.player_position();
    let result_again = game_state.update(world_bounds, &tilemap, &atlas);

    assert!(!result_again.player_moved);
    assert_eq!(game_state.player_position(), position_after_release);
}

#[test]
fn game_state_movement_up() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let initial_position = game_state.player_position();

    game_state.handle_key_press(InputKey::Up);
    let world_bounds = UVec2::new(1000, 1000);
    let result = game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());

    assert!(result.player_moved);
    assert_eq!(game_state.player_position().x, initial_position.x); // X unchanged
    assert!(game_state.player_position().y < initial_position.y); // Y decreased (up)
    assert_eq!(game_state.player_position().y, initial_position.y - 1); // Moved 1 pixel
}

#[test]
fn game_state_movement_down() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let initial_position = game_state.player_position();

    game_state.handle_key_press(InputKey::Down);
    let world_bounds = UVec2::new(1000, 1000);
    let result = game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());

    assert!(result.player_moved);
    assert_eq!(game_state.player_position().x, initial_position.x); // X unchanged
    assert!(game_state.player_position().y > initial_position.y); // Y increased (down)
    assert_eq!(game_state.player_position().y, initial_position.y + 1); // Moved 1 pixel
}

#[test]
fn game_state_movement_left() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let initial_position = game_state.player_position();

    game_state.handle_key_press(InputKey::Left);
    let world_bounds = UVec2::new(1000, 1000);
    let result = game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());

    assert!(result.player_moved);
    assert!(game_state.player_position().x < initial_position.x); // X decreased (left)
    assert_eq!(game_state.player_position().y, initial_position.y); // Y unchanged
    assert_eq!(game_state.player_position().x, initial_position.x - 1); // Moved 1 pixel
}

#[test]
fn game_state_movement_right() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let initial_position = game_state.player_position();

    game_state.handle_key_press(InputKey::Right);
    let world_bounds = UVec2::new(1000, 1000);
    let result = game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());

    assert!(result.player_moved);
    assert!(game_state.player_position().x > initial_position.x); // X increased (right)
    assert_eq!(game_state.player_position().y, initial_position.y); // Y unchanged
    assert_eq!(game_state.player_position().x, initial_position.x + 1); // Moved 1 pixel
}

#[test]
fn game_state_diagonal_movement() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let initial_position = game_state.player_position();

    // Press both up and right
    game_state.handle_key_press(InputKey::Up);
    game_state.handle_key_press(InputKey::Right);

    let world_bounds = UVec2::new(1000, 1000);
    let result = game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());

    assert!(result.player_moved);
    assert_eq!(game_state.player_position().x, initial_position.x + 1); // Moved right
    assert_eq!(game_state.player_position().y, initial_position.y - 1); // Moved up
}

#[test]
fn game_state_world_bounds_left_boundary() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    // Move to near left edge
    game_state.handle_key_press(InputKey::Left);
    let world_bounds = UVec2::new(1000, 1000);

    // Move left repeatedly until at boundary
    for _ in 0..100 {
        game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());
    }

    // Should be clamped at 0
    assert_eq!(game_state.player_position().x, 0);

    // One more update should not move further
    let result = game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());
    assert!(!result.player_moved); // Should not report movement when clamped
    assert_eq!(game_state.player_position().x, 0);
}

#[test]
fn game_state_world_bounds_top_boundary() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    // Move to near top edge
    game_state.handle_key_press(InputKey::Up);
    let world_bounds = UVec2::new(1000, 1000);

    // Move up repeatedly until at boundary
    for _ in 0..100 {
        game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());
    }

    // Should be clamped at 0
    assert_eq!(game_state.player_position().y, 0);

    // One more update should not move further
    let result = game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());
    assert!(!result.player_moved); // Should not report movement when clamped
    assert_eq!(game_state.player_position().y, 0);
}

#[test]
fn game_state_world_bounds_right_boundary() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    game_state.handle_key_press(InputKey::Right);
    let world_bounds = UVec2::new(100, 1000); // Small world width

    // Move right repeatedly until at boundary
    for _ in 0..200 {
        game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());
    }

    // Should be clamped at world_width - sprite_size
    let expected_max_x = world_bounds.x as i32 - game_state.sprite_size() as i32;
    assert_eq!(game_state.player_position().x, expected_max_x);

    // One more update should not move further
    let result = game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());
    assert!(!result.player_moved); // Should not report movement when clamped
}

#[test]
fn game_state_world_bounds_bottom_boundary() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    game_state.handle_key_press(InputKey::Down);
    let world_bounds = UVec2::new(1000, 100); // Small world height

    // Move down repeatedly until at boundary
    for _ in 0..200 {
        game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());
    }

    // Should be clamped at world_height - sprite_size
    let expected_max_y = world_bounds.y as i32 - game_state.sprite_size() as i32;
    assert_eq!(game_state.player_position().y, expected_max_y);

    // One more update should not move further
    let result = game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());
    assert!(!result.player_moved); // Should not report movement when clamped
}

#[test]
fn game_state_sprite_animation_updates() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let atlas = create_test_atlas();
    let texture_size = atlas.image_size().unwrap_or(glam::UVec2::new(64, 16));
    let _initial_frame = game_state.current_sprite_frame(&atlas, texture_size);

    let world_bounds = UVec2::new(1000, 1000);

    // Update multiple times to advance animation
    for _ in 0..10 {
        game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());
    }

    // Animation should have progressed (frame or timing)
    // Note: Since animation depends on internal timing, we mainly test that it doesn't crash
    let _current_frame = game_state.current_sprite_frame(&atlas, texture_size);
    // The exact frame depends on timing, so we just ensure it's callable
}

#[test]
fn game_state_entity_position_sync() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    // Move the player
    game_state.handle_key_press(InputKey::Right);
    let world_bounds = UVec2::new(1000, 1000);
    game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());

    // Entity position should match player sprite position
    let player_entity = game_state.player_entity().unwrap();
    assert_eq!(player_entity.position, game_state.player_position());

    // Move again
    game_state.handle_key_press(InputKey::Down);
    game_state.handle_key_release(InputKey::Right);
    game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());

    // Should still be synchronized
    let player_entity = game_state.player_entity().unwrap();
    assert_eq!(player_entity.position, game_state.player_position());
}

#[test]
fn game_state_multiple_key_handling() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    // Press multiple keys
    game_state.handle_key_press(InputKey::Up);
    game_state.handle_key_press(InputKey::Right);
    game_state.handle_key_press(InputKey::Down); // Conflicting with Up
    game_state.handle_key_press(InputKey::Left); // Conflicting with Right

    let world_bounds = UVec2::new(1000, 1000);
    let _initial_position = game_state.player_position();
    game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());

    // All directions should be processed (net effect might cancel out)
    // This tests that multiple keys don't crash the system
    let _final_position = game_state.player_position();

    // Release some keys
    game_state.handle_key_release(InputKey::Down);
    game_state.handle_key_release(InputKey::Left);

    let position_before = game_state.player_position();
    game_state.update(world_bounds, &create_test_tilemap(), &create_test_atlas());
    let position_after = game_state.player_position();

    // Should move up and right now
    assert!(position_after.x > position_before.x); // Right
    assert!(position_after.y < position_before.y); // Up
}

#[test]
fn game_state_entity_manager_access() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    // Should be able to access EntityManager
    let entity_manager = game_state.entity_manager();
    assert_eq!(entity_manager.active_entities().len(), 1);
    assert!(entity_manager.get_player().is_some());

    // Should be able to spawn additional entities
    let entity_manager = game_state.entity_manager_mut();
    let npc_id = entity_manager
        .spawn_from_definition(&test_definition("npc", "npc"), IVec2::new(100, 100))
        .expect("npc spawn from definition should succeed");
    let item_id = entity_manager
        .spawn_from_definition(&test_definition("item", "item"), IVec2::new(200, 200))
        .expect("item spawn from definition should succeed");

    assert_eq!(entity_manager.active_entities().len(), 3);
    assert!(entity_manager.get_entity(npc_id).is_some());
    assert!(entity_manager.get_entity(item_id).is_some());
}

#[test]
fn game_state_player_entity_attributes() {
    let sprite = create_test_sprite();
    let game_state = GameState::new(sprite);

    let player_entity = game_state.player_entity().unwrap();

    // Check player entity has correct attributes from factory method
    assert_eq!(player_entity.attributes.health, Some(100));
    assert_eq!(player_entity.attributes.speed, 2);
    assert!(player_entity.attributes.active);
    assert!(player_entity.attributes.can_move);
    assert!(player_entity.attributes.solid);
    assert!(player_entity.attributes.visible);
    assert_eq!(player_entity.attributes.render_layer, 0);
    assert!(player_entity.attributes.animation_controller.is_some());
}

#[test]
fn game_state_new_uses_definition_based_player_creation() {
    let sprite = create_test_sprite();
    let game_state = GameState::new(sprite);
    let player = game_state.player_entity().expect("player should exist");

    assert_eq!(player.definition_name.as_deref(), Some("player"));
}

#[test]
fn game_state_spawn_player_like_npc_uses_definition_metadata() {
    let mut game_state = GameState::new_empty();
    let npc_id = game_state.spawn_player_like_npc(IVec2::new(120, 72));
    let npc = game_state
        .entity_manager()
        .get_entity(npc_id)
        .expect("spawned npc should exist");

    assert_eq!(npc.definition_name.as_deref(), Some("player_like_npc"));
    assert_eq!(npc.entity_type, toki_core::entity::EntityType::Npc);
}
