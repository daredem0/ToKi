use glam::{IVec2, UVec2};
mod support;
use support::{test_atlas, test_entity_definition, test_tilemap};
use toki_core::entity::{
    build_decoration_entity, CombatComponent, DecorationSpec, Entity, EntityKind, EntityStats,
    MovementComponent, MovementProfile, OptionalEntityComponents, PickupDef, PrimaryProjectileDef,
    StoredEntity,
};
use toki_core::game::{
    AudioChannel, AudioEvent, GameSimulation, InputAction, InputSystem, RenderQueryService,
    SceneSystem,
};
use toki_core::scene::Scene;
use toki_core::sprite::{Animation, Frame, SpriteInstance, SpriteSheetMeta};
use toki_core::{GameState, InputKey, DEFAULT_TIMESTEP_MS};

fn create_test_sprite() -> SpriteInstance {
    let animation = Animation {
        name: "game_test".into(),
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
    SpriteInstance::new(IVec2::new(0, 0), animation, sprite_sheet)
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

fn player_id(state: &GameState) -> u32 {
    state.world().player_id().expect("player should exist")
}

fn player_inventory_count(state: &GameState, item_id: &str) -> u32 {
    state
        .world()
        .entity_manager()
        .storage()
        .components()
        .inventory(player_id(state))
        .map(|inventory| inventory.item_count(item_id))
        .unwrap_or(0)
}

fn create_solid_test_atlas() -> toki_core::assets::atlas::AtlasMeta {
    let mut atlas = test_atlas();
    atlas
        .tiles
        .get_mut("floor")
        .expect("floor tile should exist")
        .properties
        .solid = true;
    atlas
}

fn spawn_definition_entity(
    state: &mut GameState,
    definition: &toki_core::entity::EntityDefinition,
    position: IVec2,
) -> u32 {
    state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(definition, position)
        .expect("definition spawn should succeed")
}

fn prepare_player_attack_clips(state: &mut GameState) {
    use toki_core::animation::{AnimationClip, AnimationState, LoopMode};

    let player = state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .rendering
        .animation_controller
        .as_mut()
        .expect("player animation controller should exist");
    controller.add_clip(AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/idle_right".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
        loop_mode: LoopMode::Loop,
    });
    controller.add_clip(AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
        loop_mode: LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);
}

#[test]
fn fixed_tick_moves_player_right() {
    let mut state = GameState::new(create_test_sprite());
    let before = player_position(&state);

    InputSystem::handle_key_press(state.runtime_mut(), InputKey::Right);
    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(1000, 1000),
        &test_tilemap(),
        &test_atlas(),
    );

    let after = player_position(&state);
    assert!(after.x > before.x);
    assert_eq!(after.y, before.y);
}

#[test]
fn fixed_tick_allows_diagonal_player_movement() {
    let mut state = GameState::new(create_test_sprite());
    let before = player_position(&state);

    InputSystem::handle_key_press(state.runtime_mut(), InputKey::Right);
    InputSystem::handle_key_press(state.runtime_mut(), InputKey::Up);
    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(1000, 1000),
        &test_tilemap(),
        &test_atlas(),
    );

    let after = player_position(&state);
    assert!(after.x > before.x);
    assert_ne!(after, before);
}

#[test]
fn render_queries_include_visible_decoration_entities() {
    let mut state = GameState::new_empty();
    let mut scene = Scene::new("main".to_string());
    scene.add_entity(build_decoration_entity(
        7,
        DecorationSpec::new(IVec2::new(32, 48), UVec2::new(16, 16), "items", "coin"),
    ));
    SceneSystem::add_scene(&mut state, scene);
    SceneSystem::load(&mut state, "main").expect("scene should load");

    let renderable_ids = render_queries(&state)
        .renderable_entities()
        .into_iter()
        .map(|(id, _, _)| id)
        .collect::<Vec<_>>();

    assert!(renderable_ids.contains(&7));
}

#[test]
fn render_queries_skip_hidden_decoration_entities() {
    let mut state = GameState::new_empty();
    let mut scene = Scene::new("main".to_string());
    let mut decoration = build_decoration_entity(
        8,
        DecorationSpec::new(IVec2::new(16, 16), UVec2::new(16, 16), "items", "gem"),
    );
    decoration.rendering.visible = false;
    scene.add_entity(decoration);
    SceneSystem::add_scene(&mut state, scene);
    SceneSystem::load(&mut state, "main").expect("scene should load");

    let renderable_ids = render_queries(&state)
        .renderable_entities()
        .into_iter()
        .map(|(id, _, _)| id)
        .collect::<Vec<_>>();

    assert!(!renderable_ids.contains(&8));
}

#[test]
fn entity_health_bars_only_include_active_visible_combat_entities() {
    let mut state = GameState::new_empty();
    let mut scene = Scene::new("main".to_string());

    let visible_fighter = Entity {
        id: 1,
        position: IVec2::new(20, 30),
        size: UVec2::new(16, 16),
        entity_kind: EntityKind::Npc,
        category: "creature".to_string(),
        definition_name: None,
        persistent_across_saves: false,
        control_role: toki_core::entity::ControlRole::None,
        audio: toki_core::entity::EntityAudioSettings::default(),
        rendering: toki_core::entity::EntityRendering::default(),
        collision_box: None,
        solid: true,
        active: true,
        movement_accumulator: glam::Vec2::ZERO,
        tags: vec![],
    };
    let mut hidden_fighter = visible_fighter.clone();
    hidden_fighter.id = 2;
    hidden_fighter.rendering.visible = false;
    let mut inactive_fighter = visible_fighter.clone();
    inactive_fighter.id = 3;
    inactive_fighter.active = false;

    let mut stats = EntityStats::from_legacy_health(Some(20));
    stats.current.insert("health".to_string(), 12);
    let combat = CombatComponent {
        health: Some(12),
        stats,
    };

    scene.add_stored_entity(StoredEntity::new(
        visible_fighter,
        OptionalEntityComponents {
            combat: Some(combat.clone()),
            ..Default::default()
        },
    ));
    scene.add_stored_entity(StoredEntity::new(
        hidden_fighter,
        OptionalEntityComponents {
            combat: Some(combat.clone()),
            ..Default::default()
        },
    ));
    scene.add_stored_entity(StoredEntity::new(
        inactive_fighter,
        OptionalEntityComponents {
            combat: Some(combat),
            ..Default::default()
        },
    ));

    SceneSystem::add_scene(&mut state, scene);
    SceneSystem::load(&mut state, "main").expect("scene should load");

    let bars = render_queries(&state).entity_health_bars();
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].entity_id, 1);
    assert_eq!(bars[0].current, 12);
    assert_eq!(bars[0].max, 20);
}

#[test]
fn build_decoration_entity_is_inactive_but_still_visible_to_render_queries() {
    let decoration = build_decoration_entity(
        99,
        DecorationSpec::new(IVec2::new(64, 80), UVec2::new(16, 16), "items", "coin"),
    );

    assert_eq!(decoration.entity_kind, EntityKind::Decoration);
    assert!(!decoration.active);
    assert!(decoration.rendering.static_object_render.is_some());
}

#[test]
fn fixed_tick_blocks_player_against_solid_entity_collision() {
    let mut state = GameState::new(create_test_sprite());
    let before = player_position(&state);

    let mut blocker = test_entity_definition("blocker", "creature");
    blocker.components.movement = Some(MovementComponent {
        speed: 0.0,
        movement_profile: MovementProfile::None,
        can_move: false,
    });
    spawn_definition_entity(&mut state, &blocker, before + IVec2::new(2, 0));

    InputSystem::handle_key_press(state.runtime_mut(), InputKey::Right);
    let result = GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );

    assert!(!result.player_moved);
    assert_eq!(player_position(&state), before);
}

#[test]
fn fixed_tick_allows_player_to_move_through_non_solid_entity() {
    let mut state = GameState::new(create_test_sprite());
    let before = player_position(&state);

    let mut blocker = test_entity_definition("ghost", "item");
    blocker.solid = false;
    blocker.collision.enabled = false;
    blocker.components.combat = None;
    blocker.components.movement = Some(MovementComponent {
        speed: 0.0,
        movement_profile: MovementProfile::None,
        can_move: false,
    });
    spawn_definition_entity(&mut state, &blocker, before + IVec2::new(2, 0));

    InputSystem::handle_key_press(state.runtime_mut(), InputKey::Right);
    let result = GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );

    assert!(result.player_moved);
    assert!(player_position(&state).x > before.x);
}

#[test]
fn fixed_tick_collects_overlapping_pickup_and_despawns_it() {
    let mut state = GameState::new(create_test_sprite());
    let pickup_position = player_position(&state);

    let mut pickup = test_entity_definition("coin_pickup", "item");
    pickup.solid = false;
    pickup.collision.enabled = false;
    pickup.components.combat = None;
    pickup.components.movement = Some(MovementComponent {
        speed: 0.0,
        movement_profile: MovementProfile::None,
        can_move: false,
    });
    pickup.components.pickup = Some(PickupDef {
        item_id: "coin".to_string(),
        count: 3,
    });

    let pickup_id = spawn_definition_entity(&mut state, &pickup, pickup_position);
    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );

    assert_eq!(player_inventory_count(&state, "coin"), 3);
    assert!(state.world().entity_manager().get_entity(pickup_id).is_none());
}

#[test]
fn fixed_tick_pickup_collection_stacks_without_double_collecting() {
    let mut state = GameState::new(create_test_sprite());
    let pickup_position = player_position(&state);

    let mut pickup = test_entity_definition("gem_pickup", "item");
    pickup.solid = false;
    pickup.collision.enabled = false;
    pickup.components.combat = None;
    pickup.components.movement = Some(MovementComponent {
        speed: 0.0,
        movement_profile: MovementProfile::None,
        can_move: false,
    });
    pickup.components.pickup = Some(PickupDef {
        item_id: "gem".to_string(),
        count: 2,
    });

    spawn_definition_entity(&mut state, &pickup, pickup_position);
    spawn_definition_entity(&mut state, &pickup, pickup_position);

    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );
    assert_eq!(player_inventory_count(&state, "gem"), 4);

    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );
    assert_eq!(player_inventory_count(&state, "gem"), 4);
}

#[test]
fn primary_action_spawns_projectile_when_authored() {
    let mut state = GameState::new(create_test_sprite());
    prepare_player_attack_clips(&mut state);
    let player_id = player_id(&state);

    state
        .world_mut()
        .entity_manager_mut()
        .storage_mut()
        .components_mut()
        .set_primary_projectile(
            player_id,
            Some(PrimaryProjectileDef {
                sheet: "effects".to_string(),
                object_name: "bolt".to_string(),
                size: [8, 8],
                speed: 4,
                damage: 5,
                lifetime_ticks: 4,
                spawn_offset: [0, 0],
            }),
        );

    InputSystem::handle_profile_action_press(
        state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );

    assert_eq!(
        state.world().entity_manager().entities_of_kind(&EntityKind::Projectile).len(),
        1
    );
}

#[test]
fn projectile_moves_and_expires_after_lifetime() {
    let mut state = GameState::new(create_test_sprite());
    prepare_player_attack_clips(&mut state);
    let player_id = player_id(&state);

    state
        .world_mut()
        .entity_manager_mut()
        .storage_mut()
        .components_mut()
        .set_primary_projectile(
            player_id,
            Some(PrimaryProjectileDef {
                sheet: "effects".to_string(),
                object_name: "spark".to_string(),
                size: [8, 8],
                speed: 3,
                damage: 1,
                lifetime_ticks: 2,
                spawn_offset: [0, 0],
            }),
        );

    InputSystem::handle_profile_action_press(
        state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );
    let projectile_id = *state
        .world()
        .entity_manager()
        .entities_of_kind(&EntityKind::Projectile)
        .first()
        .expect("projectile should be spawned");
    let first_position = state
        .world()
        .entity_manager()
        .get_entity(projectile_id)
        .expect("projectile should exist")
        .position;
    assert!(first_position.x > player_position(&state).x);

    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );
    assert!(state.world().entity_manager().get_entity(projectile_id).is_none());
}

#[test]
fn primary_action_damages_adjacent_target_once_per_press() {
    let mut state = GameState::new(create_test_sprite());
    prepare_player_attack_clips(&mut state);
    let player_position = player_position(&state);

    let mut target = test_entity_definition("melee_target", "creature");
    target.components.combat = Some(CombatComponent {
        health: Some(20),
        stats: EntityStats::from_legacy_health(Some(20)),
    });
    let target_id = spawn_definition_entity(&mut state, &target, player_position + IVec2::new(16, 0));

    InputSystem::handle_profile_action_press(
        state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );
    let health_after_first = state
        .world()
        .entity_manager()
        .combat(target_id)
        .and_then(|combat| combat.current_stat("health"));
    assert_eq!(health_after_first, Some(10));

    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );
    let health_while_held = state
        .world()
        .entity_manager()
        .combat(target_id)
        .and_then(|combat| combat.current_stat("health"));
    assert_eq!(health_while_held, Some(10));

    InputSystem::handle_profile_action_release(
        state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    InputSystem::handle_profile_action_press(
        state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );
    assert!(state.world().entity_manager().get_entity(target_id).is_none());
}

#[test]
fn primary_action_does_not_damage_out_of_range_target() {
    let mut state = GameState::new(create_test_sprite());
    prepare_player_attack_clips(&mut state);
    let player_position = player_position(&state);

    let mut target = test_entity_definition("far_target", "creature");
    target.components.combat = Some(CombatComponent {
        health: Some(20),
        stats: EntityStats::from_legacy_health(Some(20)),
    });
    let target_id = spawn_definition_entity(&mut state, &target, player_position + IVec2::new(64, 0));

    InputSystem::handle_profile_action_press(
        state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );

    assert_eq!(
        state.world()
            .entity_manager()
            .combat(target_id)
            .and_then(|combat| combat.current_stat("health")),
        Some(20)
    );
}

#[test]
fn fractional_speed_accumulates_then_moves() {
    let mut state = GameState::new(create_test_sprite());
    let before = player_position(&state);
    let player_id = player_id(&state);
    state
        .world_mut()
        .entity_manager_mut()
        .movement_mut(player_id)
        .expect("player movement should exist")
        .speed = 0.5;

    InputSystem::handle_key_press(state.runtime_mut(), InputKey::Right);
    let first = GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );
    assert!(!first.player_moved);
    assert_eq!(player_position(&state), before);

    let second = GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );
    assert!(second.player_moved || player_position(&state) != before);
    assert_eq!(player_position(&state), before + IVec2::new(1, 0));
}

#[test]
fn changing_direction_resets_fractional_movement_accumulator() {
    let mut state = GameState::new(create_test_sprite());
    let before = player_position(&state);
    let player_id = player_id(&state);
    state
        .world_mut()
        .entity_manager_mut()
        .movement_mut(player_id)
        .expect("player movement should exist")
        .speed = 0.5;

    InputSystem::handle_key_press(state.runtime_mut(), InputKey::Left);
    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );
    assert_eq!(player_position(&state), before);

    InputSystem::handle_key_release(state.runtime_mut(), InputKey::Left);
    InputSystem::handle_key_press(state.runtime_mut(), InputKey::Right);
    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );
    assert_eq!(player_position(&state), before);

    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );
    assert_eq!(player_position(&state), before + IVec2::new(1, 0));
}

#[test]
fn delta_ticks_scale_movement_distance() {
    let mut slow = GameState::new(create_test_sprite());
    let mut fast = GameState::new(create_test_sprite());
    let before = player_position(&slow);

    InputSystem::handle_key_press(slow.runtime_mut(), InputKey::Right);
    InputSystem::handle_key_press(fast.runtime_mut(), InputKey::Right);

    GameSimulation::tick_with_delta(
        &mut slow,
        DEFAULT_TIMESTEP_MS * 0.5,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );
    GameSimulation::tick_with_delta(
        &mut fast,
        DEFAULT_TIMESTEP_MS * 2.0,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );

    assert!(player_position(&fast).x - before.x > player_position(&slow).x - before.x);
}

#[test]
fn movement_and_collision_audio_events_use_entity_audio_configuration() {
    let mut movement_state = GameState::new(create_test_sprite());
    let movement_player_id = player_id(&movement_state);
    {
        let audio = movement_state
            .world_mut()
            .entity_manager_mut()
            .storage_mut()
            .audio_component_mut(movement_player_id)
            .expect("player audio should exist");
        audio.footstep_trigger_distance = 1.0;
        audio.movement_sound = Some("custom_step".to_string());
    }

    InputSystem::handle_key_press(movement_state.runtime_mut(), InputKey::Right);
    let movement_result = GameSimulation::tick_fixed(
        &mut movement_state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &test_atlas(),
    );
    assert!(movement_result.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound {
            channel: AudioChannel::Movement,
            sound_id,
            ..
        } if sound_id == "custom_step"
    )));

    let mut collision_state = GameState::new(create_test_sprite());
    let collision_player_id = player_id(&collision_state);
    {
        let audio = collision_state
            .world_mut()
            .entity_manager_mut()
            .storage_mut()
            .audio_component_mut(collision_player_id)
            .expect("player audio should exist");
        audio.collision_sound = Some("custom_hit".to_string());
    }
    InputSystem::handle_key_press(collision_state.runtime_mut(), InputKey::Right);
    let collision_result = GameSimulation::tick_fixed(
        &mut collision_state,
        UVec2::new(160, 160),
        &test_tilemap(),
        &create_solid_test_atlas(),
    );
    assert!(collision_result.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound {
            channel: AudioChannel::Collision,
            sound_id,
            ..
        } if sound_id == "custom_hit"
    )));
}
