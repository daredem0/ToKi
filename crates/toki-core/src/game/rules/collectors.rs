//! Rule command collection.
//!
//! Contains functions for collecting rule commands based on different triggers.

use tracing::debug;

use crate::entity::EntityId;
use crate::rules::{RuleTarget, RuleTrigger, TriggerContext};

use super::events::{
    CollisionEvent, DamageEvent, DeathEvent, InteractionEvent, InteractionSpatial,
};
use super::{GameState, RuleCommand};

impl GameState {
    /// Collects rule commands for a trigger without context.
    pub(in crate::game) fn collect_rule_commands_for_trigger(
        &mut self,
        trigger: RuleTrigger,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.collect_rule_commands_for_trigger_with_context(
            trigger,
            TriggerContext::empty(),
            command_buffer,
        );
    }

    /// Collects rule commands for a trigger with entity context.
    pub(in crate::game) fn collect_rule_commands_for_trigger_with_context(
        &mut self,
        trigger: RuleTrigger,
        context: TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let mut matching_rules: Vec<&crate::rules::Rule> = self
            .rules
            .rules
            .iter()
            .filter(|rule| rule.enabled && rule.trigger == trigger)
            .filter(|rule| {
                !(rule.once
                    && self
                        .rule_runtime
                        .fired_once_rules
                        .contains(rule.id.as_str()))
            })
            .collect();

        matching_rules.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));

        let mut fired_once_ids = Vec::new();
        for rule in matching_rules {
            let conditions_result = self.rule_conditions_match(&rule.conditions, &context);
            debug!(
                rule_id = %rule.id,
                trigger = ?trigger,
                conditions_passed = conditions_result,
                "Rule evaluated"
            );

            if !conditions_result {
                continue;
            }

            for action in &rule.actions {
                debug!(rule_id = %rule.id, action = ?action, "Executing action");
                self.buffer_rule_action(action, &context, command_buffer);
            }

            if rule.once {
                fired_once_ids.push(rule.id.clone());
            }
        }
        self.rule_runtime.fired_once_rules.extend(fired_once_ids);
    }

    /// Collects rule commands for OnInteract triggers.
    pub(in crate::game) fn collect_rule_commands_for_interaction(
        &mut self,
        event: &InteractionEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let context = TriggerContext::with_pair(event.interactor, event.interactable);

        // Collect matching rule indices to avoid borrow conflicts
        let matching_indices: Vec<usize> = self
            .rules
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| {
                rule.enabled
                    && matches!(rule.trigger, RuleTrigger::OnInteract { .. })
                    && !(rule.once
                        && self
                            .rule_runtime
                            .fired_once_rules
                            .contains(rule.id.as_str()))
            })
            .filter(|(_, rule)| {
                let mode = rule.trigger.interaction_mode().unwrap_or_default();
                Self::interaction_mode_matches(mode, event.spatial)
                    && self.entity_filter_matches(
                        rule.trigger.interact_entity_filter(),
                        event.interactable,
                        &context,
                    )
            })
            .map(|(i, _)| i)
            .collect();

        // Sort by priority then id
        let mut sorted_indices = matching_indices;
        sorted_indices.sort_by(|&a, &b| {
            let rule_a = &self.rules.rules[a];
            let rule_b = &self.rules.rules[b];
            rule_b
                .priority
                .cmp(&rule_a.priority)
                .then_with(|| rule_a.id.cmp(&rule_b.id))
        });

        let mut fired_once_ids = Vec::new();
        for idx in sorted_indices {
            let rule = &self.rules.rules[idx];
            let conditions_result = self.rule_conditions_match(&rule.conditions, &context);
            debug!(
                rule_id = %rule.id,
                trigger = ?rule.trigger,
                interactor = ?event.interactor,
                interactable = ?event.interactable,
                spatial = ?event.spatial,
                conditions_passed = conditions_result,
                "Interaction rule evaluated"
            );

            if !conditions_result {
                continue;
            }

            // Clone actions to avoid borrow conflict
            let actions = rule.actions.clone();
            let rule_id = rule.id.clone();
            let rule_once = rule.once;

            for action in &actions {
                debug!(rule_id = %rule_id, action = ?action, "Executing action");
                self.buffer_rule_action(action, &context, command_buffer);
            }

            if rule_once {
                fired_once_ids.push(rule_id);
            }
        }
        self.rule_runtime.fired_once_rules.extend(fired_once_ids);
    }

    /// Checks if an interaction mode matches a spatial relationship.
    fn interaction_mode_matches(
        mode: crate::rules::InteractionMode,
        spatial: InteractionSpatial,
    ) -> bool {
        use crate::rules::InteractionMode;

        match mode {
            InteractionMode::Overlap => matches!(spatial, InteractionSpatial::Overlap),
            InteractionMode::Adjacent => true,
            InteractionMode::InFront => matches!(
                spatial,
                InteractionSpatial::InFront | InteractionSpatial::Overlap
            ),
        }
    }

    /// Collects rule commands for OnCollision triggers.
    pub(in crate::game) fn collect_rule_commands_for_collision(
        &mut self,
        event: &CollisionEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let context = if let Some(entity_b) = event.entity_b {
            TriggerContext::with_pair(event.entity_a, entity_b)
        } else {
            TriggerContext::with_self_only(event.entity_a)
        };

        let matching_indices: Vec<usize> = self
            .rules
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| {
                rule.enabled
                    && matches!(rule.trigger, RuleTrigger::OnCollision { .. })
                    && !(rule.once
                        && self
                            .rule_runtime
                            .fired_once_rules
                            .contains(rule.id.as_str()))
                    && self.entity_filter_matches(
                        rule.trigger.collision_entity_filter(),
                        event.entity_a,
                        &context,
                    )
            })
            .map(|(i, _)| i)
            .collect();

        let mut sorted_indices = matching_indices;
        sorted_indices.sort_by(|&a, &b| {
            let rule_a = &self.rules.rules[a];
            let rule_b = &self.rules.rules[b];
            rule_b
                .priority
                .cmp(&rule_a.priority)
                .then_with(|| rule_a.id.cmp(&rule_b.id))
        });

        let mut fired_once_ids = Vec::new();
        for idx in sorted_indices {
            let rule = &self.rules.rules[idx];
            let conditions_result = self.rule_conditions_match(&rule.conditions, &context);

            if event.entity_b.is_some() {
                debug!(
                    rule_id = %rule.id,
                    trigger = ?rule.trigger,
                    entity_a = ?event.entity_a,
                    entity_b = ?event.entity_b,
                    conditions_passed = conditions_result,
                    "Collision rule evaluated"
                );
            } else {
                tracing::trace!(
                    rule_id = %rule.id,
                    trigger = ?rule.trigger,
                    entity_a = ?event.entity_a,
                    conditions_passed = conditions_result,
                    "Wall collision rule evaluated"
                );
            }

            if !conditions_result {
                continue;
            }

            let actions = rule.actions.clone();
            let rule_id = rule.id.clone();
            let rule_once = rule.once;

            for action in &actions {
                if event.entity_b.is_some() {
                    debug!(rule_id = %rule_id, action = ?action, "Executing action");
                } else {
                    tracing::trace!(rule_id = %rule_id, action = ?action, "Executing wall collision action");
                }
                self.buffer_rule_action(action, &context, command_buffer);
            }

            if rule_once {
                fired_once_ids.push(rule_id);
            }
        }
        self.rule_runtime.fired_once_rules.extend(fired_once_ids);
    }

    /// Collects rule commands for OnDamaged triggers.
    pub(in crate::game) fn collect_rule_commands_for_damage(
        &mut self,
        event: &DamageEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let context = if let Some(attacker) = event.attacker {
            TriggerContext::with_pair(event.victim, attacker)
        } else {
            TriggerContext::with_self_only(event.victim)
        };

        let matching_indices: Vec<usize> = self
            .rules
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| {
                rule.enabled
                    && matches!(rule.trigger, RuleTrigger::OnDamaged { .. })
                    && !(rule.once
                        && self
                            .rule_runtime
                            .fired_once_rules
                            .contains(rule.id.as_str()))
                    && self.entity_filter_matches(
                        rule.trigger.damaged_entity_filter(),
                        event.victim,
                        &context,
                    )
            })
            .map(|(i, _)| i)
            .collect();

        let mut sorted_indices = matching_indices;
        sorted_indices.sort_by(|&a, &b| {
            let rule_a = &self.rules.rules[a];
            let rule_b = &self.rules.rules[b];
            rule_b
                .priority
                .cmp(&rule_a.priority)
                .then_with(|| rule_a.id.cmp(&rule_b.id))
        });

        let mut fired_once_ids = Vec::new();
        for idx in sorted_indices {
            let rule = &self.rules.rules[idx];
            let conditions_result = self.rule_conditions_match(&rule.conditions, &context);
            debug!(
                rule_id = %rule.id,
                trigger = ?rule.trigger,
                victim = ?event.victim,
                attacker = ?event.attacker,
                conditions_passed = conditions_result,
                "Damage rule evaluated"
            );

            if !conditions_result {
                continue;
            }

            let actions = rule.actions.clone();
            let rule_id = rule.id.clone();
            let rule_once = rule.once;

            for action in &actions {
                debug!(rule_id = %rule_id, action = ?action, "Executing action");
                self.buffer_rule_action(action, &context, command_buffer);
            }

            if rule_once {
                fired_once_ids.push(rule_id);
            }
        }
        self.rule_runtime.fired_once_rules.extend(fired_once_ids);
    }

    /// Checks if an entity filter matches a target entity.
    fn entity_filter_matches(
        &self,
        filter: Option<RuleTarget>,
        event_entity: EntityId,
        context: &TriggerContext,
    ) -> bool {
        match filter {
            None => true,
            Some(target) => self
                .resolve_rule_target(target, context)
                .is_some_and(|id| id == event_entity),
        }
    }

    /// Collects rule commands for OnDeath triggers.
    pub(in crate::game) fn collect_rule_commands_for_death(
        &mut self,
        event: &DeathEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let context = if let Some(attacker) = event.attacker {
            TriggerContext::with_pair(event.victim, attacker)
        } else {
            TriggerContext::with_self_only(event.victim)
        };

        let matching_indices: Vec<usize> = self
            .rules
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| {
                rule.enabled
                    && matches!(rule.trigger, RuleTrigger::OnDeath { .. })
                    && !(rule.once
                        && self
                            .rule_runtime
                            .fired_once_rules
                            .contains(rule.id.as_str()))
                    && self.entity_filter_matches(
                        rule.trigger.death_entity_filter(),
                        event.victim,
                        &context,
                    )
            })
            .map(|(i, _)| i)
            .collect();

        let mut sorted_indices = matching_indices;
        sorted_indices.sort_by(|&a, &b| {
            let rule_a = &self.rules.rules[a];
            let rule_b = &self.rules.rules[b];
            rule_b
                .priority
                .cmp(&rule_a.priority)
                .then_with(|| rule_a.id.cmp(&rule_b.id))
        });

        let mut fired_once_ids = Vec::new();
        for idx in sorted_indices {
            let rule = &self.rules.rules[idx];
            let conditions_result = self.rule_conditions_match(&rule.conditions, &context);
            tracing::info!(
                rule_id = %rule.id,
                trigger = ?rule.trigger,
                victim = ?event.victim,
                attacker = ?event.attacker,
                conditions_passed = conditions_result,
                "Death rule evaluated"
            );

            if !conditions_result {
                continue;
            }

            let actions = rule.actions.clone();
            let rule_id = rule.id.clone();
            let rule_once = rule.once;

            for action in &actions {
                tracing::info!(rule_id = %rule_id, action = ?action, "Executing death action");
                self.buffer_rule_action(action, &context, command_buffer);
            }

            if rule_once {
                fired_once_ids.push(rule_id);
            }
        }
        self.rule_runtime.fired_once_rules.extend(fired_once_ids);
    }

    pub(in crate::game) fn collect_rule_commands_for_key_triggers(
        &mut self,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let held_keys = self.all_held_keys();

        for input_key in held_keys {
            let trigger = RuleTrigger::OnKey {
                key: Self::to_rule_key(input_key),
            };
            self.collect_rule_commands_for_trigger(trigger, command_buffer);
        }
    }
}
