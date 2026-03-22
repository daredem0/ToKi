//! Rule condition evaluation.
//!
//! Contains logic for evaluating rule conditions and filtering matching rules.

use crate::entity::HEALTH_STAT_ID;
use crate::rules::{RuleCondition, TriggerContext};

use super::GameState;

impl GameState {
    pub(super) fn rule_conditions_match(
        &self,
        conditions: &[RuleCondition],
        context: &TriggerContext,
    ) -> bool {
        conditions.iter().all(|condition| {
            let result = self.evaluate_condition(condition, context);
            tracing::trace!(condition = ?condition, result, "Condition evaluated");
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
            RuleCondition::TargetExists { target } => self
                .resolve_rule_target(*target, context)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .is_some(),
            RuleCondition::KeyHeld { key } => {
                self.all_held_keys().contains(&Self::to_input_key(*key))
            }
            RuleCondition::EntityActive { target, is_active } => self
                .resolve_rule_target(*target, context)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .is_some_and(|entity| entity.attributes.active == *is_active),
            RuleCondition::HealthBelow { target, threshold } => self
                .resolve_rule_target(*target, context)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .and_then(|entity| entity.attributes.stats.current(HEALTH_STAT_ID))
                .is_some_and(|health| health < *threshold),
            RuleCondition::HealthAbove { target, threshold } => self
                .resolve_rule_target(*target, context)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .and_then(|entity| entity.attributes.stats.current(HEALTH_STAT_ID))
                .is_some_and(|health| health > *threshold),
            RuleCondition::TriggerOtherIsPlayer => context
                .trigger_other
                .is_some_and(|other_id| self.player_id == Some(other_id)),
            RuleCondition::EntityIsKind { target, kind } => self
                .resolve_rule_target(*target, context)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .is_some_and(|entity| entity.entity_kind == *kind),
            RuleCondition::TriggerOtherIsKind { kind } => context
                .trigger_other
                .and_then(|other_id| self.entity_manager.get_entity(other_id))
                .is_some_and(|entity| entity.entity_kind == *kind),
            RuleCondition::EntityHasTag { target, tag } => self
                .resolve_rule_target(*target, context)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .is_some_and(|entity| entity.tags.contains(tag)),
            RuleCondition::TriggerOtherHasTag { tag } => context
                .trigger_other
                .and_then(|other_id| self.entity_manager.get_entity(other_id))
                .is_some_and(|entity| entity.tags.contains(tag)),
            RuleCondition::HasInventoryItem {
                target,
                item_id,
                min_count,
            } => self
                .resolve_rule_target(*target, context)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .is_some_and(|entity| {
                    entity.attributes.inventory.item_count(item_id) >= *min_count
                }),
        }
    }
}
