use glam::IVec2;
use toki_core::animation::*;
use toki_core::entity::*;

#[test]
fn test_entity_definition_create_entity_basic() {
    let entity_def = EntityDefinition {
        name: "test_player".to_string(),
        display_name: "Test Player".to_string(),
        description: "A test player entity".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 1,
            visible: true,
            has_shadow: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: Some(100),
            stats: std::collections::HashMap::new(),
            speed: 2.0,
            solid: true,
            active: true,
            can_move: true,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::LegacyDefault,
            primary_projectile: None,
            pickup: None,
            has_inventory: true,
            interactable: false,
            interaction_reach: 0,
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
                    frame_tiles: vec!["player/idle_0".to_string(), "player/idle_1".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 300.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
                AnimationClipDef {
                    state: "walk".to_string(),
                    frame_tiles: vec![
                        "player/walk_0".to_string(),
                        "player/walk_1".to_string(),
                        "player/walk_2".to_string(),
                        "player/walk_3".to_string(),
                    ],
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
    };

    let position = IVec2::new(100, 200);
    let entity_id = 42;

    let result = entity_def.create_entity(position, entity_id);

    assert!(result.is_ok());
    let entity = result.unwrap();

    // Check basic properties
    assert_eq!(entity.id, entity_id);
    assert_eq!(entity.position, position);
    assert_eq!(entity.size.x, 16);
    assert_eq!(entity.size.y, 16);
    assert_eq!(entity.entity_kind, EntityKind::Npc);
    assert_eq!(entity.category, "human");
    assert_eq!(entity.control_role, ControlRole::LegacyDefault);
    assert_eq!(entity.effective_control_role(), ControlRole::None);

    // Check attributes
    assert_eq!(entity.attributes.health, Some(100));
    assert_eq!(entity.attributes.speed, 2.0);
    assert!(entity.attributes.solid);
    assert!(entity.attributes.visible);
    assert_eq!(entity.attributes.render_layer, 1);
    assert!(entity.attributes.active);
    assert!(entity.attributes.can_move);
    assert_eq!(
        entity.attributes.movement_profile,
        MovementProfile::LegacyDefault
    );
    assert!(entity.attributes.has_inventory);

    // Check collision
    assert!(entity.collision_box.is_some());
    let collision = entity.collision_box.unwrap();
    assert_eq!(collision.offset, IVec2::new(0, 0));
    assert_eq!(collision.size.x, 16);
    assert_eq!(collision.size.y, 16);
    assert!(!collision.trigger);

    // Check audio component conversion
    let audio_component = entity_def.create_audio_component();
    assert_eq!(audio_component.footstep_trigger_distance, 16.0);
    assert_eq!(
        audio_component.movement_sound_trigger,
        MovementSoundTrigger::Distance
    );
    assert_eq!(
        audio_component.movement_sound.as_deref(),
        Some("player_footsteps")
    );
    assert_eq!(
        audio_component.collision_sound.as_deref(),
        Some("player_collision")
    );

    // Check animation controller
    assert!(entity.attributes.animation_controller.is_some());
    let controller = entity.attributes.animation_controller.unwrap();
    assert_eq!(controller.clips.len(), 2);
    assert!(controller.clips.contains_key(&AnimationState::Idle));
    assert!(controller.clips.contains_key(&AnimationState::Walk));
}

#[test]
fn test_entity_definition_create_npc_entity() {
    let entity_def = EntityDefinition {
        name: "test_npc".to_string(),
        display_name: "Test NPC".to_string(),
        description: "A test NPC entity".to_string(),
        rendering: RenderingDef {
            size: [32, 32],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: Some(50),
            stats: std::collections::HashMap::new(),
            speed: 1.0,
            solid: true,
            active: true,
            can_move: false,
            ai_config: AiConfig::from_legacy_behavior(AiBehavior::Wander),
            movement_profile: MovementProfile::LegacyDefault,
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
            interactable: false,
            interaction_reach: 0,
        },
        collision: CollisionDef {
            enabled: false,
            offset: [0, 0],
            size: [32, 32],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "npc_footsteps".to_string(),
            collision_sound: Some("npc_collision".to_string()),
        },
        animations: AnimationsDef {
            atlas_name: "npc_atlas".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["npc/idle_0".to_string()],
                frame_positions: None,
                frame_duration_ms: 500.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle".to_string(),
        },
        category: "creature".to_string(),
        tags: vec!["friendly".to_string()],
    };

    let position = IVec2::new(50, 75);
    let entity_id = 100;

    let result = entity_def.create_entity(position, entity_id);

    assert!(result.is_ok());
    let entity = result.unwrap();

    // Check basic properties
    assert_eq!(entity.id, entity_id);
    assert_eq!(entity.position, position);
    assert_eq!(entity.size.x, 32);
    assert_eq!(entity.size.y, 32);
    assert_eq!(entity.entity_kind, EntityKind::Npc);
    assert_eq!(entity.category, "creature");
    assert_eq!(entity.effective_control_role(), ControlRole::None);

    // Check attributes specific to NPC
    assert!(!entity.attributes.can_move);
    assert!(!entity.attributes.has_inventory);
    assert_eq!(entity.attributes.ai_config.behavior, AiBehavior::Wander);
    assert_eq!(
        entity.attributes.movement_profile,
        MovementProfile::LegacyDefault
    );
    assert_eq!(entity.attributes.speed, 1.0);

    // Check no collision since disabled
    assert!(entity.collision_box.is_none());
}

#[test]
fn test_entity_definition_missing_ai_fields_defaults_to_none() {
    // Entities without any AI configuration default to behavior: None
    let entity_json = r#"
    {
      "name": "legacy_npc",
      "display_name": "Legacy NPC",
      "description": "Old NPC without ai_behavior or ai_config",
      "rendering": {
        "size": [16, 16],
        "render_layer": 0,
        "visible": true
      },
      "attributes": {
        "health": 10,
        "speed": 1,
        "solid": true,
        "active": true,
        "can_move": false,
        "has_inventory": false
      },
      "collision": {
        "enabled": true,
        "offset": [0, 0],
        "size": [16, 16],
        "trigger": false
      },
      "audio": {
        "footstep_trigger_distance": 32.0,
        "movement_sound": "npc_step"
      },
      "animations": {
        "atlas_name": "creatures",
        "clips": [
          {
            "state": "idle",
            "frame_tiles": ["slime/idle_0"],
            "frame_duration_ms": 200.0,
            "loop_mode": "loop"
          }
        ],
        "default_state": "idle"
      },
      "category": "creature",
      "tags": []
    }
    "#;

    let entity_def: EntityDefinition =
        serde_json::from_str(entity_json).expect("entity without AI fields should deserialize");
    assert_eq!(entity_def.attributes.ai_config.behavior, AiBehavior::None);

    let entity = entity_def
        .create_entity(IVec2::ZERO, 1)
        .expect("entity should still instantiate");
    assert_eq!(entity.attributes.ai_config.behavior, AiBehavior::None);
    assert_eq!(
        entity.attributes.movement_profile,
        MovementProfile::LegacyDefault
    );
}

#[test]
fn test_entity_definition_non_player_type_can_still_become_player_via_control_role_later() {
    let entity_def = EntityDefinition {
        name: "hero_creature".to_string(),
        display_name: "Hero Creature".to_string(),
        description: "Generic creature whose player role comes from the scene".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            static_object: Some(StaticObjectRenderDef {
                sheet: "items".to_string(),
                object_name: "coin".to_string(),
            }),
        },
        attributes: AttributesDef {
            health: Some(25),
            stats: std::collections::HashMap::new(),
            speed: 2.0,
            solid: true,
            active: true,
            can_move: true,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::PlayerWasd,
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
            interactable: false,
            interaction_reach: 0,
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
            movement_sound: "step".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "creatures".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["creature/idle_0".to_string()],
                frame_positions: None,
                frame_duration_ms: 200.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle".to_string(),
        },
        category: "creature".to_string(),
        tags: vec![],
    };

    let entity = entity_def
        .create_entity(IVec2::ZERO, 5)
        .expect("entity should instantiate");

    assert_eq!(entity.category, "creature");
    assert_eq!(entity.effective_control_role(), ControlRole::None);
}

#[test]
fn test_entity_definition_supports_explicit_player_wasd_movement_profile() {
    let entity_json = r#"
    {
      "name": "configured_player",
      "display_name": "Configured Player",
      "description": "Player with explicit movement profile",
      "rendering": {
        "size": [16, 16],
        "render_layer": 0,
        "visible": true
      },
      "attributes": {
        "health": 100,
        "speed": 2,
        "solid": true,
        "active": true,
        "can_move": true,
        "movement_profile": "player_wasd",
        "has_inventory": false
      },
      "collision": {
        "enabled": true,
        "offset": [0, 0],
        "size": [16, 16],
        "trigger": false
      },
      "audio": {
        "footstep_trigger_distance": 16.0,
        "movement_sound": "step"
      },
      "animations": {
        "atlas_name": "creatures",
        "clips": [
          {
            "state": "idle",
            "frame_tiles": ["slime/idle_0"],
            "frame_duration_ms": 200.0,
            "loop_mode": "loop"
          }
        ],
        "default_state": "idle"
      },
      "category": "human",
      "tags": []
    }
    "#;

    let entity_def: EntityDefinition =
        serde_json::from_str(entity_json).expect("entity json should deserialize");
    assert_eq!(
        entity_def.attributes.movement_profile,
        MovementProfile::PlayerWasd
    );

    let entity = entity_def
        .create_entity(IVec2::ZERO, 1)
        .expect("entity should instantiate");
    assert_eq!(
        entity.attributes.movement_profile,
        MovementProfile::PlayerWasd
    );
}

#[test]
fn test_entity_definition_accepts_directional_animation_states() {
    let entity_def = EntityDefinition {
        name: "directional_player".to_string(),
        display_name: "Directional Player".to_string(),
        description: "Player with directional walk clips".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 1,
            visible: true,
            has_shadow: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: Some(100),
            stats: std::collections::HashMap::new(),
            speed: 2.0,
            solid: true,
            active: true,
            can_move: true,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::LegacyDefault,
            primary_projectile: None,
            pickup: None,
            has_inventory: true,
            interactable: false,
            interaction_reach: 0,
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
            collision_sound: None,
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
                    state: "walk_down".to_string(),
                    frame_tiles: vec![
                        "player/walk_down_a".to_string(),
                        "player/walk_down_b".to_string(),
                    ],
                    frame_positions: None,
                    frame_duration_ms: 180.0,
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
            ],
            default_state: "idle_down".to_string(),
        },
        category: "human".to_string(),
        tags: vec!["player".to_string()],
    };

    let entity = entity_def
        .create_entity(IVec2::new(0, 0), 1)
        .expect("directional definition should parse");
    let controller = entity
        .attributes
        .animation_controller
        .expect("controller should exist");

    assert!(controller.clips.contains_key(&AnimationState::IdleDown));
    assert!(controller.clips.contains_key(&AnimationState::WalkDown));
    assert!(controller.clips.contains_key(&AnimationState::WalkRight));
    assert_eq!(controller.current_clip_state, AnimationState::IdleDown);
}

#[test]
fn test_entity_definition_accepts_optional_attack_animation_states() {
    let entity_def = EntityDefinition {
        name: "player".to_string(),
        display_name: "Player".to_string(),
        description: "Player with optional attack clips".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 1,
            visible: true,
            has_shadow: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: Some(100),
            stats: std::collections::HashMap::new(),
            speed: 2.0,
            solid: true,
            active: true,
            can_move: true,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::PlayerWasd,
            primary_projectile: None,
            pickup: None,
            has_inventory: true,
            interactable: false,
            interaction_reach: 0,
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
            collision_sound: None,
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
                    state: "attack_down".to_string(),
                    frame_tiles: vec![
                        "player/attack_down_a".to_string(),
                        "player/attack_down_b".to_string(),
                    ],
                    frame_positions: None,
                    frame_duration_ms: 120.0,
                    frame_durations_ms: None,
                    loop_mode: "once".to_string(),
                },
                AnimationClipDef {
                    state: "attack_left".to_string(),
                    frame_tiles: vec![
                        "player/attack_right_a".to_string(),
                        "player/attack_right_b".to_string(),
                    ],
                    frame_positions: None,
                    frame_duration_ms: 120.0,
                    frame_durations_ms: None,
                    loop_mode: "once".to_string(),
                },
                AnimationClipDef {
                    state: "attack".to_string(),
                    frame_tiles: vec!["player/attack_down_a".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 120.0,
                    frame_durations_ms: None,
                    loop_mode: "once".to_string(),
                },
            ],
            default_state: "idle_down".to_string(),
        },
        category: "human".to_string(),
        tags: vec!["player".to_string()],
    };

    let entity = entity_def
        .create_entity(IVec2::new(0, 0), 1)
        .expect("attack-capable definition should parse");
    let controller = entity
        .attributes
        .animation_controller
        .expect("controller should exist");

    assert!(controller.clips.contains_key(&AnimationState::Attack));
    assert!(controller.clips.contains_key(&AnimationState::AttackDown));
    assert!(controller.clips.contains_key(&AnimationState::AttackLeft));
    assert_eq!(controller.current_clip_state, AnimationState::IdleDown);
}

#[test]
fn test_entity_definition_seeds_generic_health_stat_from_legacy_health() {
    let entity_def = EntityDefinition {
        name: "slime".to_string(),
        display_name: "Slime".to_string(),
        description: "Stat-seeded slime".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            static_object: Some(StaticObjectRenderDef {
                sheet: "items".to_string(),
                object_name: "coin".to_string(),
            }),
        },
        attributes: AttributesDef {
            health: Some(25),
            stats: std::collections::HashMap::new(),
            speed: 1.0,
            solid: true,
            active: true,
            can_move: true,
            ai_config: AiConfig::from_legacy_behavior(AiBehavior::Wander),
            movement_profile: MovementProfile::None,
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
            interactable: false,
            interaction_reach: 0,
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
            movement_sound: "sfx_slime".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "creatures.json".to_string(),
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
        category: "creature".to_string(),
        tags: vec![],
    };

    let entity = entity_def
        .create_entity(IVec2::new(0, 0), 1)
        .expect("definition should create entity");

    assert_eq!(entity.attributes.health, Some(25));
    assert_eq!(entity.attributes.current_stat("health"), Some(25));
    assert_eq!(entity.attributes.base_stat("health"), Some(25));
}

#[test]
fn test_entity_definition_seeds_authored_attack_power_stat() {
    let entity_def = EntityDefinition {
        name: "fighter".to_string(),
        display_name: "Fighter".to_string(),
        description: "Attack-powered fighter".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            static_object: Some(StaticObjectRenderDef {
                sheet: "items".to_string(),
                object_name: "coin".to_string(),
            }),
        },
        attributes: AttributesDef {
            health: Some(30),
            stats: std::collections::HashMap::from([(ATTACK_POWER_STAT_ID.to_string(), 17)]),
            speed: 2.0,
            solid: true,
            active: true,
            can_move: true,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::PlayerWasd,
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
            interactable: false,
            interaction_reach: 0,
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
            movement_sound: "step".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "fighters".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["fighter/idle_0".to_string()],
                frame_positions: None,
                frame_duration_ms: 150.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle".to_string(),
        },
        category: "human".to_string(),
        tags: vec![],
    };

    let entity = entity_def
        .create_entity(IVec2::new(0, 0), 1)
        .expect("definition should create entity");

    assert_eq!(entity.attributes.health, Some(30));
    assert_eq!(entity.attributes.current_stat(HEALTH_STAT_ID), Some(30));
    assert_eq!(
        entity.attributes.current_stat(ATTACK_POWER_STAT_ID),
        Some(17)
    );
    assert_eq!(entity.attributes.base_stat(ATTACK_POWER_STAT_ID), Some(17));
}

#[test]
fn test_entity_definition_copies_authored_primary_projectile() {
    let entity_def = EntityDefinition {
        name: "ranger".to_string(),
        display_name: "Ranger".to_string(),
        description: "Projectile-capable ranger".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: Some(30),
            stats: std::collections::HashMap::from([(ATTACK_POWER_STAT_ID.to_string(), 8)]),
            speed: 2.0,
            solid: true,
            active: true,
            can_move: true,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::PlayerWasd,
            primary_projectile: Some(PrimaryProjectileDef {
                sheet: "fauna".to_string(),
                object_name: "rock".to_string(),
                size: [16, 16],
                speed: 4,
                damage: 8,
                lifetime_ticks: 20,
                spawn_offset: [1, 2],
            }),
            pickup: None,
            has_inventory: false,
            interactable: false,
            interaction_reach: 0,
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
            movement_sound: "step".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "ranger".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["ranger/idle_0".to_string()],
                frame_positions: None,
                frame_duration_ms: 150.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle".to_string(),
        },
        category: "human".to_string(),
        tags: vec![],
    };

    let entity = entity_def
        .create_entity(IVec2::new(0, 0), 1)
        .expect("definition should create entity");

    let projectile = entity
        .attributes
        .primary_projectile
        .expect("authored projectile should be copied to runtime entity");
    assert_eq!(projectile.sheet, "fauna");
    assert_eq!(projectile.object_name, "rock");
    assert_eq!(projectile.size, [16, 16]);
    assert_eq!(projectile.speed, 4);
    assert_eq!(projectile.damage, 8);
    assert_eq!(projectile.lifetime_ticks, 20);
    assert_eq!(projectile.spawn_offset, [1, 2]);
}

#[test]
fn test_entity_definition_copies_authored_pickup() {
    let entity_def = EntityDefinition {
        name: "coin_pickup".to_string(),
        display_name: "Coin Pickup".to_string(),
        description: "Collectible coin".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            static_object: Some(StaticObjectRenderDef {
                sheet: "items".to_string(),
                object_name: "coin".to_string(),
            }),
        },
        attributes: AttributesDef {
            health: None,
            stats: std::collections::HashMap::new(),
            speed: 0.0,
            solid: false,
            active: true,
            can_move: false,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::None,
            primary_projectile: None,
            pickup: Some(PickupDef {
                item_id: "coin".to_string(),
                count: 3,
            }),
            has_inventory: false,
            interactable: false,
            interaction_reach: 0,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: true,
        },
        audio: AudioDef {
            footstep_trigger_distance: 16.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "step".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "".to_string(),
            clips: vec![],
            default_state: "".to_string(),
        },
        category: "item".to_string(),
        tags: vec![],
    };

    let entity = entity_def
        .create_entity(IVec2::new(0, 0), 1)
        .expect("definition should create entity");

    let pickup = entity
        .attributes
        .pickup
        .expect("authored pickup should be copied to runtime entity");
    assert_eq!(pickup.item_id, "coin");
    assert_eq!(pickup.count, 3);
    assert_eq!(entity.attributes.inventory.item_count("coin"), 0);
    let static_render = entity
        .attributes
        .static_object_render
        .expect("static object render should be copied");
    assert_eq!(static_render.sheet, "items");
    assert_eq!(static_render.object_name, "coin");
    assert!(entity.attributes.animation_controller.is_none());
}

#[test]
fn test_entity_definition_unknown_category_defaults_to_actor_like_runtime_type() {
    let entity_def = EntityDefinition {
        name: "invalid".to_string(),
        display_name: "Invalid".to_string(),
        description: "Invalid entity type".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: None,
            stats: std::collections::HashMap::new(),
            speed: 1.0,
            solid: false,
            active: true,
            can_move: false,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::LegacyDefault,
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
            interactable: false,
            interaction_reach: 0,
        },
        collision: CollisionDef {
            enabled: false,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "test".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "test".to_string(),
            clips: vec![],
            default_state: "idle".to_string(),
        },
        category: "mystery".to_string(),
        tags: vec![],
    };

    let entity = entity_def
        .create_entity(IVec2::ZERO, 1)
        .expect("unknown generic category should still instantiate");
    assert_eq!(entity.entity_kind, EntityKind::Npc);
    assert_eq!(entity.category, "mystery");
}

#[test]
fn test_entity_definition_invalid_animation_state() {
    let entity_def = EntityDefinition {
        name: "invalid_anim".to_string(),
        display_name: "Invalid Animation".to_string(),
        description: "Invalid animation state".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: None,
            stats: std::collections::HashMap::new(),
            speed: 1.0,
            solid: false,
            active: true,
            can_move: false,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::LegacyDefault,
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
            interactable: false,
            interaction_reach: 0,
        },
        collision: CollisionDef {
            enabled: false,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "test".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "test".to_string(),
            clips: vec![AnimationClipDef {
                state: "invalid_state".to_string(), // Invalid state
                frame_tiles: vec!["test/frame_0".to_string()],
                frame_positions: None,
                frame_duration_ms: 100.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            }],
            default_state: "invalid_state".to_string(),
        },
        category: "test".to_string(),
        tags: vec![],
    };

    let result = entity_def.create_entity(IVec2::ZERO, 1);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown animation state"));
}

#[test]
fn test_entity_definition_invalid_loop_mode() {
    let entity_def = EntityDefinition {
        name: "invalid_loop".to_string(),
        display_name: "Invalid Loop Mode".to_string(),
        description: "Invalid loop mode".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: None,
            stats: std::collections::HashMap::new(),
            speed: 1.0,
            solid: false,
            active: true,
            can_move: false,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::LegacyDefault,
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
            interactable: false,
            interaction_reach: 0,
        },
        collision: CollisionDef {
            enabled: false,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "test".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "test".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["test/frame_0".to_string()],
                frame_positions: None,
                frame_duration_ms: 100.0,
                frame_durations_ms: None,
                loop_mode: "invalid_loop".to_string(), // Invalid loop mode
            }],
            default_state: "idle".to_string(),
        },
        category: "test".to_string(),
        tags: vec![],
    };

    let result = entity_def.create_entity(IVec2::ZERO, 1);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown loop mode"));
}

#[test]
fn test_entity_definition_serialization() {
    let entity_def = EntityDefinition {
        name: "serialize_test".to_string(),
        display_name: "Serialize Test".to_string(),
        description: "Test serialization".to_string(),
        rendering: RenderingDef {
            size: [8, 8],
            render_layer: -1,
            visible: false,
            has_shadow: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: None,
            stats: std::collections::HashMap::new(),
            speed: 0.0,
            solid: false,
            active: false,
            can_move: false,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::LegacyDefault,
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
            interactable: false,
            interaction_reach: 0,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [2, 2],
            size: [4, 4],
            trigger: true,
        },
        audio: AudioDef {
            footstep_trigger_distance: 0.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "items".to_string(),
            clips: vec![],
            default_state: "idle".to_string(),
        },
        category: "items".to_string(),
        tags: vec!["collectible".to_string(), "small".to_string()],
    };

    // Test serialization round-trip
    let json = serde_json::to_string_pretty(&entity_def).unwrap();
    let deserialized: EntityDefinition = serde_json::from_str(&json).unwrap();

    // Check that important fields are preserved
    assert_eq!(entity_def.name, deserialized.name);
    assert_eq!(entity_def.display_name, deserialized.display_name);
    assert_eq!(entity_def.rendering.size, deserialized.rendering.size);
    assert_eq!(
        entity_def.rendering.has_shadow,
        deserialized.rendering.has_shadow
    );
    assert_eq!(entity_def.attributes.speed, deserialized.attributes.speed);
    assert_eq!(entity_def.collision.enabled, deserialized.collision.enabled);
    assert_eq!(
        entity_def.audio.movement_sound,
        deserialized.audio.movement_sound
    );
    assert_eq!(
        entity_def.audio.collision_sound,
        deserialized.audio.collision_sound
    );
    assert_eq!(entity_def.category, deserialized.category);
    assert_eq!(entity_def.tags, deserialized.tags);
}

#[test]
fn entity_definition_rendering_defaults_has_shadow_to_true_when_missing() {
    let entity_json = r#"
{
  "name": "shadow_default",
  "display_name": "Shadow Default",
  "description": "",
  "rendering": {
    "size": [16, 16],
    "render_layer": 0,
    "visible": true
  },
  "attributes": {
    "health": null,
    "stats": {},
    "speed": 1.0,
    "solid": true,
    "active": true,
    "can_move": true,
    "interactable": false,
    "interaction_reach": 0,
    "ai_config": { "behavior": "none" },
    "movement_profile": "none",
    "has_inventory": false
  },
  "collision": {
    "enabled": false,
    "offset": [0, 0],
    "size": [16, 16],
    "trigger": false
  },
  "audio": {
    "footstep_trigger_distance": 0.0,
    "hearing_radius": 192,
    "movement_sound_trigger": "distance",
    "movement_sound": "",
    "collision_sound": null
  },
  "animations": {
    "atlas_name": "",
    "clips": [],
    "default_state": "idle"
  },
  "category": "npc",
  "tags": []
}
"#;

    let entity_def: EntityDefinition =
        serde_json::from_str(entity_json).expect("entity should deserialize");

    assert!(entity_def.rendering.has_shadow);
}

#[test]
fn create_entity_copies_has_shadow_from_definition_rendering() {
    let entity_def = EntityDefinition {
        name: "no_shadow".to_string(),
        display_name: "No Shadow".to_string(),
        description: "".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: false,
            static_object: None,
        },
        attributes: AttributesDef {
            health: None,
            stats: std::collections::HashMap::new(),
            speed: 1.0,
            solid: true,
            active: true,
            can_move: true,
            interactable: false,
            interaction_reach: 0,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::default(),
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: false,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 0.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "".to_string(),
            clips: vec![],
            default_state: "idle".to_string(),
        },
        category: "npc".to_string(),
        tags: vec![],
    };

    let entity = entity_def
        .create_entity(glam::IVec2::new(0, 0), 1)
        .expect("entity should be created");

    assert!(!entity.attributes.has_shadow);
}

#[test]
fn test_entity_definition_create_audio_component() {
    let entity_def = EntityDefinition {
        name: "audio_test".to_string(),
        display_name: "Audio Test".to_string(),
        description: "Audio component extraction".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: Some(100),
            stats: std::collections::HashMap::new(),
            speed: 2.0,
            solid: true,
            active: true,
            can_move: true,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::LegacyDefault,
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
            interactable: false,
            interaction_reach: 0,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 24.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::AnimationLoop,
            movement_sound: "sfx_custom_step".to_string(),
            collision_sound: Some("sfx_custom_hit".to_string()),
        },
        animations: AnimationsDef {
            atlas_name: "test".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["test_0".to_string()],
                frame_positions: None,
                frame_duration_ms: 100.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle".to_string(),
        },
        category: "test".to_string(),
        tags: vec![],
    };

    let audio_component = entity_def.create_audio_component();
    assert_eq!(audio_component.footstep_distance_accumulator, 0.0);
    assert_eq!(audio_component.footstep_trigger_distance, 24.0);
    assert_eq!(
        audio_component.movement_sound_trigger,
        MovementSoundTrigger::AnimationLoop
    );
    assert!(!audio_component.last_collision_state);
    assert_eq!(
        audio_component.movement_sound.as_deref(),
        Some("sfx_custom_step")
    );
    assert_eq!(
        audio_component.collision_sound.as_deref(),
        Some("sfx_custom_hit")
    );
}

// ============================================================================
// Phase 2A: Extended AI Behavior Tests
// ============================================================================

#[test]
fn test_ai_behavior_chase_serialization() {
    let json = r#""chase""#;
    let behavior: AiBehavior = serde_json::from_str(json).expect("should deserialize chase");
    assert_eq!(behavior, AiBehavior::Chase);

    let serialized = serde_json::to_string(&behavior).expect("should serialize chase");
    assert_eq!(serialized, r#""chase""#);
}

#[test]
fn test_ai_behavior_run_serialization() {
    let json = r#""run""#;
    let behavior: AiBehavior = serde_json::from_str(json).expect("should deserialize run");
    assert_eq!(behavior, AiBehavior::Run);

    let serialized = serde_json::to_string(&behavior).expect("should serialize run");
    assert_eq!(serialized, r#""run""#);
}

#[test]
fn test_ai_behavior_run_and_multiply_serialization() {
    let json = r#""run_and_multiply""#;
    let behavior: AiBehavior =
        serde_json::from_str(json).expect("should deserialize run_and_multiply");
    assert_eq!(behavior, AiBehavior::RunAndMultiply);

    let serialized = serde_json::to_string(&behavior).expect("should serialize run_and_multiply");
    assert_eq!(serialized, r#""run_and_multiply""#);
}

#[test]
fn test_ai_config_default_values() {
    let config = AiConfig::default();
    assert_eq!(config.behavior, AiBehavior::None);
    assert_eq!(config.detection_radius, 0);
}

#[test]
fn test_ai_config_with_chase_behavior() {
    let config = AiConfig {
        behavior: AiBehavior::Chase,
        detection_radius: 128,
    };
    assert_eq!(config.behavior, AiBehavior::Chase);
    assert_eq!(config.detection_radius, 128);
}

#[test]
fn test_ai_config_serialization_round_trip() {
    let config = AiConfig {
        behavior: AiBehavior::RunAndMultiply,
        detection_radius: 96,
    };

    let json = serde_json::to_string(&config).expect("should serialize ai_config");
    let deserialized: AiConfig = serde_json::from_str(&json).expect("should deserialize ai_config");

    assert_eq!(deserialized.behavior, config.behavior);
    assert_eq!(deserialized.detection_radius, config.detection_radius);
}

#[test]
fn test_entity_definition_with_ai_config() {
    let entity_json = r#"
    {
      "name": "chaser_npc",
      "display_name": "Chaser NPC",
      "description": "An NPC that chases the player",
      "rendering": {
        "size": [16, 16],
        "render_layer": 0,
        "visible": true
      },
      "attributes": {
        "health": 20,
        "speed": 2,
        "solid": true,
        "active": true,
        "can_move": false,
        "has_inventory": false,
        "ai_config": {
          "behavior": "chase",
          "detection_radius": 100
        }
      },
      "collision": {
        "enabled": true,
        "offset": [0, 0],
        "size": [16, 16],
        "trigger": false
      },
      "audio": {
        "footstep_trigger_distance": 32.0,
        "movement_sound": "npc_step"
      },
      "animations": {
        "atlas_name": "creatures",
        "clips": [
          {
            "state": "idle",
            "frame_tiles": ["slime/idle_0"],
            "frame_duration_ms": 200.0,
            "loop_mode": "loop"
          }
        ],
        "default_state": "idle"
      },
      "category": "creature",
      "tags": []
    }
    "#;

    let entity_def: EntityDefinition =
        serde_json::from_str(entity_json).expect("entity with ai_config should deserialize");

    assert_eq!(entity_def.attributes.ai_config.behavior, AiBehavior::Chase);
    assert_eq!(entity_def.attributes.ai_config.detection_radius, 100);

    let entity = entity_def
        .create_entity(IVec2::ZERO, 1)
        .expect("entity should instantiate");

    assert_eq!(entity.attributes.ai_config.behavior, AiBehavior::Chase);
    assert_eq!(entity.attributes.ai_config.detection_radius, 100);
}

#[test]
fn test_legacy_ai_behavior_backward_compatibility() {
    // Test that entities with only the old `ai_behavior` field still work
    let entity_json = r#"
    {
      "name": "legacy_wanderer",
      "display_name": "Legacy Wanderer",
      "description": "Old entity with just ai_behavior field",
      "rendering": {
        "size": [16, 16],
        "render_layer": 0,
        "visible": true
      },
      "attributes": {
        "health": 10,
        "speed": 1,
        "solid": true,
        "active": true,
        "can_move": false,
        "has_inventory": false,
        "ai_behavior": "wander"
      },
      "collision": {
        "enabled": true,
        "offset": [0, 0],
        "size": [16, 16],
        "trigger": false
      },
      "audio": {
        "footstep_trigger_distance": 32.0,
        "movement_sound": "npc_step"
      },
      "animations": {
        "atlas_name": "creatures",
        "clips": [
          {
            "state": "idle",
            "frame_tiles": ["slime/idle_0"],
            "frame_duration_ms": 200.0,
            "loop_mode": "loop"
          }
        ],
        "default_state": "idle"
      },
      "category": "creature",
      "tags": []
    }
    "#;

    let entity_def: EntityDefinition =
        serde_json::from_str(entity_json).expect("legacy entity should deserialize");

    // Legacy ai_behavior should be migrated to ai_config.behavior
    assert_eq!(entity_def.attributes.ai_config.behavior, AiBehavior::Wander);
    // detection_radius defaults to 0 for legacy entities
    assert_eq!(entity_def.attributes.ai_config.detection_radius, 0);

    let entity = entity_def
        .create_entity(IVec2::ZERO, 1)
        .expect("legacy entity should instantiate");

    assert_eq!(entity.attributes.ai_config.behavior, AiBehavior::Wander);
}

#[test]
fn test_ai_config_takes_precedence_over_legacy_ai_behavior() {
    // When both ai_config and ai_behavior are present, ai_config takes precedence
    let entity_json = r#"
    {
      "name": "mixed_ai_entity",
      "display_name": "Mixed AI Entity",
      "description": "Entity with both old and new AI fields",
      "rendering": {
        "size": [16, 16],
        "render_layer": 0,
        "visible": true
      },
      "attributes": {
        "health": 10,
        "speed": 1,
        "solid": true,
        "active": true,
        "can_move": false,
        "has_inventory": false,
        "ai_behavior": "wander",
        "ai_config": {
          "behavior": "chase",
          "detection_radius": 64
        }
      },
      "collision": {
        "enabled": true,
        "offset": [0, 0],
        "size": [16, 16],
        "trigger": false
      },
      "audio": {
        "footstep_trigger_distance": 32.0,
        "movement_sound": "npc_step"
      },
      "animations": {
        "atlas_name": "creatures",
        "clips": [
          {
            "state": "idle",
            "frame_tiles": ["slime/idle_0"],
            "frame_duration_ms": 200.0,
            "loop_mode": "loop"
          }
        ],
        "default_state": "idle"
      },
      "category": "creature",
      "tags": []
    }
    "#;

    let entity_def: EntityDefinition =
        serde_json::from_str(entity_json).expect("mixed entity should deserialize");

    // ai_config should take precedence
    assert_eq!(entity_def.attributes.ai_config.behavior, AiBehavior::Chase);
    assert_eq!(entity_def.attributes.ai_config.detection_radius, 64);
}

#[test]
fn test_entity_without_ai_fields_defaults_to_none() {
    let entity_json = r#"
    {
      "name": "no_ai_entity",
      "display_name": "No AI Entity",
      "description": "Entity without any AI fields",
      "rendering": {
        "size": [16, 16],
        "render_layer": 0,
        "visible": true
      },
      "attributes": {
        "health": 10,
        "speed": 1,
        "solid": true,
        "active": true,
        "can_move": false,
        "has_inventory": false
      },
      "collision": {
        "enabled": true,
        "offset": [0, 0],
        "size": [16, 16],
        "trigger": false
      },
      "audio": {
        "footstep_trigger_distance": 32.0,
        "movement_sound": "npc_step"
      },
      "animations": {
        "atlas_name": "creatures",
        "clips": [
          {
            "state": "idle",
            "frame_tiles": ["slime/idle_0"],
            "frame_duration_ms": 200.0,
            "loop_mode": "loop"
          }
        ],
        "default_state": "idle"
      },
      "category": "creature",
      "tags": []
    }
    "#;

    let entity_def: EntityDefinition =
        serde_json::from_str(entity_json).expect("entity without AI should deserialize");

    // Should default to None behavior with 0 detection radius
    assert_eq!(entity_def.attributes.ai_config.behavior, AiBehavior::None);
    assert_eq!(entity_def.attributes.ai_config.detection_radius, 0);
}

// ============================================================================
// Phase 4A: Animation Data Model - Position-Based Frame References
// ============================================================================

#[test]
fn test_animation_clip_def_with_frame_positions() {
    // Test that AnimationClipDef can use position-based frame references
    let clip_def = AnimationClipDef {
        state: "walk_down".to_string(),
        frame_tiles: vec![], // Empty when using positions
        frame_positions: Some(vec![[0, 0], [1, 0], [2, 0]]),
        frame_duration_ms: 150.0,
        frame_durations_ms: None,
        loop_mode: "loop".to_string(),
    };

    assert!(clip_def.frame_positions.is_some());
    let positions = clip_def.frame_positions.as_ref().unwrap();
    assert_eq!(positions.len(), 3);
    assert_eq!(positions[0], [0, 0]);
    assert_eq!(positions[1], [1, 0]);
    assert_eq!(positions[2], [2, 0]);
}

#[test]
fn test_animation_clip_def_with_per_frame_durations() {
    // Test optional per-frame duration overrides
    let clip_def = AnimationClipDef {
        state: "attack".to_string(),
        frame_tiles: vec![],
        frame_positions: Some(vec![[0, 1], [1, 1], [2, 1]]),
        frame_duration_ms: 100.0, // Default/fallback
        frame_durations_ms: Some(vec![50.0, 200.0, 50.0]), // Per-frame overrides
        loop_mode: "once".to_string(),
    };

    assert!(clip_def.frame_durations_ms.is_some());
    let durations = clip_def.frame_durations_ms.as_ref().unwrap();
    assert_eq!(durations.len(), 3);
    assert_eq!(durations[0], 50.0);
    assert_eq!(durations[1], 200.0);
    assert_eq!(durations[2], 50.0);
}

#[test]
fn test_animation_clip_def_json_with_frame_positions() {
    // Test JSON deserialization with position-based frames
    let json = r#"{
        "state": "idle_down",
        "frame_positions": [[0, 0], [1, 0]],
        "frame_duration_ms": 300.0,
        "loop_mode": "loop"
    }"#;

    let clip_def: AnimationClipDef =
        serde_json::from_str(json).expect("should deserialize position-based clip");

    assert!(clip_def.frame_positions.is_some());
    assert!(clip_def.frame_tiles.is_empty());
    assert_eq!(clip_def.frame_positions.as_ref().unwrap().len(), 2);
}

#[test]
fn test_animation_clip_def_json_with_per_frame_durations() {
    // Test JSON deserialization with per-frame duration overrides
    let json = r#"{
        "state": "attack_down",
        "frame_positions": [[0, 1], [1, 1], [2, 1]],
        "frame_duration_ms": 100.0,
        "frame_durations_ms": [80.0, 150.0, 80.0],
        "loop_mode": "once"
    }"#;

    let clip_def: AnimationClipDef =
        serde_json::from_str(json).expect("should deserialize clip with per-frame durations");

    assert!(clip_def.frame_durations_ms.is_some());
    let durations = clip_def.frame_durations_ms.as_ref().unwrap();
    assert_eq!(durations, &[80.0, 150.0, 80.0]);
}

#[test]
fn test_animation_clip_def_legacy_frame_tiles_still_works() {
    // Ensure backward compatibility with legacy frame_tiles format
    let json = r#"{
        "state": "walk",
        "frame_tiles": ["player/walk_0", "player/walk_1", "player/walk_2"],
        "frame_duration_ms": 150.0,
        "loop_mode": "loop"
    }"#;

    let clip_def: AnimationClipDef =
        serde_json::from_str(json).expect("legacy frame_tiles should still deserialize");

    assert_eq!(clip_def.frame_tiles.len(), 3);
    assert!(clip_def.frame_positions.is_none());
    assert!(clip_def.frame_durations_ms.is_none());
}

#[test]
fn test_entity_definition_with_position_based_animations() {
    // Test full entity definition with position-based animation clips
    let entity_def = EntityDefinition {
        name: "position_animated".to_string(),
        display_name: "Position Animated Entity".to_string(),
        description: "Entity using position-based animation frames".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 1,
            visible: true,
            has_shadow: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: Some(50),
            stats: std::collections::HashMap::new(),
            speed: 1.5,
            solid: true,
            active: true,
            can_move: true,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::LegacyDefault,
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
            interactable: false,
            interaction_reach: 0,
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
            movement_sound: "step".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "sprites.json".to_string(),
            clips: vec![
                AnimationClipDef {
                    state: "idle".to_string(),
                    frame_tiles: vec![],
                    frame_positions: Some(vec![[0, 0]]),
                    frame_duration_ms: 300.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
                AnimationClipDef {
                    state: "walk".to_string(),
                    frame_tiles: vec![],
                    frame_positions: Some(vec![[0, 0], [1, 0], [2, 0], [3, 0]]),
                    frame_duration_ms: 120.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
            ],
            default_state: "idle".to_string(),
        },
        category: "creature".to_string(),
        tags: vec![],
    };

    let entity = entity_def
        .create_entity(IVec2::new(0, 0), 1)
        .expect("entity with position-based animations should create");

    let controller = entity
        .attributes
        .animation_controller
        .expect("should have animation controller");

    assert!(controller.clips.contains_key(&AnimationState::Idle));
    assert!(controller.clips.contains_key(&AnimationState::Walk));

    // Verify frame positions are stored in the runtime clip
    let idle_clip = controller.clips.get(&AnimationState::Idle).unwrap();
    assert!(idle_clip.frame_positions.is_some());
    assert_eq!(idle_clip.frame_positions.as_ref().unwrap().len(), 1);

    let walk_clip = controller.clips.get(&AnimationState::Walk).unwrap();
    assert!(walk_clip.frame_positions.is_some());
    assert_eq!(walk_clip.frame_positions.as_ref().unwrap().len(), 4);
}

#[test]
fn test_animation_controller_per_frame_duration_timing() {
    // Test that per-frame durations work correctly in the animation controller
    use toki_core::animation::{AnimationClip, AnimationController, AnimationState, LoopMode};

    let mut controller = AnimationController::new();

    let clip = AnimationClip {
        state: AnimationState::Attack,
        atlas_name: "test".to_string(),
        frame_tile_names: vec![
            "frame_0".to_string(),
            "frame_1".to_string(),
            "frame_2".to_string(),
        ],
        frame_positions: None,
        frame_duration_ms: 100.0, // Default fallback
        frame_durations_ms: Some(vec![50.0, 200.0, 50.0]), // Per-frame overrides
        loop_mode: LoopMode::Once,
    };

    controller.add_clip(clip);
    controller.play(AnimationState::Attack);

    // Frame 0: 50ms duration
    assert_eq!(controller.current_frame_index, 0);
    controller.update(40.0); // Not enough to advance
    assert_eq!(controller.current_frame_index, 0);
    controller.update(15.0); // Now at 55ms, should advance to frame 1
    assert_eq!(controller.current_frame_index, 1);

    // Frame 1: 200ms duration
    controller.update(100.0); // At 100ms into frame 1
    assert_eq!(controller.current_frame_index, 1);
    controller.update(100.0); // At 200ms, should advance to frame 2
    assert_eq!(controller.current_frame_index, 2);

    // Frame 2: 50ms duration
    controller.update(60.0); // Should finish (LoopMode::Once)
    assert!(controller.is_finished);
}

#[test]
fn test_animation_clip_serialization_roundtrip_with_positions() {
    // Test that AnimationClipDef round-trips through JSON with new fields
    let original = AnimationClipDef {
        state: "walk_right".to_string(),
        frame_tiles: vec![],
        frame_positions: Some(vec![[4, 0], [5, 0], [6, 0]]),
        frame_duration_ms: 150.0,
        frame_durations_ms: Some(vec![100.0, 150.0, 100.0]),
        loop_mode: "loop".to_string(),
    };

    let json = serde_json::to_string(&original).expect("should serialize");
    let deserialized: AnimationClipDef = serde_json::from_str(&json).expect("should deserialize");

    assert_eq!(deserialized.frame_positions, original.frame_positions);
    assert_eq!(deserialized.frame_durations_ms, original.frame_durations_ms);
    assert_eq!(deserialized.frame_duration_ms, original.frame_duration_ms);
}
