use glam::{IVec2, UVec2};
use std::collections::HashMap;
use std::path::PathBuf;
use toki_core::assets::{
    atlas::{AtlasMeta, TileInfo, TileProperties},
    tilemap::TileMap,
};
use toki_core::game::{AudioChannel, AudioEvent};
use toki_core::rules::{
    Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleTarget, RuleTrigger,
};
use toki_core::{GameState, InputKey, Scene};

fn create_test_tilemap() -> TileMap {
    TileMap {
        size: UVec2::new(10, 10),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test_atlas.json"),
        tiles: vec!["floor".to_string(); 100],
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
        } if sound_id == "sfx_start"
    ));
    assert!(second.events.is_empty());
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
            conditions: vec![RuleCondition::Always],
            actions: vec![
                RuleAction::PlaySound {
                    channel: RuleSoundChannel::Movement,
                    sound_id: "sfx_a".to_string(),
                },
                RuleAction::SetVelocity {
                    target: RuleTarget::Player,
                    velocity: [1, -1],
                },
                RuleAction::PlayMusic {
                    track_id: "lavandia".to_string(),
                },
                RuleAction::SwitchScene {
                    scene_name: "Town".to_string(),
                },
            ],
        }],
    };

    let json = serde_json::to_string_pretty(&rules).expect("rules should serialize");
    let parsed: RuleSet = serde_json::from_str(&json).expect("rules should deserialize");
    assert_eq!(rules, parsed);
}

#[test]
fn switch_scene_placeholder_does_not_change_active_scene_or_emit_events() {
    let mut state = GameState::new_empty();
    state.add_scene(Scene::new("Scene A".to_string()));
    state.add_scene(Scene::new("Scene B".to_string()));
    state
        .load_scene("Scene A")
        .expect("initial scene should load");

    state.set_rules(RuleSet {
        rules: vec![base_rule(
            "switch-placeholder",
            RuleTrigger::OnUpdate,
            0,
            vec![RuleAction::SwitchScene {
                scene_name: "Scene B".to_string(),
            }],
        )],
    });

    let before = state
        .active_scene()
        .expect("active scene should exist")
        .name
        .clone();
    let result = state.update(
        UVec2::new(256, 256),
        &create_test_tilemap(),
        &create_test_atlas(),
    );
    let after = state
        .active_scene()
        .expect("active scene should exist")
        .name
        .clone();

    assert_eq!(before, "Scene A");
    assert_eq!(after, "Scene A");
    assert!(result.events.is_empty());
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
