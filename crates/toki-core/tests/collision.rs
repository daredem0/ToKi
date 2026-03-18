use glam::{IVec2, UVec2};
use std::collections::HashMap;
use std::path::PathBuf;
use toki_core::assets::{
    atlas::{AtlasMeta, TileInfo, TileProperties},
    tilemap::TileMap,
};
use toki_core::collision::{can_place_collision_box_at_position, CollisionBox};
use toki_core::sprite::{Animation, Frame, SpriteInstance, SpriteSheetMeta};
use toki_core::{GameState, InputKey};

/// Create a test tilemap with a mix of solid and non-solid tiles
/// Layout:
/// ```
/// F S F F
/// F S S F
/// F F F F
/// F F F F
/// ```
/// Where F = floor (non-solid), S = stone (solid)
fn create_collision_test_tilemap() -> TileMap {
    let tiles = vec![
        "floor".to_string(),
        "stone".to_string(),
        "floor".to_string(),
        "floor".to_string(),
        "floor".to_string(),
        "stone".to_string(),
        "stone".to_string(),
        "floor".to_string(),
        "floor".to_string(),
        "floor".to_string(),
        "floor".to_string(),
        "floor".to_string(),
        "floor".to_string(),
        "floor".to_string(),
        "floor".to_string(),
        "floor".to_string(),
    ];

    TileMap {
        size: UVec2::new(4, 4),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test_atlas.json"),
        tiles,
        objects: vec![],
    }
}

fn create_collision_test_atlas() -> AtlasMeta {
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

    tiles.insert(
        "stone".to_string(),
        TileInfo {
            position: UVec2::new(1, 0),
            properties: TileProperties {
                solid: true,
                trigger: false,
            },
        },
    );

    tiles.insert(
        "water".to_string(),
        TileInfo {
            position: UVec2::new(2, 0),
            properties: TileProperties {
                solid: false,
                trigger: true,
            },
        },
    );

    AtlasMeta {
        image: PathBuf::from("test_atlas.png"),
        tile_size: UVec2::new(16, 16),
        tiles,
    }
}

fn create_test_sprite() -> SpriteInstance {
    let animation = Animation {
        name: "test_anim".into(),
        looped: true,
        frames: vec![Frame {
            index: 0,
            duration_ms: 100,
        }],
    };
    let sprite_sheet = SpriteSheetMeta {
        frame_size: (16, 16),
        frame_count: 1,
        sheet_size: (16, 16),
    };
    SpriteInstance::new(IVec2::new(0, 0), animation, sprite_sheet)
}

#[test]
fn collision_entity_without_collision_box_can_move_anywhere() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();

    // Get the player entity and remove its collision box
    let player_id = game_state.player_id().unwrap();
    if let Some(player) = game_state.entity_manager_mut().get_entity_mut(player_id) {
        player.position = IVec2::new(0, 0);
        player.collision_box = None; // Remove collision box
    }

    // Should be able to move to solid tile position since no collision box
    game_state.handle_key_press(InputKey::Right);
    let initial_pos = game_state.player_position();

    game_state.update(UVec2::new(1000, 1000), &tilemap, &atlas);
    let final_pos = game_state.player_position();

    // Should move even though destination has solid stone
    assert_ne!(initial_pos, final_pos);
}

#[test]
fn collision_small_entity_blocked_by_solid_tile() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();

    // Get the player entity and modify its collision box
    let player_id = game_state.player_id().unwrap();
    if let Some(player) = game_state.entity_manager_mut().get_entity_mut(player_id) {
        player.position = IVec2::new(0, 0); // Start at (0,0) - on floor tile
        player.collision_box = Some(CollisionBox::solid_box(UVec2::new(16, 16)));
    }

    // Try to move right into solid tile at (16, 0)
    game_state.handle_key_press(InputKey::Right);
    let initial_pos = game_state.player_position();

    game_state.update(UVec2::new(1000, 1000), &tilemap, &atlas);
    let final_pos = game_state.player_position();

    // Should not have moved because tile at (16, 0) is solid stone
    assert_eq!(initial_pos, final_pos);
}

#[test]
fn collision_small_entity_can_move_on_non_solid_tiles() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();

    // Get the player entity and modify its collision box
    let player_id = game_state.player_id().unwrap();
    if let Some(player) = game_state.entity_manager_mut().get_entity_mut(player_id) {
        player.position = IVec2::new(0, 0); // Start at (0,0) - on floor tile
        player.collision_box = Some(CollisionBox::solid_box(UVec2::new(16, 16)));
    }

    // Try to move down to another floor tile at (0, 16)
    game_state.handle_key_press(InputKey::Down);
    let initial_pos = game_state.player_position();

    game_state.update(UVec2::new(1000, 1000), &tilemap, &atlas);
    let final_pos = game_state.player_position();

    // Should have moved because tile at (0, 16) is floor
    // Player has default speed of 2.0, so moves 2 pixels
    assert_ne!(initial_pos, final_pos);
    assert_eq!(final_pos.y, initial_pos.y + 2); // Moved down by 2 pixels (default speed)
}

#[test]
fn collision_entity_with_offset_collision_box() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();

    // Get the player entity and give it an offset collision box
    let player_id = game_state.player_id().unwrap();
    if let Some(player) = game_state.entity_manager_mut().get_entity_mut(player_id) {
        player.position = IVec2::new(8, 0); // Start at (8,0)
                                            // Collision box offset by (8,0) with size (16,16)
                                            // This means collision box spans from (16,0) to (32,16) - overlapping solid stone
        player.collision_box = Some(CollisionBox::new(
            IVec2::new(8, 0),   // Offset
            UVec2::new(16, 16), // Size
            false,              // Not a trigger
        ));
    }

    // Entity at (8,0) with collision box at (16,0) should be blocked
    game_state.handle_key_press(InputKey::Right);
    let initial_pos = game_state.player_position();

    game_state.update(UVec2::new(1000, 1000), &tilemap, &atlas);
    let final_pos = game_state.player_position();

    // Should not move because collision box would overlap solid stone
    assert_eq!(initial_pos, final_pos);
}

#[test]
fn collision_large_entity_spanning_multiple_tiles() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();

    // Get the player entity and give it a large collision box
    let player_id = game_state.player_id().unwrap();
    if let Some(player) = game_state.entity_manager_mut().get_entity_mut(player_id) {
        player.position = IVec2::new(0, 0); // Start at (0,0)
                                            // Large collision box spanning multiple tiles
        player.collision_box = Some(CollisionBox::solid_box(UVec2::new(32, 32)));
    }

    // Large entity should be blocked from moving because it would overlap solid stones
    game_state.handle_key_press(InputKey::Right);
    let initial_pos = game_state.player_position();

    game_state.update(UVec2::new(1000, 1000), &tilemap, &atlas);
    let final_pos = game_state.player_position();

    // Should not move because large collision box would overlap stone tiles
    assert_eq!(initial_pos, final_pos);
}

#[test]
fn collision_trigger_entity_does_not_block_movement() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();

    // Get the player entity and give it a trigger collision box
    let player_id = game_state.player_id().unwrap();
    if let Some(player) = game_state.entity_manager_mut().get_entity_mut(player_id) {
        player.position = IVec2::new(0, 0);
        // Trigger collision box should not block movement
        player.collision_box = Some(CollisionBox::trigger_box(UVec2::new(16, 16)));
    }

    // Even with trigger collision box, should be able to move onto solid tile
    game_state.handle_key_press(InputKey::Right);
    let initial_pos = game_state.player_position();

    game_state.update(UVec2::new(1000, 1000), &tilemap, &atlas);
    let final_pos = game_state.player_position();

    // Should move because trigger boxes don't block movement
    assert_ne!(initial_pos, final_pos);
}

#[test]
fn collision_negative_coordinates_blocked() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();

    // Get the player entity and position it at the edge
    let player_id = game_state.player_id().unwrap();
    if let Some(player) = game_state.entity_manager_mut().get_entity_mut(player_id) {
        player.position = IVec2::new(0, 0); // At the very edge
        player.collision_box = Some(CollisionBox::solid_box(UVec2::new(16, 16)));
    }

    // Try to move left (would result in negative coordinates)
    game_state.handle_key_press(InputKey::Left);
    let initial_pos = game_state.player_position();

    game_state.update(UVec2::new(1000, 1000), &tilemap, &atlas);
    let final_pos = game_state.player_position();

    // Should not move into negative coordinates
    assert_eq!(initial_pos, final_pos);
}

#[test]
fn collision_out_of_bounds_tiles_block_movement() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let tilemap = create_collision_test_tilemap(); // 4x4 tilemap (64x64 pixels)
    let atlas = create_collision_test_atlas();

    // Get the player entity and position it near the edge
    let player_id = game_state.player_id().unwrap();
    if let Some(player) = game_state.entity_manager_mut().get_entity_mut(player_id) {
        player.position = IVec2::new(48, 48); // Near the bottom-right corner
        player.collision_box = Some(CollisionBox::solid_box(UVec2::new(16, 16)));
    }

    // Try to move right (would go out of bounds)
    game_state.handle_key_press(InputKey::Right);
    let initial_pos = game_state.player_position();

    game_state.update(UVec2::new(1000, 1000), &tilemap, &atlas);
    let final_pos = game_state.player_position();

    // Should not move out of bounds
    assert_eq!(initial_pos, final_pos);
}

#[test]
fn collision_movement_step_collision_boundaries() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();

    // Position entity right next to a solid tile
    let player_id = game_state.player_id().unwrap();
    if let Some(player) = game_state.entity_manager_mut().get_entity_mut(player_id) {
        player.position = IVec2::new(15, 0); // 1 pixel before solid stone at (16, 0)
        player.collision_box = Some(CollisionBox::solid_box(UVec2::new(16, 16)));
    }

    // Try to move right - should be blocked because collision box would overlap stone
    game_state.handle_key_press(InputKey::Right);
    let initial_pos = game_state.player_position();

    game_state.update(UVec2::new(1000, 1000), &tilemap, &atlas);
    let final_pos = game_state.player_position();

    // Should not move because collision box would overlap solid stone
    assert_eq!(initial_pos, final_pos);
}

#[test]
fn collision_exact_tile_boundary_movement() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();

    // Position entity in a safe area with room to move
    let player_id = game_state.player_id().unwrap();
    if let Some(player) = game_state.entity_manager_mut().get_entity_mut(player_id) {
        player.position = IVec2::new(32, 32); // Bottom area, all floor tiles
        player.collision_box = Some(CollisionBox::solid_box(UVec2::new(16, 16)));
    }

    // Try to move down - should be allowed (floor tiles)
    game_state.handle_key_press(InputKey::Down);
    let initial_pos = game_state.player_position();

    game_state.update(UVec2::new(1000, 1000), &tilemap, &atlas);
    let final_pos = game_state.player_position();

    // Should move because moving within floor tile area
    assert_ne!(initial_pos, final_pos);
}

#[test]
fn collision_multi_direction_movement() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();

    // Position entity in free area
    let player_id = game_state.player_id().unwrap();
    if let Some(player) = game_state.entity_manager_mut().get_entity_mut(player_id) {
        player.position = IVec2::new(48, 48); // Bottom-right area (all floor tiles)
        player.collision_box = Some(CollisionBox::solid_box(UVec2::new(16, 16)));
    }

    // Try diagonal movement (up+left) - both directions should be allowed
    game_state.handle_key_press(InputKey::Up);
    game_state.handle_key_press(InputKey::Left);
    let initial_pos = game_state.player_position();

    game_state.update(UVec2::new(1000, 1000), &tilemap, &atlas);
    let final_pos = game_state.player_position();

    // Should move in both directions
    assert!(final_pos.x < initial_pos.x); // Moved left
    assert!(final_pos.y < initial_pos.y); // Moved up
}

#[test]
fn collision_entity_larger_than_tiles() {
    // Create a tilemap with smaller tile size for this test
    let mut tilemap = create_collision_test_tilemap();
    tilemap.tile_size = UVec2::new(8, 8); // Smaller tiles
    let atlas = create_collision_test_atlas();

    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    // Entity with collision box larger than tile size
    let player_id = game_state.player_id().unwrap();
    if let Some(player) = game_state.entity_manager_mut().get_entity_mut(player_id) {
        player.position = IVec2::new(0, 0);
        player.collision_box = Some(CollisionBox::solid_box(UVec2::new(24, 24)));
        // Spans 3x3 tiles
    }

    // Large entity should be blocked by the solid stones in the tilemap
    game_state.handle_key_press(InputKey::Right);
    let initial_pos = game_state.player_position();

    game_state.update(UVec2::new(1000, 1000), &tilemap, &atlas);
    let final_pos = game_state.player_position();

    // Should not move because large collision box overlaps solid tiles
    assert_eq!(initial_pos, final_pos);
}

#[test]
fn placement_collision_without_collision_box_is_valid() {
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();

    assert!(can_place_collision_box_at_position(
        None,
        IVec2::new(16, 16),
        &tilemap,
        &atlas
    ));
}

#[test]
fn placement_collision_trigger_box_is_valid() {
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();
    let trigger_box = CollisionBox::trigger_box(UVec2::new(16, 16));

    assert!(can_place_collision_box_at_position(
        Some(&trigger_box),
        IVec2::new(0, 0),
        &tilemap,
        &atlas
    ));
}

#[test]
fn placement_collision_detects_solid_tile_overlap() {
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();
    let solid_box = CollisionBox::solid_box(UVec2::new(16, 16));

    // (16, 0) is a solid tile in create_collision_test_tilemap()
    assert!(!can_place_collision_box_at_position(
        Some(&solid_box),
        IVec2::new(16, 0),
        &tilemap,
        &atlas
    ));
}

#[test]
fn placement_collision_allows_non_solid_tile_overlap() {
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();
    let solid_box = CollisionBox::solid_box(UVec2::new(16, 16));

    // (0, 32) is a floor tile in create_collision_test_tilemap()
    assert!(can_place_collision_box_at_position(
        Some(&solid_box),
        IVec2::new(0, 32),
        &tilemap,
        &atlas
    ));
}

#[test]
fn placement_collision_rejects_negative_position() {
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();
    let solid_box = CollisionBox::solid_box(UVec2::new(16, 16));

    assert!(!can_place_collision_box_at_position(
        Some(&solid_box),
        IVec2::new(-1, 0),
        &tilemap,
        &atlas
    ));
    assert!(!can_place_collision_box_at_position(
        Some(&solid_box),
        IVec2::new(0, -1),
        &tilemap,
        &atlas
    ));
}

#[test]
fn placement_collision_rejects_out_of_bounds_tiles() {
    let tilemap = create_collision_test_tilemap();
    let atlas = create_collision_test_atlas();
    let solid_box = CollisionBox::solid_box(UVec2::new(16, 16));

    // 4x4 map with 16px tiles => valid x range [0..63]
    assert!(!can_place_collision_box_at_position(
        Some(&solid_box),
        IVec2::new(64, 0),
        &tilemap,
        &atlas
    ));
}
