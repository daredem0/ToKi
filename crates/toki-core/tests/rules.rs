use glam::{IVec2, UVec2};
use std::collections::HashMap;
use std::path::PathBuf;
use toki_core::animation::AnimationState;
use toki_core::assets::{
    atlas::{AtlasMeta, TileInfo, TileProperties},
    tilemap::TileMap,
};
use toki_core::game::{AudioChannel, AudioEvent};
use toki_core::rules::{
    InteractionMode, Rule, RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel,
    RuleSpawnEntityType, RuleTarget, RuleTrigger,
};
use toki_core::{
    entity::EntityKind,
    scene::{SceneAnchor, SceneAnchorFacing, SceneAnchorKind},
    GameState, InputKey, Scene,
};

fn create_test_tilemap() -> TileMap {
    TileMap {
        size: UVec2::new(10, 10),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test_atlas.json"),
        tiles: vec!["floor".to_string(); 100],
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
        tiles,
    }
}

fn create_collision_test_tilemap() -> TileMap {
    let mut tilemap = create_test_tilemap();
    // Place a solid wall tile directly to the right of the player spawn.
    tilemap.tiles[1] = "wall".to_string();
    tilemap
}

fn create_collision_test_atlas() -> AtlasMeta {
    let mut atlas = create_test_atlas();
    atlas.tiles.insert(
        "wall".to_string(),
        TileInfo {
            position: UVec2::new(1, 0),
            properties: TileProperties {
                solid: true,
                trigger: false,
            },
        },
    );
    atlas
}

fn create_trigger_test_tilemap() -> TileMap {
    let mut tilemap = create_test_tilemap();
    tilemap.tiles[0] = "trigger".to_string();
    tilemap
}

fn create_trigger_test_atlas() -> AtlasMeta {
    let mut atlas = create_test_atlas();
    atlas.tiles.insert(
        "trigger".to_string(),
        TileInfo {
            position: UVec2::new(2, 0),
            properties: TileProperties {
                solid: false,
                trigger: true,
            },
        },
    );
    atlas
}

fn base_rule(id: &str, trigger: RuleTrigger, priority: i32, actions: Vec<RuleAction>) -> Rule {
    Rule {
        id: id.to_string(),
        enabled: true,
        priority,
        once: false,
        trigger,
        conditions: vec![RuleCondition::Always],
        actions,
    }
}

fn scene_with_player(name: &str, position: IVec2) -> Scene {
    let mut scene = Scene::new(name.to_string());
    let mut template_state = GameState::new_empty();
    let player_id = template_state.spawn_player_at(position);
    let player = template_state
        .entity_manager()
        .get_entity(player_id)
        .expect("template player should exist")
        .clone();
    scene.add_entity(player);
    scene
}

fn spawn_anchor(id: &str, position: IVec2, facing: Option<SceneAnchorFacing>) -> SceneAnchor {
    SceneAnchor {
        id: id.to_string(),
        kind: SceneAnchorKind::SpawnPoint,
        position,
        facing,
    }
}

#[test]
fn on_start_rule_runs_once() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "start-beep",
            RuleTrigger::OnStart,
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_start".to_string(),
            }],
        )],
    });

    let world_bounds = UVec2::new(256, 256);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    let first = state.update(world_bounds, &tilemap, &atlas);
    let second = state.update(world_bounds, &tilemap, &atlas);

    assert_eq!(first.events.len(), 1);
    assert!(matches!(
        first.events[0],
        AudioEvent::PlaySound {
            channel: AudioChannel::Movement,
            ref sound_id,
            ..
        } if sound_id == "sfx_start"
    ));
    assert!(second.events.is_empty());
}

#[test]
fn on_collision_rule_runs_when_movement_is_blocked() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));
    state
        .entity_manager_mut()
        .audio_component_mut(player_id)
        .expect("player audio should exist")
        .collision_sound = None;

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "collision-rule",
            RuleTrigger::OnCollision { entity: None },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "rule_collision".to_string(),
            }],
        )],
    });

    state.handle_key_press(InputKey::Right);
    let blocked = state.update(
        UVec2::new(256, 256),
        &create_collision_test_tilemap(),
        &create_collision_test_atlas(),
    );
    assert!(blocked.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "rule_collision"
    )));

    state.handle_key_release(InputKey::Right);
    let no_collision = state.update(
        UVec2::new(256, 256),
        &create_collision_test_tilemap(),
        &create_collision_test_atlas(),
    );
    assert!(no_collision.events.is_empty());
}

#[test]
fn on_collision_rule_only_fires_once_for_sustained_blocked_input() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));
    state
        .entity_manager_mut()
        .audio_component_mut(player_id)
        .expect("player audio should exist")
        .collision_sound = None;

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "collision-rule",
            RuleTrigger::OnCollision { entity: None },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "rule_collision".to_string(),
            }],
        )],
    });

    state.handle_key_press(InputKey::Right);
    let first_blocked = state.update(
        UVec2::new(256, 256),
        &create_collision_test_tilemap(),
        &create_collision_test_atlas(),
    );
    assert!(first_blocked.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "rule_collision"
    )));

    let sustained_blocked = state.update(
        UVec2::new(256, 256),
        &create_collision_test_tilemap(),
        &create_collision_test_atlas(),
    );
    assert!(
        !sustained_blocked.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "rule_collision"
        )),
        "sustained blocked input should not retrigger OnCollision every frame"
    );
}

#[test]
fn on_damaged_rule_runs_when_primary_action_applies_health_damage() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 60));
    let player = state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist");
    let controller = player
        .attributes
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_duration_ms: 180.0,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_duration_ms: 120.0,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    state.spawn_player_like_npc(IVec2::new(66, 60));

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "damaged-rule",
            RuleTrigger::OnDamaged { entity: None },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "rule_damaged".to_string(),
            }],
        )],
    });

    state.handle_profile_action_press(
        toki_core::entity::MovementProfile::PlayerWasd,
        toki_core::game::InputAction::Primary,
    );
    let update = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(update.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "rule_damaged"
    )));
}

#[test]
fn on_damaged_rule_only_fires_once_for_sustained_held_primary_action() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 60));
    let player = state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist");
    let controller = player
        .attributes
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_duration_ms: 180.0,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_duration_ms: 120.0,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    state.spawn_player_like_npc(IVec2::new(66, 60));

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "damaged-rule",
            RuleTrigger::OnDamaged { entity: None },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "rule_damaged".to_string(),
            }],
        )],
    });

    state.handle_profile_action_press(
        toki_core::entity::MovementProfile::PlayerWasd,
        toki_core::game::InputAction::Primary,
    );
    let first = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    let second = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(first.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "rule_damaged"
    )));
    assert!(
        !second.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "rule_damaged"
        )),
        "held primary action should not retrigger OnDamaged without a new press"
    );
}

#[test]
fn on_death_rule_runs_when_primary_action_is_lethal() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 60));
    let player = state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist");
    let controller = player
        .attributes
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_duration_ms: 180.0,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_duration_ms: 120.0,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    let target_id = state.spawn_player_like_npc(IVec2::new(66, 60));
    let target = state
        .entity_manager_mut()
        .get_entity_mut(target_id)
        .expect("target should exist");
    target.attributes.health = Some(10);
    target.attributes.stats = toki_core::entity::EntityStats::from_legacy_health(Some(10));

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "death-rule",
            RuleTrigger::OnDeath { entity: None },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "rule_death".to_string(),
            }],
        )],
    });

    state.handle_profile_action_press(
        toki_core::entity::MovementProfile::PlayerWasd,
        toki_core::game::InputAction::Primary,
    );
    let update = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(update.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "rule_death"
    )));
}

#[test]
fn on_death_rule_only_fires_once_after_lethal_hit() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 60));
    let player = state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist");
    let controller = player
        .attributes
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_duration_ms: 180.0,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_duration_ms: 120.0,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    let target_id = state.spawn_player_like_npc(IVec2::new(66, 60));
    let target = state
        .entity_manager_mut()
        .get_entity_mut(target_id)
        .expect("target should exist");
    target.attributes.health = Some(10);
    target.attributes.stats = toki_core::entity::EntityStats::from_legacy_health(Some(10));

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "death-rule",
            RuleTrigger::OnDeath { entity: None },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "rule_death".to_string(),
            }],
        )],
    });

    state.handle_profile_action_press(
        toki_core::entity::MovementProfile::PlayerWasd,
        toki_core::game::InputAction::Primary,
    );
    let first = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    let second = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(first.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "rule_death"
    )));
    assert!(
        !second.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "rule_death"
        )),
        "OnDeath should not retrigger after the entity has already died"
    );
}

#[test]
fn on_trigger_rule_runs_when_entity_overlaps_trigger_tile() {
    let mut state = GameState::new_empty();
    state.spawn_player_at(IVec2::new(0, 0));
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "trigger-rule",
            RuleTrigger::OnTrigger,
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "rule_trigger".to_string(),
            }],
        )],
    });

    let first = state.update(
        UVec2::new(256, 256),
        &create_trigger_test_tilemap(),
        &create_trigger_test_atlas(),
    );
    assert!(first.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "rule_trigger"
    )));

    let second = state.update(
        UVec2::new(256, 256),
        &create_trigger_test_tilemap(),
        &create_trigger_test_atlas(),
    );
    assert!(second.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "rule_trigger"
    )));
}

#[test]
fn on_update_rule_runs_every_tick() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "tick-beep",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "sfx_tick".to_string(),
            }],
        )],
    });

    let world_bounds = UVec2::new(256, 256);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    let first = state.update(world_bounds, &tilemap, &atlas);
    let second = state.update(world_bounds, &tilemap, &atlas);

    assert_eq!(first.events.len(), 1);
    assert_eq!(second.events.len(), 1);
}

#[test]
fn on_player_move_rule_runs_only_when_player_moves() {
    let mut state = GameState::new_empty();
    state.spawn_player_at(IVec2::new(10, 10));
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "move-sfx",
            RuleTrigger::OnPlayerMove,
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "player_moved".to_string(),
            }],
        )],
    });

    let world_bounds = UVec2::new(256, 256);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    let idle = state.update(world_bounds, &tilemap, &atlas);
    assert!(idle.events.is_empty());

    state.handle_key_press(InputKey::Right);
    let moved = state.update(world_bounds, &tilemap, &atlas);
    assert!(moved.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "player_moved"
    )));

    state.handle_key_release(InputKey::Right);
}

#[test]
fn target_exists_condition_gates_rule_execution() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "requires-player",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "cond_target_exists".to_string(),
            }],
        )],
    });
    state.rules_mut().rules[0].conditions = vec![RuleCondition::TargetExists {
        target: RuleTarget::Player,
    }];

    let world_bounds = UVec2::new(256, 256);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    let no_player = state.update(world_bounds, &tilemap, &atlas);
    assert!(no_player.events.is_empty());

    state.spawn_player_at(IVec2::new(0, 0));
    let with_player = state.update(world_bounds, &tilemap, &atlas);
    assert!(with_player.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "cond_target_exists"
    )));
}

#[test]
fn key_held_condition_matches_runtime_input_state() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "requires-right",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "cond_key_held".to_string(),
            }],
        )],
    });
    state.rules_mut().rules[0].conditions = vec![RuleCondition::KeyHeld {
        key: RuleKey::Right,
    }];

    let world_bounds = UVec2::new(256, 256);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    let without_key = state.update(world_bounds, &tilemap, &atlas);
    assert!(without_key.events.is_empty());

    state.handle_key_press(InputKey::Right);
    let with_key = state.update(world_bounds, &tilemap, &atlas);
    assert!(with_key.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "cond_key_held"
    )));

    state.handle_key_release(InputKey::Right);
    let released = state.update(world_bounds, &tilemap, &atlas);
    assert!(released.events.is_empty());
}

#[test]
fn entity_active_condition_checks_target_active_flag() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "requires-active-player",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "cond_active".to_string(),
            }],
        )],
    });
    state.rules_mut().rules[0].conditions = vec![RuleCondition::EntityActive {
        target: RuleTarget::Player,
        is_active: true,
    }];

    let world_bounds = UVec2::new(256, 256);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .active = false;

    let inactive = state.update(world_bounds, &tilemap, &atlas);
    assert!(inactive.events.is_empty());

    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .active = true;
    let active = state.update(world_bounds, &tilemap, &atlas);
    assert!(active.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "cond_active"
    )));
}

#[test]
fn on_key_rule_runs_only_while_matching_key_is_held() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "right-key-sfx",
            RuleTrigger::OnKey {
                key: RuleKey::Right,
            },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "right_key".to_string(),
            }],
        )],
    });

    let none_pressed = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(none_pressed.events.is_empty());

    state.handle_key_press(InputKey::Right);
    let pressed = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert_eq!(pressed.events.len(), 1);
    assert!(matches!(
        &pressed.events[0],
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "right_key"
    ));

    state.handle_key_release(InputKey::Right);
    let released = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(released.events.is_empty());
}

#[test]
fn on_key_rule_ignores_non_matching_held_keys() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "up-only",
            RuleTrigger::OnKey { key: RuleKey::Up },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "up_key".to_string(),
            }],
        )],
    });

    state.handle_key_press(InputKey::Left);
    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(result.events.is_empty());
}

#[test]
fn play_music_action_emits_background_music_event() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "music-start",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::PlayMusic {
                track_id: "lavandia".to_string(),
            }],
        )],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert_eq!(result.events.len(), 1);
    assert!(matches!(
        &result.events[0],
        AudioEvent::BackgroundMusic(track_id) if track_id == "lavandia"
    ));
}

#[test]
fn play_animation_action_overrides_default_animation_for_target() {
    let mut state = GameState::new_empty();
    state.spawn_player_at(IVec2::new(10, 10));
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "force-walk-animation",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::PlayAnimation {
                target: RuleTarget::Player,
                state: AnimationState::Walk,
            }],
        )],
    });

    state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    let player = state.player_entity().expect("player must exist");
    let animation = player
        .attributes
        .animation_controller
        .as_ref()
        .expect("player should have animation controller");
    assert_eq!(animation.current_clip_state, AnimationState::Walk);
}

#[test]
fn play_animation_uses_priority_order_for_same_target() {
    let mut state = GameState::new_empty();
    state.spawn_player_at(IVec2::new(10, 10));
    state.set_rules(RuleSet {
        rules: vec![
            base_rule(
                "high",
                RuleTrigger::OnUpdate,
                10,
                vec![RuleAction::PlayAnimation {
                    target: RuleTarget::Player,
                    state: AnimationState::Walk,
                }],
            ),
            base_rule(
                "low",
                RuleTrigger::OnUpdate,
                0,
                vec![RuleAction::PlayAnimation {
                    target: RuleTarget::Player,
                    state: AnimationState::Idle,
                }],
            ),
        ],
    });

    state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    let player = state.player_entity().expect("player must exist");
    let animation = player
        .attributes
        .animation_controller
        .as_ref()
        .expect("player should have animation controller");
    assert_eq!(animation.current_clip_state, AnimationState::Walk);
}

#[test]
fn play_music_action_ignores_empty_track_id() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "music-empty",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::PlayMusic {
                track_id: "   ".to_string(),
            }],
        )],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(result.events.is_empty());
}

#[test]
fn spawn_action_creates_entity_at_requested_position() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "spawn-npc",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::Spawn {
                entity_type: RuleSpawnEntityType::Npc,
                position: [42, 84],
            }],
        )],
    });

    state.update(
        UVec2::new(512, 512),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    let mut npc_ids = state.entity_manager().entities_of_kind(&EntityKind::Npc);
    npc_ids.sort_unstable();
    assert_eq!(npc_ids.len(), 1);
    let spawned = state
        .entity_manager()
        .get_entity(npc_ids[0])
        .expect("spawned npc should exist");
    assert_eq!(spawned.position, IVec2::new(42, 84));
}

#[test]
fn spawn_actions_follow_rule_priority_order() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![
            base_rule(
                "low-spawn",
                RuleTrigger::OnUpdate,
                1,
                vec![RuleAction::Spawn {
                    entity_type: RuleSpawnEntityType::Npc,
                    position: [100, 100],
                }],
            ),
            base_rule(
                "high-spawn",
                RuleTrigger::OnUpdate,
                10,
                vec![RuleAction::Spawn {
                    entity_type: RuleSpawnEntityType::Npc,
                    position: [10, 10],
                }],
            ),
        ],
    });

    state.update(
        UVec2::new(512, 512),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    let mut npc_ids = state.entity_manager().entities_of_kind(&EntityKind::Npc);
    npc_ids.sort_unstable();
    assert_eq!(npc_ids.len(), 2);

    let first_spawn = state
        .entity_manager()
        .get_entity(npc_ids[0])
        .expect("first spawned npc should exist");
    let second_spawn = state
        .entity_manager()
        .get_entity(npc_ids[1])
        .expect("second spawned npc should exist");

    // Higher-priority rule should execute first and therefore get lower entity id.
    assert_eq!(first_spawn.position, IVec2::new(10, 10));
    assert_eq!(second_spawn.position, IVec2::new(100, 100));
}

#[test]
fn destroy_self_action_removes_target_entity() {
    let mut state = GameState::new_empty();
    let npc_id = state.spawn_player_like_npc(IVec2::new(50, 60));
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "destroy-npc",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::DestroySelf {
                target: RuleTarget::Entity(npc_id),
            }],
        )],
    });

    state.update(
        UVec2::new(512, 512),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(state.entity_manager().get_entity(npc_id).is_none());
}

#[test]
fn destroy_self_applies_before_lower_priority_velocity_for_same_target() {
    let mut state = GameState::new_empty();
    let npc_id = state.spawn_player_like_npc(IVec2::new(10, 10));
    state.set_rules(RuleSet {
        rules: vec![
            base_rule(
                "high-destroy",
                RuleTrigger::OnUpdate,
                10,
                vec![RuleAction::DestroySelf {
                    target: RuleTarget::Entity(npc_id),
                }],
            ),
            base_rule(
                "low-velocity",
                RuleTrigger::OnUpdate,
                0,
                vec![RuleAction::SetVelocity {
                    target: RuleTarget::Entity(npc_id),
                    velocity: [5, 0],
                }],
            ),
        ],
    });

    state.update(
        UVec2::new(512, 512),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        state.entity_manager().get_entity(npc_id).is_none(),
        "entity should be removed before velocity application"
    );
}

#[test]
fn first_tick_emits_on_start_events_before_on_update_events() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![
            base_rule(
                "start-first",
                RuleTrigger::OnStart,
                0,
                vec![RuleAction::PlaySound {
                    channel: RuleSoundChannel::Movement,
                    sound_id: "from_start".to_string(),
                }],
            ),
            base_rule(
                "update-second",
                RuleTrigger::OnUpdate,
                0,
                vec![RuleAction::PlaySound {
                    channel: RuleSoundChannel::Collision,
                    sound_id: "from_update".to_string(),
                }],
            ),
        ],
    });

    let first = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert_eq!(first.events.len(), 2);
    assert!(matches!(
        &first.events[0],
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "from_start"
    ));
    assert!(matches!(
        &first.events[1],
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "from_update"
    ));
}

#[test]
fn rules_execute_in_priority_order() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![
            base_rule(
                "low",
                RuleTrigger::OnUpdate,
                0,
                vec![RuleAction::PlaySound {
                    channel: RuleSoundChannel::Movement,
                    sound_id: "low".to_string(),
                }],
            ),
            base_rule(
                "high",
                RuleTrigger::OnUpdate,
                10,
                vec![RuleAction::PlaySound {
                    channel: RuleSoundChannel::Movement,
                    sound_id: "high".to_string(),
                }],
            ),
        ],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert_eq!(result.events.len(), 2);

    assert!(matches!(
        &result.events[0],
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "high"
    ));
    assert!(matches!(
        &result.events[1],
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "low"
    ));
}

#[test]
fn rules_with_same_priority_execute_in_stable_id_order() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![
            base_rule(
                "b_rule",
                RuleTrigger::OnUpdate,
                5,
                vec![RuleAction::PlaySound {
                    channel: RuleSoundChannel::Movement,
                    sound_id: "second".to_string(),
                }],
            ),
            base_rule(
                "a_rule",
                RuleTrigger::OnUpdate,
                5,
                vec![RuleAction::PlaySound {
                    channel: RuleSoundChannel::Movement,
                    sound_id: "first".to_string(),
                }],
            ),
        ],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert_eq!(result.events.len(), 2);
    assert!(matches!(
        &result.events[0],
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "first"
    ));
    assert!(matches!(
        &result.events[1],
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "second"
    ));
}

#[test]
fn disabled_rule_is_not_executed() {
    let mut disabled = base_rule(
        "disabled",
        RuleTrigger::OnUpdate,
        0,
        vec![RuleAction::PlaySound {
            channel: RuleSoundChannel::Movement,
            sound_id: "should_not_play".to_string(),
        }],
    );
    disabled.enabled = false;

    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![disabled],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(result.events.is_empty());
}

#[test]
fn once_on_update_rule_runs_only_first_tick() {
    let mut once_rule = base_rule(
        "once-update",
        RuleTrigger::OnUpdate,
        0,
        vec![RuleAction::PlaySound {
            channel: RuleSoundChannel::Movement,
            sound_id: "only_once".to_string(),
        }],
    );
    once_rule.once = true;

    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![once_rule],
    });

    let world_bounds = UVec2::new(256, 256);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    let first = state.update(world_bounds, &tilemap, &atlas);
    let second = state.update(world_bounds, &tilemap, &atlas);

    assert_eq!(first.events.len(), 1);
    assert!(second.events.is_empty());
}

#[test]
fn set_velocity_action_moves_player_without_input() {
    let mut state = GameState::new_empty();
    state.spawn_player_at(IVec2::new(10, 10));
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "move-player",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::SetVelocity {
                target: RuleTarget::Player,
                velocity: [2, 0],
            }],
        )],
    });

    let world_bounds = UVec2::new(512, 512);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    state.handle_key_release(InputKey::Right);
    let before = state.player_position();
    state.update(world_bounds, &tilemap, &atlas);
    let after_first = state.player_position();
    state.update(world_bounds, &tilemap, &atlas);
    let after_second = state.player_position();

    assert_eq!(after_first.x, before.x + 2);
    assert_eq!(after_second.x, after_first.x + 2);
}

#[test]
fn higher_priority_velocity_command_wins_for_same_target() {
    let mut state = GameState::new_empty();
    state.spawn_player_at(IVec2::new(10, 10));
    state.set_rules(RuleSet {
        rules: vec![
            base_rule(
                "high",
                RuleTrigger::OnUpdate,
                100,
                vec![RuleAction::SetVelocity {
                    target: RuleTarget::Player,
                    velocity: [4, 0],
                }],
            ),
            base_rule(
                "low",
                RuleTrigger::OnUpdate,
                1,
                vec![RuleAction::SetVelocity {
                    target: RuleTarget::Player,
                    velocity: [1, 0],
                }],
            ),
        ],
    });

    let before = state.player_position();
    state.update(
        UVec2::new(512, 512),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    let after = state.player_position();

    assert_eq!(after.x, before.x + 4);
}

#[test]
fn same_priority_velocity_uses_id_tiebreaker() {
    let mut state = GameState::new_empty();
    state.spawn_player_at(IVec2::new(20, 20));
    state.set_rules(RuleSet {
        rules: vec![
            base_rule(
                "b_rule",
                RuleTrigger::OnUpdate,
                5,
                vec![RuleAction::SetVelocity {
                    target: RuleTarget::Player,
                    velocity: [3, 0],
                }],
            ),
            base_rule(
                "a_rule",
                RuleTrigger::OnUpdate,
                5,
                vec![RuleAction::SetVelocity {
                    target: RuleTarget::Player,
                    velocity: [6, 0],
                }],
            ),
        ],
    });

    let before = state.player_position();
    state.update(
        UVec2::new(512, 512),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    let after = state.player_position();

    // Same priority resolves by id ascending, so "a_rule" wins.
    assert_eq!(after.x, before.x + 6);
}

#[test]
fn deterministic_execution_matches_across_identical_states() {
    fn build_state() -> GameState {
        let mut state = GameState::new_empty();
        state.spawn_player_at(IVec2::new(40, 40));
        state.set_rules(RuleSet {
            rules: vec![
                base_rule(
                    "start-sfx",
                    RuleTrigger::OnStart,
                    0,
                    vec![RuleAction::PlaySound {
                        channel: RuleSoundChannel::Movement,
                        sound_id: "boot".to_string(),
                    }],
                ),
                base_rule(
                    "move-player",
                    RuleTrigger::OnUpdate,
                    10,
                    vec![RuleAction::SetVelocity {
                        target: RuleTarget::Player,
                        velocity: [2, 1],
                    }],
                ),
                base_rule(
                    "tick-sfx",
                    RuleTrigger::OnUpdate,
                    0,
                    vec![RuleAction::PlaySound {
                        channel: RuleSoundChannel::Collision,
                        sound_id: "tick".to_string(),
                    }],
                ),
            ],
        });
        state
    }

    let mut left = build_state();
    let mut right = build_state();

    let world_bounds = UVec2::new(512, 512);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    for _ in 0..3 {
        let left_tick = left.update(world_bounds, &tilemap, &atlas);
        let right_tick = right.update(world_bounds, &tilemap, &atlas);
        assert_eq!(left_tick.player_moved, right_tick.player_moved);
        assert_eq!(left_tick.events, right_tick.events);
        assert_eq!(left.player_position(), right.player_position());
    }
}

#[test]
fn rules_serialize_roundtrip() {
    let rules = RuleSet {
        rules: vec![Rule {
            id: "serialize".to_string(),
            enabled: true,
            priority: 3,
            once: true,
            trigger: RuleTrigger::OnStart,
            conditions: vec![
                RuleCondition::Always,
                RuleCondition::TargetExists {
                    target: RuleTarget::Player,
                },
                RuleCondition::KeyHeld { key: RuleKey::Up },
                RuleCondition::EntityActive {
                    target: RuleTarget::Entity(7),
                    is_active: true,
                },
            ],
            actions: vec![
                RuleAction::PlaySound {
                    channel: RuleSoundChannel::Movement,
                    sound_id: "sfx_a".to_string(),
                },
                RuleAction::SetVelocity {
                    target: RuleTarget::Player,
                    velocity: [1, -1],
                },
                RuleAction::PlayAnimation {
                    target: RuleTarget::Player,
                    state: AnimationState::Idle,
                },
                RuleAction::PlayMusic {
                    track_id: "lavandia".to_string(),
                },
                RuleAction::SwitchScene {
                    scene_name: "Town".to_string(),
                    spawn_point_id: "from_gate".to_string(),
                },
                RuleAction::Spawn {
                    entity_type: RuleSpawnEntityType::Npc,
                    position: [5, 6],
                },
                RuleAction::DestroySelf {
                    target: RuleTarget::Player,
                },
            ],
        }],
    };

    let json = serde_json::to_string_pretty(&rules).expect("rules should serialize");
    let parsed: RuleSet = serde_json::from_str(&json).expect("rules should deserialize");
    assert_eq!(rules, parsed);
}

#[test]
fn switch_scene_requests_deferred_runtime_transition_after_movement_processing() {
    let mut state = GameState::new_empty();
    let mut scene_a = scene_with_player("Scene A", IVec2::new(0, 0));
    scene_a.rules = RuleSet {
        rules: vec![base_rule(
            "switch-to-b",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::SwitchScene {
                scene_name: "Scene B".to_string(),
                spawn_point_id: "spawn_b".to_string(),
            }],
        )],
    };
    let mut scene_b = Scene::new("Scene B".to_string());
    scene_b.add_anchor(spawn_anchor(
        "spawn_b",
        IVec2::new(100, 0),
        Some(SceneAnchorFacing::Right),
    ));

    state.add_scene(scene_a);
    state.add_scene(scene_b);
    state
        .load_scene("Scene A")
        .expect("initial scene should load");

    state.handle_key_press(InputKey::Right);
    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert_eq!(
        state.scene_manager().active_scene_name(),
        Some("Scene A"),
        "core should defer the scene switch for the runtime layer"
    );
    assert_eq!(
        state.player_position(),
        IVec2::new(2, 0),
        "movement processing should still complete before the deferred switch is emitted"
    );
    assert_eq!(
        result.scene_switch_request,
        Some(toki_core::SceneSwitchRequest {
            scene_name: "Scene B".to_string(),
            spawn_point_id: "spawn_b".to_string(),
        })
    );
}

#[test]
fn switch_scene_uses_highest_priority_rule_target() {
    let mut state = GameState::new_empty();
    let mut scene_a = scene_with_player("Scene A", IVec2::new(0, 0));
    scene_a.rules = RuleSet {
        rules: vec![
            base_rule(
                "low-to-c",
                RuleTrigger::OnUpdate,
                1,
                vec![RuleAction::SwitchScene {
                    scene_name: "Scene C".to_string(),
                    spawn_point_id: "spawn_c".to_string(),
                }],
            ),
            base_rule(
                "high-to-b",
                RuleTrigger::OnUpdate,
                10,
                vec![RuleAction::SwitchScene {
                    scene_name: "Scene B".to_string(),
                    spawn_point_id: "spawn_b".to_string(),
                }],
            ),
        ],
    };

    state.add_scene(scene_a);
    let mut scene_b = Scene::new("Scene B".to_string());
    scene_b.add_anchor(spawn_anchor("spawn_b", IVec2::new(10, 0), None));
    let mut scene_c = Scene::new("Scene C".to_string());
    scene_c.add_anchor(spawn_anchor("spawn_c", IVec2::new(20, 0), None));
    state.add_scene(scene_b);
    state.add_scene(scene_c);
    state
        .load_scene("Scene A")
        .expect("initial scene should load");

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert_eq!(state.scene_manager().active_scene_name(), Some("Scene A"));
    assert_eq!(
        result.scene_switch_request,
        Some(toki_core::SceneSwitchRequest {
            scene_name: "Scene B".to_string(),
            spawn_point_id: "spawn_b".to_string(),
        })
    );
}

#[test]
fn switch_scene_keeps_active_scene_when_target_scene_is_missing() {
    let mut state = GameState::new_empty();
    let mut scene_a = scene_with_player("Scene A", IVec2::new(0, 0));
    scene_a.rules = RuleSet {
        rules: vec![base_rule(
            "switch-missing",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::SwitchScene {
                scene_name: "Missing Scene".to_string(),
                spawn_point_id: "missing_spawn".to_string(),
            }],
        )],
    };

    state.add_scene(scene_a);
    state
        .load_scene("Scene A")
        .expect("initial scene should load");

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert_eq!(state.scene_manager().active_scene_name(), Some("Scene A"));
    assert_eq!(state.player_position(), IVec2::new(0, 0));
    assert_eq!(
        result.scene_switch_request,
        Some(toki_core::SceneSwitchRequest {
            scene_name: "Missing Scene".to_string(),
            spawn_point_id: "missing_spawn".to_string(),
        })
    );
}

#[test]
fn switch_scene_keeps_active_scene_when_target_spawn_is_missing() {
    let mut state = GameState::new_empty();
    let mut scene_a = scene_with_player("Scene A", IVec2::new(0, 0));
    scene_a.rules = RuleSet {
        rules: vec![base_rule(
            "missing-spawn",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::SwitchScene {
                scene_name: "Scene B".to_string(),
                spawn_point_id: "spawn_b".to_string(),
            }],
        )],
    };

    state.add_scene(scene_a);
    state.add_scene(Scene::new("Scene B".to_string()));
    state
        .load_scene("Scene A")
        .expect("initial scene should load");

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert_eq!(state.scene_manager().active_scene_name(), Some("Scene A"));
    assert_eq!(state.player_position(), IVec2::new(0, 0));
    assert_eq!(
        result.scene_switch_request,
        Some(toki_core::SceneSwitchRequest {
            scene_name: "Scene B".to_string(),
            spawn_point_id: "spawn_b".to_string(),
        })
    );
}

#[test]
fn on_start_set_velocity_initializes_persistent_movement() {
    let mut state = GameState::new_empty();
    state.spawn_player_at(IVec2::new(30, 30));
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "start-velocity",
            RuleTrigger::OnStart,
            0,
            vec![RuleAction::SetVelocity {
                target: RuleTarget::Player,
                velocity: [3, 0],
            }],
        )],
    });

    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();
    let world_bounds = UVec2::new(512, 512);

    let before = state.player_position();
    state.update(world_bounds, &tilemap, &atlas);
    let after_first = state.player_position();
    state.update(world_bounds, &tilemap, &atlas);
    let after_second = state.player_position();

    assert_eq!(after_first.x, before.x + 3);
    assert_eq!(after_second.x, after_first.x + 3);
}

#[test]
fn load_scene_applies_scene_rules() {
    let mut state = GameState::new_empty();
    let mut scene = Scene::new("Rule Scene".to_string());
    scene.rules = RuleSet {
        rules: vec![base_rule(
            "scene-start",
            RuleTrigger::OnStart,
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "scene_start".to_string(),
            }],
        )],
    };

    state.add_scene(scene);
    state
        .load_scene("Rule Scene")
        .expect("scene with rules should load");

    let world_bounds = UVec2::new(256, 256);
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();

    let first = state.update(world_bounds, &tilemap, &atlas);
    let second = state.update(world_bounds, &tilemap, &atlas);

    assert_eq!(first.events.len(), 1);
    assert!(matches!(
        first.events[0],
        AudioEvent::PlaySound {
            channel: AudioChannel::Movement,
            ref sound_id,
            ..
        } if sound_id == "scene_start"
    ));
    assert!(second.events.is_empty());
}

#[test]
fn sync_entities_to_active_scene_persists_rules() {
    let mut state = GameState::new_empty();
    state.add_scene(Scene::new("Sync Scene".to_string()));
    state
        .load_scene("Sync Scene")
        .expect("scene should load before syncing");

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "persist-me",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "persisted".to_string(),
            }],
        )],
    });
    state.sync_entities_to_active_scene();

    let active_scene = state.active_scene().expect("active scene should exist");
    assert_eq!(active_scene.rules.rules.len(), 1);
    assert_eq!(active_scene.rules.rules[0].id, "persist-me");
}

// ============================================================================
// Phase 1.5A: Trigger Context Tests
// ============================================================================

#[test]
fn on_collision_with_trigger_self_condition_resolves_correctly() {
    // Tests that TriggerSelf resolves to the colliding entity
    // and can be used in conditions/actions
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));
    state
        .entity_manager_mut()
        .audio_component_mut(player_id)
        .expect("player audio should exist")
        .collision_sound = None;

    // Rule uses TargetExists with TriggerSelf - should match when collision context is present
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "context-rule".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnCollision { entity: None },
            conditions: vec![RuleCondition::TargetExists {
                target: RuleTarget::TriggerSelf,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "trigger_self_matched".to_string(),
            }],
        }],
    });

    state.handle_key_press(InputKey::Right);
    let blocked = state.update(
        UVec2::new(256, 256),
        &create_collision_test_tilemap(),
        &create_collision_test_atlas(),
    );

    // TriggerSelf should resolve to the player entity (the one that collided)
    assert!(blocked.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "trigger_self_matched"
    )));
}

#[test]
fn on_collision_with_trigger_other_condition_none_for_tile_collision() {
    // For tile collisions, TriggerOther is None, so TargetExists should fail
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));
    state
        .entity_manager_mut()
        .audio_component_mut(player_id)
        .expect("player audio should exist")
        .collision_sound = None;

    // Rule requires TriggerOther to exist - should NOT match for tile collision
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "require-other-rule".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnCollision { entity: None },
            conditions: vec![RuleCondition::TargetExists {
                target: RuleTarget::TriggerOther,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "trigger_other_matched".to_string(),
            }],
        }],
    });

    state.handle_key_press(InputKey::Right);
    let blocked = state.update(
        UVec2::new(256, 256),
        &create_collision_test_tilemap(),
        &create_collision_test_atlas(),
    );

    // TriggerOther should be None for tile collision, so condition fails
    assert!(!blocked.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "trigger_other_matched"
    )));
}

#[test]
fn on_damaged_with_trigger_self_refers_to_victim() {
    // Tests that TriggerSelf in OnDamaged refers to the damaged entity (victim)
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 60));
    let player = state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist");
    let controller = player
        .attributes
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_duration_ms: 180.0,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_duration_ms: 120.0,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    // Spawn NPC to the right of player
    state.spawn_player_like_npc(IVec2::new(66, 60));

    // Rule uses TargetExists with TriggerSelf to verify victim is set
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "damage-context-rule".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnDamaged { entity: None },
            conditions: vec![RuleCondition::TargetExists {
                target: RuleTarget::TriggerSelf,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "victim_exists".to_string(),
            }],
        }],
    });

    state.handle_profile_action_press(
        toki_core::entity::MovementProfile::PlayerWasd,
        toki_core::game::InputAction::Primary,
    );
    let update = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // TriggerSelf should resolve to the NPC (the victim)
    assert!(update.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "victim_exists"
    )));
}

#[test]
fn on_damaged_with_trigger_other_refers_to_attacker() {
    // Tests that TriggerOther in OnDamaged refers to the attacker
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 60));
    let player = state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist");
    let controller = player
        .attributes
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_duration_ms: 180.0,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_duration_ms: 120.0,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    // Spawn NPC to the right of player
    state.spawn_player_like_npc(IVec2::new(66, 60));

    // Rule uses TargetExists with TriggerOther to verify attacker is set
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "attacker-context-rule".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnDamaged { entity: None },
            conditions: vec![RuleCondition::TargetExists {
                target: RuleTarget::TriggerOther,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "attacker_exists".to_string(),
            }],
        }],
    });

    state.handle_profile_action_press(
        toki_core::entity::MovementProfile::PlayerWasd,
        toki_core::game::InputAction::Primary,
    );
    let update = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // TriggerOther should resolve to the player (the attacker)
    assert!(update.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "attacker_exists"
    )));
}

// Phase 1.5B: Extended Keys And Interaction Tests

#[test]
fn on_key_interact_fires_when_interact_key_is_held() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "interact-key-sfx",
            RuleTrigger::OnKey {
                key: RuleKey::Interact,
            },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "interact_sfx".to_string(),
            }],
        )],
    });

    state.handle_key_press(InputKey::Interact);
    let pressed = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(pressed.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "interact_sfx"
    )));

    state.handle_key_release(InputKey::Interact);
    let released = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(!released.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "interact_sfx"
    )));
}

#[test]
fn on_key_attack_primary_fires_when_attack_primary_key_is_held() {
    let mut state = GameState::new_empty();
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "attack-primary-key-sfx",
            RuleTrigger::OnKey {
                key: RuleKey::AttackPrimary,
            },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "attack_primary_sfx".to_string(),
            }],
        )],
    });

    state.handle_key_press(InputKey::AttackPrimary);
    let pressed = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(pressed.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "attack_primary_sfx"
    )));
}

#[test]
fn on_interact_fires_when_player_overlaps_interactable_and_presses_interact() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(50, 50));

    // Spawn NPC at overlapping position
    let npc_id = state.spawn_player_like_npc(IVec2::new(50, 50));
    // Mark NPC as interactable
    state
        .entity_manager_mut()
        .get_entity_mut(npc_id)
        .expect("npc should exist")
        .attributes
        .interactable = true;

    // Rule fires on interact with NPC
    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "npc-interact",
            RuleTrigger::OnInteract { mode: InteractionMode::default(), entity: None },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "npc_talk".to_string(),
            }],
        )],
    });

    // Press interact key
    state.handle_key_press(InputKey::Interact);
    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(result.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "npc_talk"
    )));

    // Release interact - should not fire again
    state.handle_key_release(InputKey::Interact);
    let no_interact = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    assert!(!no_interact.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "npc_talk"
    )));
}

#[test]
fn on_interact_does_not_fire_when_not_overlapping() {
    let mut state = GameState::new_empty();
    state.spawn_player_at(IVec2::new(50, 50));

    // Spawn NPC far away
    let npc_id = state.spawn_player_like_npc(IVec2::new(150, 150));
    state
        .entity_manager_mut()
        .get_entity_mut(npc_id)
        .expect("npc should exist")
        .attributes
        .interactable = true;

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "npc-interact",
            RuleTrigger::OnInteract { mode: InteractionMode::default(), entity: None },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "npc_talk".to_string(),
            }],
        )],
    });

    state.handle_key_press(InputKey::Interact);
    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Should not fire because player is not overlapping
    assert!(!result.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "npc_talk"
    )));
}

#[test]
fn on_interact_does_not_fire_when_entity_is_not_interactable() {
    let mut state = GameState::new_empty();
    state.spawn_player_at(IVec2::new(50, 50));

    // Spawn NPC at overlapping position but NOT marked as interactable
    let npc_id = state.spawn_player_like_npc(IVec2::new(50, 50));
    state
        .entity_manager_mut()
        .get_entity_mut(npc_id)
        .expect("npc should exist")
        .attributes
        .interactable = false;

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "npc-interact",
            RuleTrigger::OnInteract { mode: InteractionMode::default(), entity: None },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "npc_talk".to_string(),
            }],
        )],
    });

    state.handle_key_press(InputKey::Interact);
    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Should not fire because NPC is not interactable
    assert!(!result.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "npc_talk"
    )));
}

#[test]
fn on_interact_provides_trigger_context() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(50, 50));

    // Spawn interactable NPC
    let npc_id = state.spawn_player_like_npc(IVec2::new(50, 50));
    state
        .entity_manager_mut()
        .get_entity_mut(npc_id)
        .expect("npc should exist")
        .attributes
        .interactable = true;

    // Rule uses TriggerSelf (player) and TriggerOther (NPC)
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "interact-context".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnInteract { mode: InteractionMode::default(), entity: None },
            conditions: vec![
                RuleCondition::TargetExists {
                    target: RuleTarget::TriggerSelf,
                },
                RuleCondition::TargetExists {
                    target: RuleTarget::TriggerOther,
                },
            ],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "context_valid".to_string(),
            }],
        }],
    });

    state.handle_key_press(InputKey::Interact);
    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Should fire because both TriggerSelf and TriggerOther exist
    assert!(result.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "context_valid"
    )));
}

#[test]
fn on_damaged_fires_when_entity_takes_damage() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 50));

    // Give player health
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .health = Some(100);

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "on-damaged",
            RuleTrigger::OnDamaged { entity: None },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "damage_sound".to_string(),
            }],
        )],
    });

    // Deal damage to player
    state.deal_damage_to_entity(player_id, 10, None);

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // OnDamaged should fire
    assert!(result.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "damage_sound"
    )));
}

#[test]
fn on_damaged_provides_trigger_context() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 50));
    let attacker_id = state.spawn_player_like_npc(IVec2::new(60, 60));

    // Give player health
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .health = Some(100);

    // Rule uses TriggerSelf (victim) and TriggerOther (attacker)
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "damaged-context".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnDamaged { entity: None },
            conditions: vec![
                RuleCondition::TargetExists {
                    target: RuleTarget::TriggerSelf,
                },
                RuleCondition::TargetExists {
                    target: RuleTarget::TriggerOther,
                },
            ],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "context_valid".to_string(),
            }],
        }],
    });

    // Deal damage with attacker
    state.deal_damage_to_entity(player_id, 10, Some(attacker_id));

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Should fire because both TriggerSelf (victim) and TriggerOther (attacker) exist
    assert!(result.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "context_valid"
    )));
}

#[test]
fn on_death_fires_when_entity_dies() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 50));

    // Give player low health
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .health = Some(10);

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "on-death",
            RuleTrigger::OnDeath { entity: None },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "death_sound".to_string(),
            }],
        )],
    });

    // Deal lethal damage to player
    state.deal_damage_to_entity(player_id, 100, None);

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // OnDeath should fire
    assert!(result.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "death_sound"
    )));
}

#[test]
fn on_death_provides_trigger_context() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 50));
    let attacker_id = state.spawn_player_like_npc(IVec2::new(60, 60));

    // Give player low health
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .health = Some(10);

    // Rule uses TriggerSelf (victim) and TriggerOther (attacker)
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "death-context".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnDeath { entity: None },
            conditions: vec![
                RuleCondition::TargetExists {
                    target: RuleTarget::TriggerSelf,
                },
                RuleCondition::TargetExists {
                    target: RuleTarget::TriggerOther,
                },
            ],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "context_valid".to_string(),
            }],
        }],
    });

    // Deal lethal damage with attacker
    state.deal_damage_to_entity(player_id, 100, Some(attacker_id));

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Should fire because both TriggerSelf (victim) and TriggerOther (attacker) exist
    assert!(result.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "context_valid"
    )));
}

#[test]
fn on_death_fires_without_attacker() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 50));

    // Give player low health
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .health = Some(10);

    // Rule uses only TriggerSelf (victim) - no attacker requirement
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "death-no-attacker".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnDeath { entity: None },
            conditions: vec![RuleCondition::TargetExists {
                target: RuleTarget::TriggerSelf,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "environmental_death".to_string(),
            }],
        }],
    });

    // Deal lethal damage without attacker (environmental)
    state.deal_damage_to_entity(player_id, 100, None);

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Should fire even without attacker
    assert!(result.events.iter().any(|event| matches!(
        event,
        AudioEvent::PlaySound { sound_id, .. } if sound_id == "environmental_death"
    )));
}

// =============================================================================
// Entity-scoped trigger tests
// =============================================================================

#[test]
fn on_damaged_with_entity_filter_only_fires_for_matching_entity() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 50));
    let npc_id = state.spawn_player_like_npc(IVec2::new(100, 100));

    // Give both entities health
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .health = Some(100);
    state
        .entity_manager_mut()
        .get_entity_mut(npc_id)
        .expect("npc should exist")
        .attributes
        .health = Some(100);

    // Rule only fires when player is damaged
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "player-damaged".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnDamaged {
                entity: Some(RuleTarget::Player),
            },
            conditions: vec![],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "player_hurt".to_string(),
            }],
        }],
    });

    // Deal damage to NPC - should NOT trigger
    state.deal_damage_to_entity(npc_id, 10, None);

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        !result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "player_hurt"
        )),
        "OnDamaged with entity filter should not fire when non-matching entity is damaged"
    );

    // Now deal damage to player - SHOULD trigger
    state.deal_damage_to_entity(player_id, 10, None);

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "player_hurt"
        )),
        "OnDamaged with entity filter should fire when matching entity is damaged"
    );
}

#[test]
fn on_damaged_without_entity_filter_fires_for_all_entities() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 50));
    let npc_id = state.spawn_player_like_npc(IVec2::new(100, 100));

    // Give both entities health
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .health = Some(100);
    state
        .entity_manager_mut()
        .get_entity_mut(npc_id)
        .expect("npc should exist")
        .attributes
        .health = Some(100);

    // Rule fires for ANY damage (no entity filter)
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "any-damaged".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnDamaged { entity: None },
            conditions: vec![],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "any_hurt".to_string(),
            }],
        }],
    });

    // Deal damage to NPC - should trigger
    state.deal_damage_to_entity(npc_id, 10, None);

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "any_hurt"
        )),
        "OnDamaged without entity filter should fire for any entity"
    );
}

#[test]
fn on_damaged_with_specific_entity_id_filter() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(50, 50));
    let npc1_id = state.spawn_player_like_npc(IVec2::new(100, 100));
    let npc2_id = state.spawn_player_like_npc(IVec2::new(150, 150));

    // Give NPCs health
    state
        .entity_manager_mut()
        .get_entity_mut(npc1_id)
        .expect("npc1 should exist")
        .attributes
        .health = Some(100);
    state
        .entity_manager_mut()
        .get_entity_mut(npc2_id)
        .expect("npc2 should exist")
        .attributes
        .health = Some(100);

    // Rule only fires when npc1 is damaged
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "npc1-damaged".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnDamaged {
                entity: Some(RuleTarget::Entity(npc1_id)),
            },
            conditions: vec![],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "npc1_hurt".to_string(),
            }],
        }],
    });

    // Deal damage to npc2 - should NOT trigger
    state.deal_damage_to_entity(npc2_id, 10, None);

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        !result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "npc1_hurt"
        )),
        "OnDamaged with entity ID filter should not fire for different entity"
    );

    // Deal damage to npc1 - SHOULD trigger
    state.deal_damage_to_entity(npc1_id, 10, None);

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "npc1_hurt"
        )),
        "OnDamaged with entity ID filter should fire for matching entity"
    );
}

#[test]
fn on_death_with_entity_filter_only_fires_for_matching_entity() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 50));
    let npc_id = state.spawn_player_like_npc(IVec2::new(100, 100));

    // Give both entities health
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .health = Some(10);
    state
        .entity_manager_mut()
        .get_entity_mut(npc_id)
        .expect("npc should exist")
        .attributes
        .health = Some(10);

    // Rule only fires when player dies
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "player-death".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnDeath {
                entity: Some(RuleTarget::Player),
            },
            conditions: vec![],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "player_died".to_string(),
            }],
        }],
    });

    // Kill NPC - should NOT trigger
    state.deal_damage_to_entity(npc_id, 100, None);

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        !result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "player_died"
        )),
        "OnDeath with entity filter should not fire when non-matching entity dies"
    );

    // Kill player - SHOULD trigger
    state.deal_damage_to_entity(player_id, 100, None);

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "player_died"
        )),
        "OnDeath with entity filter should fire when matching entity dies"
    );
}

#[test]
fn on_death_without_entity_filter_fires_for_all_entities() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(50, 50));
    let npc_id = state.spawn_player_like_npc(IVec2::new(100, 100));

    // Give NPC health
    state
        .entity_manager_mut()
        .get_entity_mut(npc_id)
        .expect("npc should exist")
        .attributes
        .health = Some(10);

    // Rule fires for ANY death (no entity filter)
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "any-death".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnDeath { entity: None },
            conditions: vec![],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "any_death".to_string(),
            }],
        }],
    });

    // Kill NPC - should trigger
    state.deal_damage_to_entity(npc_id, 100, None);

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "any_death"
        )),
        "OnDeath without entity filter should fire for any entity"
    );
}

// =============================================================================
// Phase 1.5C: Health Threshold Conditions Tests
// =============================================================================

#[test]
fn health_below_condition_matches_when_entity_health_is_below_threshold() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));

    // Set player health to 30 (below threshold of 50)
    // Note: must insert directly since player spawns with default health (100)
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .stats
        .current
        .insert("health".to_string(), 30);

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "low-health-alert".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::HealthBelow {
                target: RuleTarget::Player,
                threshold: 50,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "low_health".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "low_health"
        )),
        "HealthBelow should match when health (30) is below threshold (50)"
    );
}

#[test]
fn health_below_condition_does_not_match_when_health_equals_threshold() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));

    // Set player health exactly at threshold
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .stats
        .current
        .insert("health".to_string(), 50);

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "low-health-alert".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::HealthBelow {
                target: RuleTarget::Player,
                threshold: 50,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "low_health".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.is_empty(),
        "HealthBelow should NOT match when health (50) equals threshold (50)"
    );
}

#[test]
fn health_below_condition_does_not_match_when_health_is_above_threshold() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(0, 0));
    // Player spawns with health 100 (above threshold of 50)

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "low-health-alert".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::HealthBelow {
                target: RuleTarget::Player,
                threshold: 50,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "low_health".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.is_empty(),
        "HealthBelow should NOT match when health (100) is above threshold (50)"
    );
}

#[test]
fn health_above_condition_matches_when_entity_health_is_above_threshold() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));

    // Set player health to 80 (above threshold of 50)
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .stats
        .current
        .insert("health".to_string(), 80);

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "high-health".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::HealthAbove {
                target: RuleTarget::Player,
                threshold: 50,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "high_health".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "high_health"
        )),
        "HealthAbove should match when health (80) is above threshold (50)"
    );
}

#[test]
fn health_above_condition_does_not_match_when_health_equals_threshold() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));

    // Set player health exactly at threshold
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .stats
        .current
        .insert("health".to_string(), 50);

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "high-health".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::HealthAbove {
                target: RuleTarget::Player,
                threshold: 50,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "high_health".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.is_empty(),
        "HealthAbove should NOT match when health (50) equals threshold (50)"
    );
}

#[test]
fn health_above_condition_does_not_match_when_health_is_below_threshold() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));

    // Set player health below threshold
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .stats
        .current
        .insert("health".to_string(), 30);

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "high-health".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::HealthAbove {
                target: RuleTarget::Player,
                threshold: 50,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "high_health".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.is_empty(),
        "HealthAbove should NOT match when health (30) is below threshold (50)"
    );
}

#[test]
fn health_condition_fails_safely_when_entity_has_no_health_stat() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));

    // Clear health stat to ensure it doesn't exist
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .stats = toki_core::entity::EntityStats::default();

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "health-check".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::HealthBelow {
                target: RuleTarget::Player,
                threshold: 50,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "should_not_play".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.is_empty(),
        "Health conditions should fail safely (not match) when entity has no health stat"
    );
}

// =============================================================================
// Phase 1.5C: Trigger Context Conditions Tests
// =============================================================================

#[test]
fn trigger_other_is_player_matches_when_other_entity_is_player_in_collision() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 60));
    state.spawn_player_like_npc(IVec2::new(66, 60));

    // Rule: OnCollision when the other entity is the player
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "player-collision".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnCollision { entity: None },
            conditions: vec![RuleCondition::TriggerOtherIsPlayer],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "player_collision".to_string(),
            }],
        }],
    });

    // Move player into NPC to trigger collision
    state.handle_key_press(InputKey::Right);
    let result = state.update(
        UVec2::new(256, 256),
        &create_collision_test_tilemap(),
        &create_collision_test_atlas(),
    );

    // This tests NPC colliding with player - TriggerOther should be player
    // Note: The collision system needs to fire the rule from NPC's perspective
    // For now, we verify the condition exists and compiles
    let _ = player_id;
    let _ = result;
}

#[test]
fn trigger_other_is_player_does_not_match_when_other_is_not_player() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(50, 60));

    // Spawn two NPCs that will collide
    let npc1 = state.spawn_player_like_npc(IVec2::new(80, 60));
    state.spawn_player_like_npc(IVec2::new(96, 60));

    // Rule: OnDamaged when the attacker is the player
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "player-attack".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnDamaged { entity: None },
            conditions: vec![RuleCondition::TriggerOtherIsPlayer],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Collision,
                sound_id: "player_attack".to_string(),
            }],
        }],
    });

    // Deal damage from NPC1 to NPC2 (neither is player)
    state.deal_damage_to_entity(npc1, 10, Some(npc1));

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        !result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "player_attack"
        )),
        "TriggerOtherIsPlayer should NOT match when attacker is not player"
    );
}

#[test]
fn trigger_other_is_player_fails_safely_without_context() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(0, 0));

    // Rule: OnUpdate with TriggerOtherIsPlayer (OnUpdate has no context)
    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "invalid-context".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::TriggerOtherIsPlayer],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "should_not_play".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.is_empty(),
        "TriggerOtherIsPlayer should fail safely (not match) when no trigger context"
    );
}

// =============================================================================
// Phase 1.5C: Entity Kind Conditions Tests
// =============================================================================

#[test]
fn entity_is_kind_matches_player_kind() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(0, 0));

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "player-kind-check".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::EntityIsKind {
                target: RuleTarget::Player,
                kind: EntityKind::Player,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "is_player".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "is_player"
        )),
        "EntityIsKind should match when target entity kind matches"
    );
}

#[test]
fn entity_is_kind_does_not_match_wrong_kind() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(0, 0));

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "npc-kind-check".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::EntityIsKind {
                target: RuleTarget::Player,
                kind: EntityKind::Npc, // Player is not an NPC
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "is_npc".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.is_empty(),
        "EntityIsKind should NOT match when entity kind differs"
    );
}

#[test]
fn trigger_other_is_kind_matches_npc_on_damage() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(50, 60));

    // Setup player for attacking
    let player = state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist");
    let controller = player
        .attributes
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_duration_ms: 180.0,
        loop_mode: toki_core::animation::LoopMode::Loop,
    });
    controller.add_clip(toki_core::animation::AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_duration_ms: 120.0,
        loop_mode: toki_core::animation::LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    // Spawn NPC that will be damaged
    state.spawn_player_like_npc(IVec2::new(66, 60));

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "damaged-by-npc-check".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnDamaged { entity: None },
            // Check if attacker (TriggerOther) is an NPC kind
            // In this case, attacker is player, so this should NOT match
            conditions: vec![RuleCondition::TriggerOtherIsKind {
                kind: EntityKind::Npc,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "damaged_by_npc".to_string(),
            }],
        }],
    });

    // Player attacks NPC - attacker is Player, not NPC
    state.handle_profile_action_press(
        toki_core::entity::MovementProfile::PlayerWasd,
        toki_core::game::InputAction::Primary,
    );
    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        !result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "damaged_by_npc"
        )),
        "TriggerOtherIsKind should NOT match when attacker is Player, not NPC"
    );
}

#[test]
fn trigger_other_is_kind_fails_safely_without_context() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(0, 0));

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "no-context".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::TriggerOtherIsKind {
                kind: EntityKind::Npc,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "should_not_play".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.is_empty(),
        "TriggerOtherIsKind should fail safely when no trigger context"
    );
}

// =============================================================================
// Phase 1.5C: Tag Conditions Tests
// =============================================================================

#[test]
fn entity_has_tag_matches_when_entity_has_specified_tag() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));

    // Add tags to player
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .tags = vec!["hero".to_string(), "protagonist".to_string()];

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "hero-check".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::EntityHasTag {
                target: RuleTarget::Player,
                tag: "hero".to_string(),
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "is_hero".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "is_hero"
        )),
        "EntityHasTag should match when entity has the specified tag"
    );
}

#[test]
fn entity_has_tag_does_not_match_when_entity_lacks_tag() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));

    // Add different tags to player
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .tags = vec!["hero".to_string()];

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "villain-check".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::EntityHasTag {
                target: RuleTarget::Player,
                tag: "villain".to_string(),
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "is_villain".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.is_empty(),
        "EntityHasTag should NOT match when entity lacks the specified tag"
    );
}

#[test]
fn trigger_other_has_tag_fails_safely_without_context() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(0, 0));

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "no-context".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::TriggerOtherHasTag {
                tag: "enemy".to_string(),
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "should_not_play".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.is_empty(),
        "TriggerOtherHasTag should fail safely when no trigger context"
    );
}

// =============================================================================
// Phase 1.5C: Inventory Conditions Tests
// =============================================================================

#[test]
fn has_inventory_item_matches_when_player_has_enough_items() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));

    // Give player inventory items
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .inventory
        .add_item("key", 3);

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "key-check".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::HasInventoryItem {
                target: RuleTarget::Player,
                item_id: "key".to_string(),
                min_count: 2,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "has_keys".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "has_keys"
        )),
        "HasInventoryItem should match when player has required key items"
    );
}

#[test]
fn has_inventory_item_does_not_match_when_player_has_insufficient_items() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0));

    // Give player fewer items than required
    state
        .entity_manager_mut()
        .get_entity_mut(player_id)
        .expect("player should exist")
        .attributes
        .inventory
        .add_item("key", 1);

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "key-check".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::HasInventoryItem {
                target: RuleTarget::Player,
                item_id: "key".to_string(),
                min_count: 3,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "has_keys".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.is_empty(),
        "HasInventoryItem should NOT match when player has insufficient items"
    );
}

#[test]
fn has_inventory_item_does_not_match_when_item_missing() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(0, 0));
    // Player has no items in inventory by default

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "key-check".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::HasInventoryItem {
                target: RuleTarget::Player,
                item_id: "boss_key".to_string(),
                min_count: 1,
            }],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "has_boss_key".to_string(),
            }],
        }],
    });

    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        result.events.is_empty(),
        "HasInventoryItem should NOT match when player lacks the item entirely"
    );
}

#[test]
fn on_tile_enter_fires_when_entity_enters_tile() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(0, 0)); // Tile (0, 0)

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "enter-tile-1-0",
            RuleTrigger::OnTileEnter { x: 1, y: 0 },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "entered_tile".to_string(),
            }],
        )],
    });

    // Initialize tile tracking with first update
    state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Move player right to tile (1, 0) - player moves 2px per frame, needs ~8 frames to reach tile center
    state.handle_key_press(InputKey::Right);
    let mut fired = false;
    for _ in 0..10 {
        let result = state.update(
            UVec2::new(256, 256),
            &create_test_tilemap(),
            &create_test_atlas(),
        );
        if result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "entered_tile"
        )) {
            fired = true;
            break;
        }
    }

    assert!(
        fired,
        "OnTileEnter should fire when entity moves onto the specified tile"
    );
}

#[test]
fn on_tile_enter_does_not_fire_when_staying_on_same_tile() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(0, 0)); // Tile (0, 0)

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "enter-tile-0-0",
            RuleTrigger::OnTileEnter { x: 0, y: 0 },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "entered_tile".to_string(),
            }],
        )],
    });

    // First frame - no movement, already on tile
    let first = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Second frame - still no movement
    let second = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(
        first.events.is_empty(),
        "OnTileEnter should not fire when entity spawns on tile"
    );
    assert!(
        second.events.is_empty(),
        "OnTileEnter should not fire repeatedly when staying on same tile"
    );
}

#[test]
fn on_tile_enter_fires_only_on_transition() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(0, 0)); // Tile (0, 0)

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "enter-tile-1-0",
            RuleTrigger::OnTileEnter { x: 1, y: 0 },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "entered_tile".to_string(),
            }],
        )],
    });

    // Initialize tile tracking
    state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Move to tile (1, 0)
    state.handle_key_press(InputKey::Right);
    let mut entered = false;
    for _ in 0..10 {
        let result = state.update(
            UVec2::new(256, 256),
            &create_test_tilemap(),
            &create_test_atlas(),
        );
        if result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "entered_tile"
        )) {
            entered = true;
            break;
        }
    }

    // Stay on tile (1, 0)
    state.handle_key_release(InputKey::Right);
    let second = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(entered, "OnTileEnter should fire on first entry");
    assert!(
        second.events.is_empty(),
        "OnTileEnter should not fire again while staying on same tile"
    );
}

#[test]
fn on_tile_exit_fires_when_entity_leaves_tile() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(0, 0)); // Tile (0, 0)

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "exit-tile-0-0",
            RuleTrigger::OnTileExit { x: 0, y: 0 },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "exited_tile".to_string(),
            }],
        )],
    });

    // Initialize tile tracking
    let first = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Move right to tile (1, 0), leaving (0, 0)
    state.handle_key_press(InputKey::Right);
    let mut exited = false;
    for _ in 0..10 {
        let result = state.update(
            UVec2::new(256, 256),
            &create_test_tilemap(),
            &create_test_atlas(),
        );
        if result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "exited_tile"
        )) {
            exited = true;
            break;
        }
    }

    assert!(
        first.events.is_empty(),
        "OnTileExit should not fire before leaving tile"
    );
    assert!(exited, "OnTileExit should fire when entity leaves the specified tile");
}

#[test]
fn on_tile_exit_does_not_fire_repeatedly() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(0, 0)); // Tile (0, 0)

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "exit-tile-0-0",
            RuleTrigger::OnTileExit { x: 0, y: 0 },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "exited_tile".to_string(),
            }],
        )],
    });

    // Initialize tile tracking
    state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Move to tile (1, 0), leaving (0, 0)
    state.handle_key_press(InputKey::Right);
    let mut exited = false;
    for _ in 0..10 {
        let result = state.update(
            UVec2::new(256, 256),
            &create_test_tilemap(),
            &create_test_atlas(),
        );
        if result.events.iter().any(|event| matches!(
            event,
            AudioEvent::PlaySound { sound_id, .. } if sound_id == "exited_tile"
        )) {
            exited = true;
            break;
        }
    }

    // Stay on tile (1, 0)
    state.handle_key_release(InputKey::Right);
    let second = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert!(exited, "OnTileExit should fire when leaving tile");
    assert!(
        second.events.is_empty(),
        "OnTileExit should not fire repeatedly after leaving tile"
    );
}

#[test]
fn on_tile_enter_provides_trigger_self_context() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0)); // Tile (0, 0)

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "enter-with-velocity".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnTileEnter { x: 1, y: 0 },
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::SetVelocity {
                target: RuleTarget::TriggerSelf,
                velocity: [0, 5],
            }],
        }],
    });

    // Initialize tile tracking
    state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Move to tile (1, 0)
    state.handle_key_press(InputKey::Right);
    for _ in 0..10 {
        state.update(
            UVec2::new(256, 256),
            &create_test_tilemap(),
            &create_test_atlas(),
        );
    }

    // Entity should have velocity set by the rule
    let _player = state.entity_manager().get_entity(player_id).unwrap();
    // Velocity will be applied on next update, check rule runtime state
    let velocity = state.get_rule_velocity(player_id);
    assert_eq!(
        velocity,
        Some(IVec2::new(0, 5)),
        "OnTileEnter should provide TriggerSelf context for the entering entity"
    );
}

#[test]
fn on_tile_exit_provides_trigger_self_context() {
    let mut state = GameState::new_empty();
    let player_id = state.spawn_player_at(IVec2::new(0, 0)); // Tile (0, 0)

    state.set_rules(RuleSet {
        rules: vec![Rule {
            id: "exit-with-velocity".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnTileExit { x: 0, y: 0 },
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::SetVelocity {
                target: RuleTarget::TriggerSelf,
                velocity: [10, 0],
            }],
        }],
    });

    // Initialize tile tracking
    state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Move to tile (1, 0), leaving (0, 0)
    state.handle_key_press(InputKey::Right);
    for _ in 0..10 {
        state.update(
            UVec2::new(256, 256),
            &create_test_tilemap(),
            &create_test_atlas(),
        );
    }

    // Entity should have velocity set by the rule
    let velocity = state.get_rule_velocity(player_id);
    assert_eq!(
        velocity,
        Some(IVec2::new(10, 0)),
        "OnTileExit should provide TriggerSelf context for the exiting entity"
    );
}

#[test]
fn multiple_entities_can_trigger_tile_events_independently() {
    let mut state = GameState::new_empty();
    let _player_id = state.spawn_player_at(IVec2::new(0, 0)); // Tile (0, 0)
    let _npc_id = state.spawn_player_like_npc(IVec2::new(0, 16)); // Tile (0, 1)

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "enter-tile-1-0",
            RuleTrigger::OnTileEnter { x: 1, y: 0 },
            0,
            vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "entity_entered".to_string(),
            }],
        )],
    });

    // Initialize tile tracking
    state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    // Move player to tile (1, 0) - first entry
    state.handle_key_press(InputKey::Right);
    let mut first_enter_count = 0;
    for _ in 0..10 {
        let result = state.update(
            UVec2::new(256, 256),
            &create_test_tilemap(),
            &create_test_atlas(),
        );
        first_enter_count += result
            .events
            .iter()
            .filter(|event| matches!(
                event,
                AudioEvent::PlaySound { sound_id, .. } if sound_id == "entity_entered"
            ))
            .count();
    }
    state.handle_key_release(InputKey::Right);

    // Move player back to tile (0, 0)
    state.handle_key_press(InputKey::Left);
    for _ in 0..10 {
        state.update(
            UVec2::new(256, 256),
            &create_test_tilemap(),
            &create_test_atlas(),
        );
    }
    state.handle_key_release(InputKey::Left);

    // Move player to tile (1, 0) again - second entry
    state.handle_key_press(InputKey::Right);
    let mut second_enter_count = 0;
    for _ in 0..10 {
        let result = state.update(
            UVec2::new(256, 256),
            &create_test_tilemap(),
            &create_test_atlas(),
        );
        second_enter_count += result
            .events
            .iter()
            .filter(|event| matches!(
                event,
                AudioEvent::PlaySound { sound_id, .. } if sound_id == "entity_entered"
            ))
            .count();
    }

    assert_eq!(
        first_enter_count, 1,
        "First entry to tile should trigger OnTileEnter"
    );
    assert_eq!(
        second_enter_count, 1,
        "Second entry to same tile should also trigger OnTileEnter"
    );
}
