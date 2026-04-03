use glam::{IVec2, UVec2};
use std::collections::HashMap;
use toki_core::entity::{
    build_decoration_entity, AnimationClipDef, AnimationsDef, AudioDef, CollisionDef,
    CombatComponent, ComponentsDef, ControlRole, DecorationSpec, EntityDefinition, EntityKind,
    EntityStats, Inventory, MovementComponent, MovementProfile, MovementSoundTrigger, RenderingDef,
};
use toki_core::scene::{Scene, SceneAnchor, SceneAnchorFacing, SceneAnchorKind, ScenePlayerEntry};
use toki_core::{animation::AnimationState, game::SceneSystem, GameState};

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
            has_drop_shadow: false,
            palette_override: None,
            static_object: None,
            grounding: Default::default(),
        },
        solid: true,
        active: true,
        components: ComponentsDef {
            movement: Some(MovementComponent {
                speed: 2.0,
                movement_profile: MovementProfile::PlayerWasd,
                can_move: true,
            }),
            combat: Some(CombatComponent {
                health: Some(100),
                stats: EntityStats {
                    base: HashMap::from([
                        ("health".to_string(), 100),
                        ("attack_power".to_string(), 8),
                    ]),
                    current: HashMap::from([
                        ("health".to_string(), 100),
                        ("attack_power".to_string(), 8),
                    ]),
                },
            }),
            inventory: Some(Inventory::default()),
            ..Default::default()
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
fn scene_transition_preserves_durable_player_state() {
    let mut game_state = GameState::new_empty();
    game_state
        .world_mut()
        .insert_entity_definition(player_definition("player"));
    game_state
        .world_mut()
        .insert_entity_definition(player_definition("player_knight"));

    let mut scene_a = Scene::new("Scene A".to_string());
    let mut hero = player_definition("player")
        .create_spawn_bundle(IVec2::new(8, 8), 7)
        .expect("hero should instantiate");
    hero.entity.control_role = ControlRole::PlayerCharacter;
    hero.entity.entity_kind = EntityKind::Player;
    scene_a.add_stored_entity(toki_core::entity::StoredEntity::new(
        hero.entity,
        hero.optional_components,
    ));

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
            .combat_mut(player_id)
            .expect("player combat component should exist")
            .apply_stat_delta("health", -35);
        entity_manager
            .storage_mut()
            .components_mut()
            .ensure_inventory(player_id)
            .add_item("coin", 3);
        let player = entity_manager
            .get_entity_mut(player_id)
            .expect("player should exist");
        let controller = player
            .rendering
            .animation_controller
            .as_mut()
            .expect("player controller should exist");
        controller.play(AnimationState::AttackRight);
        controller.is_finished = true;
    }

    SceneSystem::transition(&mut game_state, "Scene B", "from_gate")
        .expect("scene transition should succeed");

    let player = game_state
        .world()
        .entity_manager()
        .get_entity(player_id)
        .expect("player should still exist");
    assert_eq!(
        game_state.scene().scene_manager().active_scene_name(),
        Some("Scene B")
    );
    assert_eq!(player.position, IVec2::new(128, 64));
    assert_eq!(player.definition_name.as_deref(), Some("player_knight"));
    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .combat(player_id)
            .and_then(|combat| combat.current_stat("health")),
        Some(65)
    );
    assert_eq!(
        game_state
            .world()
            .entity_manager()
            .storage()
            .components()
            .inventory(player_id)
            .expect("inventory should persist")
            .item_count("coin"),
        3
    );
}

#[test]
fn scene_transition_remaps_preserved_player_id_when_destination_scene_uses_it() {
    let mut game_state = GameState::new_empty();
    game_state
        .world_mut()
        .insert_entity_definition(player_definition("player"));

    let mut scene_a = Scene::new("Scene A".to_string());
    scene_a.add_anchor(scene_anchor(
        "spawn_a",
        IVec2::new(8, 8),
        Some(SceneAnchorFacing::Down),
    ));
    scene_a.player_entry = Some(ScenePlayerEntry {
        entity_definition_name: "player".into(),
        spawn_point_id: "spawn_a".to_string(),
    });
    scene_a.add_entity(build_decoration_entity(
        27,
        DecorationSpec::new(IVec2::new(0, 0), UVec2::new(16, 16), "items", "coin"),
    ));

    let mut scene_b = Scene::new("Scene B".to_string());
    scene_b.add_anchor(scene_anchor("door", IVec2::new(64, 64), None));
    scene_b.add_entity(build_decoration_entity(
        28,
        DecorationSpec::new(IVec2::new(32, 32), UVec2::new(16, 16), "items", "gem"),
    ));

    SceneSystem::add_scene(&mut game_state, scene_a);
    SceneSystem::add_scene(&mut game_state, scene_b);
    SceneSystem::load(&mut game_state, "Scene A").expect("initial scene should load");

    let startup_player_id = game_state.world().player_id().expect("player should exist");
    assert_eq!(startup_player_id, 28);

    SceneSystem::transition(&mut game_state, "Scene B", "door")
        .expect("scene transition should succeed");

    let transitioned_player_id = game_state.world().player_id().expect("player should exist");
    assert_eq!(transitioned_player_id, 29);

    let destination_entity = game_state
        .world()
        .entity_manager()
        .get_entity(28)
        .expect("authored scene entity should still exist");
    assert_eq!(destination_entity.entity_kind, EntityKind::Decoration);
    let static_render = destination_entity
        .rendering
        .static_object_render
        .as_ref()
        .expect("destination entity should keep its static render");
    assert_eq!(static_render.sheet, "items");
    assert_eq!(static_render.object_name, "gem");

    let player = game_state
        .world()
        .entity_manager()
        .get_entity(transitioned_player_id)
        .expect("remapped player should exist");
    assert_eq!(player.entity_kind, EntityKind::Player);
    assert_eq!(player.position, IVec2::new(64, 64));
}
