use glam::{IVec2, UVec2};
use std::collections::HashMap;
use std::path::PathBuf;
use toki_core::animation::AnimationState;
use toki_core::assets::{
    atlas::{AtlasMeta, TileInfo, TileProperties},
    tilemap::{MapObjectInstance, TileMap},
};
use toki_core::entity::{
    AnimationClipDef, AnimationsDef, AttributesDef, AudioDef, CollisionDef, ControlRole,
    EntityDefinition, EntityFootprint, EntityGrounding, EntityKind, MovementProfile,
    MovementSoundTrigger, PickupDef, PrimaryProjectileDef, RenderingDef, ATTACK_POWER_STAT_ID,
};
use toki_core::rules::{Rule, RuleAction, RuleSet, RuleTarget, RuleTrigger};
use toki_core::sprite::{Animation, Frame, SpriteInstance, SpriteSheetMeta};
use toki_core::{
    entity::{AiBehavior, AiConfig},
    game::{
        AudioChannel, AudioEvent, GameSimulation, InputAction, InputSystem, RenderQueryService,
        RuleSystem, SceneSystem,
    },
    scene::{Scene, SceneAnchor, SceneAnchorFacing, SceneAnchorKind, ScenePlayerEntry},
    GameState, InputKey,
};

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
        objects: vec![],
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
        color_mode: toki_core::assets::atlas::ColorMode::TrueColor,
        palette: None,
        tiles,
    }
}

fn render_queries(state: &GameState) -> RenderQueryService<'_> {
    RenderQueryService::new(
        state.world().entity_manager(),
        state.world().player_id(),
        state.runtime().debug_collision_rendering(),
    )
}

fn player_position(state: &GameState) -> IVec2 {
    render_queries(state).player_position()
}

fn player_entity(state: &GameState) -> Option<&toki_core::entity::Entity> {
    state
        .world()
        .player_id()
        .and_then(|player_id| state.world().entity_manager().get_entity(player_id))
}

fn sprite_render_requests(state: &GameState) -> Vec<toki_core::sprite_render::SpriteRenderRequest> {
    render_queries(state).sprite_render_requests()
}

fn renderable_entities(
    state: &GameState,
) -> Vec<(toki_core::entity::EntityId, glam::IVec2, glam::UVec2)> {
    render_queries(state).renderable_entities()
}

fn entity_health_bars(state: &GameState) -> Vec<toki_core::game::EntityHealthBar> {
    render_queries(state).entity_health_bars()
}

fn entity_sprite_flip_x(state: &GameState, entity_id: toki_core::entity::EntityId) -> bool {
    render_queries(state).entity_sprite_flip_x(entity_id)
}

fn current_sprite_frame(
    state: &GameState,
    atlas: &AtlasMeta,
    texture_size: glam::UVec2,
) -> toki_core::sprite::SpriteFrame {
    render_queries(state).current_sprite_frame(atlas, texture_size)
}

fn active_entities(state: &GameState) -> Vec<&toki_core::entity::Entity> {
    state
        .world()
        .entity_manager()
        .active_entities()
        .iter()
        .filter_map(|&id| state.world().entity_manager().get_entity(id))
        .collect()
}

fn create_solid_test_atlas() -> AtlasMeta {
    let mut tiles = HashMap::new();
    tiles.insert(
        "floor".to_string(),
        TileInfo {
            position: UVec2::new(0, 0),
            properties: TileProperties {
                solid: true,
                trigger: false,
            },
        },
    );

    AtlasMeta {
        image: PathBuf::from("test_atlas.png"),
        tile_size: UVec2::new(16, 16),
        color_mode: toki_core::assets::atlas::ColorMode::TrueColor,
        palette: None,
        tiles,
    }
}

fn test_definition(name: &str, category: &str) -> EntityDefinition {
    EntityDefinition {
        name: name.into(),
        display_name: format!("Display {name}"),
        description: format!("Definition for {name}"),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            palette_override: None,
            static_object: None,
            grounding: Default::default(),
        },
        attributes: AttributesDef {
            health: Some(100),
            stats: std::collections::HashMap::new(),
            speed: 2.0,
            solid: true,
            active: true,
            can_move: true,
            interactable: false,
            interaction_reach: 0,
            ai_config: if category == "creature" {
                toki_core::entity::AiConfig::from_legacy_behavior(
                    toki_core::entity::AiBehavior::Wander,
                )
            } else {
                toki_core::entity::AiConfig::default()
            },
            movement_profile: if category == "human" {
                MovementProfile::PlayerWasd
            } else {
                MovementProfile::None
            },
            primary_projectile: None,
            pickup: None,
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
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "sfx_step".to_string(),
            collision_sound: Some("sfx_hit2".to_string()),
        },
        animations: AnimationsDef {
            atlas_name: "creatures".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["slime/idle_0".to_string()],
                frame_positions: None,
                frame_duration_ms: 150.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle".to_string(),
        },
        category: category.to_string(),
        tags: vec!["test".to_string()],
    }
}

fn player_definition(name: &str) -> EntityDefinition {
    EntityDefinition {
        name: name.into(),
        display_name: format!("Display {name}"),
        description: format!("Definition for {name}"),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            palette_override: None,
            static_object: None,
            grounding: Default::default(),
        },
        attributes: AttributesDef {
            health: Some(100),
            stats: HashMap::from([("health".to_string(), 100), ("attack_power".to_string(), 8)]),
            speed: 2.0,
            solid: true,
            active: true,
            can_move: true,
            interactable: false,
            interaction_reach: 0,
            ai_config: toki_core::entity::AiConfig::default(),
            movement_profile: MovementProfile::PlayerWasd,
            primary_projectile: None,
            pickup: None,
            has_inventory: true,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 16.0,
            hearing_radius: 100,
            movement_sound_trigger: MovementSoundTrigger::AnimationLoop,
            movement_sound: "sfx_step".to_string(),
            collision_sound: Some("sfx_hit2".to_string()),
        },
        animations: AnimationsDef {
            atlas_name: "players.json".to_string(),
            clips: vec![
                AnimationClipDef {
                    state: "idle_down".to_string(),
                    frame_tiles: vec!["player/walk_down_a".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 300.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
                AnimationClipDef {
                    state: "idle_up".to_string(),
                    frame_tiles: vec!["player/walk_up_a".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 300.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
                AnimationClipDef {
                    state: "idle_left".to_string(),
                    frame_tiles: vec!["player/walk_left_a".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 300.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
                AnimationClipDef {
                    state: "idle_right".to_string(),
                    frame_tiles: vec!["player/walk_right_a".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 300.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
                AnimationClipDef {
                    state: "walk_right".to_string(),
                    frame_tiles: vec![
                        "player/walk_right_a".to_string(),
                        "player/walk_right_b".to_string(),
                    ],
                    frame_positions: None,
                    frame_duration_ms: 180.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
                AnimationClipDef {
                    state: "attack_right".to_string(),
                    frame_tiles: vec![
                        "player/attack_right_a".to_string(),
                        "player/attack_right_b".to_string(),
                    ],
                    frame_positions: None,
                    frame_duration_ms: 180.0,
                    frame_durations_ms: None,
                    loop_mode: "once".to_string(),
                },
            ],
            default_state: "idle_down".to_string(),
        },
        category: "human".to_string(),
        tags: vec!["player".to_string()],
    }
}

fn scene_anchor(id: &str, position: IVec2, facing: Option<SceneAnchorFacing>) -> SceneAnchor {
    SceneAnchor {
        id: id.to_string(),
        kind: SceneAnchorKind::SpawnPoint,
        position,
        facing,
    }
}

#[test]
fn game_state_new_initializes_correctly() {
    let sprite = create_test_sprite();
    let initial_position = sprite.position;
    let game_state = GameState::new(sprite);

    assert_eq!(player_position(&game_state), initial_position);
    // Test EntityManager integration
    assert_eq!(active_entities(&game_state).len(), 1);
    assert_eq!(game_state.world().player_id(), Some(1));
    assert!(player_entity(&game_state).is_some());
    assert_eq!(
        player_entity(&game_state).unwrap().position,
        initial_position
    );
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
    let initial_position = player_position(&game_state);

    // Press a key
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);

    // Should not move yet (requires update)
    assert_eq!(player_position(&game_state), initial_position);

    // Update should process the input
    let world_bounds = UVec2::new(1000, 1000);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();
    let result = GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    assert!(result.player_moved);
    assert!(player_position(&game_state).x > initial_position.x);

    // Release the key
    InputSystem::handle_key_release(game_state.runtime_mut(), InputKey::Right);

    // Another update should not move further
    let position_after_release = player_position(&game_state);
    let result_again = GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    assert!(!result_again.player_moved);
    assert_eq!(player_position(&game_state), position_after_release);
}

#[test]
fn game_state_movement_up() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let initial_position = player_position(&game_state);

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Up);
    let world_bounds = UVec2::new(1000, 1000);
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        world_bounds,
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(result.player_moved);
    assert_eq!(player_position(&game_state).x, initial_position.x); // X unchanged
    assert!(player_position(&game_state).y < initial_position.y); // Y decreased (up)
    assert_eq!(player_position(&game_state).y, initial_position.y - 2); // Moved 2 pixels (default speed)
}

#[test]
fn game_state_movement_down() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let initial_position = player_position(&game_state);

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Down);
    let world_bounds = UVec2::new(1000, 1000);
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        world_bounds,
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(result.player_moved);
    assert_eq!(player_position(&game_state).x, initial_position.x); // X unchanged
    assert!(player_position(&game_state).y > initial_position.y); // Y increased (down)
    assert_eq!(player_position(&game_state).y, initial_position.y + 2); // Moved 2 pixels (default speed)
}

#[test]
fn game_state_movement_left() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let initial_position = player_position(&game_state);

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Left);
    let world_bounds = UVec2::new(1000, 1000);
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        world_bounds,
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(result.player_moved);
    assert!(player_position(&game_state).x < initial_position.x); // X decreased (left)
    assert_eq!(player_position(&game_state).y, initial_position.y); // Y unchanged
    assert_eq!(player_position(&game_state).x, initial_position.x - 2); // Moved 2 pixels (default speed)
}

#[test]
fn game_state_movement_right() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let initial_position = player_position(&game_state);

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    let world_bounds = UVec2::new(1000, 1000);
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        world_bounds,
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(result.player_moved);
    assert!(player_position(&game_state).x > initial_position.x); // X increased (right)
    assert_eq!(player_position(&game_state).y, initial_position.y); // Y unchanged
    assert_eq!(player_position(&game_state).x, initial_position.x + 2); // Moved 2 pixels (default speed)
}

#[test]
fn game_state_diagonal_movement() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let initial_position = player_position(&game_state);

    // Press both up and right
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Up);
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);

    let world_bounds = UVec2::new(1000, 1000);
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        world_bounds,
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(result.player_moved);
    assert_eq!(player_position(&game_state).x, initial_position.x + 2); // Moved right (2 pixels)
    assert_eq!(player_position(&game_state).y, initial_position.y - 2); // Moved up (2 pixels)
}

#[test]
fn game_state_world_bounds_left_boundary() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    // Move to near left edge
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Left);
    let world_bounds = UVec2::new(1000, 1000);

    // Move left repeatedly until at boundary
    for _ in 0..100 {
        GameSimulation::tick_fixed(
            &mut game_state,
            world_bounds,
            &create_test_tilemap(),
            &create_test_atlas(),
        );
    }

    // Should be clamped at 0
    assert_eq!(player_position(&game_state).x, 0);

    // One more update should not move further
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        world_bounds,
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(!result.player_moved); // Should not report movement when clamped
    assert_eq!(player_position(&game_state).x, 0);
}

#[test]
fn game_state_world_bounds_top_boundary() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    // Move to near top edge
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Up);
    let world_bounds = UVec2::new(1000, 1000);

    // Move up repeatedly until at boundary
    for _ in 0..100 {
        GameSimulation::tick_fixed(
            &mut game_state,
            world_bounds,
            &create_test_tilemap(),
            &create_test_atlas(),
        );
    }

    // Should be clamped at 0
    assert_eq!(player_position(&game_state).y, 0);

    // One more update should not move further
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        world_bounds,
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(!result.player_moved); // Should not report movement when clamped
    assert_eq!(player_position(&game_state).y, 0);
}

#[test]
fn game_state_world_bounds_right_boundary() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    let world_bounds = UVec2::new(100, 1000); // Small world width

    // Move right repeatedly until at boundary
    for _ in 0..200 {
        GameSimulation::tick_fixed(
            &mut game_state,
            world_bounds,
            &create_test_tilemap(),
            &create_test_atlas(),
        );
    }

    // Should be clamped at world_width - entity_size (16x16 from sprite)
    let player_size = player_entity(&game_state).unwrap().size;
    let expected_max_x = world_bounds.x as i32 - player_size.x as i32;
    assert_eq!(player_position(&game_state).x, expected_max_x);

    // One more update should not move further
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        world_bounds,
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(!result.player_moved); // Should not report movement when clamped
}

#[test]
fn game_state_world_bounds_clamp_uses_footprint_not_full_sprite_size() {
    let mut scene = Scene::new("Main Scene".to_string());
    scene.anchors.push(SceneAnchor {
        id: "spawn".to_string(),
        kind: SceneAnchorKind::SpawnPoint,
        position: IVec2::ZERO,
        facing: None,
    });
    scene.player_entry = Some(ScenePlayerEntry {
        entity_definition_name: "foot_player".into(),
        spawn_point_id: "spawn".to_string(),
    });

    let mut definition = player_definition("foot_player");
    definition.rendering.size = [32, 48];
    definition.rendering.grounding = EntityGrounding {
        origin: Some([16, 47]),
        footprint: Some(EntityFootprint::new([8, 40], [16, 8])),
    };
    definition.collision.offset = [0, 0];
    definition.collision.size = [32, 48];

    let mut game_state =
        toki_core::project_content::build_game_state_from_scene(scene, [definition])
            .expect("scene should load");

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    let world_bounds = UVec2::new(32, 128);
    for _ in 0..40 {
        GameSimulation::tick_fixed(
            &mut game_state,
            world_bounds,
            &create_test_tilemap(),
            &create_test_atlas(),
        );
    }

    assert_eq!(player_position(&game_state).x, 8);
}

#[test]
fn game_state_world_bounds_bottom_boundary() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Down);
    let world_bounds = UVec2::new(1000, 100); // Small world height

    // Move down repeatedly until at boundary
    for _ in 0..200 {
        GameSimulation::tick_fixed(
            &mut game_state,
            world_bounds,
            &create_test_tilemap(),
            &create_test_atlas(),
        );
    }

    // Should be clamped at world_height - entity_size (16x16 from sprite)
    let player_size = player_entity(&game_state).unwrap().size;
    let expected_max_y = world_bounds.y as i32 - player_size.y as i32;
    assert_eq!(player_position(&game_state).y, expected_max_y);

    // One more update should not move further
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        world_bounds,
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(!result.player_moved); // Should not report movement when clamped
}

#[test]
fn game_state_directional_walk_animation_follows_movement_direction() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleDown,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_down_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::WalkDown,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_down_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::WalkUp,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_up_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::WalkLeft,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::WalkRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.play(AnimationState::IdleDown);

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Up);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    InputSystem::handle_key_release(game_state.runtime_mut(), InputKey::Up);
    let state_after_up = player_entity(&game_state)
        .and_then(|entity| entity.attributes.rendering.animation_controller.as_ref())
        .map(|controller| controller.current_clip_state);
    assert_eq!(state_after_up, Some(AnimationState::WalkUp));

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    InputSystem::handle_key_release(game_state.runtime_mut(), InputKey::Right);
    let state_after_right = player_entity(&game_state)
        .and_then(|entity| entity.attributes.rendering.animation_controller.as_ref())
        .map(|controller| controller.current_clip_state);
    assert_eq!(state_after_right, Some(AnimationState::WalkRight));
}

#[test]
fn game_state_left_direction_requests_horizontal_flip() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleDown,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_down_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::WalkLeft,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.play(AnimationState::IdleDown);

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Left);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    InputSystem::handle_key_release(game_state.runtime_mut(), InputKey::Left);

    let player_id = game_state
        .world()
        .player_id()
        .expect("player id should exist");
    assert!(entity_sprite_flip_x(&game_state, player_id));
}

#[test]
fn game_state_attack_left_requests_horizontal_flip() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackLeft,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::AttackLeft);

    let player_id = game_state
        .world()
        .player_id()
        .expect("player id should exist");
    assert!(entity_sprite_flip_x(&game_state, player_id));
}

#[test]
fn game_state_primary_action_plays_attack_clip_when_present() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleDown,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_down_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackDown,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_down_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleDown);

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    let current_state = player_entity(&game_state)
        .and_then(|entity| entity.attributes.rendering.animation_controller.as_ref())
        .map(|controller| controller.current_clip_state);
    assert_eq!(current_state, Some(AnimationState::AttackDown));
}

#[test]
fn game_state_primary_action_is_ignored_without_attack_clip() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleDown,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_down_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.play(AnimationState::IdleDown);

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    let current_state = player_entity(&game_state)
        .and_then(|entity| entity.attributes.rendering.animation_controller.as_ref())
        .map(|controller| controller.current_clip_state);
    assert_eq!(current_state, Some(AnimationState::IdleDown));
}

#[test]
fn game_state_attack_animation_persists_while_clip_is_unfinished() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleDown,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_down_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackDown,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_down_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 100.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleDown);

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    let controller = player_entity(&game_state)
        .and_then(|entity| entity.attributes.rendering.animation_controller.as_ref())
        .expect("player controller should exist");
    assert_eq!(controller.current_clip_state, AnimationState::AttackDown);
    assert!(!controller.is_finished);
}

#[test]
fn game_state_returns_to_locomotion_after_attack_animation_finishes() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleDown,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_down_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackDown,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_down_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 20.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleDown);

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    let controller = player_entity(&game_state)
        .and_then(|entity| entity.attributes.rendering.animation_controller.as_ref())
        .expect("player controller should exist");
    assert_eq!(controller.current_clip_state, AnimationState::IdleDown);
    assert!(!controller.is_finished);
}

#[test]
fn game_state_attack_animation_overrides_walk_while_movement_is_held() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::WalkRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 100.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_profile_key_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputKey::Right,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    let controller = player_entity(&game_state)
        .and_then(|entity| entity.attributes.rendering.animation_controller.as_ref())
        .expect("player controller should exist");
    assert_eq!(controller.current_clip_state, AnimationState::AttackRight);
    assert!(!controller.is_finished);
}

#[test]
fn game_state_returns_to_walk_after_attack_animation_finishes_with_movement_held() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::WalkRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 20.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_profile_key_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputKey::Right,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    let controller = player_entity(&game_state)
        .and_then(|entity| entity.attributes.rendering.animation_controller.as_ref())
        .expect("player controller should exist");
    assert_eq!(controller.current_clip_state, AnimationState::WalkRight);
    assert!(!controller.is_finished);
}

#[test]
fn game_state_primary_action_applies_damage_to_adjacent_target_health_stat() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    let mut target_definition = test_definition("target", "creature");
    target_definition.attributes.health = Some(25);
    let target_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&target_definition, IVec2::new(66, 60))
        .expect("target should spawn");

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    let target = game_state
        .world()
        .entity_manager()
        .get_entity(target_id)
        .expect("target should still exist after non-lethal hit");
    assert_eq!(target.attributes.gameplay.health, Some(15));
    assert_eq!(target.attributes.current_stat("health"), Some(15));
}

#[test]
fn game_state_primary_action_uses_attack_power_stat_for_damage() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    player
        .attributes
        .gameplay
        .stats
        .ensure_stat(ATTACK_POWER_STAT_ID, 17);
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    let mut target_definition = test_definition("target_attack_power", "creature");
    target_definition.attributes.health = Some(25);
    let target_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&target_definition, IVec2::new(66, 60))
        .expect("target should spawn");

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    let target = game_state
        .world()
        .entity_manager()
        .get_entity(target_id)
        .expect("target should still exist after non-lethal hit");
    assert_eq!(target.attributes.gameplay.health, Some(8));
    assert_eq!(target.attributes.current_stat("health"), Some(8));
}

#[test]
fn game_state_held_primary_action_does_not_apply_repeated_damage_every_frame() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    let mut target_definition = test_definition("held_primary_target", "creature");
    target_definition.attributes.health = Some(25);
    let target_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&target_definition, IVec2::new(66, 60))
        .expect("target should spawn");

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    let target = game_state
        .world()
        .entity_manager()
        .get_entity(target_id)
        .expect("target should still exist after non-lethal hit");
    assert_eq!(target.attributes.gameplay.health, Some(15));
    assert_eq!(target.attributes.current_stat("health"), Some(15));
}

#[test]
fn game_state_primary_action_can_damage_again_after_release_and_repress() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    let mut target_definition = test_definition("repress_primary_target", "creature");
    target_definition.attributes.health = Some(25);
    let target_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&target_definition, IVec2::new(66, 60))
        .expect("target should spawn");

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    InputSystem::handle_profile_action_release(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    let target = game_state
        .world()
        .entity_manager()
        .get_entity(target_id)
        .expect("target should still exist after two non-lethal hits");
    assert_eq!(target.attributes.gameplay.health, Some(5));
    assert_eq!(target.attributes.current_stat("health"), Some(5));
}

#[test]
fn game_state_primary_action_does_not_damage_out_of_range_target() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    let mut target_definition = test_definition("far_target", "creature");
    target_definition.attributes.health = Some(25);
    let target_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&target_definition, IVec2::new(82, 60))
        .expect("target should spawn");

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    let target = game_state
        .world()
        .entity_manager()
        .get_entity(target_id)
        .expect("out-of-range target should still exist");
    assert_eq!(target.attributes.gameplay.health, Some(25));
    assert_eq!(target.attributes.current_stat("health"), Some(25));
}

#[test]
fn game_state_primary_action_despawns_target_when_health_reaches_zero() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    let mut target_definition = test_definition("fragile_target", "creature");
    target_definition.attributes.health = Some(10);
    let target_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&target_definition, IVec2::new(66, 60))
        .expect("target should spawn");

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    assert!(
        game_state
            .world()
            .entity_manager()
            .get_entity(target_id)
            .is_none(),
        "lethal primary-action damage should despawn the target"
    );
}

#[test]
fn game_state_primary_action_damages_scene_loaded_legacy_health_target() {
    let mut game_state = GameState::new_empty();

    let hero_id = 5;
    let mut hero = test_definition("hero", "human")
        .create_entity(IVec2::new(50, 60), hero_id)
        .expect("hero entity should instantiate");
    hero.control_role = ControlRole::PlayerCharacter;
    hero.entity_kind = toki_core::entity::EntityKind::Player;
    let controller = hero
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("hero animation controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    let mut target_definition = test_definition("scene_target", "creature");
    target_definition.attributes.health = Some(25);
    let mut target = target_definition
        .create_entity(IVec2::new(66, 60), 6)
        .expect("target entity should instantiate");
    target.attributes.gameplay.stats = toki_core::entity::EntityStats::default();

    let mut scene = Scene::new("Legacy Arena".to_string());
    scene.add_entity(hero);
    scene.add_entity(target);

    SceneSystem::add_scene(&mut game_state, scene);
    SceneSystem::load(&mut game_state, "Legacy Arena").expect("scene should load successfully");

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(128, 128),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    let target = game_state
        .world()
        .entity_manager()
        .get_entity(6)
        .expect("legacy health target should still exist after non-lethal hit");
    assert_eq!(target.attributes.gameplay.health, Some(15));
    assert_eq!(target.attributes.current_stat("health"), Some(15));
}

#[test]
fn game_state_primary_action_spawns_projectile_when_authored() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");
    game_state
        .world_mut()
        .entity_manager_mut()
        .storage_mut()
        .components_mut()
        .set_primary_projectile(
            player_id,
            Some(PrimaryProjectileDef {
                sheet: "fauna".to_string(),
                object_name: "rock".to_string(),
                size: [16, 16],
                speed: 4,
                damage: 8,
                lifetime_ticks: 5,
                spawn_offset: [0, 0],
            }),
        );
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(128, 128),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    let projectile_ids = game_state
        .world()
        .entity_manager()
        .active_entities()
        .into_iter()
        .filter(|&entity_id| {
            game_state
                .world()
                .entity_manager()
                .get_entity(entity_id)
                .is_some_and(|entity| entity.entity_kind == EntityKind::Projectile)
        })
        .collect::<Vec<_>>();
    assert_eq!(projectile_ids.len(), 1);

    let renderables = sprite_render_requests(&game_state);
    let projectile = renderables
        .iter()
        .find(|request| {
            request.origin
                == toki_core::sprite_render::SpriteRenderOrigin::Projectile(projectile_ids[0])
        })
        .expect("projectile render request should exist");
    assert_eq!(
        projectile.visual,
        toki_core::sprite_render::SpriteVisualRef::ObjectSheetObject {
            sheet_name: "fauna".to_string(),
            object_name: "rock".to_string(),
        }
    );
    assert_eq!(projectile.position, IVec2::new(70, 60));
}

#[test]
fn game_state_projectile_moves_and_expires_after_lifetime() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");
    game_state
        .world_mut()
        .entity_manager_mut()
        .storage_mut()
        .components_mut()
        .set_primary_projectile(
            player_id,
            Some(PrimaryProjectileDef {
                sheet: "fauna".to_string(),
                object_name: "rock".to_string(),
                size: [16, 16],
                speed: 4,
                damage: 8,
                lifetime_ticks: 3,
                spawn_offset: [0, 0],
            }),
        );
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    let projectile_position = sprite_render_requests(&game_state)
        .into_iter()
        .find_map(|request| match request.origin {
            toki_core::sprite_render::SpriteRenderOrigin::Projectile(_) => Some(request.position),
            _ => None,
        })
        .expect("projectile render request should exist");
    assert_eq!(projectile_position, IVec2::new(70, 60));

    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    let projectile_position = sprite_render_requests(&game_state)
        .into_iter()
        .find_map(|request| match request.origin {
            toki_core::sprite_render::SpriteRenderOrigin::Projectile(_) => Some(request.position),
            _ => None,
        })
        .expect("projectile render request should still exist");
    assert_eq!(projectile_position, IVec2::new(74, 60));

    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    assert!(
        !sprite_render_requests(&game_state)
            .into_iter()
            .any(|request| matches!(
                request.origin,
                toki_core::sprite_render::SpriteRenderOrigin::Projectile(_)
            )),
        "projectile should despawn after its lifetime expires"
    );
}

#[test]
fn game_state_projectile_applies_damage_and_despawns_on_hit() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");
    game_state
        .world_mut()
        .entity_manager_mut()
        .storage_mut()
        .components_mut()
        .set_primary_projectile(
            player_id,
            Some(PrimaryProjectileDef {
                sheet: "fauna".to_string(),
                object_name: "rock".to_string(),
                size: [16, 16],
                speed: 4,
                damage: 8,
                lifetime_ticks: 10,
                spawn_offset: [0, 0],
            }),
        );
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    let mut target_definition = test_definition("projectile_target", "creature");
    target_definition.attributes.health = Some(25);
    let target_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&target_definition, IVec2::new(90, 60))
        .expect("target should spawn");

    let world_bounds = UVec2::new(160, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    let target = game_state
        .world()
        .entity_manager()
        .get_entity(target_id)
        .expect("target should survive non-lethal projectile damage");
    assert_eq!(target.attributes.gameplay.health, Some(17));
    assert_eq!(target.attributes.current_stat("health"), Some(17));
    assert!(
        !sprite_render_requests(&game_state)
            .into_iter()
            .any(|request| matches!(
                request.origin,
                toki_core::sprite_render::SpriteRenderOrigin::Projectile(_)
            )),
        "projectile should despawn on hit"
    );
}

#[test]
fn game_state_collects_overlapping_pickup_into_inventory_and_despawns_item() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");

    let mut pickup_definition = test_definition("coin_pickup", "item");
    pickup_definition.attributes.health = None;
    pickup_definition.attributes.solid = false;
    pickup_definition.attributes.can_move = false;
    pickup_definition.attributes.pickup = Some(PickupDef {
        item_id: "coin".to_string(),
        count: 2,
    });
    let pickup_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&pickup_definition, IVec2::new(50, 60))
        .expect("pickup should spawn");

    GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(128, 128),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .storage()
            .components()
            .inventory(player_id)
            .expect("player inventory should exist")
            .item_count("coin"),
        2
    );
    assert!(
        game_state
            .world()
            .entity_manager()
            .get_entity(pickup_id)
            .is_none(),
        "pickup should despawn after collection"
    );
}

#[test]
fn game_state_pickup_collection_stacks_and_does_not_double_collect() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");

    let mut pickup_definition = test_definition("coin_pickup_stack", "item");
    pickup_definition.attributes.health = None;
    pickup_definition.attributes.solid = false;
    pickup_definition.attributes.can_move = false;
    pickup_definition.attributes.pickup = Some(PickupDef {
        item_id: "coin".to_string(),
        count: 1,
    });

    let first_pickup_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&pickup_definition, IVec2::new(50, 60))
        .expect("first pickup should spawn");
    let second_pickup_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&pickup_definition, IVec2::new(50, 60))
        .expect("second pickup should spawn");

    GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(128, 128),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(128, 128),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .storage()
            .components()
            .inventory(player_id)
            .expect("player inventory should exist")
            .item_count("coin"),
        2
    );
    assert!(game_state
        .world()
        .entity_manager()
        .get_entity(first_pickup_id)
        .is_none());
    assert!(game_state
        .world()
        .entity_manager()
        .get_entity(second_pickup_id)
        .is_none());
}

#[test]
fn game_state_static_entity_renderables_include_object_sheet_backed_entities() {
    let mut game_state = GameState::new_empty();
    let static_pickup = EntityDefinition {
        name: "coin_pickup_render".into(),
        display_name: "Coin Pickup Render".to_string(),
        description: "Static object-sheet-backed pickup".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            palette_override: None,
            static_object: Some(toki_core::entity::StaticObjectRenderDef {
                sheet: "items".to_string(),
                object_name: "coin".to_string(),
            }),
            grounding: Default::default(),
        },
        attributes: AttributesDef {
            health: None,
            stats: std::collections::HashMap::new(),
            speed: 0.0,
            solid: false,
            active: true,
            can_move: false,
            interactable: false,
            interaction_reach: 0,
            ai_config: toki_core::entity::AiConfig::default(),
            movement_profile: MovementProfile::None,
            primary_projectile: None,
            pickup: Some(PickupDef {
                item_id: "coin".to_string(),
                count: 1,
            }),
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: true,
        },
        audio: AudioDef {
            footstep_trigger_distance: 16.0,
            hearing_radius: 64,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "".to_string(),
            clips: vec![],
            default_state: "".to_string(),
        },
        category: "item".to_string(),
        tags: vec!["pickup".to_string()],
    };

    let entity_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&static_pickup, IVec2::new(32, 48))
        .expect("static pickup should spawn");

    let static_renderables = sprite_render_requests(&game_state);
    let static_request = static_renderables
        .iter()
        .find(|request| {
            request.origin == toki_core::sprite_render::SpriteRenderOrigin::StaticEntity(entity_id)
        })
        .expect("static entity request should exist");
    assert_eq!(
        static_request.visual,
        toki_core::sprite_render::SpriteVisualRef::ObjectSheetObject {
            sheet_name: "items".to_string(),
            object_name: "coin".to_string(),
        }
    );
    assert_eq!(static_request.position, IVec2::new(32, 48));
    assert_eq!(renderable_entities(&game_state).len(), 1);
}

#[test]
fn game_state_entity_health_bars_include_visible_damageable_entities() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    let mut target_definition = test_definition("health_bar_target", "creature");
    target_definition.attributes.health = Some(25);
    let target_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&target_definition, IVec2::new(66, 60))
        .expect("target should spawn");

    let health_bars = entity_health_bars(&game_state);
    let target_bar = health_bars
        .into_iter()
        .find(|bar| bar.entity_id == target_id)
        .expect("visible target should produce health bar data");

    assert_eq!(target_bar.position, IVec2::new(66, 60));
    assert_eq!(target_bar.size, UVec2::new(16, 16));
    assert_eq!(target_bar.current, 25);
    assert_eq!(target_bar.max, 25);
}

#[test]
fn game_state_player_is_blocked_by_solid_entity_collision() {
    let mut game_state = GameState::new_empty();
    let player_id = SceneSystem::spawn_player_at(&mut game_state, IVec2::new(0, 0));
    let blocker_definition = test_definition("blocker", "creature");
    game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&blocker_definition, IVec2::new(16, 0))
        .expect("blocker should spawn");

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    let result = GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    InputSystem::handle_key_release(game_state.runtime_mut(), InputKey::Right);

    assert!(!result.player_moved);
    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .get_entity(player_id)
            .expect("player should exist")
            .position,
        IVec2::new(0, 0)
    );
}

#[test]
fn game_state_blocked_player_input_still_updates_facing_direction() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleDown,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_down_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    // Walk animation plays when trying to move (intent-based), even if blocked
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::WalkRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.play(AnimationState::IdleDown);

    let player_id = game_state
        .world()
        .player_id()
        .expect("player id should exist");
    let player_position = game_state
        .world()
        .entity_manager()
        .get_entity(player_id)
        .expect("player should exist")
        .position;
    let blocker_definition = test_definition("blocker", "creature");
    game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&blocker_definition, player_position + IVec2::new(16, 0))
        .expect("blocker should spawn");

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_profile_key_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputKey::Right,
    );
    let result = GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    InputSystem::handle_profile_key_release(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputKey::Right,
    );

    assert!(!result.player_moved);
    let player = game_state
        .world()
        .entity_manager()
        .get_entity(player_id)
        .expect("player should exist");
    let current_state = player
        .attributes
        .rendering
        .animation_controller
        .as_ref()
        .expect("player controller should exist")
        .current_clip_state;
    // Now uses intent-based animation: shows WalkRight when trying to move right
    assert_eq!(current_state, AnimationState::WalkRight);
}

#[test]
fn game_state_player_can_move_through_non_solid_entity() {
    let mut game_state = GameState::new_empty();
    let player_id = SceneSystem::spawn_player_at(&mut game_state, IVec2::new(0, 0));
    let mut non_solid_definition = test_definition("ghost", "creature");
    non_solid_definition.attributes.solid = false;
    game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&non_solid_definition, IVec2::new(16, 0))
        .expect("ghost should spawn");

    let world_bounds = UVec2::new(128, 128);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    let result = GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    InputSystem::handle_key_release(game_state.runtime_mut(), InputKey::Right);

    assert!(result.player_moved);
    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .get_entity(player_id)
            .expect("player should exist")
            .position,
        IVec2::new(2, 0) // Moved 2 pixels right (default speed)
    );
}

#[test]
fn game_state_player_is_blocked_by_solid_map_object_collision() {
    let mut game_state = GameState::new_empty();
    let player_id = SceneSystem::spawn_player_at(&mut game_state, IVec2::new(0, 0));
    let mut tilemap = create_test_tilemap();
    tilemap.objects.push(MapObjectInstance {
        sheet: PathBuf::from("fauna.json"),
        object_name: "bush".to_string(),
        position: UVec2::new(16, 0),
        size_px: UVec2::new(16, 16),
        grounding: Default::default(),
        visible: true,
        solid: true,
    });
    let atlas = create_test_atlas();

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    let result =
        GameSimulation::tick_fixed(&mut game_state, UVec2::new(128, 128), &tilemap, &atlas);
    InputSystem::handle_key_release(game_state.runtime_mut(), InputKey::Right);

    assert!(!result.player_moved);
    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .get_entity(player_id)
            .expect("player should exist")
            .position,
        IVec2::new(0, 0)
    );
}

#[test]
fn game_state_only_updates_npcs_with_wander_ai() {
    fastrand::seed(7);

    let mut game_state = GameState::new_empty();
    let mut wandering_npc = test_definition("wandering_npc", "creature");
    wandering_npc.attributes.ai_config =
        toki_core::entity::AiConfig::from_legacy_behavior(toki_core::entity::AiBehavior::Wander);
    let wandering_npc_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&wandering_npc, IVec2::new(32, 32))
        .expect("wandering npc should spawn");

    let mut idle_npc = test_definition("idle_npc", "creature");
    idle_npc.attributes.ai_config = toki_core::entity::AiConfig::default();
    let idle_npc_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&idle_npc, IVec2::new(96, 96))
        .expect("idle npc should spawn");

    let initial_wandering_position = game_state
        .world()
        .entity_manager()
        .get_entity(wandering_npc_id)
        .expect("wandering npc exists")
        .position;
    let initial_idle_position = game_state
        .world()
        .entity_manager()
        .get_entity(idle_npc_id)
        .expect("idle npc exists")
        .position;

    let mut wandering_npc_moved = false;
    for _ in 0..(60 * 12) {
        GameSimulation::tick_fixed(
            &mut game_state,
            UVec2::new(512, 512),
            &create_test_tilemap(),
            &create_test_atlas(),
        );
        if game_state
            .world()
            .entity_manager()
            .get_entity(wandering_npc_id)
            .expect("wandering npc exists")
            .position
            != initial_wandering_position
        {
            wandering_npc_moved = true;
            break;
        }
    }

    assert!(wandering_npc_moved, "wander npc should eventually move");
    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .get_entity(idle_npc_id)
            .expect("idle npc exists")
            .position,
        initial_idle_position,
        "npc with ai_behavior = none should remain stationary"
    );
}

#[test]
fn game_state_player_input_requires_player_wasd_movement_profile() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state
        .world()
        .player_id()
        .expect("player id should exist");
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .behavior
        .movement_profile = MovementProfile::None;

    let initial_position = player_position(&game_state);
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(1000, 1000),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(!result.player_moved);
    assert_eq!(player_position(&game_state), initial_position);
}

#[test]
fn game_state_legacy_default_player_profile_still_moves() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state
        .world()
        .player_id()
        .expect("player id should exist");
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .behavior
        .movement_profile = MovementProfile::LegacyDefault;

    let initial_position = player_position(&game_state);
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(1000, 1000),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(result.player_moved);
    assert_eq!(player_position(&game_state).x, initial_position.x + 2); // 2 pixels (default speed)
}

#[test]
fn game_state_non_player_entity_with_player_wasd_profile_moves_from_input() {
    let mut game_state = GameState::new_empty();
    let player_id = SceneSystem::spawn_player_at(&mut game_state, IVec2::new(0, 0));
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .behavior
        .movement_profile = MovementProfile::None;

    let mut controlled_npc = test_definition("controlled_npc", "creature");
    controlled_npc.attributes.movement_profile = MovementProfile::PlayerWasd;
    let npc_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&controlled_npc, IVec2::new(32, 32))
        .expect("controlled npc should spawn");

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(512, 512),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(!result.player_moved);
    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .get_entity(player_id)
            .expect("player should exist")
            .position,
        IVec2::new(0, 0)
    );
    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .get_entity(npc_id)
            .expect("npc should exist")
            .position,
        IVec2::new(34, 32) // Moved 2 pixels right (default speed)
    );
}

#[test]
fn game_state_multiple_player_wasd_entities_move_together() {
    let mut game_state = GameState::new_empty();

    let mut first = test_definition("first", "creature");
    first.attributes.movement_profile = MovementProfile::PlayerWasd;
    let first_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&first, IVec2::new(10, 10))
        .expect("first controlled entity should spawn");

    let mut second = test_definition("second", "creature");
    second.attributes.movement_profile = MovementProfile::PlayerWasd;
    let second_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&second, IVec2::new(40, 10))
        .expect("second controlled entity should spawn");

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Down);
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(512, 512),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .get_entity(first_id)
            .expect("first entity should exist")
            .position,
        IVec2::new(12, 12) // Moved 2 pixels diagonal (default speed)
    );
    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .get_entity(second_id)
            .expect("second entity should exist")
            .position,
        IVec2::new(42, 12) // Moved 2 pixels diagonal (default speed)
    );
}

#[test]
fn game_state_profile_scoped_input_moves_only_matching_profile_entities() {
    let mut game_state = GameState::new_empty();

    let mut controlled = test_definition("controlled", "creature");
    controlled.attributes.movement_profile = MovementProfile::PlayerWasd;
    let controlled_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&controlled, IVec2::new(10, 10))
        .expect("controlled entity should spawn");

    let mut passive = test_definition("passive", "creature");
    passive.attributes.movement_profile = MovementProfile::None;
    let passive_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&passive, IVec2::new(40, 10))
        .expect("passive entity should spawn");

    InputSystem::handle_profile_key_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputKey::Right,
    );
    GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(512, 512),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .get_entity(controlled_id)
            .expect("controlled entity should exist")
            .position,
        IVec2::new(12, 10) // Moved 2 pixels right (default speed)
    );
    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .get_entity(passive_id)
            .expect("passive entity should exist")
            .position,
        IVec2::new(40, 10)
    );
}

#[test]
fn game_state_load_scene_uses_control_role_for_player_identity() {
    let mut game_state = GameState::new_empty();
    let mut hero_definition = test_definition("hero_slime", "creature");
    hero_definition.attributes.movement_profile = MovementProfile::PlayerWasd;
    let mut hero = hero_definition
        .create_entity(IVec2::new(20, 24), 7)
        .expect("hero entity should instantiate");
    hero.control_role = ControlRole::PlayerCharacter;
    hero.category = "creature".to_string();

    let mut scene = Scene::new("Arena".to_string());
    scene.add_entity(hero);

    SceneSystem::add_scene(&mut game_state, scene);
    SceneSystem::load(&mut game_state, "Arena").expect("scene should load successfully");

    assert_eq!(game_state.world().player_id(), Some(7));
    assert_eq!(
        player_entity(&game_state)
            .expect("player-character entity should be resolved")
            .category,
        "creature"
    );
}

#[test]
fn game_state_load_scene_spawns_player_from_scene_player_entry() {
    let mut game_state = GameState::new_empty();
    game_state
        .world_mut()
        .insert_entity_definition(player_definition("player"));

    let mut scene = Scene::new("Entry Scene".to_string());
    scene.add_anchor(scene_anchor(
        "main_spawn",
        IVec2::new(96, 48),
        Some(SceneAnchorFacing::Left),
    ));
    scene.player_entry = Some(ScenePlayerEntry {
        entity_definition_name: "player".into(),
        spawn_point_id: "main_spawn".to_string(),
    });

    SceneSystem::add_scene(&mut game_state, scene);
    SceneSystem::load(&mut game_state, "Entry Scene").expect("scene should load successfully");

    let player = player_entity(&game_state).expect("scene player should spawn");
    assert_eq!(player.position, IVec2::new(96, 48));
    assert_eq!(player.definition_name.as_deref(), Some("player"));
    assert_eq!(player.control_role, ControlRole::PlayerCharacter);
    assert_eq!(player.entity_kind, EntityKind::Player);
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_ref()
        .expect("player animation controller should exist");
    assert_eq!(controller.current_clip_state, AnimationState::IdleLeft);
}

#[test]
fn game_state_transition_to_scene_preserves_durable_player_state_and_resets_transient_state() {
    let mut game_state = GameState::new_empty();
    game_state
        .world_mut()
        .insert_entity_definition(player_definition("player"));
    game_state
        .world_mut()
        .insert_entity_definition(player_definition("player_knight"));

    let mut scene_a = Scene::new("Scene A".to_string());
    let mut hero = player_definition("player")
        .create_entity(IVec2::new(8, 8), 7)
        .expect("hero should instantiate");
    hero.control_role = ControlRole::PlayerCharacter;
    hero.entity_kind = EntityKind::Player;
    scene_a.add_entity(hero);

    let mut scene_b = Scene::new("Scene B".to_string());
    scene_b.add_anchor(scene_anchor(
        "from_gate",
        IVec2::new(128, 64),
        Some(SceneAnchorFacing::Left),
    ));
    scene_b.player_entry = Some(ScenePlayerEntry {
        entity_definition_name: "player_knight".into(),
        spawn_point_id: "default_spawn".to_string(),
    });

    SceneSystem::add_scene(&mut game_state, scene_a);
    SceneSystem::add_scene(&mut game_state, scene_b);
    SceneSystem::load(&mut game_state, "Scene A").expect("initial scene should load");

    let player_id = game_state.world().player_id().expect("player should exist");
    {
        let entity_manager = game_state.world_mut().entity_manager_mut();
        entity_manager
            .get_entity_mut(player_id)
            .expect("player should exist")
            .attributes
            .apply_stat_delta("health", -35);
        entity_manager
            .storage_mut()
            .components_mut()
            .ensure_inventory(player_id)
            .add_item("coin", 3);
        let player = entity_manager
            .get_entity_mut(player_id)
            .expect("player should exist");
        player.movement_accumulator = glam::Vec2::new(0.5, 0.25);
        let controller = player
            .attributes
            .rendering
            .animation_controller
            .as_mut()
            .expect("player controller should exist");
        controller.play(AnimationState::AttackRight);
        controller.current_frame_index = 1;
        controller.frame_timer = 99.0;
        controller.is_finished = true;
    }
    let audio = game_state
        .world_mut()
        .entity_manager_mut()
        .storage_mut()
        .audio_component_mut(player_id)
        .expect("player audio should exist");
    audio.footstep_distance_accumulator = 42.0;
    audio.last_collision_state = true;

    SceneSystem::transition(&mut game_state, "Scene B", "from_gate")
        .expect("scene transition should succeed");

    assert_eq!(
        game_state.scene().scene_manager().active_scene_name(),
        Some("Scene B")
    );
    assert_eq!(game_state.world().player_id(), Some(player_id));

    let player = player_entity(&game_state).expect("player should still exist");
    assert_eq!(player.position, IVec2::new(128, 64));
    assert_eq!(player.definition_name.as_deref(), Some("player_knight"));
    assert_eq!(player.attributes.current_stat("health"), Some(65));
    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .storage()
            .components()
            .inventory(player_id)
            .expect("player inventory should exist")
            .item_count("coin"),
        3
    );
    assert_eq!(player.movement_accumulator, glam::Vec2::ZERO);
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_ref()
        .expect("player controller should exist");
    assert_eq!(controller.current_clip_state, AnimationState::IdleLeft);
    assert_eq!(controller.current_frame_index, 0);
    assert_eq!(controller.frame_timer, 0.0);
    assert!(!controller.is_finished);

    let audio = game_state
        .world()
        .entity_manager()
        .storage()
        .audio_component(player_id)
        .expect("player audio should exist after transition");
    assert_eq!(audio.footstep_distance_accumulator, 0.0);
    assert!(!audio.last_collision_state);
}

#[test]
fn game_state_transition_to_scene_resets_non_player_entities_on_return() {
    let mut game_state = GameState::new_empty();
    game_state
        .world_mut()
        .insert_entity_definition(player_definition("player"));

    let mut scene_a = Scene::new("Scene A".to_string());
    scene_a.add_anchor(scene_anchor("return_spawn", IVec2::new(16, 16), None));
    let mut hero = player_definition("player")
        .create_entity(IVec2::new(0, 0), 1)
        .expect("hero should instantiate");
    hero.control_role = ControlRole::PlayerCharacter;
    hero.entity_kind = EntityKind::Player;
    scene_a.add_entity(hero);
    let npc_id = scene_a.add_entity(
        test_definition("villager", "creature")
            .create_entity(IVec2::new(40, 40), 9)
            .expect("npc should instantiate"),
    );

    let mut scene_b = Scene::new("Scene B".to_string());
    scene_b.add_anchor(scene_anchor("entry_b", IVec2::new(96, 32), None));

    SceneSystem::add_scene(&mut game_state, scene_a);
    SceneSystem::add_scene(&mut game_state, scene_b);
    SceneSystem::load(&mut game_state, "Scene A").expect("initial scene should load");

    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(npc_id)
        .expect("npc should exist")
        .position = IVec2::new(80, 80);

    SceneSystem::transition(&mut game_state, "Scene B", "entry_b")
        .expect("transition to scene B should succeed");
    SceneSystem::transition(&mut game_state, "Scene A", "return_spawn")
        .expect("transition back should succeed");

    let npc = game_state
        .world()
        .entity_manager()
        .get_entity(npc_id)
        .expect("authored npc should be restored on return");
    assert_eq!(npc.position, IVec2::new(40, 40));
}

#[test]
fn game_state_transition_to_scene_preserves_non_player_entities_when_scene_is_synced() {
    let mut game_state = GameState::new_empty();
    game_state
        .world_mut()
        .insert_entity_definition(player_definition("player"));

    let mut scene_a = Scene::new("Scene A".to_string());
    scene_a.add_anchor(scene_anchor("return_spawn", IVec2::new(16, 16), None));
    let mut hero = player_definition("player")
        .create_entity(IVec2::new(0, 0), 1)
        .expect("hero should instantiate");
    hero.control_role = ControlRole::PlayerCharacter;
    hero.entity_kind = EntityKind::Player;
    scene_a.add_entity(hero);
    let npc_id = scene_a.add_entity(
        test_definition("villager", "creature")
            .create_entity(IVec2::new(40, 40), 9)
            .expect("npc should instantiate"),
    );

    let mut scene_b = Scene::new("Scene B".to_string());
    scene_b.add_anchor(scene_anchor("entry_b", IVec2::new(96, 32), None));

    SceneSystem::add_scene(&mut game_state, scene_a);
    SceneSystem::add_scene(&mut game_state, scene_b);
    SceneSystem::load(&mut game_state, "Scene A").expect("initial scene should load");

    {
        let npc = game_state
            .world_mut()
            .entity_manager_mut()
            .get_entity_mut(npc_id)
            .expect("npc should exist");
        npc.position = IVec2::new(80, 80);
        npc.attributes.behavior.active = false;
    }
    SceneSystem::sync_entities_to_active_scene(&mut game_state);

    SceneSystem::transition(&mut game_state, "Scene B", "entry_b")
        .expect("transition to scene B should succeed");
    SceneSystem::transition(&mut game_state, "Scene A", "return_spawn")
        .expect("transition back should succeed");

    let npc = game_state
        .world()
        .entity_manager()
        .get_entity(npc_id)
        .expect("synced npc should be restored on return");
    assert_eq!(npc.position, IVec2::new(80, 80));
    assert!(!npc.attributes.behavior.active);
}

#[test]
fn game_state_transition_to_scene_missing_spawn_fails_without_corrupting_state() {
    let mut game_state = GameState::new_empty();
    game_state
        .world_mut()
        .insert_entity_definition(player_definition("player"));

    let mut scene_a = Scene::new("Scene A".to_string());
    let mut hero = player_definition("player")
        .create_entity(IVec2::new(12, 24), 3)
        .expect("hero should instantiate");
    hero.control_role = ControlRole::PlayerCharacter;
    hero.entity_kind = EntityKind::Player;
    scene_a.add_entity(hero);

    let scene_b = Scene::new("Scene B".to_string());

    SceneSystem::add_scene(&mut game_state, scene_a);
    SceneSystem::add_scene(&mut game_state, scene_b);
    SceneSystem::load(&mut game_state, "Scene A").expect("initial scene should load");

    let before_position = player_position(&game_state);
    let error = SceneSystem::transition(&mut game_state, "Scene B", "missing_spawn")
        .expect_err("missing spawn point should fail");
    assert!(matches!(
        error,
        toki_core::game::SceneLoadError::MissingSpawnAnchor { ref spawn_point_id, .. }
            if spawn_point_id == "missing_spawn"
    ));
    assert_eq!(
        game_state.scene().scene_manager().active_scene_name(),
        Some("Scene A")
    );
    assert_eq!(player_position(&game_state), before_position);
}

#[test]
fn game_state_sprite_animation_updates() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let atlas = create_test_atlas();
    let texture_size = atlas.image_size().unwrap_or(glam::UVec2::new(64, 16));
    let _initial_frame = current_sprite_frame(&game_state, &atlas, texture_size);

    let world_bounds = UVec2::new(1000, 1000);

    // Update multiple times to advance animation
    for _ in 0..10 {
        GameSimulation::tick_fixed(
            &mut game_state,
            world_bounds,
            &create_test_tilemap(),
            &create_test_atlas(),
        );
    }

    // Animation should have progressed (frame or timing)
    // Note: Since animation depends on internal timing, we mainly test that it doesn't crash
    let _current_frame = current_sprite_frame(&game_state, &atlas, texture_size);
    // The exact frame depends on timing, so we just ensure it's callable
}

#[test]
fn game_state_entity_position_sync() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    // Move the player
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    let world_bounds = UVec2::new(1000, 1000);
    GameSimulation::tick_fixed(
        &mut game_state,
        world_bounds,
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Entity position should match player sprite position
    let player = player_entity(&game_state).unwrap();
    assert_eq!(player.position, player_position(&game_state));

    // Move again
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Down);
    InputSystem::handle_key_release(game_state.runtime_mut(), InputKey::Right);
    GameSimulation::tick_fixed(
        &mut game_state,
        world_bounds,
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Should still be synchronized
    let player = player_entity(&game_state).unwrap();
    assert_eq!(player.position, player_position(&game_state));
}

#[test]
fn game_state_multiple_key_handling() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    // Press multiple keys
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Up);
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Down); // Conflicting with Up
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Left); // Conflicting with Right

    let world_bounds = UVec2::new(1000, 1000);
    let _initial_position = player_position(&game_state);
    GameSimulation::tick_fixed(
        &mut game_state,
        world_bounds,
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // All directions should be processed (net effect might cancel out)
    // This tests that multiple keys don't crash the system
    let _final_position = player_position(&game_state);

    // Release some keys
    InputSystem::handle_key_release(game_state.runtime_mut(), InputKey::Down);
    InputSystem::handle_key_release(game_state.runtime_mut(), InputKey::Left);

    let position_before = player_position(&game_state);
    GameSimulation::tick_fixed(
        &mut game_state,
        world_bounds,
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    let position_after = player_position(&game_state);

    // Should move up and right now
    assert!(position_after.x > position_before.x); // Right
    assert!(position_after.y < position_before.y); // Up
}

#[test]
fn game_state_entity_manager_access() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);

    // Should be able to access EntityManager
    let entity_manager = game_state.world().entity_manager();
    assert_eq!(entity_manager.active_entities().len(), 1);
    assert!(entity_manager.get_player().is_some());

    // Should be able to spawn additional entities
    let entity_manager = game_state.world_mut().entity_manager_mut();
    let npc_id = entity_manager
        .spawn_from_definition(&test_definition("npc", "creature"), IVec2::new(100, 100))
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

    let player_entity = player_entity(&game_state).unwrap();

    // Check player entity has correct attributes from factory method
    assert_eq!(player_entity.attributes.gameplay.health, Some(100));
    assert_eq!(player_entity.attributes.gameplay.speed, 2.0);
    assert!(player_entity.attributes.behavior.active);
    assert!(player_entity.attributes.behavior.can_move);
    assert!(player_entity.attributes.gameplay.solid);
    assert!(player_entity.attributes.rendering.visible);
    assert_eq!(player_entity.attributes.rendering.render_layer, 0);
    assert!(player_entity
        .attributes
        .rendering
        .animation_controller
        .is_some());
}

#[test]
fn game_state_new_uses_definition_based_player_creation() {
    let sprite = create_test_sprite();
    let game_state = GameState::new(sprite);
    let player = player_entity(&game_state).expect("player should exist");

    assert_eq!(player.definition_name.as_deref(), Some("player"));
}

#[test]
fn game_state_spawn_player_like_npc_uses_definition_metadata() {
    let mut game_state = GameState::new_empty();
    let npc_id = SceneSystem::spawn_player_like_npc(&mut game_state, IVec2::new(120, 72));
    let npc = game_state
        .world()
        .entity_manager()
        .get_entity(npc_id)
        .expect("spawned npc should exist");

    assert_eq!(npc.definition_name.as_deref(), Some("player_like_npc"));
    assert_eq!(npc.entity_kind, toki_core::entity::EntityKind::Player);
}

#[test]
fn game_state_spawn_player_like_npc_uses_player_visuals_with_npc_behavior() {
    let mut game_state = GameState::new_empty();
    let npc_id = SceneSystem::spawn_player_like_npc(&mut game_state, IVec2::new(120, 72));
    let npc = game_state
        .world()
        .entity_manager()
        .get_entity(npc_id)
        .expect("spawned npc should exist");

    assert_eq!(npc.attributes.gameplay.health, Some(50));
    assert_eq!(npc.attributes.gameplay.speed, 1.0);
    assert!(!npc.attributes.behavior.can_move);
    assert_eq!(
        npc.attributes.behavior.movement_profile,
        toki_core::entity::MovementProfile::None
    );
    assert!(npc.attributes.rendering.animation_controller.is_some());
    assert!(game_state
        .world()
        .entity_manager()
        .storage()
        .components()
        .inventory(npc_id)
        .is_none());
}

#[test]
fn game_state_emits_movement_audio_event_with_component_sound_id() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");
    let player_audio = game_state
        .world_mut()
        .entity_manager_mut()
        .storage_mut()
        .audio_component_mut(player_id)
        .expect("player audio component should exist");
    player_audio.footstep_trigger_distance = 1.0;
    player_audio.movement_sound = Some("sfx_custom_step".to_string());

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(1000, 1000),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(result.events.iter().any(|event| {
        matches!(
            event,
            AudioEvent::PlaySound {
                channel: AudioChannel::Movement,
                sound_id,
                source_position: Some(_),
                hearing_radius: Some(_),
            } if sound_id == "sfx_custom_step"
        )
    }));
}

#[test]
fn game_state_emits_movement_audio_on_animation_loop_when_configured() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");

    {
        let player_audio = game_state
            .world_mut()
            .entity_manager_mut()
            .storage_mut()
            .audio_component_mut(player_id)
            .expect("player audio component should exist");
        player_audio.footstep_trigger_distance = 9999.0;
        player_audio.movement_sound = Some("sfx_anim_step".to_string());
        player_audio.movement_sound_trigger = MovementSoundTrigger::AnimationLoop;
    }

    let controller = game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .and_then(|entity| entity.attributes.rendering.animation_controller.as_mut())
        .expect("player animation controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::Walk,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 1.0,
        frame_durations_ms: None,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(1000, 1000),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(result.events.iter().any(|event| {
        matches!(
            event,
            AudioEvent::PlaySound {
                channel: AudioChannel::Movement,
                sound_id,
                source_position: Some(_),
                hearing_radius: Some(_),
            } if sound_id == "sfx_anim_step"
        )
    }));
}

#[test]
fn game_state_emits_movement_audio_for_wander_ai_movement() {
    fastrand::seed(7);

    let mut game_state = GameState::new_empty();
    let mut wandering_npc = test_definition("wandering_npc", "creature");
    wandering_npc.audio.footstep_trigger_distance = 1.0;
    wandering_npc.audio.movement_sound = "sfx_wander_step".to_string();
    let wandering_npc_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&wandering_npc, IVec2::new(32, 32))
        .expect("wandering npc should spawn");

    let initial_position = game_state
        .world()
        .entity_manager()
        .get_entity(wandering_npc_id)
        .expect("wandering npc should exist")
        .position;

    let mut moved = false;
    let mut emitted_sound = false;
    for _ in 0..(60 * 12) {
        let result = GameSimulation::tick_fixed(
            &mut game_state,
            UVec2::new(512, 512),
            &create_test_tilemap(),
            &create_test_atlas(),
        );
        if result.events.iter().any(|event| {
            matches!(
                event,
                AudioEvent::PlaySound {
                    channel: AudioChannel::Movement,
                    sound_id,
                    source_position: Some(_),
                    hearing_radius: Some(_),
                } if sound_id == "sfx_wander_step"
            )
        }) {
            emitted_sound = true;
        }
        if game_state
            .world()
            .entity_manager()
            .get_entity(wandering_npc_id)
            .expect("wandering npc should exist")
            .position
            != initial_position
        {
            moved = true;
        }

        if moved && emitted_sound {
            break;
        }
    }

    assert!(moved, "wander npc should eventually move");
    assert!(
        emitted_sound,
        "wander npc movement should emit its configured movement sound"
    );
}

#[test]
fn game_state_emits_movement_audio_for_rule_velocity_movement() {
    let mut game_state = GameState::new_empty();
    let mut mover = test_definition("rule_mover", "creature");
    mover.attributes.ai_config = toki_core::entity::AiConfig::default();
    mover.audio.footstep_trigger_distance = 1.0;
    mover.audio.movement_sound = "sfx_rule_step".to_string();
    let mover_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&mover, IVec2::new(16, 16))
        .expect("rule mover should spawn");

    RuleSystem::set_rules(
        &mut game_state,
        RuleSet {
            rules: vec![Rule {
                id: "move_rule_mover".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                log_enabled: false,
                trigger: RuleTrigger::OnUpdate,
                conditions: vec![],
                actions: vec![RuleAction::SetVelocity {
                    target: RuleTarget::Entity(mover_id),
                    velocity: [1, 0],
                }],
            }],
        },
    );

    let result = GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(512, 512),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(result.events.iter().any(|event| {
        matches!(
            event,
            AudioEvent::PlaySound {
                channel: AudioChannel::Movement,
                sound_id,
                source_position: Some(_),
                hearing_radius: Some(_),
            } if sound_id == "sfx_rule_step"
        )
    }));
    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .get_entity(mover_id)
            .expect("rule mover should exist")
            .position,
        IVec2::new(17, 16)
    );
}

#[test]
fn game_state_emits_collision_audio_event_with_component_sound_id() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");
    let player_audio = game_state
        .world_mut()
        .entity_manager_mut()
        .storage_mut()
        .audio_component_mut(player_id)
        .expect("player audio component should exist");
    player_audio.collision_sound = Some("sfx_custom_collision".to_string());

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(1000, 1000),
        &create_test_tilemap(),
        &create_solid_test_atlas(),
    );

    assert!(result.events.iter().any(|event| {
        matches!(
            event,
            AudioEvent::PlaySound {
                channel: AudioChannel::Collision,
                sound_id,
                source_position: Some(_),
                hearing_radius: Some(_),
            } if sound_id == "sfx_custom_collision"
        )
    }));
}

// =============================================================================
// Entity-based movement tests (speed and size from entity, not global)
// =============================================================================

#[test]
fn game_state_entity_speed_controls_movement_distance() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");

    // Set player speed to 3.0 pixels per tick
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .attributes
        .gameplay
        .speed = 3.0;

    let initial_position = player_position(&game_state);
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(1000, 1000),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Should have moved 3 pixels (entity speed), not 1 pixel (old hardcoded)
    assert_eq!(player_position(&game_state).x, initial_position.x + 3);
}

#[test]
fn game_state_entity_speed_fractional_rounds_down() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");

    // Set player speed to 1.7 pixels per tick
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .attributes
        .gameplay
        .speed = 1.7;

    let initial_position = player_position(&game_state);
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Down);
    GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(1000, 1000),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Should move 1 pixel (1.7 truncated to 1)
    assert_eq!(player_position(&game_state).y, initial_position.y + 1);
}

#[test]
fn game_state_entity_speed_below_one_accumulates_movement() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");

    // Set player speed to 0.5 pixels per tick
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .attributes
        .gameplay
        .speed = 0.5;

    let initial_position = player_position(&game_state);
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Left);

    // First update: accumulate 0.5, not enough to move
    let result1 = GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(1000, 1000),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(!result1.player_moved);
    assert_eq!(player_position(&game_state), initial_position);

    // Second update: accumulate another 0.5, now 1.0 - should move 1 pixel
    let result2 = GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(1000, 1000),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(result2.player_moved);
    assert_eq!(player_position(&game_state).x, initial_position.x - 1);
}

#[test]
fn game_state_entity_speed_fractional_accumulates_remainder() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");

    // Set player speed to 1.5 pixels per tick
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .attributes
        .gameplay
        .speed = 1.5;

    let initial_position = player_position(&game_state);
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);

    // First update: 1.5 -> move 1, accumulate 0.5
    GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(1000, 1000),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert_eq!(player_position(&game_state).x, initial_position.x + 1);

    // Second update: 0.5 + 1.5 = 2.0 -> move 2, accumulate 0
    GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(1000, 1000),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert_eq!(player_position(&game_state).x, initial_position.x + 3);
}

#[test]
fn game_state_entity_speed_accumulator_resets_on_direction_change() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");

    // Set player speed to 0.5 pixels per tick
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .attributes
        .gameplay
        .speed = 0.5;

    let initial_position = player_position(&game_state);
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Left);

    // First update: accumulate 0.5 left
    GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(1000, 1000),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert_eq!(player_position(&game_state), initial_position);

    // Change direction to right
    InputSystem::handle_key_release(game_state.runtime_mut(), InputKey::Left);
    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);

    // Second update: should start fresh at 0.5 right, not move
    GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(1000, 1000),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert_eq!(player_position(&game_state), initial_position);

    // Third update: now 1.0 accumulated right, should move right
    let result = GameSimulation::tick_fixed(
        &mut game_state,
        UVec2::new(1000, 1000),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(result.player_moved);
    assert_eq!(player_position(&game_state).x, initial_position.x + 1);
}

#[test]
fn game_state_entity_size_used_for_right_boundary_clamping() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");

    // Set player to have a 32x32 size (larger than default 16x16)
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .size = UVec2::new(32, 32);
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .collision_box
        .as_mut()
        .expect("player collision should exist")
        .size = UVec2::new(32, 32);
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .attributes
        .rendering
        .grounding
        .footprint = Some(EntityFootprint::new([0, 0], [32, 32]));

    // Also set speed high so we reach boundary quickly
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .attributes
        .gameplay
        .speed = 10.0;

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);
    let world_bounds = UVec2::new(100, 100);

    // Move right until boundary
    for _ in 0..50 {
        GameSimulation::tick_fixed(
            &mut game_state,
            world_bounds,
            &create_test_tilemap(),
            &create_test_atlas(),
        );
    }

    // Should be clamped at world_width - entity_size (100 - 32 = 68)
    assert_eq!(player_position(&game_state).x, 68);
}

#[test]
fn game_state_entity_size_used_for_bottom_boundary_clamping() {
    let sprite = create_test_sprite();
    let mut game_state = GameState::new(sprite);
    let player_id = game_state.world().player_id().expect("player should exist");

    // Set player to have a 24x24 size
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .size = UVec2::new(24, 24);
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .collision_box
        .as_mut()
        .expect("player collision should exist")
        .size = UVec2::new(24, 24);
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .attributes
        .rendering
        .grounding
        .footprint = Some(EntityFootprint::new([0, 0], [24, 24]));

    // Set speed high
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .attributes
        .gameplay
        .speed = 10.0;

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Down);
    let world_bounds = UVec2::new(100, 100);

    // Move down until boundary
    for _ in 0..50 {
        GameSimulation::tick_fixed(
            &mut game_state,
            world_bounds,
            &create_test_tilemap(),
            &create_test_atlas(),
        );
    }

    // Should be clamped at world_height - entity_size (100 - 24 = 76)
    assert_eq!(player_position(&game_state).y, 76);
}

// ============================================================================
// Delta timestep tests
// ============================================================================

use toki_core::DEFAULT_TIMESTEP_MS;

#[test]
fn update_with_delta_at_default_timestep_matches_fixed_update() {
    // Create two identical game states
    let mut fixed_state = GameState::new(create_test_sprite());
    let mut delta_state = GameState::new(create_test_sprite());

    let world_bounds = UVec2::new(1000, 1000);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Press right on both
    InputSystem::handle_key_press(fixed_state.runtime_mut(), InputKey::Right);
    InputSystem::handle_key_press(delta_state.runtime_mut(), InputKey::Right);

    // Update fixed state with update()
    GameSimulation::tick_fixed(&mut fixed_state, world_bounds, &tilemap, &atlas);

    // Update delta state with update_with_delta at default timestep
    GameSimulation::tick_with_delta(
        &mut delta_state,
        DEFAULT_TIMESTEP_MS,
        world_bounds,
        &tilemap,
        &atlas,
    );

    // Positions should match
    assert_eq!(player_position(&fixed_state), player_position(&delta_state));
}

#[test]
fn update_with_delta_double_timestep_moves_further() {
    let mut game_state = GameState::new(create_test_sprite());
    let player_id = game_state.world().player_id().unwrap();

    // Set speed to 1.0 for predictable movement
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .attributes
        .gameplay
        .speed = 1.0;

    let world_bounds = UVec2::new(1000, 1000);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();
    let initial_position = player_position(&game_state);

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);

    // Update with 2x delta (33.33ms)
    GameSimulation::tick_with_delta(
        &mut game_state,
        DEFAULT_TIMESTEP_MS * 2.0,
        world_bounds,
        &tilemap,
        &atlas,
    );

    // Should have moved 2 pixels (speed 1.0 * 2.0 scale factor = 2 pixels)
    let position_after = player_position(&game_state);
    assert_eq!(position_after.x - initial_position.x, 2);
}

#[test]
fn update_with_delta_half_timestep_moves_less() {
    let mut game_state = GameState::new(create_test_sprite());
    let player_id = game_state.world().player_id().unwrap();

    // Speed = 2.0 means 2 pixels per frame at 60fps
    // At half delta (8.33ms), accumulator gets 1.0, which is 1 whole pixel
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .attributes
        .gameplay
        .speed = 2.0;

    let world_bounds = UVec2::new(1000, 1000);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();
    let initial_position = player_position(&game_state);

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);

    // Update with 0.5x delta (8.33ms)
    GameSimulation::tick_with_delta(
        &mut game_state,
        DEFAULT_TIMESTEP_MS * 0.5,
        world_bounds,
        &tilemap,
        &atlas,
    );

    // Should have moved 1 pixel (speed 2.0 * 0.5 scale = 1 pixel)
    let position_after = player_position(&game_state);
    assert_eq!(position_after.x - initial_position.x, 1);
}

#[test]
fn game_state_ai_speed_below_one_accumulates_movement() {
    let world_bounds = UVec2::new(1000, 1000);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    let mut game_state = GameState::new(create_test_sprite());
    let player_id = game_state.world().player_id().expect("player should exist");
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .position = IVec2::new(32, 32);

    let mut chaser = test_definition("fractional_chaser", "creature");
    chaser.attributes.ai_config = AiConfig {
        behavior: AiBehavior::Chase,
        detection_radius: 500,
    };
    chaser.attributes.speed = 0.5;

    let chaser_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&chaser, IVec2::new(64, 32))
        .expect("chaser should spawn");

    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    let first_position = game_state
        .world()
        .entity_manager()
        .get_entity(chaser_id)
        .expect("chaser should exist")
        .position;
    assert_eq!(first_position, IVec2::new(64, 32));

    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    let second_position = game_state
        .world()
        .entity_manager()
        .get_entity(chaser_id)
        .expect("chaser should exist")
        .position;
    assert_eq!(second_position, IVec2::new(63, 32));
}

#[test]
fn update_with_delta_accumulates_fractional_movement() {
    let mut game_state = GameState::new(create_test_sprite());
    let player_id = game_state.world().player_id().unwrap();

    // Speed = 1.0 means 1 pixel per frame at 60fps
    game_state
        .world_mut()
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .unwrap()
        .attributes
        .gameplay
        .speed = 1.0;

    let world_bounds = UVec2::new(1000, 1000);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();
    let initial_position = player_position(&game_state);

    InputSystem::handle_key_press(game_state.runtime_mut(), InputKey::Right);

    // Update with 0.5x delta twice - should accumulate to 1 pixel total
    GameSimulation::tick_with_delta(
        &mut game_state,
        DEFAULT_TIMESTEP_MS * 0.5,
        world_bounds,
        &tilemap,
        &atlas,
    );
    let position_after_first = player_position(&game_state);
    // First update: accumulator = 0.5, no whole pixel
    assert_eq!(position_after_first.x - initial_position.x, 0);

    GameSimulation::tick_with_delta(
        &mut game_state,
        DEFAULT_TIMESTEP_MS * 0.5,
        world_bounds,
        &tilemap,
        &atlas,
    );
    let position_after_second = player_position(&game_state);
    // Second update: accumulator = 1.0, extracts 1 whole pixel
    assert_eq!(position_after_second.x - initial_position.x, 1);
}

#[test]
fn update_with_delta_scales_animation_timing() {
    let mut game_state = GameState::new(create_test_sprite());

    let world_bounds = UVec2::new(1000, 1000);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    // Get initial frame timer
    let initial_frame_timer = player_entity(&game_state)
        .and_then(|e| e.attributes.rendering.animation_controller.as_ref())
        .map(|c| c.frame_timer)
        .unwrap_or(0.0);

    // Update with 2x delta (33.33ms)
    GameSimulation::tick_with_delta(
        &mut game_state,
        DEFAULT_TIMESTEP_MS * 2.0,
        world_bounds,
        &tilemap,
        &atlas,
    );

    // Animation frame_timer should have advanced by approximately 33.33ms
    // (unless it wrapped around due to frame advancement)
    let final_frame_timer = player_entity(&game_state)
        .and_then(|e| e.attributes.rendering.animation_controller.as_ref())
        .map(|c| c.frame_timer)
        .unwrap_or(0.0);

    // The frame_timer accumulates delta until it exceeds frame_duration,
    // then it wraps. We verify it changed from the initial value.
    // With 33.33ms added to initial 0.0, we expect some accumulation.
    assert!(
        final_frame_timer > 0.0 || initial_frame_timer != final_frame_timer,
        "Animation timing should have changed"
    );
}

#[test]
fn update_with_delta_keeps_ai_consistent_across_frame_chunking() {
    let world_bounds = UVec2::new(1000, 1000);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    let mut single_frame_state = GameState::new(create_test_sprite());
    let mut split_frame_state = GameState::new(create_test_sprite());

    let mut chaser = test_definition("chaser", "creature");
    chaser.attributes.ai_config = AiConfig {
        behavior: AiBehavior::Chase,
        detection_radius: 500,
    };
    chaser.attributes.speed = 1.0;

    let single_frame_chaser = single_frame_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&chaser, IVec2::new(100, 60))
        .expect("chaser should spawn");
    let split_frame_chaser = split_frame_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&chaser, IVec2::new(100, 60))
        .expect("chaser should spawn");

    GameSimulation::tick_with_delta(
        &mut single_frame_state,
        DEFAULT_TIMESTEP_MS * 2.0,
        world_bounds,
        &tilemap,
        &atlas,
    );
    GameSimulation::tick_with_delta(
        &mut split_frame_state,
        DEFAULT_TIMESTEP_MS,
        world_bounds,
        &tilemap,
        &atlas,
    );
    GameSimulation::tick_with_delta(
        &mut split_frame_state,
        DEFAULT_TIMESTEP_MS,
        world_bounds,
        &tilemap,
        &atlas,
    );

    let single_frame_position = single_frame_state
        .world()
        .entity_manager()
        .get_entity(single_frame_chaser)
        .expect("single-frame chaser should exist")
        .position;
    let split_frame_position = split_frame_state
        .world()
        .entity_manager()
        .get_entity(split_frame_chaser)
        .expect("split-frame chaser should exist")
        .position;

    assert_eq!(single_frame_position, split_frame_position);
}
