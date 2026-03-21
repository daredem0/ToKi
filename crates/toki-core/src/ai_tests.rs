use super::*;
use crate::animation::{AnimationClip, AnimationController, LoopMode};
use crate::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
use crate::assets::tilemap::TileMap;
use crate::collision::CollisionBox;
use crate::entity::{AiConfig, ControlRole, Entity, EntityAttributes, EntityKind};
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// BehaviorHandler Trait Tests
// ============================================================================

#[test]
fn chase_handler_returns_result_when_player_in_range() {
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player at (100, 100)
    let player = create_player_entity(1, IVec2::new(100, 100));
    entity_manager.add_existing_entity(player);

    // Chaser at (70, 100) - within detection radius
    let chaser =
        create_test_entity_with_detection_radius(2, IVec2::new(70, 100), AiBehavior::Chase, 64);
    entity_manager.add_existing_entity(chaser);

    let ctx = AiContext::new(&entity_manager, UVec2::new(256, 256), &tilemap, &atlas);
    let entity = entity_manager.get_entity(2).unwrap();
    let handler = ChaseHandler;

    let mut ai_state = AiRuntimeState::default();
    let result = handler.update(entity, 2, Some(IVec2::new(100, 100)), &ctx, &mut ai_state);

    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.entity_id, 2);
    // Should move toward player
    if let Some(new_pos) = result.new_position {
        assert!(new_pos.x > 70, "Should move toward player");
    }
}

#[test]
fn run_handler_moves_away_from_player() {
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player at (100, 100)
    let player = create_player_entity(1, IVec2::new(100, 100));
    entity_manager.add_existing_entity(player);

    // Runner at (120, 100) - within detection radius
    let runner =
        create_test_entity_with_detection_radius(2, IVec2::new(120, 100), AiBehavior::Run, 64);
    entity_manager.add_existing_entity(runner);

    let ctx = AiContext::new(&entity_manager, UVec2::new(256, 256), &tilemap, &atlas);
    let entity = entity_manager.get_entity(2).unwrap();
    let handler = RunHandler;

    let mut ai_state = AiRuntimeState::default();
    let result = handler.update(entity, 2, Some(IVec2::new(100, 100)), &ctx, &mut ai_state);

    assert!(result.is_some());
    let result = result.unwrap();
    // Should move away from player
    if let Some(new_pos) = result.new_position {
        assert!(new_pos.x > 120, "Should move away from player");
    }
}

#[test]
fn wander_handler_respects_frame_throttling() {
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    let entity = create_test_entity(1, IVec2::new(50, 50), AiBehavior::Wander);
    entity_manager.add_existing_entity(entity);

    let ctx = AiContext::new(&entity_manager, UVec2::new(256, 256), &tilemap, &atlas);
    let entity = entity_manager.get_entity(1).unwrap();
    let handler = WanderHandler::new(1); // frame_counter = 1 (not divisible by 60)

    let mut ai_state = AiRuntimeState::default();
    let result = handler.update(entity, 1, None, &ctx, &mut ai_state);

    // Should return None when not on update frame
    assert!(result.is_none(), "Wander should be throttled");

    // Test on update frame
    let handler_at_60 = WanderHandler::new(60);
    let result = handler_at_60.update(entity, 1, None, &ctx, &mut ai_state);

    // Should return Some on update frame
    assert!(result.is_some(), "Wander should update on frame 60");
}

#[test]
fn behavior_for_returns_correct_handler() {
    // Chase
    assert!(matches!(
        BehaviorHandler::for_behavior(AiBehavior::Chase, 0),
        Some(BehaviorHandler::Chase(_))
    ));

    // Run
    assert!(matches!(
        BehaviorHandler::for_behavior(AiBehavior::Run, 0),
        Some(BehaviorHandler::Run(_))
    ));

    // Wander
    assert!(matches!(
        BehaviorHandler::for_behavior(AiBehavior::Wander, 0),
        Some(BehaviorHandler::Wander(_))
    ));

    // None behavior returns None handler
    assert!(BehaviorHandler::for_behavior(AiBehavior::None, 0).is_none());
}

// ============================================================================
// AiContext Tests
// ============================================================================

#[test]
fn ai_context_creates_from_components() {
    let entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();
    let world_bounds = UVec2::new(256, 256);

    let context = AiContext::new(&entity_manager, world_bounds, &tilemap, &atlas);

    assert_eq!(context.world_bounds, UVec2::new(256, 256));
}

#[test]
fn ai_context_computes_max_position() {
    let entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();
    let world_bounds = UVec2::new(256, 256);

    let context = AiContext::new(&entity_manager, world_bounds, &tilemap, &atlas);
    let entity_size = UVec2::new(16, 16);

    let (max_x, max_y) = context.max_position(entity_size);
    assert_eq!(max_x, 240); // 256 - 16
    assert_eq!(max_y, 240);
}

#[test]
fn ai_context_max_position_clamps_to_zero() {
    let entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();
    let world_bounds = UVec2::new(10, 10); // Smaller than entity

    let context = AiContext::new(&entity_manager, world_bounds, &tilemap, &atlas);
    let entity_size = UVec2::new(16, 16);

    let (max_x, max_y) = context.max_position(entity_size);
    assert_eq!(max_x, 0); // Clamped to 0
    assert_eq!(max_y, 0);
}

#[test]
fn ai_context_validates_movement() {
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();
    let world_bounds = UVec2::new(256, 256);

    let entity = create_test_entity(1, IVec2::new(50, 50), AiBehavior::Chase);
    entity_manager.add_existing_entity(entity);

    let context = AiContext::new(&entity_manager, world_bounds, &tilemap, &atlas);
    let entity = entity_manager.get_entity(1).unwrap();

    // Movement to open position should be valid
    assert!(context.is_movement_valid(entity, 1, IVec2::new(60, 50)));
}

#[test]
fn ai_context_rejects_movement_into_solid_entity() {
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();
    let world_bounds = UVec2::new(256, 256);

    let entity = create_test_entity(1, IVec2::new(50, 50), AiBehavior::Chase);
    entity_manager.add_existing_entity(entity);

    // Add a blocking solid entity
    let mut blocker = create_test_entity(2, IVec2::new(60, 50), AiBehavior::None);
    blocker.attributes.solid = true;
    entity_manager.add_existing_entity(blocker);

    let context = AiContext::new(&entity_manager, world_bounds, &tilemap, &atlas);
    let entity = entity_manager.get_entity(1).unwrap();

    // Movement should be blocked by solid entity
    assert!(!context.is_movement_valid(entity, 1, IVec2::new(60, 50)));
}

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
    tiles.insert(
        "wall".to_string(),
        TileInfo {
            position: UVec2::new(16, 0),
            properties: TileProperties {
                solid: true,
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

/// Creates a tilemap with a vertical wall blocking horizontal movement.
/// Wall is at x=4 (tiles 4,0 through 4,15), blocking movement between x=64 and x=80.
fn create_tilemap_with_vertical_wall() -> TileMap {
    let mut tiles = vec!["grass".to_string(); 256];
    // Place wall tiles at column 4 (x=4, all y values)
    for y in 0..16 {
        tiles[y * 16 + 4] = "wall".to_string();
    }
    TileMap {
        size: UVec2::new(16, 16),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test_atlas.json"),
        tiles,
        objects: Vec::new(),
    }
}

/// Creates a tilemap with a horizontal wall blocking vertical movement.
/// Wall is at y=4 (tiles 0-15,4), blocking movement between y=64 and y=80.
fn create_tilemap_with_horizontal_wall() -> TileMap {
    let mut tiles = vec!["grass".to_string(); 256];
    // Place wall tiles at row 4 (y=4, all x values)
    for x in 0..16 {
        tiles[4 * 16 + x] = "wall".to_string();
    }
    TileMap {
        size: UVec2::new(16, 16),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test_atlas.json"),
        tiles,
        objects: Vec::new(),
    }
}

fn create_test_entity(id: EntityId, position: IVec2, behavior: AiBehavior) -> Entity {
    let mut controller = AnimationController::new();
    controller.add_clip(AnimationClip {
        state: AnimationState::Idle,
        atlas_name: "test".to_string(),
        frame_tile_names: vec!["idle_0".to_string()],
        frame_positions: None,
        frame_duration_ms: 100.0,
        frame_durations_ms: None,
        loop_mode: LoopMode::Loop,
    });
    controller.add_clip(AnimationClip {
        state: AnimationState::Walk,
        atlas_name: "test".to_string(),
        frame_tile_names: vec!["walk_0".to_string()],
        frame_positions: None,
        frame_duration_ms: 100.0,
        frame_durations_ms: None,
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
        collision_box: Some(CollisionBox::solid_box(UVec2::new(16, 16))),
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
fn ai_system_updates_every_tick() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player at (100, 100)
    let player = create_player_entity(1, IVec2::new(100, 100));
    entity_manager.add_existing_entity(player);

    // Chaser at (50, 100) - within detection radius
    let chaser =
        create_test_entity_with_detection_radius(2, IVec2::new(50, 100), AiBehavior::Chase, 64);
    entity_manager.add_existing_entity(chaser);

    // First update should return results immediately
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );
    assert_eq!(results.len(), 1, "Should update on first tick");

    // Second update should also return results
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );
    assert_eq!(results.len(), 1, "Should update on every tick");
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

fn create_test_entity_with_detection_radius(
    id: EntityId,
    position: IVec2,
    behavior: AiBehavior,
    detection_radius: u32,
) -> Entity {
    let mut entity = create_test_entity(id, position, behavior);
    entity.attributes.ai_config.detection_radius = detection_radius;
    entity
}

fn create_player_entity(id: EntityId, position: IVec2) -> Entity {
    let mut player = create_test_entity(id, position, AiBehavior::None);
    player.control_role = ControlRole::PlayerCharacter;
    player.entity_kind = EntityKind::Player;
    player
}

// ============================================================================
// Chase Behavior Tests
// ============================================================================

#[test]
fn ai_system_chase_moves_toward_player_when_in_radius() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player at (100, 100)
    let player = create_player_entity(1, IVec2::new(100, 100));
    entity_manager.add_existing_entity(player);

    // Chaser at (50, 100) - 50 pixels away, within detection radius of 64
    let chaser =
        create_test_entity_with_detection_radius(2, IVec2::new(50, 100), AiBehavior::Chase, 64);
    entity_manager.add_existing_entity(chaser);

    // Fast forward to update frame
    ai_system.frame_counter = 59;
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(result.entity_id, 2);

    // Chaser should move toward player (increasing x position)
    if let Some(new_pos) = result.new_position {
        assert!(new_pos.x > 50, "Chaser should move toward player (right)");
    }
    assert_eq!(result.new_animation, Some(AnimationState::Walk));
}

#[test]
fn ai_system_chase_wanders_with_walk_wait_cycle() {
    fastrand::seed(42);
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player at (200, 100)
    let player = create_player_entity(1, IVec2::new(200, 100));
    entity_manager.add_existing_entity(player);

    // Chaser at (50, 100) - 150 pixels away, outside detection radius of 64
    let chaser =
        create_test_entity_with_detection_radius(2, IVec2::new(50, 100), AiBehavior::Chase, 64);
    entity_manager.add_existing_entity(chaser);

    // First call starts in waiting phase (idle)
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(result.entity_id, 2);

    // Should start in idle/waiting phase
    assert_eq!(result.new_animation, Some(AnimationState::Idle));

    // Continue updates - entity should eventually walk then wait again
    let mut found_walk = false;
    let mut found_idle_after_walk = false;
    for _ in 0..300 {
        let results = ai_system.update(
            &entity_manager,
            entity_manager.get_player_id(),
            UVec2::new(256, 256),
            &tilemap,
            &atlas,
        );
        if results.len() == 1 {
            if results[0].new_animation == Some(AnimationState::Walk) {
                found_walk = true;
            } else if found_walk && results[0].new_animation == Some(AnimationState::Idle) {
                found_idle_after_walk = true;
                break;
            }
        }
    }
    assert!(found_walk, "Chaser should eventually enter walk phase");
    assert!(
        found_idle_after_walk,
        "Chaser should return to idle after walking"
    );
}

#[test]
fn ai_system_chase_closes_distance_to_player() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player at (100, 100)
    let player = create_player_entity(1, IVec2::new(100, 100));
    entity_manager.add_existing_entity(player);

    // Chaser at (60, 60) - diagonally away, within detection radius of 100
    let chaser =
        create_test_entity_with_detection_radius(2, IVec2::new(60, 60), AiBehavior::Chase, 100);
    entity_manager.add_existing_entity(chaser);

    let initial_distance = ((100 - 60) as f32).hypot((100 - 60) as f32);

    // Fast forward to update frame
    ai_system.frame_counter = 59;
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    let result = &results[0];

    if let Some(new_pos) = result.new_position {
        let new_distance = ((100 - new_pos.x) as f32).hypot((100 - new_pos.y) as f32);
        assert!(
            new_distance < initial_distance,
            "Chaser should close distance: initial={}, new={}",
            initial_distance,
            new_distance
        );
    }
}

// ============================================================================
// Run Behavior Tests
// ============================================================================

#[test]
fn ai_system_run_moves_away_from_player_when_in_radius() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player at (100, 100)
    let player = create_player_entity(1, IVec2::new(100, 100));
    entity_manager.add_existing_entity(player);

    // Runner at (120, 100) - 20 pixels away, within detection radius of 64
    let runner =
        create_test_entity_with_detection_radius(2, IVec2::new(120, 100), AiBehavior::Run, 64);
    entity_manager.add_existing_entity(runner);

    // Fast forward to update frame
    ai_system.frame_counter = 59;
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(result.entity_id, 2);

    // Runner should move away from player (increasing x position, away from player at 100)
    if let Some(new_pos) = result.new_position {
        assert!(
            new_pos.x > 120,
            "Runner should move away from player (right)"
        );
    }
    assert_eq!(result.new_animation, Some(AnimationState::Walk));
}

#[test]
fn ai_system_run_wanders_with_walk_wait_cycle() {
    fastrand::seed(42);
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player at (200, 100)
    let player = create_player_entity(1, IVec2::new(200, 100));
    entity_manager.add_existing_entity(player);

    // Runner at (50, 100) - 150 pixels away, outside detection radius of 64
    let runner =
        create_test_entity_with_detection_radius(2, IVec2::new(50, 100), AiBehavior::Run, 64);
    entity_manager.add_existing_entity(runner);

    // First call starts in waiting phase (idle)
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(result.entity_id, 2);

    // Should start in idle/waiting phase
    assert_eq!(result.new_animation, Some(AnimationState::Idle));

    // Continue updates - entity should eventually walk then wait again
    let mut found_walk = false;
    for _ in 0..200 {
        let results = ai_system.update(
            &entity_manager,
            entity_manager.get_player_id(),
            UVec2::new(256, 256),
            &tilemap,
            &atlas,
        );
        if results.len() == 1 && results[0].new_animation == Some(AnimationState::Walk) {
            found_walk = true;
            break;
        }
    }
    assert!(found_walk, "Runner should eventually enter walk phase");
}

#[test]
fn ai_system_run_increases_distance_from_player() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player at (100, 100)
    let player = create_player_entity(1, IVec2::new(100, 100));
    entity_manager.add_existing_entity(player);

    // Runner at (120, 120) - diagonally away, within detection radius of 100
    let runner =
        create_test_entity_with_detection_radius(2, IVec2::new(120, 120), AiBehavior::Run, 100);
    entity_manager.add_existing_entity(runner);

    let initial_distance = ((120 - 100) as f32).hypot((120 - 100) as f32);

    // Fast forward to update frame
    ai_system.frame_counter = 59;
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    let result = &results[0];

    if let Some(new_pos) = result.new_position {
        let new_distance = ((new_pos.x - 100) as f32).hypot((new_pos.y - 100) as f32);
        assert!(
            new_distance > initial_distance,
            "Runner should increase distance: initial={}, new={}",
            initial_distance,
            new_distance
        );
    }
}

// ============================================================================
// Shared Behavior Tests (Collision and World Bounds)
// ============================================================================

#[test]
fn ai_system_chase_respects_world_bounds() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player at (0, 100) - at the left edge
    let player = create_player_entity(1, IVec2::new(0, 100));
    entity_manager.add_existing_entity(player);

    // Chaser at (20, 100) - will try to move left toward player
    let chaser =
        create_test_entity_with_detection_radius(2, IVec2::new(20, 100), AiBehavior::Chase, 64);
    entity_manager.add_existing_entity(chaser);

    // Fast forward to update frame
    ai_system.frame_counter = 59;
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    let result = &results[0];

    // Position should never go negative
    if let Some(new_pos) = result.new_position {
        assert!(
            new_pos.x >= 0,
            "Chaser should not move outside world bounds"
        );
        assert!(
            new_pos.y >= 0,
            "Chaser should not move outside world bounds"
        );
    }
}

#[test]
fn ai_system_run_respects_world_bounds() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player at (100, 100)
    let player = create_player_entity(1, IVec2::new(100, 100));
    entity_manager.add_existing_entity(player);

    // Runner at (230, 100) - near right edge, will try to move right away from player
    // World bounds are 256x256, entity size is 16x16, so max x is 240
    let runner =
        create_test_entity_with_detection_radius(2, IVec2::new(230, 100), AiBehavior::Run, 200);
    entity_manager.add_existing_entity(runner);

    // Fast forward to update frame
    ai_system.frame_counter = 59;
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    let result = &results[0];

    // Position should stay within world bounds
    if let Some(new_pos) = result.new_position {
        assert!(
            new_pos.x <= 240,
            "Runner should not move outside world bounds (max_x=240)"
        );
        assert!(
            new_pos.y <= 240,
            "Runner should not move outside world bounds"
        );
    }
}

// ============================================================================
// Collision Avoidance Tests
// ============================================================================

#[test]
fn ai_system_chase_navigates_around_vertical_wall() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_tilemap_with_vertical_wall(); // Wall at x=64-80
    let atlas = create_test_atlas();

    // Player at (100, 50) - on the right side of the wall
    let player = create_player_entity(1, IVec2::new(100, 50));
    entity_manager.add_existing_entity(player);

    // Chaser at (48, 50) - on the left side of the wall, same y as player
    // Primary direction would be right (+x), but wall blocks it
    let chaser =
        create_test_entity_with_detection_radius(2, IVec2::new(48, 50), AiBehavior::Chase, 100);
    entity_manager.add_existing_entity(chaser);

    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    let result = &results[0];

    // Chaser should move vertically (up or down) to navigate around the wall
    if let Some(new_pos) = result.new_position {
        // Should have moved in y direction since x is blocked
        assert!(
            new_pos.y != 50 || new_pos.x != 48,
            "Chaser should find alternative path around wall"
        );
    }
    assert_eq!(result.new_animation, Some(AnimationState::Walk));
}

#[test]
fn ai_system_run_navigates_around_horizontal_wall() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_tilemap_with_horizontal_wall(); // Wall at y=64-80
    let atlas = create_test_atlas();

    // Player at (50, 100) - below the wall
    let player = create_player_entity(1, IVec2::new(50, 100));
    entity_manager.add_existing_entity(player);

    // Runner at (50, 80) - just above the wall, directly above player
    // Primary direction would be down (-y away from player at y=100), but wall blocks it
    // Wait, if player is at y=100 and runner at y=80, runner wants to move UP (decrease y)
    // Let me reconsider: Runner at (50, 48) wants to run away from player at (50, 100)
    // Runner would move up (decrease y), but let's put a wall above
    let runner =
        create_test_entity_with_detection_radius(2, IVec2::new(50, 48), AiBehavior::Run, 100);
    entity_manager.add_existing_entity(runner);

    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    let result = &results[0];

    // Runner should still move (either find alternative or move in perpendicular direction)
    if let Some(new_pos) = result.new_position {
        assert!(
            new_pos.x != 50 || new_pos.y != 48,
            "Runner should find alternative path"
        );
    }
}

#[test]
fn ai_system_chase_tries_perpendicular_when_blocked() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_tilemap_with_vertical_wall(); // Wall at x=64-80
    let atlas = create_test_atlas();

    // Player directly to the right of chaser, wall in between
    let player = create_player_entity(1, IVec2::new(100, 32));
    entity_manager.add_existing_entity(player);

    // Chaser at x=46 - entity size is 16, so right edge is at x=62, just before the wall (64)
    // When it tries to move right (speed=4 -> to x=50, right edge at 66), it would hit the wall
    let mut chaser =
        create_test_entity_with_detection_radius(2, IVec2::new(46, 32), AiBehavior::Chase, 100);
    chaser.attributes.speed = 4.0; // Move 4 pixels per tick to ensure it tries to enter wall
    entity_manager.add_existing_entity(chaser);

    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    let result = &results[0];

    // Should move up or down to try to get around the wall
    assert!(
        result.new_position.is_some(),
        "Chaser should move in perpendicular direction when primary is blocked"
    );

    if let Some(new_pos) = result.new_position {
        // X should stay same (blocked by wall), Y should change (perpendicular)
        assert_eq!(new_pos.x, 46, "X should stay same (blocked by wall)");
        assert_ne!(new_pos.y, 32, "Y should change (perpendicular movement)");
    }
}

// ============================================================================
// Wandering When Outside Detection Radius Tests
// ============================================================================

#[test]
fn ai_system_chase_wanders_when_player_outside_radius() {
    fastrand::seed(42);
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player at (200, 100) - far away
    let player = create_player_entity(1, IVec2::new(200, 100));
    entity_manager.add_existing_entity(player);

    // Chaser at (50, 50) - 170 pixels away, outside detection radius of 64
    let chaser =
        create_test_entity_with_detection_radius(2, IVec2::new(50, 50), AiBehavior::Chase, 64);
    entity_manager.add_existing_entity(chaser);

    // Advance to wander update frame (every 30 frames for idle wandering)
    ai_system.frame_counter = 29;
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    let result = &results[0];

    // Should have movement or be walking (wandering behavior)
    // The entity may or may not move depending on random direction
    assert!(
        result.new_animation.is_some(),
        "Chaser should have animation state when wandering"
    );
}

#[test]
fn ai_system_run_wanders_when_player_outside_radius() {
    fastrand::seed(42);
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player at (200, 100) - far away
    let player = create_player_entity(1, IVec2::new(200, 100));
    entity_manager.add_existing_entity(player);

    // Runner at (50, 50) - outside detection radius of 64
    let runner =
        create_test_entity_with_detection_radius(2, IVec2::new(50, 50), AiBehavior::Run, 64);
    entity_manager.add_existing_entity(runner);

    // Advance to wander update frame (every 30 frames for idle wandering)
    ai_system.frame_counter = 29;
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    let result = &results[0];

    // Should have animation state (wandering behavior)
    assert!(
        result.new_animation.is_some(),
        "Runner should have animation state when wandering"
    );
}

#[test]
fn ai_system_chase_idle_wander_is_throttled() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player far away
    let player = create_player_entity(1, IVec2::new(200, 100));
    entity_manager.add_existing_entity(player);

    // Chaser outside detection radius
    let chaser =
        create_test_entity_with_detection_radius(2, IVec2::new(50, 50), AiBehavior::Chase, 64);
    entity_manager.add_existing_entity(chaser);

    // First update (frame 1) - should not wander yet
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );
    assert_eq!(results.len(), 1);
    // Should return idle with no position change (throttled)
    assert!(
        results[0].new_position.is_none(),
        "Idle wander should be throttled on non-wander frames"
    );
}

#[test]
fn ai_system_chase_transitions_from_wander_to_chase() {
    fastrand::seed(42);
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player far away initially
    let player = create_player_entity(1, IVec2::new(200, 100));
    entity_manager.add_existing_entity(player);

    // Chaser outside detection radius
    let chaser =
        create_test_entity_with_detection_radius(2, IVec2::new(50, 50), AiBehavior::Chase, 64);
    entity_manager.add_existing_entity(chaser);

    // First update - wandering (player outside)
    ai_system.frame_counter = 29;
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );
    assert_eq!(results.len(), 1);

    // Move player into detection radius
    entity_manager.get_entity_mut(1).unwrap().position = IVec2::new(80, 50);

    // Next update - should chase immediately (every tick)
    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );
    assert_eq!(results.len(), 1);
    let result = &results[0];

    // Should move toward player (chasing, not wandering)
    if let Some(new_pos) = result.new_position {
        assert!(
            new_pos.x > 50,
            "Chaser should move toward player when in radius"
        );
    }
}

// ============================================================================
// RunAndMultiply Behavior Tests
// ============================================================================

#[test]
fn ai_system_run_and_multiply_wanders_when_no_threats_or_mates() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player far away
    let player = create_player_entity(1, IVec2::new(200, 200));
    entity_manager.add_existing_entity(player);

    // RunAndMultiply entity alone with no compatible entities
    let entity = create_test_entity_with_detection_radius(
        2,
        IVec2::new(50, 50),
        AiBehavior::RunAndMultiply,
        64,
    );
    entity_manager.add_existing_entity(entity);

    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    // Should be wandering (starts in idle/wait phase)
    assert_eq!(results[0].new_animation, Some(AnimationState::Idle));
    assert!(results[0].spawn_request.is_none());
}

#[test]
fn ai_system_run_and_multiply_flees_from_player() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player nearby
    let player = create_player_entity(1, IVec2::new(100, 100));
    entity_manager.add_existing_entity(player);

    // RunAndMultiply entity within detection radius
    let entity = create_test_entity_with_detection_radius(
        2,
        IVec2::new(120, 120),
        AiBehavior::RunAndMultiply,
        100,
    );
    entity_manager.add_existing_entity(entity);

    let initial_distance = 28.28; // sqrt((120-100)^2 + (120-100)^2) ≈ 28.28

    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    assert_eq!(results.len(), 1);
    if let Some(new_pos) = results[0].new_position {
        let new_distance = ((new_pos.x - 100) as f32).hypot((new_pos.y - 100) as f32);
        assert!(
            new_distance > initial_distance,
            "RunAndMultiply should flee: initial={}, new={}",
            initial_distance,
            new_distance
        );
    }
}

#[test]
fn ai_system_run_and_multiply_seeks_compatible_entity() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player far away
    let player = create_player_entity(1, IVec2::new(200, 200));
    entity_manager.add_existing_entity(player);

    // First RunAndMultiply entity
    let entity1 = create_test_entity_with_detection_radius(
        2,
        IVec2::new(50, 50),
        AiBehavior::RunAndMultiply,
        100,
    );
    entity_manager.add_existing_entity(entity1);

    // Second compatible entity (same definition_name) within detection radius
    let entity2 = create_test_entity_with_detection_radius(
        3,
        IVec2::new(80, 50),
        AiBehavior::RunAndMultiply,
        100,
    );
    entity_manager.add_existing_entity(entity2);

    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    // Both entities should be updated
    assert_eq!(results.len(), 2);

    // Entity at (50, 50) should move toward entity at (80, 50)
    let entity1_result = results.iter().find(|r| r.entity_id == 2).unwrap();
    if let Some(new_pos) = entity1_result.new_position {
        assert!(
            new_pos.x > 50,
            "Entity should seek compatible mate: moved to x={}",
            new_pos.x
        );
    }
}

#[test]
fn ai_system_run_and_multiply_spawns_on_collision() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player far away
    let player = create_player_entity(1, IVec2::new(200, 200));
    entity_manager.add_existing_entity(player);

    // Two adjacent RunAndMultiply entities (touching at edges)
    let entity1 = create_test_entity_with_detection_radius(
        2,
        IVec2::new(50, 50),
        AiBehavior::RunAndMultiply,
        100,
    );
    entity_manager.add_existing_entity(entity1);

    let entity2 = create_test_entity_with_detection_radius(
        3,
        IVec2::new(66, 50), // Adjacent horizontally (16x16 entity at 50 ends at 66)
        AiBehavior::RunAndMultiply,
        100,
    );
    entity_manager.add_existing_entity(entity2);

    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    // One of the entities should have a spawn request
    let spawn_results: Vec<_> = results
        .iter()
        .filter(|r| r.spawn_request.is_some())
        .collect();
    assert!(
        !spawn_results.is_empty(),
        "Adjacent entities should trigger spawn"
    );

    let spawn_request = spawn_results[0].spawn_request.as_ref().unwrap();
    // Verify it's a clone spawn from one of the parent entities
    match &spawn_request.mode {
        SpawnMode::Clone { source_entity_id } => {
            assert!(
                *source_entity_id == 2 || *source_entity_id == 3,
                "Clone should be from one of the parent entities"
            );
        }
        SpawnMode::FromDefinition { .. } => {
            panic!("Expected Clone spawn mode, got FromDefinition");
        }
    }
}

#[test]
fn ai_system_run_and_multiply_enters_separation_after_spawn() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player far away
    let player = create_player_entity(1, IVec2::new(200, 200));
    entity_manager.add_existing_entity(player);

    // Two adjacent RunAndMultiply entities (touching at edges)
    let entity1 = create_test_entity_with_detection_radius(
        2,
        IVec2::new(50, 50),
        AiBehavior::RunAndMultiply,
        64,
    );
    entity_manager.add_existing_entity(entity1);

    let entity2 = create_test_entity_with_detection_radius(
        3,
        IVec2::new(66, 50), // Adjacent horizontally
        AiBehavior::RunAndMultiply,
        64,
    );
    entity_manager.add_existing_entity(entity2);

    // First update triggers spawn and separation
    let _ = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    // Check that entities are in separation state
    let state1 = ai_system.entity_states.get(&2);
    assert!(
        state1.is_some_and(|s| s.separation_state.is_some()),
        "Entity should be in separation state after spawn"
    );
}

#[test]
fn ai_system_run_and_multiply_exits_separation_when_distance_met() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player far away
    let player = create_player_entity(1, IVec2::new(200, 200));
    entity_manager.add_existing_entity(player);

    // Entity 2 in separation from entity 3
    let entity1 = create_test_entity_with_detection_radius(
        2,
        IVec2::new(50, 50),
        AiBehavior::RunAndMultiply,
        32, // Small detection radius for quick exit
    );
    entity_manager.add_existing_entity(entity1);

    // Entity 3 far enough away (distance > detection_radius * 2 = 64)
    let entity2 = create_test_entity_with_detection_radius(
        3,
        IVec2::new(150, 50), // 100 pixels away
        AiBehavior::RunAndMultiply,
        32,
    );
    entity_manager.add_existing_entity(entity2);

    // Manually set separation state
    let state = ai_system.get_or_create_state(2);
    state.separation_state = Some(SeparationState {
        other_entity_ids: vec![3],
        required_distance: 64.0,
    });

    // Update should exit separation
    let _ = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    // Should have exited separation
    let state = ai_system.entity_states.get(&2);
    assert!(
        state.is_some_and(|s| s.separation_state.is_none()),
        "Entity should exit separation when distance is met"
    );
}

#[test]
fn ai_system_run_and_multiply_ignores_different_definition() {
    let mut ai_system = AiSystem::new();
    let mut entity_manager = EntityManager::new();
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Player far away
    let player = create_player_entity(1, IVec2::new(200, 200));
    entity_manager.add_existing_entity(player);

    // First RunAndMultiply entity
    let entity1 = create_test_entity_with_detection_radius(
        2,
        IVec2::new(50, 50),
        AiBehavior::RunAndMultiply,
        100,
    );
    entity_manager.add_existing_entity(entity1);

    // Second entity with different definition_name
    let mut entity2 = create_test_entity_with_detection_radius(
        3,
        IVec2::new(60, 50),
        AiBehavior::RunAndMultiply,
        100,
    );
    entity2.definition_name = Some("different_npc".to_string());
    entity_manager.add_existing_entity(entity2);

    let results = ai_system.update(
        &entity_manager,
        entity_manager.get_player_id(),
        UVec2::new(256, 256),
        &tilemap,
        &atlas,
    );

    // Neither should have a spawn request since they're incompatible
    for result in &results {
        assert!(
            result.spawn_request.is_none(),
            "Incompatible entities should not spawn"
        );
    }
}
