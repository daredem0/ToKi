use glam::IVec2;
use std::collections::HashMap;
use toki_core::animation::AnimationState;
use toki_core::entity::*;

fn sample_definition() -> EntityDefinition {
    EntityDefinition {
        name: "test_player".to_string().into(),
        display_name: "Test Player".to_string(),
        description: "A test player entity".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 1,
            visible: true,
            has_shadow: true,
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
                        (HEALTH_STAT_ID.to_string(), 100),
                        (ATTACK_POWER_STAT_ID.to_string(), 8),
                    ]),
                    current: HashMap::from([
                        (HEALTH_STAT_ID.to_string(), 100),
                        (ATTACK_POWER_STAT_ID.to_string(), 8),
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
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "player_footsteps".to_string(),
            collision_sound: Some("player_collision".to_string()),
        },
        animations: AnimationsDef {
            atlas_name: "player_atlas".to_string(),
            clips: vec![
                AnimationClipDef {
                    state: "idle".to_string(),
                    frame_tiles: vec!["player/idle_0".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 300.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
                AnimationClipDef {
                    state: "walk".to_string(),
                    frame_tiles: vec!["player/walk_0".to_string(), "player/walk_1".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 150.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
            ],
            default_state: "idle".to_string(),
        },
        category: "human".to_string(),
        tags: vec!["player".to_string(), "hero".to_string()],
    }
}

#[test]
fn entity_definition_create_spawn_bundle_preserves_top_level_fields_and_components() {
    let entity_def = sample_definition();

    let bundle = entity_def
        .create_spawn_bundle(IVec2::new(100, 200), 42)
        .expect("definition should build");

    assert_eq!(bundle.entity.id, 42);
    assert_eq!(bundle.entity.position, IVec2::new(100, 200));
    assert_eq!(bundle.entity.size, glam::UVec2::new(16, 16));
    assert_eq!(bundle.entity.entity_kind, EntityKind::Player);
    assert_eq!(bundle.entity.category, "human");
    assert!(bundle.entity.solid);
    assert!(bundle.entity.active);
    assert!(bundle.entity.rendering.visible);
    assert_eq!(bundle.entity.rendering.render_layer, 1);
    assert_eq!(
        bundle
            .optional_components
            .movement
            .as_ref()
            .expect("movement should exist")
            .movement_profile,
        MovementProfile::PlayerWasd
    );
    assert_eq!(
        bundle
            .optional_components
            .combat
            .as_ref()
            .expect("combat should exist")
            .current_stat(HEALTH_STAT_ID),
        Some(100)
    );
    assert!(bundle.optional_components.inventory.is_some());
    assert!(bundle.entity.collision_box.is_some());
}

#[test]
fn entity_definition_create_entity_builds_animation_controller() {
    let entity = sample_definition()
        .create_entity(IVec2::new(8, 12), 7)
        .expect("entity should instantiate");

    let controller = entity
        .rendering
        .animation_controller
        .expect("animation controller should exist");
    assert!(controller.clips.contains_key(&AnimationState::Idle));
    assert!(controller.clips.contains_key(&AnimationState::Walk));
}

#[test]
fn entity_definition_without_collision_yields_no_collision_box() {
    let mut entity_def = sample_definition();
    entity_def.collision.enabled = false;

    let entity = entity_def
        .create_entity(IVec2::new(5, 9), 99)
        .expect("entity should instantiate");

    assert!(entity.collision_box.is_none());
}

#[test]
fn entity_definition_npc_can_use_ai_without_inventory_or_movement() {
    let mut entity_def = sample_definition();
    entity_def.category = "creature".to_string();
    entity_def.components.inventory = None;
    entity_def.components.movement = Some(MovementComponent {
        speed: 1.0,
        movement_profile: MovementProfile::LegacyDefault,
        can_move: false,
    });
    entity_def.components.ai = Some(AiComponent {
        ai_config: AiConfig::from_legacy_behavior(AiBehavior::Wander),
    });

    let bundle = entity_def
        .create_spawn_bundle(IVec2::new(40, 50), 100)
        .expect("npc bundle should build");

    assert_eq!(bundle.entity.entity_kind, EntityKind::Npc);
    assert!(!bundle
        .optional_components
        .movement
        .as_ref()
        .expect("movement should exist")
        .can_move);
    assert_eq!(
        bundle
            .optional_components
            .ai
            .as_ref()
            .expect("ai should exist")
            .ai_config
            .behavior,
        AiBehavior::Wander
    );
    assert!(bundle.optional_components.inventory.is_none());
}

#[test]
fn entity_definition_round_trip_serialization_uses_componentized_shape() {
    let entity_def = sample_definition();

    let json = serde_json::to_string_pretty(&entity_def).expect("definition should serialize");
    assert!(json.contains("\"components\""));
    assert!(json.contains("\"movement\""));
    assert!(json.contains("\"combat\""));
    assert!(!json.contains("\"attributes\""));

    let deserialized: EntityDefinition =
        serde_json::from_str(&json).expect("definition should deserialize");
    assert!(deserialized.solid);
    assert!(deserialized.active);
    assert_eq!(
        deserialized
            .components
            .movement
            .as_ref()
            .expect("movement should exist")
            .speed,
        2.0
    );
    assert!(deserialized.components.inventory.is_some());
}

#[test]
fn entity_definition_supports_static_object_rendering_without_shadow() {
    let mut entity_def = sample_definition();
    entity_def.rendering.has_shadow = false;
    entity_def.rendering.static_object = Some(StaticObjectRenderDef {
        sheet: "items".to_string(),
        object_name: "coin".to_string(),
    });

    let entity = entity_def
        .create_entity(IVec2::new(24, 24), 11)
        .expect("entity should instantiate");

    assert!(!entity.rendering.has_shadow);
    let static_render = entity
        .rendering
        .static_object_render
        .as_ref()
        .expect("static render should exist");
    assert_eq!(static_render.sheet, "items");
    assert_eq!(static_render.object_name, "coin");
}

#[test]
fn entity_definition_rejects_unknown_animation_state() {
    let mut entity_def = sample_definition();
    entity_def.animations.clips[0].state = "not_a_real_state".to_string();

    let error = entity_def
        .create_entity(IVec2::ZERO, 1)
        .expect_err("invalid animation state should fail");
    assert!(matches!(
        error,
        EntityDefinitionError::UnknownAnimationState { .. }
    ));
}

#[test]
fn entity_definition_rejects_unknown_loop_mode() {
    let mut entity_def = sample_definition();
    entity_def.animations.clips[0].loop_mode = "bad_loop".to_string();

    let error = entity_def
        .create_entity(IVec2::ZERO, 1)
        .expect_err("invalid loop mode should fail");
    assert!(matches!(
        error,
        EntityDefinitionError::UnknownLoopMode { .. }
    ));
}
