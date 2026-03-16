use super::{
    AiBehavior, EntityPropertyDraft, InspectorSystem, MovementProfile, MultiEntityBatchEdit,
    ProjectSettingsDraft, RuleActionKind, RuleConditionKind, RuleTriggerKind,
};
use crate::project::Project;
use crate::ui::EditorUI;
use glam::{IVec2, UVec2};
use std::fs;
use toki_core::animation::AnimationState;
use toki_core::collision::CollisionBox;
use toki_core::entity::{
    ControlRole, EntityAttributes, EntityKind, EntityManager, MovementSoundTrigger,
    ATTACK_POWER_STAT_ID, HEALTH_STAT_ID,
};
use toki_core::rules::{
    Rule, RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel, RuleSpawnEntityType,
    RuleTarget, RuleTrigger,
};
use toki_core::Scene;

fn sample_entity_with_id(id: u32) -> toki_core::entity::Entity {
    let mut manager = EntityManager::new();
    let spawned_id = manager.spawn_entity(
        EntityKind::Npc,
        IVec2::new(10, 20),
        UVec2::new(16, 16),
        EntityAttributes {
            health: Some(25),
            stats: toki_core::entity::EntityStats::from_legacy_health(Some(25)),
            speed: 3,
            solid: true,
            visible: true,
            animation_controller: None,
            static_object_render: None,
            render_layer: 1,
            active: true,
            can_move: true,
            ai_behavior: AiBehavior::Wander,
            movement_profile: MovementProfile::LegacyDefault,
            primary_projectile: None,
            projectile: None,
            pickup: None,
            inventory: toki_core::entity::Inventory::default(),
            has_inventory: false,
        },
    );
    let mut entity = manager
        .get_entity(spawned_id)
        .expect("missing spawned entity")
        .clone();
    entity.id = id;
    entity.category = "creature".to_string();
    entity.control_role = ControlRole::None;
    entity.collision_box = Some(CollisionBox::new(
        IVec2::new(0, 0),
        UVec2::new(16, 16),
        false,
    ));
    entity
}

fn sample_rule(id: &str) -> Rule {
    Rule {
        id: id.to_string(),
        enabled: true,
        priority: 0,
        once: false,
        trigger: RuleTrigger::OnUpdate,
        conditions: vec![RuleCondition::Always],
        actions: vec![RuleAction::PlaySound {
            channel: RuleSoundChannel::Movement,
            sound_id: "sfx_step".to_string(),
        }],
    }
}

#[test]
fn apply_entity_property_draft_clamps_and_sets_values() {
    let mut entity = sample_entity_with_id(1);
    let mut draft = EntityPropertyDraft::from_entity(&entity);
    draft.position_x = 100;
    draft.position_y = 200;
    draft.size_x = 0;
    draft.size_y = -5;
    draft.visible = false;
    draft.active = false;
    draft.solid = false;
    draft.can_move = false;
    draft.control_role = ControlRole::PlayerCharacter;
    draft.ai_behavior = AiBehavior::None;
    draft.movement_profile = MovementProfile::PlayerWasd;
    draft.movement_sound_trigger = MovementSoundTrigger::AnimationLoop;
    draft.footstep_trigger_distance = -5.0;
    draft.movement_sound = "sfx_custom_step".to_string();
    draft.has_inventory = true;
    draft.speed = -10;
    draft.render_layer = 8;
    draft.health_enabled = true;
    draft.health_value = -4;
    draft.attack_power_enabled = true;
    draft.attack_power_value = 12;
    draft.collision_enabled = true;
    draft.collision_offset_x = 3;
    draft.collision_offset_y = -2;
    draft.collision_size_x = 0;
    draft.collision_size_y = -7;
    draft.collision_trigger = true;

    let changed = InspectorSystem::apply_entity_property_draft(&mut entity, &draft);

    assert!(changed);
    assert_eq!(entity.position, IVec2::new(100, 200));
    assert_eq!(entity.size, UVec2::new(1, 1));
    assert!(!entity.attributes.visible);
    assert!(!entity.attributes.active);
    assert!(!entity.attributes.solid);
    assert!(!entity.attributes.can_move);
    assert_eq!(entity.control_role, ControlRole::PlayerCharacter);
    assert_eq!(entity.attributes.ai_behavior, AiBehavior::None);
    assert_eq!(
        entity.attributes.movement_profile,
        MovementProfile::PlayerWasd
    );
    assert_eq!(
        entity.audio.movement_sound_trigger,
        MovementSoundTrigger::AnimationLoop
    );
    assert_eq!(entity.audio.footstep_trigger_distance, 0.0);
    assert_eq!(
        entity.audio.movement_sound.as_deref(),
        Some("sfx_custom_step")
    );
    assert!(entity.attributes.has_inventory);
    assert_eq!(entity.attributes.speed, 0);
    assert_eq!(entity.attributes.render_layer, 8);
    assert_eq!(entity.attributes.health, Some(0));
    assert_eq!(entity.attributes.current_stat(HEALTH_STAT_ID), Some(0));
    assert_eq!(
        entity.attributes.current_stat(ATTACK_POWER_STAT_ID),
        Some(12)
    );

    let collision = entity
        .collision_box
        .as_ref()
        .expect("collision should be enabled");
    assert_eq!(collision.offset, IVec2::new(3, -2));
    assert_eq!(collision.size, UVec2::new(1, 1));
    assert!(collision.trigger);
}

#[test]
fn apply_entity_property_draft_disables_health_and_collision() {
    let mut entity = sample_entity_with_id(1);
    InspectorSystem::set_optional_runtime_stat(
        &mut entity.attributes,
        ATTACK_POWER_STAT_ID,
        Some(9),
    );
    let mut draft = EntityPropertyDraft::from_entity(&entity);
    draft.health_enabled = false;
    draft.attack_power_enabled = false;
    draft.collision_enabled = false;

    let changed = InspectorSystem::apply_entity_property_draft(&mut entity, &draft);

    assert!(changed);
    assert_eq!(entity.attributes.health, None);
    assert_eq!(entity.attributes.current_stat(HEALTH_STAT_ID), None);
    assert_eq!(entity.attributes.current_stat(ATTACK_POWER_STAT_ID), None);
    assert!(entity.collision_box.is_none());
}

#[test]
fn apply_project_settings_draft_updates_metadata_and_marks_project_dirty() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let mut project = Project::new("Demo".to_string(), temp_dir.path().join("Demo"));
    let original_modified = project.metadata.project.modified;
    let draft = ProjectSettingsDraft {
        name: "Renamed Demo".to_string(),
        version: "2.0.0".to_string(),
        description: "Updated description".to_string(),
        splash_duration_ms: 4500,
        show_entity_health_bars: true,
        master_mix_percent: 85,
        music_mix_percent: 70,
        movement_mix_percent: 55,
        collision_mix_percent: 35,
    };

    let changed = InspectorSystem::apply_project_settings_draft(&mut project, &draft);

    assert!(changed);
    assert_eq!(project.name, "Renamed Demo");
    assert_eq!(project.metadata.project.name, "Renamed Demo");
    assert_eq!(project.metadata.project.version, "2.0.0");
    assert_eq!(project.metadata.project.description, "Updated description");
    assert_eq!(project.metadata.runtime.splash.duration_ms, 4500);
    assert!(project.metadata.runtime.display.show_entity_health_bars);
    assert_eq!(project.metadata.runtime.audio.master_percent, 85);
    assert_eq!(project.metadata.runtime.audio.music_percent, 70);
    assert_eq!(project.metadata.runtime.audio.movement_percent, 55);
    assert_eq!(project.metadata.runtime.audio.collision_percent, 35);
    assert!(project.is_dirty);
    assert!(project.metadata.project.modified >= original_modified);
}

#[test]
fn collect_multi_entity_common_state_reports_mixed_values() {
    let mut first = sample_entity_with_id(1);
    let mut second = sample_entity_with_id(2);

    first.attributes.visible = true;
    second.attributes.visible = false;
    first.attributes.active = true;
    second.attributes.active = true;
    first.attributes.render_layer = 2;
    second.attributes.render_layer = 2;
    second.collision_box = None;

    let entities = vec![&first, &second];
    let common = InspectorSystem::collect_multi_entity_common_state(&entities);

    assert_eq!(common.visible, None);
    assert_eq!(common.active, Some(true));
    assert_eq!(common.render_layer, Some(2));
    assert_eq!(common.collision_enabled, None);
}

#[test]
fn apply_multi_entity_batch_edit_updates_all_selected_entities() {
    let mut first = sample_entity_with_id(1);
    let mut second = sample_entity_with_id(2);
    second.collision_box = None;

    let edit = MultiEntityBatchEdit {
        set_visible: Some(false),
        set_active: Some(false),
        set_collision_enabled: Some(true),
        set_render_layer: Some(7),
        position_delta: Some(IVec2::new(2, -3)),
    };
    let changed = InspectorSystem::apply_multi_entity_batch_edit_to_entity(&mut first, edit)
        | InspectorSystem::apply_multi_entity_batch_edit_to_entity(&mut second, edit);

    assert!(changed);
    assert!(!first.attributes.visible);
    assert!(!second.attributes.visible);
    assert!(!first.attributes.active);
    assert!(!second.attributes.active);
    assert_eq!(first.attributes.render_layer, 7);
    assert_eq!(second.attributes.render_layer, 7);
    assert_eq!(first.position, IVec2::new(12, 17));
    assert_eq!(second.position, IVec2::new(12, 17));
    assert!(first.collision_box.is_some());
    assert!(second.collision_box.is_some());
}

#[test]
fn find_selected_scene_entity_returns_entity_from_active_scene() {
    let mut ui_state = EditorUI::new();
    let entity = sample_entity_with_id(7);
    let scene = ui_state
        .scenes
        .iter_mut()
        .find(|scene| scene.name == "Main Scene")
        .expect("missing default scene");
    scene.entities.push(entity);

    let selected_entity =
        InspectorSystem::find_selected_scene_entity(&ui_state, 7).expect("entity should be found");
    assert_eq!(selected_entity.id, 7);
    assert_eq!(selected_entity.position, IVec2::new(10, 20));
}

#[test]
fn find_selected_scene_entity_returns_none_for_inactive_scene() {
    let mut ui_state = EditorUI::new();
    ui_state.scenes.push(Scene::new("Other".to_string()));
    ui_state.active_scene = Some("Other".to_string());

    let scene = ui_state
        .scenes
        .iter_mut()
        .find(|scene| scene.name == "Main Scene")
        .expect("missing default scene");
    scene.entities.push(sample_entity_with_id(42));

    assert!(InspectorSystem::find_selected_scene_entity(&ui_state, 42).is_none());
}

#[test]
fn apply_entity_property_draft_with_undo_round_trips() {
    let mut ui_state = EditorUI::new();
    let entity = sample_entity_with_id(7);
    let scene = ui_state
        .scenes
        .iter_mut()
        .find(|scene| scene.name == "Main Scene")
        .expect("missing default scene");
    scene.entities.push(entity.clone());

    let mut draft = EntityPropertyDraft::from_entity(&entity);
    draft.position_x = 99;
    draft.position_y = -8;
    draft.visible = false;

    assert!(InspectorSystem::apply_entity_property_draft_with_undo(
        &mut ui_state,
        7,
        &draft
    ));
    let edited = InspectorSystem::find_selected_scene_entity(&ui_state, 7)
        .expect("entity should still exist");
    assert_eq!(edited.position, IVec2::new(99, -8));
    assert!(!edited.attributes.visible);

    assert!(ui_state.undo());
    let restored = InspectorSystem::find_selected_scene_entity(&ui_state, 7)
        .expect("entity should still exist");
    assert_eq!(restored.position, IVec2::new(10, 20));
    assert!(restored.attributes.visible);
}

#[test]
fn apply_entity_property_draft_with_undo_enforces_single_player_character() {
    let mut ui_state = EditorUI::new();
    let mut first = sample_entity_with_id(1);
    first.control_role = ControlRole::PlayerCharacter;
    let second = sample_entity_with_id(2);
    let scene = ui_state
        .scenes
        .iter_mut()
        .find(|scene| scene.name == "Main Scene")
        .expect("missing default scene");
    scene.entities.push(first);
    scene.entities.push(second.clone());

    let mut draft = EntityPropertyDraft::from_entity(&second);
    draft.control_role = ControlRole::PlayerCharacter;

    assert!(InspectorSystem::apply_entity_property_draft_with_undo(
        &mut ui_state,
        2,
        &draft,
    ));

    let scene = ui_state
        .scenes
        .iter()
        .find(|scene| scene.name == "Main Scene")
        .expect("missing default scene");
    let first = scene
        .entities
        .iter()
        .find(|entity| entity.id == 1)
        .expect("first entity should exist");
    let second = scene
        .entities
        .iter()
        .find(|entity| entity.id == 2)
        .expect("second entity should exist");

    assert_eq!(first.control_role, ControlRole::None);
    assert_eq!(second.control_role, ControlRole::PlayerCharacter);
}

#[test]
fn next_rule_id_fills_first_available_gap() {
    let rules = RuleSet {
        rules: vec![
            toki_core::rules::Rule {
                id: "rule_1".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnUpdate,
                conditions: vec![RuleCondition::Always],
                actions: vec![],
            },
            toki_core::rules::Rule {
                id: "rule_3".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnUpdate,
                conditions: vec![RuleCondition::Always],
                actions: vec![],
            },
        ],
    };

    let next = InspectorSystem::next_rule_id(&rules);
    assert_eq!(next, "rule_2");
}

#[test]
fn add_default_rule_appends_editable_placeholder_rule() {
    let mut rules = RuleSet::default();
    let id = InspectorSystem::add_default_rule(&mut rules);

    assert_eq!(id, "rule_1");
    assert_eq!(rules.rules.len(), 1);

    let rule = &rules.rules[0];
    assert_eq!(rule.id, "rule_1");
    assert!(rule.enabled);
    assert_eq!(rule.priority, 0);
    assert!(!rule.once);
    assert_eq!(rule.trigger, RuleTrigger::OnUpdate);
    assert_eq!(rule.conditions, vec![RuleCondition::Always]);
    assert_eq!(rule.actions.len(), 1);
    assert_eq!(
        rule.actions[0],
        RuleAction::PlaySound {
            channel: RuleSoundChannel::Movement,
            sound_id: "sfx_placeholder".to_string(),
        }
    );
}

#[test]
fn set_rule_trigger_kind_sets_expected_trigger_payload() {
    let mut rule = sample_rule("rule_1");

    InspectorSystem::set_rule_trigger_kind(&mut rule, RuleTriggerKind::Start);
    assert_eq!(rule.trigger, RuleTrigger::OnStart);

    InspectorSystem::set_rule_trigger_kind(&mut rule, RuleTriggerKind::Update);
    assert_eq!(rule.trigger, RuleTrigger::OnUpdate);

    InspectorSystem::set_rule_trigger_kind(&mut rule, RuleTriggerKind::PlayerMove);
    assert_eq!(rule.trigger, RuleTrigger::OnPlayerMove);

    InspectorSystem::set_rule_trigger_kind(&mut rule, RuleTriggerKind::Collision);
    assert_eq!(rule.trigger, RuleTrigger::OnCollision);

    InspectorSystem::set_rule_trigger_kind(&mut rule, RuleTriggerKind::Damaged);
    assert_eq!(rule.trigger, RuleTrigger::OnDamaged);

    InspectorSystem::set_rule_trigger_kind(&mut rule, RuleTriggerKind::Death);
    assert_eq!(rule.trigger, RuleTrigger::OnDeath);

    InspectorSystem::set_rule_trigger_kind(&mut rule, RuleTriggerKind::Trigger);
    assert_eq!(rule.trigger, RuleTrigger::OnTrigger);

    InspectorSystem::set_rule_trigger_kind(&mut rule, RuleTriggerKind::Key);
    assert_eq!(rule.trigger, RuleTrigger::OnKey { key: RuleKey::Up });
}

#[test]
fn duplicate_rule_clones_payload_with_new_id_and_insert_position() {
    let mut rules = RuleSet {
        rules: vec![sample_rule("rule_1"), sample_rule("rule_2")],
    };

    let inserted_index =
        InspectorSystem::duplicate_rule(&mut rules, 0).expect("duplicate should succeed");

    assert_eq!(inserted_index, 1);
    assert_eq!(rules.rules.len(), 3);
    assert_eq!(rules.rules[0].id, "rule_1");
    assert_eq!(rules.rules[1].id, "rule_3");
    assert_eq!(rules.rules[2].id, "rule_2");
    assert_eq!(rules.rules[1].actions, rules.rules[0].actions);
}

#[test]
fn remove_rule_returns_next_selection_or_previous_for_last() {
    let mut rules = RuleSet {
        rules: vec![
            sample_rule("rule_1"),
            sample_rule("rule_2"),
            sample_rule("rule_3"),
        ],
    };

    let selected_after_middle =
        InspectorSystem::remove_rule(&mut rules, 1).expect("selection should stay valid");
    assert_eq!(selected_after_middle, 1);
    assert_eq!(
        rules
            .rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<Vec<_>>(),
        vec!["rule_1", "rule_3"]
    );

    let selected_after_last =
        InspectorSystem::remove_rule(&mut rules, 1).expect("last removal should select prev");
    assert_eq!(selected_after_last, 0);
    assert_eq!(
        rules
            .rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<Vec<_>>(),
        vec!["rule_1"]
    );

    let selected_after_final = InspectorSystem::remove_rule(&mut rules, 0);
    assert!(selected_after_final.is_none());
    assert!(rules.rules.is_empty());
}

#[test]
fn move_rule_up_and_down_reorders_and_handles_boundaries() {
    let mut rules = RuleSet {
        rules: vec![
            sample_rule("rule_1"),
            sample_rule("rule_2"),
            sample_rule("rule_3"),
        ],
    };

    let up_index = InspectorSystem::move_rule_up(&mut rules, 1).expect("move up should work");
    assert_eq!(up_index, 0);
    assert_eq!(
        rules
            .rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<Vec<_>>(),
        vec!["rule_2", "rule_1", "rule_3"]
    );

    let noop_up = InspectorSystem::move_rule_up(&mut rules, 0).expect("boundary no-op");
    assert_eq!(noop_up, 0);
    assert_eq!(
        rules
            .rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<Vec<_>>(),
        vec!["rule_2", "rule_1", "rule_3"]
    );

    let down_index = InspectorSystem::move_rule_down(&mut rules, 1).expect("move down should work");
    assert_eq!(down_index, 2);
    assert_eq!(
        rules
            .rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<Vec<_>>(),
        vec!["rule_2", "rule_3", "rule_1"]
    );

    let noop_down = InspectorSystem::move_rule_down(&mut rules, 2).expect("boundary no-op");
    assert_eq!(noop_down, 2);
}

#[test]
fn add_remove_and_switch_action_types() {
    let mut rule = sample_rule("rule_1");
    assert_eq!(rule.actions.len(), 1);

    InspectorSystem::add_action(&mut rule, RuleActionKind::PlayMusic);
    InspectorSystem::add_action(&mut rule, RuleActionKind::PlayAnimation);
    InspectorSystem::add_action(&mut rule, RuleActionKind::SetVelocity);
    InspectorSystem::add_action(&mut rule, RuleActionKind::Spawn);
    InspectorSystem::add_action(&mut rule, RuleActionKind::DestroySelf);
    InspectorSystem::add_action(&mut rule, RuleActionKind::SwitchScene);
    assert_eq!(rule.actions.len(), 7);
    assert!(matches!(
        rule.actions[1],
        RuleAction::PlayMusic { ref track_id } if track_id == "music_placeholder"
    ));
    assert!(matches!(
        rule.actions[2],
        RuleAction::PlayAnimation {
            target: RuleTarget::Player,
            state: AnimationState::Idle
        }
    ));
    assert!(matches!(
        rule.actions[3],
        RuleAction::SetVelocity {
            target: RuleTarget::Player,
            velocity: [0, 0]
        }
    ));
    assert!(matches!(
        rule.actions[4],
        RuleAction::Spawn {
            entity_type: RuleSpawnEntityType::Npc,
            position: [0, 0]
        }
    ));
    assert!(matches!(
        rule.actions[5],
        RuleAction::DestroySelf {
            target: RuleTarget::Player
        }
    ));
    assert!(matches!(
        rule.actions[6],
        RuleAction::SwitchScene { ref scene_name } if scene_name.is_empty()
    ));

    InspectorSystem::switch_action_kind(&mut rule.actions[0], RuleActionKind::SetVelocity);
    assert!(matches!(
        rule.actions[0],
        RuleAction::SetVelocity {
            target: RuleTarget::Player,
            velocity: [0, 0]
        }
    ));
    InspectorSystem::switch_action_kind(&mut rule.actions[0], RuleActionKind::PlaySound);
    assert!(matches!(
        rule.actions[0],
        RuleAction::PlaySound {
            channel: RuleSoundChannel::Movement,
            ref sound_id,
        } if sound_id == "sfx_placeholder"
    ));
    InspectorSystem::switch_action_kind(&mut rule.actions[0], RuleActionKind::DestroySelf);
    assert!(matches!(
        rule.actions[0],
        RuleAction::DestroySelf {
            target: RuleTarget::Player
        }
    ));

    assert!(InspectorSystem::remove_action(&mut rule, 1));
    assert_eq!(rule.actions.len(), 6);
    assert!(!InspectorSystem::remove_action(&mut rule, 99));
}

#[test]
fn add_remove_and_switch_condition_types() {
    let mut rule = sample_rule("rule_1");
    assert_eq!(rule.conditions, vec![RuleCondition::Always]);

    InspectorSystem::add_condition(&mut rule, RuleConditionKind::TargetExists);
    InspectorSystem::add_condition(&mut rule, RuleConditionKind::KeyHeld);
    InspectorSystem::add_condition(&mut rule, RuleConditionKind::EntityActive);

    assert_eq!(rule.conditions.len(), 4);
    assert!(matches!(
        rule.conditions[1],
        RuleCondition::TargetExists {
            target: RuleTarget::Player
        }
    ));
    assert!(matches!(
        rule.conditions[2],
        RuleCondition::KeyHeld { key: RuleKey::Up }
    ));
    assert!(matches!(
        rule.conditions[3],
        RuleCondition::EntityActive {
            target: RuleTarget::Player,
            is_active: true
        }
    ));

    InspectorSystem::switch_condition_kind(
        &mut rule.conditions[0],
        RuleConditionKind::EntityActive,
    );
    assert!(matches!(
        rule.conditions[0],
        RuleCondition::EntityActive {
            target: RuleTarget::Player,
            is_active: true
        }
    ));

    assert!(InspectorSystem::remove_condition(&mut rule, 2));
    assert_eq!(rule.conditions.len(), 3);
    assert!(!InspectorSystem::remove_condition(&mut rule, 99));
}

#[test]
fn validate_rule_set_reports_duplicate_ids_and_invalid_action_payloads() {
    let mut first = sample_rule("dupe");
    first.conditions = vec![RuleCondition::TargetExists {
        target: RuleTarget::Entity(0),
    }];
    first.actions = vec![
        RuleAction::PlaySound {
            channel: RuleSoundChannel::Movement,
            sound_id: "   ".to_string(),
        },
        RuleAction::SetVelocity {
            target: RuleTarget::Entity(0),
            velocity: [1, 0],
        },
        RuleAction::DestroySelf {
            target: RuleTarget::Entity(0),
        },
    ];

    let second = Rule {
        id: "dupe".to_string(),
        enabled: true,
        priority: 1,
        once: false,
        trigger: RuleTrigger::OnStart,
        conditions: vec![RuleCondition::Always],
        actions: vec![RuleAction::SwitchScene {
            scene_name: "   ".to_string(),
        }],
    };

    let rules = RuleSet {
        rules: vec![first, second],
    };

    let issues = InspectorSystem::validate_rule_set(&rules);
    assert!(issues
        .iter()
        .any(|issue| issue.message.contains("Duplicate rule id 'dupe'")));
    assert!(issues.iter().any(|issue| issue
        .message
        .contains("PlaySound requires a non-empty sound id")));
    assert!(issues.iter().any(|issue| issue
        .message
        .contains("SetVelocity entity target must be non-zero")));
    assert!(issues.iter().any(|issue| issue
        .message
        .contains("DestroySelf entity target must be non-zero")));
    assert!(issues.iter().any(|issue| issue
        .message
        .contains("Condition 1 entity target must be non-zero")));
    assert!(issues
        .iter()
        .any(|issue| issue.message.contains("SwitchScene requires a scene name")));
}

#[test]
fn validate_rule_set_reports_empty_play_music_track() {
    let rules = RuleSet {
        rules: vec![Rule {
            id: "music-rule".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlayMusic {
                track_id: "   ".to_string(),
            }],
        }],
    };

    let issues = InspectorSystem::validate_rule_set(&rules);
    assert!(issues.iter().any(|issue| issue
        .message
        .contains("PlayMusic requires a non-empty track id")));
}

#[test]
fn discover_audio_asset_names_reads_supported_audio_files() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(temp_dir.path().join("battle_theme.ogg"), "x").expect("ogg file write");
    fs::write(temp_dir.path().join("ambience.mp3"), "x").expect("mp3 file write");
    fs::write(temp_dir.path().join("impact.wav"), "x").expect("wav file write");
    fs::write(temp_dir.path().join("ignore.txt"), "x").expect("txt file write");
    fs::create_dir(temp_dir.path().join("sub")).expect("subdir create");
    fs::write(temp_dir.path().join("sub").join("nested.ogg"), "x").expect("nested write");

    let names = InspectorSystem::discover_audio_asset_names(temp_dir.path());
    assert_eq!(names, vec!["ambience", "battle_theme", "impact"]);
}

#[test]
fn selected_map_editor_tile_metadata_reads_solid_and_trigger_flags() {
    let mut tiles = std::collections::HashMap::new();
    tiles.insert(
        "grass".to_string(),
        toki_core::assets::atlas::TileInfo {
            position: UVec2::new(0, 0),
            properties: toki_core::assets::atlas::TileProperties {
                solid: true,
                trigger: false,
            },
        },
    );
    let atlas = toki_core::assets::atlas::AtlasMeta {
        image: std::path::PathBuf::from("terrain.png"),
        tile_size: UVec2::new(8, 8),
        tiles,
    };

    assert_eq!(
        InspectorSystem::selected_map_editor_tile_metadata(&atlas, "grass"),
        Some((true, false))
    );
    assert_eq!(
        InspectorSystem::selected_map_editor_tile_metadata(&atlas, "missing"),
        None
    );
}

#[test]
fn animation_state_options_include_attack_states() {
    let options = super::animation_state_options();
    assert!(options.contains(&AnimationState::Attack));
    assert!(options.contains(&AnimationState::AttackDown));
    assert!(options.contains(&AnimationState::AttackUp));
    assert!(options.contains(&AnimationState::AttackLeft));
    assert!(options.contains(&AnimationState::AttackRight));
    assert_eq!(
        super::animation_state_label(AnimationState::AttackLeft),
        "Attack Left"
    );
}

#[test]
fn save_entity_definition_persists_audio_updates() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let entity_file = temp_dir.path().join("player.json");

    let definition = toki_core::entity::EntityDefinition {
        name: "player".to_string(),
        display_name: "Player".to_string(),
        description: "desc".to_string(),
        rendering: toki_core::entity::RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            static_object: None,
        },
        attributes: toki_core::entity::AttributesDef {
            health: Some(100),
            stats: std::collections::HashMap::from([(ATTACK_POWER_STAT_ID.to_string(), 14)]),
            speed: 2,
            solid: true,
            active: true,
            can_move: true,
            ai_behavior: AiBehavior::None,
            movement_profile: MovementProfile::PlayerWasd,
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
        },
        collision: toki_core::entity::CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: toki_core::entity::AudioDef {
            footstep_trigger_distance: 42.0,
            hearing_radius: 144,
            movement_sound_trigger: MovementSoundTrigger::AnimationLoop,
            movement_sound: "sfx_step".to_string(),
            collision_sound: None,
        },
        animations: toki_core::entity::AnimationsDef {
            atlas_name: "players".to_string(),
            clips: vec![],
            default_state: "idle".to_string(),
        },
        category: "human".to_string(),
        tags: vec![],
    };

    InspectorSystem::save_entity_definition(&definition, &entity_file)
        .expect("entity definition should save");

    let content =
        fs::read_to_string(&entity_file).expect("saved entity definition should be readable");
    let reloaded: toki_core::entity::EntityDefinition =
        serde_json::from_str(&content).expect("saved entity definition should parse");

    assert_eq!(reloaded.audio.footstep_trigger_distance, 42.0);
    assert_eq!(reloaded.audio.hearing_radius, 144);
    assert_eq!(
        reloaded.audio.movement_sound_trigger,
        MovementSoundTrigger::AnimationLoop
    );
    assert_eq!(reloaded.audio.movement_sound, "sfx_step");
    assert_eq!(
        reloaded.attributes.stats.get(ATTACK_POWER_STAT_ID).copied(),
        Some(14)
    );
}
