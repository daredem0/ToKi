//! Rule condition evaluation.
//!
//! Contains logic for evaluating rule conditions and filtering matching rules.

use crate::entity::HEALTH_STAT_ID;
use crate::rules::{RuleCondition, TriggerContext};
use tracing::info;

use super::RuleEngine;

impl RuleEngine<'_> {
    pub(super) fn rule_conditions_match(
        &self,
        rule_id: &str,
        log_enabled: bool,
        conditions: &[RuleCondition],
        context: &TriggerContext,
    ) -> bool {
        conditions.iter().all(|condition| {
            let result = self.evaluate_condition(condition, context);
            tracing::trace!(condition = ?condition, result, "Condition evaluated");
            if result && log_enabled {
                info!(rule_id = %rule_id, condition = ?condition, "Rule condition passed");
            }
            result
        })
    }

    pub(super) fn evaluate_condition(
        &self,
        condition: &RuleCondition,
        context: &TriggerContext,
    ) -> bool {
        match condition {
            RuleCondition::Always => true,
            RuleCondition::Expression { expression } => {
                match crate::expression::Expression::parse(expression)
                    .and_then(|expr| expr.evaluate(self.value_path_context(context)))
                {
                    Ok(crate::value_path::ResolvedValue::Bool(value)) => value,
                    Ok(other) => {
                        tracing::warn!(
                            condition = %expression,
                            result = ?other,
                            "Rule condition expression did not resolve to bool"
                        );
                        false
                    }
                    Err(error) => {
                        tracing::warn!(
                            condition = %expression,
                            error = %error,
                            "Failed to evaluate rule condition expression"
                        );
                        false
                    }
                }
            }
            RuleCondition::TargetExists { target } => {
                self.resolve_entity(*target, context).is_some()
            }
            RuleCondition::KeyHeld { key } => self
                .held_keys
                .contains(&crate::game::GameState::to_input_key(*key)),
            RuleCondition::EntityActive { target, is_active } => self
                .resolve_entity(*target, context)
                .is_some_and(|entity| entity.active == *is_active),
            RuleCondition::HealthBelow { target, threshold } => self
                .resolve_entity(*target, context)
                .and_then(|entity| self.entity_manager.combat(entity.id))
                .and_then(|combat| combat.current_stat(HEALTH_STAT_ID))
                .is_some_and(|health| health < *threshold),
            RuleCondition::HealthAbove { target, threshold } => self
                .resolve_entity(*target, context)
                .and_then(|entity| self.entity_manager.combat(entity.id))
                .and_then(|combat| combat.current_stat(HEALTH_STAT_ID))
                .is_some_and(|health| health > *threshold),
            RuleCondition::TriggerOtherIsPlayer => context
                .trigger_other
                .is_some_and(|other_id| self.player_id == Some(other_id)),
            RuleCondition::EntityIsKind { target, kind } => self
                .resolve_entity(*target, context)
                .is_some_and(|entity| entity.entity_kind == *kind),
            RuleCondition::TriggerOtherIsKind { kind } => context
                .trigger_other
                .and_then(|other_id| self.entity_manager.get_entity(other_id))
                .is_some_and(|entity| entity.entity_kind == *kind),
            RuleCondition::EntityHasTag { target, tag } => self
                .resolve_entity(*target, context)
                .is_some_and(|entity| entity.tags.contains(tag)),
            RuleCondition::TriggerOtherHasTag { tag } => context
                .trigger_other
                .and_then(|other_id| self.entity_manager.get_entity(other_id))
                .is_some_and(|entity| entity.tags.contains(tag)),
            RuleCondition::HasInventoryItem {
                target,
                item_id,
                min_count,
            } => self.resolve_entity(*target, context).is_some_and(|entity| {
                self.entity_manager
                    .storage()
                    .components()
                    .inventory(entity.id)
                    .is_some_and(|inventory| inventory.item_count(item_id) >= *min_count)
            }),
            RuleCondition::FlagEquals { flag, value } => self.game_flags.get(flag) == Some(value),
            RuleCondition::FlagSet { flag } => self.game_flags.is_set(flag),
            RuleCondition::FlagGreaterThan { flag, value } => self
                .game_flags
                .get(flag)
                .and_then(|flag_value| flag_value.as_int())
                .is_some_and(|flag_value| flag_value > *value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::engine::{RuleEngine, RuleEngineContext};
    use crate::entity::{
        CombatComponent, EntityId, EntityKind, EntityManager, EntityRendering, EntityStats,
        Inventory, OptionalEntityComponents, HEALTH_STAT_ID,
    };
    use crate::flags::{FlagValue, GameFlags};
    use crate::game::{InputKey, RuleRuntimeState};
    use crate::rules::{RuleCondition, RuleKey, RuleSet, RuleTarget, TriggerContext};
    use glam::{IVec2, UVec2};

    fn make_engine<'a>(
        entity_manager: &'a EntityManager,
        player_id: Option<EntityId>,
        held_keys: &'a [InputKey],
        game_flags: &'a GameFlags,
        rules: &'a RuleSet,
        runtime: &'a mut RuleRuntimeState,
    ) -> RuleEngine<'a> {
        RuleEngine::new(
            RuleEngineContext {
                entity_manager,
                player_id,
                held_keys,
                game_flags,
                rules,
            },
            runtime,
        )
    }

    fn spawn_npc(manager: &mut EntityManager) -> EntityId {
        manager.spawn_entity(
            EntityKind::Npc,
            IVec2::ZERO,
            UVec2::new(16, 16),
            EntityRendering::default(),
            false,
            true,
            OptionalEntityComponents::default(),
        )
    }

    fn spawn_with_health(manager: &mut EntityManager, current: i32) -> EntityId {
        let stats = {
            let mut s = EntityStats::default();
            s.base.insert(HEALTH_STAT_ID.to_string(), 100);
            s.current.insert(HEALTH_STAT_ID.to_string(), current);
            s
        };
        manager.spawn_entity(
            EntityKind::Npc,
            IVec2::ZERO,
            UVec2::new(16, 16),
            EntityRendering::default(),
            false,
            true,
            OptionalEntityComponents {
                combat: Some(CombatComponent {
                    health: None,
                    stats,
                }),
                ..Default::default()
            },
        )
    }

    // --- Always ---

    #[test]
    fn always_returns_true() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        assert!(engine.evaluate_condition(&RuleCondition::Always, &TriggerContext::empty()));
    }

    // --- Expression ---

    #[test]
    fn expression_true_when_bool_result_is_true() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::Expression {
            expression: "1 == 1".to_string(),
        };
        assert!(engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn expression_false_when_bool_result_is_false() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::Expression {
            expression: "1 == 2".to_string(),
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn expression_false_when_result_is_not_bool() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        // Arithmetic expression evaluates to Int, not Bool — must not panic.
        let cond = RuleCondition::Expression {
            expression: "1 + 1".to_string(),
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn expression_false_on_parse_error() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::Expression {
            expression: "@@invalid@@".to_string(),
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    // --- TargetExists ---

    #[test]
    fn target_exists_false_when_no_player() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::TargetExists {
            target: RuleTarget::Player,
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn target_exists_true_when_entity_in_manager() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager);
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::TargetExists {
            target: RuleTarget::Entity(id),
        };
        assert!(engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    // --- KeyHeld ---

    #[test]
    fn key_held_true_when_key_in_held_list() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let held = [InputKey::Up];
        let engine = make_engine(&manager, None, &held, &flags, &rules, &mut runtime);
        let cond = RuleCondition::KeyHeld { key: RuleKey::Up };
        assert!(engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn key_held_false_when_key_not_held() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::KeyHeld { key: RuleKey::Up };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    // --- EntityActive ---

    #[test]
    fn entity_active_true_when_active_matches() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager); // active = true
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::EntityActive {
            target: RuleTarget::Entity(id),
            is_active: true,
        };
        assert!(engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn entity_active_false_when_flag_does_not_match() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager); // active = true
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::EntityActive {
            target: RuleTarget::Entity(id),
            is_active: false,
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn entity_active_false_when_entity_missing() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::EntityActive {
            target: RuleTarget::Entity(99),
            is_active: true,
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    // --- HealthBelow ---

    #[test]
    fn health_below_true_when_strictly_below_threshold() {
        let mut manager = EntityManager::new();
        let id = spawn_with_health(&mut manager, 30);
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::HealthBelow {
            target: RuleTarget::Entity(id),
            threshold: 50,
        };
        assert!(engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn health_below_false_when_equal_to_threshold() {
        let mut manager = EntityManager::new();
        let id = spawn_with_health(&mut manager, 50);
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::HealthBelow {
            target: RuleTarget::Entity(id),
            threshold: 50,
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn health_below_false_when_no_combat_component() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager); // no combat
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::HealthBelow {
            target: RuleTarget::Entity(id),
            threshold: 50,
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    // --- HealthAbove ---

    #[test]
    fn health_above_true_when_strictly_above_threshold() {
        let mut manager = EntityManager::new();
        let id = spawn_with_health(&mut manager, 60);
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::HealthAbove {
            target: RuleTarget::Entity(id),
            threshold: 50,
        };
        assert!(engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn health_above_false_when_equal_to_threshold() {
        let mut manager = EntityManager::new();
        let id = spawn_with_health(&mut manager, 50);
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::HealthAbove {
            target: RuleTarget::Entity(id),
            threshold: 50,
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    // --- TriggerOtherIsPlayer ---

    #[test]
    fn trigger_other_is_player_true_when_other_matches_player_id() {
        let mut manager = EntityManager::new();
        let player_id = spawn_npc(&mut manager);
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, Some(player_id), &[], &flags, &rules, &mut runtime);
        let ctx = TriggerContext {
            trigger_other: Some(player_id),
            ..TriggerContext::empty()
        };
        assert!(engine.evaluate_condition(&RuleCondition::TriggerOtherIsPlayer, &ctx));
    }

    #[test]
    fn trigger_other_is_player_false_when_other_is_different_entity() {
        let mut manager = EntityManager::new();
        let player_id = spawn_npc(&mut manager);
        let other_id = spawn_npc(&mut manager);
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, Some(player_id), &[], &flags, &rules, &mut runtime);
        let ctx = TriggerContext {
            trigger_other: Some(other_id),
            ..TriggerContext::empty()
        };
        assert!(!engine.evaluate_condition(&RuleCondition::TriggerOtherIsPlayer, &ctx));
    }

    #[test]
    fn trigger_other_is_player_false_when_no_trigger_other() {
        let mut manager = EntityManager::new();
        let player_id = spawn_npc(&mut manager);
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, Some(player_id), &[], &flags, &rules, &mut runtime);
        assert!(!engine.evaluate_condition(
            &RuleCondition::TriggerOtherIsPlayer,
            &TriggerContext::empty()
        ));
    }

    // --- EntityIsKind ---

    #[test]
    fn entity_is_kind_true_when_kinds_match() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager);
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::EntityIsKind {
            target: RuleTarget::Entity(id),
            kind: EntityKind::Npc,
        };
        assert!(engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn entity_is_kind_false_when_kinds_differ() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager);
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::EntityIsKind {
            target: RuleTarget::Entity(id),
            kind: EntityKind::Player,
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    // --- TriggerOtherIsKind ---

    #[test]
    fn trigger_other_is_kind_true_when_other_has_kind() {
        let mut manager = EntityManager::new();
        let npc_id = spawn_npc(&mut manager);
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let ctx = TriggerContext {
            trigger_other: Some(npc_id),
            ..TriggerContext::empty()
        };
        let cond = RuleCondition::TriggerOtherIsKind {
            kind: EntityKind::Npc,
        };
        assert!(engine.evaluate_condition(&cond, &ctx));
    }

    #[test]
    fn trigger_other_is_kind_false_when_no_trigger_other() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::TriggerOtherIsKind {
            kind: EntityKind::Npc,
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    // --- EntityHasTag ---

    #[test]
    fn entity_has_tag_true_when_tag_present() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager);
        manager
            .get_entity_mut(id)
            .unwrap()
            .tags
            .push("enemy".to_string());
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::EntityHasTag {
            target: RuleTarget::Entity(id),
            tag: "enemy".to_string(),
        };
        assert!(engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn entity_has_tag_false_when_tag_absent() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager);
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::EntityHasTag {
            target: RuleTarget::Entity(id),
            tag: "enemy".to_string(),
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    // --- TriggerOtherHasTag ---

    #[test]
    fn trigger_other_has_tag_true_when_other_has_tag() {
        let mut manager = EntityManager::new();
        let other_id = spawn_npc(&mut manager);
        manager
            .get_entity_mut(other_id)
            .unwrap()
            .tags
            .push("boss".to_string());
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let ctx = TriggerContext {
            trigger_other: Some(other_id),
            ..TriggerContext::empty()
        };
        let cond = RuleCondition::TriggerOtherHasTag {
            tag: "boss".to_string(),
        };
        assert!(engine.evaluate_condition(&cond, &ctx));
    }

    #[test]
    fn trigger_other_has_tag_false_when_tag_absent() {
        let mut manager = EntityManager::new();
        let other_id = spawn_npc(&mut manager);
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let ctx = TriggerContext {
            trigger_other: Some(other_id),
            ..TriggerContext::empty()
        };
        let cond = RuleCondition::TriggerOtherHasTag {
            tag: "boss".to_string(),
        };
        assert!(!engine.evaluate_condition(&cond, &ctx));
    }

    // --- HasInventoryItem ---

    #[test]
    fn has_inventory_item_true_when_count_meets_minimum() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager);
        let mut inv = Inventory::default();
        inv.add_item("sword", 3);
        manager
            .storage_mut()
            .components_mut()
            .set_inventory(id, Some(inv));
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::HasInventoryItem {
            target: RuleTarget::Entity(id),
            item_id: "sword".to_string(),
            min_count: 2,
        };
        assert!(engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn has_inventory_item_false_when_count_below_minimum() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager);
        let mut inv = Inventory::default();
        inv.add_item("sword", 1);
        manager
            .storage_mut()
            .components_mut()
            .set_inventory(id, Some(inv));
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::HasInventoryItem {
            target: RuleTarget::Entity(id),
            item_id: "sword".to_string(),
            min_count: 2,
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn has_inventory_item_false_when_item_not_in_inventory() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager);
        manager
            .storage_mut()
            .components_mut()
            .set_inventory(id, Some(Inventory::default()));
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::HasInventoryItem {
            target: RuleTarget::Entity(id),
            item_id: "sword".to_string(),
            min_count: 1,
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    // --- FlagEquals ---

    #[test]
    fn flag_equals_true_when_value_matches() {
        let manager = EntityManager::new();
        let mut flags = GameFlags::default();
        flags.set("ready", FlagValue::Bool(true));
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::FlagEquals {
            flag: "ready".to_string(),
            value: FlagValue::Bool(true),
        };
        assert!(engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn flag_equals_false_when_value_differs() {
        let manager = EntityManager::new();
        let mut flags = GameFlags::default();
        flags.set("ready", FlagValue::Bool(false));
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::FlagEquals {
            flag: "ready".to_string(),
            value: FlagValue::Bool(true),
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn flag_equals_false_when_flag_not_set() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::FlagEquals {
            flag: "ready".to_string(),
            value: FlagValue::Bool(true),
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    // --- FlagSet ---

    #[test]
    fn flag_set_true_when_flag_exists() {
        let manager = EntityManager::new();
        let mut flags = GameFlags::default();
        flags.set("active", FlagValue::Bool(false)); // exists even with false value
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::FlagSet {
            flag: "active".to_string(),
        };
        assert!(engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn flag_set_false_when_flag_absent() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::FlagSet {
            flag: "active".to_string(),
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    // --- FlagGreaterThan ---

    #[test]
    fn flag_greater_than_true_when_int_strictly_greater() {
        let manager = EntityManager::new();
        let mut flags = GameFlags::default();
        flags.set("coins", FlagValue::Int(10));
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::FlagGreaterThan {
            flag: "coins".to_string(),
            value: 5,
        };
        assert!(engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn flag_greater_than_false_when_equal_to_threshold() {
        let manager = EntityManager::new();
        let mut flags = GameFlags::default();
        flags.set("coins", FlagValue::Int(5));
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::FlagGreaterThan {
            flag: "coins".to_string(),
            value: 5,
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn flag_greater_than_false_when_flag_is_bool() {
        let manager = EntityManager::new();
        let mut flags = GameFlags::default();
        flags.set("active", FlagValue::Bool(true));
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::FlagGreaterThan {
            flag: "active".to_string(),
            value: 0,
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    #[test]
    fn flag_greater_than_false_when_flag_absent() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let cond = RuleCondition::FlagGreaterThan {
            flag: "coins".to_string(),
            value: 0,
        };
        assert!(!engine.evaluate_condition(&cond, &TriggerContext::empty()));
    }

    // --- rule_conditions_match ---

    #[test]
    fn conditions_match_true_for_empty_list() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        assert!(engine.rule_conditions_match("r", false, &[], &TriggerContext::empty()));
    }

    #[test]
    fn conditions_match_true_when_all_conditions_pass() {
        let manager = EntityManager::new();
        let mut flags = GameFlags::default();
        flags.set("done", FlagValue::Bool(true));
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let conds = vec![
            RuleCondition::Always,
            RuleCondition::FlagSet {
                flag: "done".to_string(),
            },
        ];
        assert!(engine.rule_conditions_match("r", false, &conds, &TriggerContext::empty()));
    }

    #[test]
    fn conditions_match_false_when_any_condition_fails() {
        let manager = EntityManager::new();
        let flags = GameFlags::default();
        let rules = RuleSet::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(&manager, None, &[], &flags, &rules, &mut runtime);
        let conds = vec![
            RuleCondition::Always,
            RuleCondition::FlagSet {
                flag: "missing".to_string(),
            },
        ];
        assert!(!engine.rule_conditions_match("r", false, &conds, &TriggerContext::empty()));
    }
}
