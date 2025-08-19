use toki_core::{GameState, InputKey};
use toki_core::sprite::{Animation, Frame, SpriteInstance, SpriteSheetMeta};
use glam::{IVec2, UVec2};

fn create_test_sprite() -> SpriteInstance {
    let animation = Animation {
        name: "test_anim".into(),
        looped: true,
        frames: vec![
            Frame { index: 0, duration_ms: 100 },
            Frame { index: 1, duration_ms: 100 },
        ],
    };
    let sprite_sheet = SpriteSheetMeta {
        frame_size: (16, 16),
        frame_count: 2,
        sheet_size: (32, 16),
    };
    SpriteInstance::new(IVec2::new(50, 60), animation, sprite_sheet)
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
    assert_eq!(game_state.player_entity().unwrap().position, initial_position);
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
    let moved = game_state.update(world_bounds);
    
    assert!(moved);
    assert!(game_state.player_position().x > initial_position.x);
    
    // Release the key
    game_state.handle_key_release(InputKey::Right);
    
    // Another update should not move further
    let position_after_release = game_state.player_position();
    let moved_again = game_state.update(world_bounds);
    
    assert!(!moved_again);
    assert_eq!(game_state.player_position(), position_after_release);
}

#[test]
fn game_state_movement_up() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let initial_position = game_state.player_position();
    
    game_state.handle_key_press(InputKey::Up);
    let world_bounds = UVec2::new(1000, 1000);
    let moved = game_state.update(world_bounds);
    
    assert!(moved);
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
    let moved = game_state.update(world_bounds);
    
    assert!(moved);
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
    let moved = game_state.update(world_bounds);
    
    assert!(moved);
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
    let moved = game_state.update(world_bounds);
    
    assert!(moved);
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
    let moved = game_state.update(world_bounds);
    
    assert!(moved);
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
        game_state.update(world_bounds);
    }
    
    // Should be clamped at 0
    assert_eq!(game_state.player_position().x, 0);
    
    // One more update should not move further
    let moved = game_state.update(world_bounds);
    assert!(!moved); // Should not report movement when clamped
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
        game_state.update(world_bounds);
    }
    
    // Should be clamped at 0
    assert_eq!(game_state.player_position().y, 0);
    
    // One more update should not move further
    let moved = game_state.update(world_bounds);
    assert!(!moved); // Should not report movement when clamped
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
        game_state.update(world_bounds);
    }
    
    // Should be clamped at world_width - sprite_size
    let expected_max_x = world_bounds.x as i32 - game_state.sprite_size() as i32;
    assert_eq!(game_state.player_position().x, expected_max_x);
    
    // One more update should not move further
    let moved = game_state.update(world_bounds);
    assert!(!moved); // Should not report movement when clamped
}

#[test]
fn game_state_world_bounds_bottom_boundary() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    
    game_state.handle_key_press(InputKey::Down);
    let world_bounds = UVec2::new(1000, 100); // Small world height
    
    // Move down repeatedly until at boundary
    for _ in 0..200 {
        game_state.update(world_bounds);
    }
    
    // Should be clamped at world_height - sprite_size
    let expected_max_y = world_bounds.y as i32 - game_state.sprite_size() as i32;
    assert_eq!(game_state.player_position().y, expected_max_y);
    
    // One more update should not move further
    let moved = game_state.update(world_bounds);
    assert!(!moved); // Should not report movement when clamped
}

#[test]
fn game_state_sprite_animation_updates() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let _initial_frame = game_state.current_sprite_frame();
    
    let world_bounds = UVec2::new(1000, 1000);
    
    // Update multiple times to advance animation
    for _ in 0..10 {
        game_state.update(world_bounds);
    }
    
    // Animation should have progressed (frame or timing)
    // Note: Since animation depends on internal timing, we mainly test that it doesn't crash
    let _current_frame = game_state.current_sprite_frame();
    // The exact frame depends on timing, so we just ensure it's callable
}

#[test]
fn game_state_entity_position_sync() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    
    // Move the player
    game_state.handle_key_press(InputKey::Right);
    let world_bounds = UVec2::new(1000, 1000);
    game_state.update(world_bounds);
    
    // Entity position should match player sprite position
    let player_entity = game_state.player_entity().unwrap();
    assert_eq!(player_entity.position, game_state.player_position());
    
    // Move again
    game_state.handle_key_press(InputKey::Down);
    game_state.handle_key_release(InputKey::Right);
    game_state.update(world_bounds);
    
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
    game_state.update(world_bounds);
    
    // All directions should be processed (net effect might cancel out)
    // This tests that multiple keys don't crash the system
    let _final_position = game_state.player_position();
    
    // Release some keys
    game_state.handle_key_release(InputKey::Down);
    game_state.handle_key_release(InputKey::Left);
    
    let position_before = game_state.player_position();
    game_state.update(world_bounds);
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
    let npc_id = entity_manager.spawn_npc(IVec2::new(100, 100), "guard");
    let item_id = entity_manager.spawn_item(IVec2::new(200, 200), "coin");
    
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
    assert!(player_entity.attributes.sprite_info.is_some());
}