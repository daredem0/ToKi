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
    Rule, RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel, RuleSpawnEntityType,
    RuleTarget, RuleTrigger,
};
use toki_core::{entity::EntityKind, GameState, InputKey, Scene};

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
            RuleTrigger::OnCollision,
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
            RuleTrigger::OnCollision,
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
            RuleTrigger::OnDamaged,
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
            RuleTrigger::OnDamaged,
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
            RuleTrigger::OnDeath,
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
            RuleTrigger::OnDeath,
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
fn switch_scene_applies_at_tick_boundary_after_movement_processing() {
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
    let scene_b = scene_with_player("Scene B", IVec2::new(100, 0));

    state.add_scene(scene_a);
    state.add_scene(scene_b);
    state
        .load_scene("Scene A")
        .expect("initial scene should load");

    state.handle_key_press(InputKey::Right);
    state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert_eq!(
        state.scene_manager().active_scene_name(),
        Some("Scene B"),
        "switch should apply by end of tick"
    );
    assert_eq!(
        state.player_position(),
        IVec2::new(100, 0),
        "destination scene should load after the tick, not be moved by this tick's input"
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
    state.add_scene(scene_with_player("Scene B", IVec2::new(10, 0)));
    state.add_scene(scene_with_player("Scene C", IVec2::new(20, 0)));
    state
        .load_scene("Scene A")
        .expect("initial scene should load");

    state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert_eq!(state.scene_manager().active_scene_name(), Some("Scene B"));
    assert_eq!(state.player_position(), IVec2::new(10, 0));
}

#[test]
fn switch_scene_keeps_active_scene_when_target_is_missing() {
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

    state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert_eq!(state.scene_manager().active_scene_name(), Some("Scene A"));
    assert_eq!(state.player_position(), IVec2::new(0, 0));
}

#[test]
fn switch_scene_syncs_outgoing_scene_entities_before_loading_target() {
    let mut state = GameState::new_empty();
    let mut scene_a = scene_with_player("Scene A", IVec2::new(0, 0));
    scene_a.rules = RuleSet {
        rules: vec![base_rule(
            "move-and-switch",
            RuleTrigger::OnUpdate,
            0,
            vec![
                RuleAction::SetVelocity {
                    target: RuleTarget::Player,
                    velocity: [1, 0],
                },
                RuleAction::SwitchScene {
                    scene_name: "Scene B".to_string(),
                    spawn_point_id: "spawn_b".to_string(),
                },
            ],
        )],
    };

    state.add_scene(scene_a);
    state.add_scene(scene_with_player("Scene B", IVec2::new(50, 0)));
    state
        .load_scene("Scene A")
        .expect("initial scene should load");

    state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );

    assert_eq!(state.scene_manager().active_scene_name(), Some("Scene B"));
    let scene_a_after = state
        .scene_manager()
        .get_scene("Scene A")
        .expect("scene A should still exist");
    let persisted_player = scene_a_after
        .entities
        .iter()
        .find(|entity| matches!(entity.entity_kind, EntityKind::Player))
        .expect("scene A should still contain its player entity");
    assert_eq!(persisted_player.position, IVec2::new(1, 0));
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
