use glam::IVec2;
use std::collections::HashMap;
use toki_core::entity::{
    AnimationClipDef, AnimationsDef, AttributesDef, AudioDef, CollisionDef, ControlRole,
    EntityDefinition, EntityKind, MovementProfile, MovementSoundTrigger, RenderingDef,
};
use toki_core::scene::{Scene, SceneAnchor, SceneAnchorFacing, SceneAnchorKind, ScenePlayerEntry};
use toki_core::{
    animation::AnimationState,
    game::SceneSystem,
    GameState,
};

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
        let controller = player
            .attributes
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
    assert_eq!(player.attributes.current_stat("health"), Some(65));
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
