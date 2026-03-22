//! TDD tests for the unified rule evaluation refactoring.
//!
//! These tests define the expected behavior of the rule collection system
//! before and after extracting the common pattern into a unified helper.

use super::rules::{
    CollisionEvent, DamageEvent, DeathEvent, InteractionEvent, InteractionSpatial, RuleCommand,
};
use super::GameState;
use crate::entity::{EntityAttributes, EntityId, EntityKind};
use crate::rules::{
    InteractionMode, Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleTarget,
    RuleTrigger,
};

fn create_test_game_state() -> GameState {
    GameState::new_empty()
}

fn create_test_rule(id: &str, trigger: RuleTrigger) -> Rule {
    Rule {
        id: id.to_string(),
        enabled: true,
        priority: 0,
        once: false,
        trigger,
        conditions: vec![],
        actions: vec![RuleAction::PlaySound {
            channel: RuleSoundChannel::Collision,
            sound_id: "test_sound".to_string(),
        }],
    }
}

fn create_rule_with_priority(id: &str, trigger: RuleTrigger, priority: i32) -> Rule {
    Rule {
        id: id.to_string(),
        enabled: true,
        priority,
        once: false,
        trigger,
        conditions: vec![],
        actions: vec![RuleAction::PlaySound {
            channel: RuleSoundChannel::Collision,
            sound_id: format!("{}_sound", id),
        }],
    }
}

fn create_once_rule(id: &str, trigger: RuleTrigger) -> Rule {
    Rule {
        id: id.to_string(),
        enabled: true,
        priority: 0,
        once: true,
        trigger,
        conditions: vec![],
        actions: vec![RuleAction::PlaySound {
            channel: RuleSoundChannel::Collision,
            sound_id: "once_sound".to_string(),
        }],
    }
}

fn create_disabled_rule(id: &str, trigger: RuleTrigger) -> Rule {
    Rule {
        id: id.to_string(),
        enabled: false,
        priority: 0,
        once: false,
        trigger,
        conditions: vec![],
        actions: vec![RuleAction::PlaySound {
            channel: RuleSoundChannel::Collision,
            sound_id: "disabled_sound".to_string(),
        }],
    }
}

fn spawn_test_entity(game_state: &mut GameState) -> EntityId {
    game_state.entity_manager_mut().spawn_entity(
        EntityKind::Npc,
        glam::IVec2::new(100, 100),
        glam::UVec2::new(16, 16),
        EntityAttributes::default(),
    )
}

// ═══════════════════════════════════════════════════════════════════════════
// COLLISION RULE TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn collision_rules_fire_for_matching_trigger() {
    let mut game_state = create_test_game_state();
    let entity_a = spawn_test_entity(&mut game_state);
    let entity_b = spawn_test_entity(&mut game_state);

    let rule = create_test_rule("collision_rule", RuleTrigger::OnCollision { entity: None });
    game_state.set_rules(RuleSet { rules: vec![rule] });

    let event = CollisionEvent {
        entity_a,
        entity_b: Some(entity_b),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_collision(&event, &mut commands);

    assert_eq!(commands.len(), 1);
    assert!(matches!(commands[0], RuleCommand::PlaySound { .. }));
}

#[test]
fn collision_rules_respect_priority_ordering() {
    let mut game_state = create_test_game_state();
    let entity_a = spawn_test_entity(&mut game_state);
    let entity_b = spawn_test_entity(&mut game_state);

    let low_priority =
        create_rule_with_priority("low", RuleTrigger::OnCollision { entity: None }, 0);
    let high_priority =
        create_rule_with_priority("high", RuleTrigger::OnCollision { entity: None }, 10);

    game_state.set_rules(RuleSet {
        rules: vec![low_priority, high_priority],
    });

    let event = CollisionEvent {
        entity_a,
        entity_b: Some(entity_b),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_collision(&event, &mut commands);

    assert_eq!(commands.len(), 2);
    // High priority should execute first
    match &commands[0] {
        RuleCommand::PlaySound { sound_id, .. } => assert_eq!(sound_id, "high_sound"),
        _ => panic!("Expected PlaySound command"),
    }
}

#[test]
fn collision_rules_skip_disabled_rules() {
    let mut game_state = create_test_game_state();
    let entity_a = spawn_test_entity(&mut game_state);
    let entity_b = spawn_test_entity(&mut game_state);

    let rule = create_disabled_rule("disabled", RuleTrigger::OnCollision { entity: None });
    game_state.set_rules(RuleSet { rules: vec![rule] });

    let event = CollisionEvent {
        entity_a,
        entity_b: Some(entity_b),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_collision(&event, &mut commands);

    assert!(commands.is_empty());
}

#[test]
fn collision_rules_fire_once_rules_only_once() {
    let mut game_state = create_test_game_state();
    let entity_a = spawn_test_entity(&mut game_state);
    let entity_b = spawn_test_entity(&mut game_state);

    let rule = create_once_rule("once_rule", RuleTrigger::OnCollision { entity: None });
    game_state.set_rules(RuleSet { rules: vec![rule] });

    let event = CollisionEvent {
        entity_a,
        entity_b: Some(entity_b),
    };

    // First call should fire
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_collision(&event, &mut commands);
    assert_eq!(commands.len(), 1);

    // Second call should not fire
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_collision(&event, &mut commands);
    assert!(commands.is_empty());
}

#[test]
fn collision_rules_filter_by_entity() {
    let mut game_state = create_test_game_state();
    let entity_a = spawn_test_entity(&mut game_state);
    let entity_b = spawn_test_entity(&mut game_state);

    // Rule that only fires for entity_a
    let filtered_rule = create_test_rule(
        "filtered",
        RuleTrigger::OnCollision {
            entity: Some(RuleTarget::Entity(entity_a)),
        },
    );
    game_state.set_rules(RuleSet {
        rules: vec![filtered_rule],
    });

    // Collision with entity_a as subject should fire
    let event = CollisionEvent {
        entity_a,
        entity_b: Some(entity_b),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_collision(&event, &mut commands);
    assert_eq!(commands.len(), 1);

    // Collision with entity_b as subject should NOT fire
    let event = CollisionEvent {
        entity_a: entity_b,
        entity_b: Some(entity_a),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_collision(&event, &mut commands);
    assert!(commands.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// DAMAGE RULE TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn damage_rules_fire_for_matching_trigger() {
    let mut game_state = create_test_game_state();
    let victim = spawn_test_entity(&mut game_state);
    let attacker = spawn_test_entity(&mut game_state);

    let rule = create_test_rule("damage_rule", RuleTrigger::OnDamaged { entity: None });
    game_state.set_rules(RuleSet { rules: vec![rule] });

    let event = DamageEvent {
        victim,
        attacker: Some(attacker),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_damage(&event, &mut commands);

    assert_eq!(commands.len(), 1);
}

#[test]
fn damage_rules_respect_priority_ordering() {
    let mut game_state = create_test_game_state();
    let victim = spawn_test_entity(&mut game_state);
    let attacker = spawn_test_entity(&mut game_state);

    let low = create_rule_with_priority("low", RuleTrigger::OnDamaged { entity: None }, 0);
    let high = create_rule_with_priority("high", RuleTrigger::OnDamaged { entity: None }, 10);

    game_state.set_rules(RuleSet {
        rules: vec![low, high],
    });

    let event = DamageEvent {
        victim,
        attacker: Some(attacker),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_damage(&event, &mut commands);

    assert_eq!(commands.len(), 2);
    match &commands[0] {
        RuleCommand::PlaySound { sound_id, .. } => assert_eq!(sound_id, "high_sound"),
        _ => panic!("Expected PlaySound command"),
    }
}

#[test]
fn damage_rules_filter_by_entity() {
    let mut game_state = create_test_game_state();
    let victim = spawn_test_entity(&mut game_state);
    let other = spawn_test_entity(&mut game_state);

    let filtered = create_test_rule(
        "filtered",
        RuleTrigger::OnDamaged {
            entity: Some(RuleTarget::Entity(victim)),
        },
    );
    game_state.set_rules(RuleSet {
        rules: vec![filtered],
    });

    // Damage to victim should fire
    let event = DamageEvent {
        victim,
        attacker: Some(other),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_damage(&event, &mut commands);
    assert_eq!(commands.len(), 1);

    // Damage to other entity should NOT fire
    let event = DamageEvent {
        victim: other,
        attacker: Some(victim),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_damage(&event, &mut commands);
    assert!(commands.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// DEATH RULE TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn death_rules_fire_for_matching_trigger() {
    let mut game_state = create_test_game_state();
    let victim = spawn_test_entity(&mut game_state);
    let attacker = spawn_test_entity(&mut game_state);

    let rule = create_test_rule("death_rule", RuleTrigger::OnDeath { entity: None });
    game_state.set_rules(RuleSet { rules: vec![rule] });

    let event = DeathEvent {
        victim,
        attacker: Some(attacker),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_death(&event, &mut commands);

    assert_eq!(commands.len(), 1);
}

#[test]
fn death_rules_respect_priority_ordering() {
    let mut game_state = create_test_game_state();
    let victim = spawn_test_entity(&mut game_state);

    let low = create_rule_with_priority("low", RuleTrigger::OnDeath { entity: None }, 0);
    let high = create_rule_with_priority("high", RuleTrigger::OnDeath { entity: None }, 10);

    game_state.set_rules(RuleSet {
        rules: vec![low, high],
    });

    let event = DeathEvent {
        victim,
        attacker: None,
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_death(&event, &mut commands);

    assert_eq!(commands.len(), 2);
    match &commands[0] {
        RuleCommand::PlaySound { sound_id, .. } => assert_eq!(sound_id, "high_sound"),
        _ => panic!("Expected PlaySound command"),
    }
}

#[test]
fn death_rules_filter_by_entity() {
    let mut game_state = create_test_game_state();
    let victim = spawn_test_entity(&mut game_state);
    let other = spawn_test_entity(&mut game_state);

    let filtered = create_test_rule(
        "filtered",
        RuleTrigger::OnDeath {
            entity: Some(RuleTarget::Entity(victim)),
        },
    );
    game_state.set_rules(RuleSet {
        rules: vec![filtered],
    });

    // Death of victim should fire
    let event = DeathEvent {
        victim,
        attacker: None,
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_death(&event, &mut commands);
    assert_eq!(commands.len(), 1);

    // Death of other entity should NOT fire
    let event = DeathEvent {
        victim: other,
        attacker: None,
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_death(&event, &mut commands);
    assert!(commands.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// INTERACTION RULE TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn interaction_rules_fire_for_matching_trigger() {
    let mut game_state = create_test_game_state();
    let interactor = spawn_test_entity(&mut game_state);
    let interactable = spawn_test_entity(&mut game_state);

    let rule = create_test_rule(
        "interact_rule",
        RuleTrigger::OnInteract {
            mode: InteractionMode::Adjacent,
            entity: None,
        },
    );
    game_state.set_rules(RuleSet { rules: vec![rule] });

    let event = InteractionEvent {
        interactor,
        interactable,
        spatial: InteractionSpatial::Adjacent,
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_interaction(&event, &mut commands);

    assert_eq!(commands.len(), 1);
}

#[test]
fn interaction_rules_filter_by_mode() {
    let mut game_state = create_test_game_state();
    let interactor = spawn_test_entity(&mut game_state);
    let interactable = spawn_test_entity(&mut game_state);

    // Rule requiring overlap
    let overlap_rule = create_test_rule(
        "overlap_rule",
        RuleTrigger::OnInteract {
            mode: InteractionMode::Overlap,
            entity: None,
        },
    );
    game_state.set_rules(RuleSet {
        rules: vec![overlap_rule],
    });

    // Adjacent interaction should NOT fire overlap rule
    let event = InteractionEvent {
        interactor,
        interactable,
        spatial: InteractionSpatial::Adjacent,
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_interaction(&event, &mut commands);
    assert!(commands.is_empty());

    // Overlap interaction SHOULD fire overlap rule
    let event = InteractionEvent {
        interactor,
        interactable,
        spatial: InteractionSpatial::Overlap,
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_interaction(&event, &mut commands);
    assert_eq!(commands.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTEXT TESTS - Verify TriggerContext is properly passed
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn collision_provides_trigger_context() {
    let mut game_state = create_test_game_state();
    let entity_a = spawn_test_entity(&mut game_state);
    let entity_b = spawn_test_entity(&mut game_state);
    game_state.set_player_id(entity_b);

    // Rule with TriggerOtherIsPlayer condition
    let rule = Rule {
        id: "context_test".to_string(),
        enabled: true,
        priority: 0,
        once: false,
        trigger: RuleTrigger::OnCollision { entity: None },
        conditions: vec![RuleCondition::TriggerOtherIsPlayer],
        actions: vec![RuleAction::PlaySound {
            channel: RuleSoundChannel::Collision,
            sound_id: "context_sound".to_string(),
        }],
    };
    game_state.set_rules(RuleSet { rules: vec![rule] });

    // Collision where entity_b (player) is the "other" - should fire
    let event = CollisionEvent {
        entity_a,
        entity_b: Some(entity_b),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_collision(&event, &mut commands);
    assert_eq!(commands.len(), 1);

    // Collision where entity_a is the "other" (not player) - should NOT fire
    let event = CollisionEvent {
        entity_a: entity_b,
        entity_b: Some(entity_a),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_collision(&event, &mut commands);
    assert!(commands.is_empty());
}

#[test]
fn damage_provides_trigger_context() {
    let mut game_state = create_test_game_state();
    let victim = spawn_test_entity(&mut game_state);
    let attacker = spawn_test_entity(&mut game_state);
    game_state.set_player_id(attacker);

    // Rule with TriggerOtherIsPlayer condition (attacker is player)
    let rule = Rule {
        id: "damage_context".to_string(),
        enabled: true,
        priority: 0,
        once: false,
        trigger: RuleTrigger::OnDamaged { entity: None },
        conditions: vec![RuleCondition::TriggerOtherIsPlayer],
        actions: vec![RuleAction::PlaySound {
            channel: RuleSoundChannel::Collision,
            sound_id: "player_attacked".to_string(),
        }],
    };
    game_state.set_rules(RuleSet { rules: vec![rule] });

    // Damage where attacker is player - should fire
    let event = DamageEvent {
        victim,
        attacker: Some(attacker),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_damage(&event, &mut commands);
    assert_eq!(commands.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIFIED HELPER TESTS - Tests for the new extracted method
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn unified_collect_matches_collision_behavior() {
    // This test verifies that after refactoring, the behavior is identical
    let mut game_state = create_test_game_state();
    let entity_a = spawn_test_entity(&mut game_state);
    let entity_b = spawn_test_entity(&mut game_state);

    let rules = vec![
        create_rule_with_priority("p10", RuleTrigger::OnCollision { entity: None }, 10),
        create_rule_with_priority("p5", RuleTrigger::OnCollision { entity: None }, 5),
        create_disabled_rule("disabled", RuleTrigger::OnCollision { entity: None }),
        create_once_rule("once", RuleTrigger::OnCollision { entity: None }),
    ];
    game_state.set_rules(RuleSet { rules });

    let event = CollisionEvent {
        entity_a,
        entity_b: Some(entity_b),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_collision(&event, &mut commands);

    // Should have 3 commands: p10, p5, once (disabled is skipped)
    assert_eq!(commands.len(), 3);

    // Verify priority ordering
    match &commands[0] {
        RuleCommand::PlaySound { sound_id, .. } => assert_eq!(sound_id, "p10_sound"),
        _ => panic!("Expected PlaySound"),
    }
    match &commands[1] {
        RuleCommand::PlaySound { sound_id, .. } => assert_eq!(sound_id, "p5_sound"),
        _ => panic!("Expected PlaySound"),
    }
}

#[test]
fn trigger_self_resolves_in_collision_context() {
    let mut game_state = create_test_game_state();
    let entity_a = spawn_test_entity(&mut game_state);
    let entity_b = spawn_test_entity(&mut game_state);

    // Rule that destroys TriggerSelf (entity_a in collision)
    let rule = Rule {
        id: "destroy_self".to_string(),
        enabled: true,
        priority: 0,
        once: false,
        trigger: RuleTrigger::OnCollision { entity: None },
        conditions: vec![],
        actions: vec![RuleAction::DestroySelf {
            target: RuleTarget::TriggerSelf,
        }],
    };
    game_state.set_rules(RuleSet { rules: vec![rule] });

    let event = CollisionEvent {
        entity_a,
        entity_b: Some(entity_b),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_collision(&event, &mut commands);

    assert_eq!(commands.len(), 1);
    match &commands[0] {
        RuleCommand::DestroySelf { entity_id } => assert_eq!(*entity_id, entity_a),
        _ => panic!("Expected DestroySelf command"),
    }
}

#[test]
fn trigger_other_resolves_in_collision_context() {
    let mut game_state = create_test_game_state();
    let entity_a = spawn_test_entity(&mut game_state);
    let entity_b = spawn_test_entity(&mut game_state);

    // Rule that destroys TriggerOther (entity_b in collision)
    let rule = Rule {
        id: "destroy_other".to_string(),
        enabled: true,
        priority: 0,
        once: false,
        trigger: RuleTrigger::OnCollision { entity: None },
        conditions: vec![],
        actions: vec![RuleAction::DestroySelf {
            target: RuleTarget::TriggerOther,
        }],
    };
    game_state.set_rules(RuleSet { rules: vec![rule] });

    let event = CollisionEvent {
        entity_a,
        entity_b: Some(entity_b),
    };
    let mut commands = Vec::new();
    game_state.collect_rule_commands_for_collision(&event, &mut commands);

    assert_eq!(commands.len(), 1);
    match &commands[0] {
        RuleCommand::DestroySelf { entity_id } => assert_eq!(*entity_id, entity_b),
        _ => panic!("Expected DestroySelf command"),
    }
}
