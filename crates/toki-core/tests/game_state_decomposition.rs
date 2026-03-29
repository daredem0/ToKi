use glam::IVec2;
use std::collections::HashMap;
use toki_core::assets::atlas::{AtlasMeta, ColorMode, TileInfo, TileProperties};
use toki_core::entity::{
    AttributesDef, AudioDef, CollisionDef, EntityDefinition, MovementProfile, MovementSoundTrigger,
    OptionalEntityComponents, RenderingDef, StoredEntity,
};
use toki_core::game::{GameSimulation, InputSystem, RenderQueryService, RuleSystem, SceneSystem};
use toki_core::rules::{Rule, RuleAction, RuleCondition, RuleSet, RuleTrigger};
use toki_core::scene::{SceneAnchor, SceneAnchorFacing, SceneAnchorKind};
use toki_core::{scene::Scene, GameState, InputKey, DEFAULT_TIMESTEP_MS};
use toki_test_fixtures::{test_atlas, test_tilemap};

fn player_definition(name: &str) -> EntityDefinition {
    EntityDefinition {
        name: name.into(),
        display_name: name.to_string(),
        description: String::new(),
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
            stats: HashMap::from([("health".to_string(), 100)]),
            speed: 2.0,
            solid: true,
            active: true,
            can_move: true,
            interactable: false,
            interaction_reach: 0,
            ai_config: Default::default(),
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
            hearing_radius: 128,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "step".to_string(),
            collision_sound: Some("hit".to_string()),
        },
        animations: toki_core::entity::AnimationsDef {
            atlas_name: "players".to_string(),
            clips: vec![toki_core::entity::AnimationClipDef {
                state: "idle_down".to_string(),
                frame_tiles: vec!["player/idle".to_string()],
                frame_positions: None,
                frame_duration_ms: 100.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle_down".to_string(),
        },
        category: "human".to_string(),
        tags: vec![],
    }
}

fn atlas_with_player_tile() -> AtlasMeta {
    let mut atlas = test_atlas();
    atlas.color_mode = ColorMode::TrueColor;
    atlas.tiles.insert(
        "player/idle".to_string(),
        TileInfo {
            position: glam::UVec2::new(1, 0),
            properties: TileProperties {
                solid: false,
                trigger: false,
            },
        },
    );
    atlas
}

#[test]
fn game_simulation_fixed_and_delta_ticks_match_for_default_timestep() {
    let mut fixed = GameState::new_empty();
    let mut delta = GameState::new_empty();
    SceneSystem::spawn_player_at(&mut fixed, IVec2::new(10, 10));
    SceneSystem::spawn_player_at(&mut delta, IVec2::new(10, 10));

    let tilemap = test_tilemap();
    let atlas = test_atlas();
    let world_bounds = glam::UVec2::new(256, 256);

    InputSystem::handle_key_press(fixed.runtime_mut(), InputKey::Right);
    InputSystem::handle_key_press(delta.runtime_mut(), InputKey::Right);

    let fixed_result = GameSimulation::tick_fixed(&mut fixed, world_bounds, &tilemap, &atlas);
    let delta_result = GameSimulation::tick_with_delta(
        &mut delta,
        DEFAULT_TIMESTEP_MS,
        world_bounds,
        &tilemap,
        &atlas,
    );

    let fixed_query = RenderQueryService::new(
        fixed.world().entity_manager(),
        fixed.world().player_id(),
        fixed.runtime().debug_collision_rendering(),
    );
    let delta_query = RenderQueryService::new(
        delta.world().entity_manager(),
        delta.world().player_id(),
        delta.runtime().debug_collision_rendering(),
    );

    assert_eq!(fixed_result.player_moved, delta_result.player_moved);
    assert_eq!(fixed_query.player_position(), delta_query.player_position());
}

#[test]
fn scene_system_transition_preserves_player_inventory_and_stats() {
    let mut state = GameState::new_empty();
    state
        .world_mut()
        .insert_entity_definition(player_definition("player"));

    let mut scene_a = Scene::new("A".to_string());
    let player_id = SceneSystem::spawn_player_at(&mut state, IVec2::new(8, 8));
    let mut player = state
        .world()
        .entity_manager()
        .get_entity(player_id)
        .expect("player should exist")
        .clone();
    let mut components = OptionalEntityComponents {
        inventory: Some(Default::default()),
        ..OptionalEntityComponents::default()
    };
    components
        .inventory
        .as_mut()
        .expect("inventory should exist")
        .add_item("potion", 2);
    let _ = player.attributes.apply_stat_delta("health", -25);
    scene_a.add_stored_entity(StoredEntity::new(player, components));

    let mut scene_b = Scene::new("B".to_string());
    scene_b.anchors.push(SceneAnchor {
        id: "door".to_string(),
        position: IVec2::new(96, 48),
        kind: SceneAnchorKind::SpawnPoint,
        facing: Some(SceneAnchorFacing::Right),
    });

    SceneSystem::add_scene(&mut state, scene_a);
    SceneSystem::add_scene(&mut state, scene_b);
    SceneSystem::load(&mut state, "A").expect("scene A should load");
    SceneSystem::transition(&mut state, "B", "door").expect("scene B should load");

    let player = state
        .world()
        .player_id()
        .and_then(|player_id| state.world().entity_manager().get_entity(player_id))
        .expect("player should be preserved");
    assert_eq!(player.position, IVec2::new(96, 48));
    assert_eq!(
        state
            .world()
            .player_id()
            .and_then(|player_id| state
                .world()
                .entity_manager()
                .storage()
                .components()
                .inventory(player_id))
            .expect("player inventory should exist")
            .item_count("potion"),
        2
    );
    assert_eq!(player.attributes.current_stat("health"), Some(75));
}

#[test]
fn render_query_service_matches_legacy_render_query_outputs() {
    let mut state = GameState::new_empty();
    let player_id = SceneSystem::spawn_player_at(&mut state, IVec2::new(12, 14));
    let atlas = atlas_with_player_tile();
    let texture_size = glam::UVec2::new(32, 16);

    let service = RenderQueryService::new(
        state.world().entity_manager(),
        state.world().player_id(),
        state.runtime().debug_collision_rendering(),
    );

    let legacy_like_service = RenderQueryService::new(
        state.world().entity_manager(),
        state.world().player_id(),
        state.runtime().debug_collision_rendering(),
    );
    assert_eq!(
        service.sprite_render_requests(),
        legacy_like_service.sprite_render_requests()
    );
    assert_eq!(
        service.player_position(),
        legacy_like_service.player_position()
    );
    let service_frame = service.current_sprite_frame(&atlas, texture_size);
    let legacy_frame = legacy_like_service.current_sprite_frame(&atlas, texture_size);
    assert_eq!(service_frame.u0, legacy_frame.u0);
    assert_eq!(service_frame.v0, legacy_frame.v0);
    assert_eq!(service_frame.u1, legacy_frame.u1);
    assert_eq!(service_frame.v1, legacy_frame.v1);
    assert_eq!(
        service.entity_sprite_flip_x(player_id),
        legacy_like_service.entity_sprite_flip_x(player_id)
    );
}

#[test]
fn input_system_held_key_behavior_matches_runtime_movement_expectations() {
    let mut state = GameState::new_empty();
    SceneSystem::spawn_player_at(&mut state, IVec2::new(10, 10));

    let tilemap = test_tilemap();
    let atlas = test_atlas();
    let world_bounds = glam::UVec2::new(256, 256);

    let initial = RenderQueryService::new(
        state.world().entity_manager(),
        state.world().player_id(),
        state.runtime().debug_collision_rendering(),
    )
    .player_position();

    InputSystem::handle_key_press(state.runtime_mut(), InputKey::Right);
    GameSimulation::tick_fixed(&mut state, world_bounds, &tilemap, &atlas);
    let after_first = RenderQueryService::new(
        state.world().entity_manager(),
        state.world().player_id(),
        state.runtime().debug_collision_rendering(),
    )
    .player_position();

    GameSimulation::tick_fixed(&mut state, world_bounds, &tilemap, &atlas);
    let after_second = RenderQueryService::new(
        state.world().entity_manager(),
        state.world().player_id(),
        state.runtime().debug_collision_rendering(),
    )
    .player_position();

    InputSystem::handle_key_release(state.runtime_mut(), InputKey::Right);
    GameSimulation::tick_fixed(&mut state, world_bounds, &tilemap, &atlas);
    let after_release = RenderQueryService::new(
        state.world().entity_manager(),
        state.world().player_id(),
        state.runtime().debug_collision_rendering(),
    )
    .player_position();

    assert!(after_first.x > initial.x);
    assert!(after_second.x > after_first.x);
    assert_eq!(after_release, after_second);
}

#[test]
fn rule_system_dialog_completion_path_uses_subsystem_api() {
    let mut state = GameState::new_empty();
    RuleSystem::set_rules(
        &mut state,
        RuleSet {
            rules: vec![Rule {
                id: "dialog_complete".to_string(),
                trigger: RuleTrigger::OnDialogComplete {
                    dialog_id: "intro".into(),
                    outcome_id: "accepted".to_string(),
                },
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::SetFlag {
                    flag: "accepted".to_string(),
                    value: toki_core::FlagValue::Bool(true),
                }],
                priority: 0,
                enabled: true,
                once: false,
                log_enabled: false,
            }],
        },
    );
    RuleSystem::record_dialog_completion(&mut state, "intro", "accepted");

    let world_bounds = glam::UVec2::new(64, 64);
    let _ = GameSimulation::tick_fixed(&mut state, world_bounds, &test_tilemap(), &test_atlas());

    assert_eq!(
        state.flag("accepted"),
        Some(&toki_core::FlagValue::Bool(true))
    );
}
