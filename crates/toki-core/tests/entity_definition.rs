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
        },
        attributes: AttributesDef {
            health: Some(100),
            speed: 2,
            solid: true,
            active: true,
            can_move: true,
            ai_behavior: AiBehavior::None,
            movement_profile: MovementProfile::LegacyDefault,
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
                    frame_duration_ms: 300.0,
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
                    frame_duration_ms: 150.0,
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
    assert_eq!(entity.attributes.speed, 2);
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
        },
        attributes: AttributesDef {
            health: Some(50),
            speed: 1,
            solid: true,
            active: true,
            can_move: false,
            ai_behavior: AiBehavior::Wander,
            movement_profile: MovementProfile::LegacyDefault,
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: false,
            offset: [0, 0],
            size: [32, 32],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "npc_footsteps".to_string(),
            collision_sound: Some("npc_collision".to_string()),
        },
        animations: AnimationsDef {
            atlas_name: "npc_atlas".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["npc/idle_0".to_string()],
                frame_duration_ms: 500.0,
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
    assert_eq!(entity.attributes.ai_behavior, AiBehavior::Wander);
    assert_eq!(
        entity.attributes.movement_profile,
        MovementProfile::LegacyDefault
    );
    assert_eq!(entity.attributes.speed, 1);

    // Check no collision since disabled
    assert!(entity.collision_box.is_none());
}

#[test]
fn test_entity_definition_missing_ai_behavior_defaults_to_wander() {
    let entity_json = r#"
    {
      "name": "legacy_npc",
      "display_name": "Legacy NPC",
      "description": "Old NPC without ai_behavior",
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
        serde_json::from_str(entity_json).expect("legacy entity json should deserialize");
    assert_eq!(entity_def.attributes.ai_behavior, AiBehavior::Wander);

    let entity = entity_def
        .create_entity(IVec2::ZERO, 1)
        .expect("legacy entity should still instantiate");
    assert_eq!(entity.attributes.ai_behavior, AiBehavior::Wander);
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
        },
        attributes: AttributesDef {
            health: Some(25),
            speed: 2,
            solid: true,
            active: true,
            can_move: true,
            ai_behavior: AiBehavior::None,
            movement_profile: MovementProfile::PlayerWasd,
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 16.0,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "step".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "creatures".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["creature/idle_0".to_string()],
                frame_duration_ms: 200.0,
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
        },
        attributes: AttributesDef {
            health: Some(100),
            speed: 2,
            solid: true,
            active: true,
            can_move: true,
            ai_behavior: AiBehavior::None,
            movement_profile: MovementProfile::LegacyDefault,
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
                    frame_duration_ms: 300.0,
                    loop_mode: "loop".to_string(),
                },
                AnimationClipDef {
                    state: "walk_down".to_string(),
                    frame_tiles: vec![
                        "player/walk_down_a".to_string(),
                        "player/walk_down_b".to_string(),
                    ],
                    frame_duration_ms: 180.0,
                    loop_mode: "loop".to_string(),
                },
                AnimationClipDef {
                    state: "walk_right".to_string(),
                    frame_tiles: vec![
                        "player/walk_right_a".to_string(),
                        "player/walk_right_b".to_string(),
                    ],
                    frame_duration_ms: 180.0,
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
fn test_entity_definition_unknown_category_defaults_to_actor_like_runtime_type() {
    let entity_def = EntityDefinition {
        name: "invalid".to_string(),
        display_name: "Invalid".to_string(),
        description: "Invalid entity type".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
        },
        attributes: AttributesDef {
            health: None,
            speed: 1,
            solid: false,
            active: true,
            can_move: false,
            ai_behavior: AiBehavior::None,
            movement_profile: MovementProfile::LegacyDefault,
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: false,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
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
        },
        attributes: AttributesDef {
            health: None,
            speed: 1,
            solid: false,
            active: true,
            can_move: false,
            ai_behavior: AiBehavior::None,
            movement_profile: MovementProfile::LegacyDefault,
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: false,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "test".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "test".to_string(),
            clips: vec![AnimationClipDef {
                state: "invalid_state".to_string(), // Invalid state
                frame_tiles: vec!["test/frame_0".to_string()],
                frame_duration_ms: 100.0,
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
        },
        attributes: AttributesDef {
            health: None,
            speed: 1,
            solid: false,
            active: true,
            can_move: false,
            ai_behavior: AiBehavior::None,
            movement_profile: MovementProfile::LegacyDefault,
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: false,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "test".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "test".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["test/frame_0".to_string()],
                frame_duration_ms: 100.0,
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
        },
        attributes: AttributesDef {
            health: None,
            speed: 0,
            solid: false,
            active: false,
            can_move: false,
            ai_behavior: AiBehavior::None,
            movement_profile: MovementProfile::LegacyDefault,
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [2, 2],
            size: [4, 4],
            trigger: true,
        },
        audio: AudioDef {
            footstep_trigger_distance: 0.0,
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
fn test_entity_definition_create_audio_component() {
    let entity_def = EntityDefinition {
        name: "audio_test".to_string(),
        display_name: "Audio Test".to_string(),
        description: "Audio component extraction".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
        },
        attributes: AttributesDef {
            health: Some(100),
            speed: 2,
            solid: true,
            active: true,
            can_move: true,
            ai_behavior: AiBehavior::None,
            movement_profile: MovementProfile::LegacyDefault,
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 24.0,
            movement_sound_trigger: MovementSoundTrigger::AnimationLoop,
            movement_sound: "sfx_custom_step".to_string(),
            collision_sound: Some("sfx_custom_hit".to_string()),
        },
        animations: AnimationsDef {
            atlas_name: "test".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["test_0".to_string()],
                frame_duration_ms: 100.0,
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
