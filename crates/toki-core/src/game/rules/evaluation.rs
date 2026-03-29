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
            RuleCondition::TargetExists { target } => {
                self.resolve_entity(*target, context).is_some()
            }
            RuleCondition::KeyHeld { key } => self
                .held_keys
                .contains(&crate::game::GameState::to_input_key(*key)),
            RuleCondition::EntityActive { target, is_active } => self
                .resolve_entity(*target, context)
                .is_some_and(|entity| entity.attributes.behavior.active == *is_active),
            RuleCondition::HealthBelow { target, threshold } => self
                .resolve_entity(*target, context)
                .and_then(|entity| entity.attributes.gameplay.stats.current(HEALTH_STAT_ID))
                .is_some_and(|health| health < *threshold),
            RuleCondition::HealthAbove { target, threshold } => self
                .resolve_entity(*target, context)
                .and_then(|entity| entity.attributes.gameplay.stats.current(HEALTH_STAT_ID))
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
