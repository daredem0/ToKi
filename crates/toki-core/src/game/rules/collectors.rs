//! Rule command collection.
//!
//! Contains functions for collecting rule commands based on different triggers.

use tracing::{debug, info};

use crate::entity::EntityId;
use crate::rules::{RuleAction, RuleTarget, RuleTrigger, TriggerContext};

use super::events::{
    CollisionEvent, DamageEvent, DeathEvent, DialogCompletionEvent, InteractionEvent,
    InteractionSpatial,
};
#[cfg(test)]
use super::GameState;
use super::{RuleCommand, RuleEngine, RuleEvaluationService};

impl RuleEngine<'_> {
    pub(in crate::game::rules) fn rule_is_collectible(&self, rule: &crate::rules::Rule) -> bool {
        rule.enabled
            && !(rule.once
                && self
                    .rule_runtime
                    .fired_once_rules
                    .contains(rule.id.as_str()))
    }

    pub(in crate::game::rules) fn sort_rule_indices(&self, indices: &mut [usize]) {
        indices.sort_by(|&a, &b| {
            let rule_a = &self.rules.rules[a];
            let rule_b = &self.rules.rules[b];
            rule_b
                .priority
                .cmp(&rule_a.priority)
                .then_with(|| rule_a.id.cmp(&rule_b.id))
        });
    }

    pub(in crate::game::rules) fn execute_sorted_rule_indices<LogRule, LogAction>(
        &mut self,
        sorted_indices: &[usize],
        context: &TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
        mut log_rule: LogRule,
        mut log_action: LogAction,
    ) where
        LogRule: FnMut(&crate::rules::Rule, bool),
        LogAction: FnMut(&str, &RuleAction),
    {
        let mut fired_once_ids = Vec::new();

        for &idx in sorted_indices {
            // Extract owned data in a block to release the borrow on self.rules
            // before calling self.buffer_rule_action below.
            let Some((actions, rule_id, rule_once, log_enabled)) = ({
                let rule = &self.rules.rules[idx];
                if rule.log_enabled {
                    info!(rule_id = %rule.id, trigger = ?rule.trigger, "Rule trigger passed");
                }
                let conditions_result = self.rule_conditions_match(
                    &rule.id,
                    rule.log_enabled,
                    &rule.conditions,
                    context,
                );
                log_rule(rule, conditions_result);
                if !conditions_result {
                    None
                } else {
                    Some((
                        rule.actions.clone(),
                        rule.id.clone(),
                        rule.once,
                        rule.log_enabled,
                    ))
                }
            }) else {
                continue;
            };

            for action in &actions {
                log_action(&rule_id, action);
                self.buffer_rule_action(&rule_id, log_enabled, action, context, command_buffer);
            }

            if rule_once {
                fired_once_ids.push(rule_id);
            }
        }

        self.rule_runtime.fired_once_rules.extend(fired_once_ids);
    }

    fn collect_rules_for_event<MatchRule, LogRule, LogAction>(
        &mut self,
        context: TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
        mut matches_rule: MatchRule,
        log_rule: LogRule,
        log_action: LogAction,
    ) where
        MatchRule: FnMut(&Self, &crate::rules::Rule, &TriggerContext) -> bool,
        LogRule: FnMut(&crate::rules::Rule, bool),
        LogAction: FnMut(&str, &RuleAction),
    {
        let mut sorted_indices = self
            .rules
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| {
                self.rule_is_collectible(rule) && matches_rule(self, rule, &context)
            })
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();
        self.sort_rule_indices(&mut sorted_indices);
        self.execute_sorted_rule_indices(
            &sorted_indices,
            &context,
            command_buffer,
            log_rule,
            log_action,
        );
    }

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
        self.collect_rules_for_event(
            context,
            command_buffer,
            |_, rule, _| rule.trigger == trigger,
            |rule, conditions_result| {
                debug!(
                    rule_id = %rule.id,
                    trigger = ?trigger,
                    conditions_passed = conditions_result,
                    "Rule evaluated"
                );
            },
            |rule_id, action| {
                debug!(rule_id = %rule_id, action = ?action, "Executing action");
            },
        );
    }

    /// Collects rule commands for OnInteract triggers.
    pub(in crate::game) fn collect_rule_commands_for_interaction(
        &mut self,
        event: &InteractionEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let context = TriggerContext::with_pair(event.interactor, event.interactable);

        self.collect_rules_for_event(
            context,
            command_buffer,
            |state, rule, context| {
                if !matches!(rule.trigger, RuleTrigger::OnInteract { .. }) {
                    return false;
                }
                let mode = rule.trigger.interaction_mode().unwrap_or_default();
                Self::interaction_mode_matches(mode, event.spatial)
                    && state.entity_filter_matches(
                        rule.trigger.interact_entity_filter(),
                        event.interactable,
                        context,
                    )
            },
            |rule, conditions_result| {
                debug!(
                    rule_id = %rule.id,
                    trigger = ?rule.trigger,
                    interactor = ?event.interactor,
                    interactable = ?event.interactable,
                    spatial = ?event.spatial,
                    conditions_passed = conditions_result,
                    "Interaction rule evaluated"
                );
            },
            |rule_id, action| {
                debug!(rule_id = %rule_id, action = ?action, "Executing action");
            },
        );
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

    pub(in crate::game) fn collect_rule_commands_for_dialog_completion(
        &mut self,
        event: &DialogCompletionEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.collect_rule_commands_for_trigger(
            RuleTrigger::OnDialogComplete {
                dialog_id: event.dialog_id.clone(),
                outcome_id: event.outcome_id.clone(),
            },
            command_buffer,
        );
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

        self.collect_rules_for_event(
            context,
            command_buffer,
            |state, rule, context| {
                matches!(rule.trigger, RuleTrigger::OnCollision { .. })
                    && state.entity_filter_matches(
                        rule.trigger.collision_entity_filter(),
                        event.entity_a,
                        context,
                    )
            },
            |rule, conditions_result| {
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
            },
            |rule_id, action| {
                if event.entity_b.is_some() {
                    debug!(rule_id = %rule_id, action = ?action, "Executing action");
                } else {
                    tracing::trace!(rule_id = %rule_id, action = ?action, "Executing wall collision action");
                }
            },
        );
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

        self.collect_rules_for_event(
            context,
            command_buffer,
            |state, rule, context| {
                matches!(rule.trigger, RuleTrigger::OnDamaged { .. })
                    && state.entity_filter_matches(
                        rule.trigger.damaged_entity_filter(),
                        event.victim,
                        context,
                    )
            },
            |rule, conditions_result| {
                debug!(
                    rule_id = %rule.id,
                    trigger = ?rule.trigger,
                    victim = ?event.victim,
                    attacker = ?event.attacker,
                    conditions_passed = conditions_result,
                    "Damage rule evaluated"
                );
            },
            |rule_id, action| {
                debug!(rule_id = %rule_id, action = ?action, "Executing action");
            },
        );
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

        self.collect_rules_for_event(
            context,
            command_buffer,
            |state, rule, context| {
                matches!(rule.trigger, RuleTrigger::OnDeath { .. })
                    && state.entity_filter_matches(
                        rule.trigger.death_entity_filter(),
                        event.victim,
                        context,
                    )
            },
            |rule, conditions_result| {
                tracing::info!(
                    rule_id = %rule.id,
                    trigger = ?rule.trigger,
                    victim = ?event.victim,
                    attacker = ?event.attacker,
                    conditions_passed = conditions_result,
                    "Death rule evaluated"
                );
            },
            |rule_id, action| {
                tracing::info!(rule_id = %rule_id, action = ?action, "Executing death action");
            },
        );
    }

    pub(in crate::game) fn collect_rule_commands_for_key_triggers(
        &mut self,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        for &input_key in self.held_keys {
            let trigger = RuleTrigger::OnKey {
                key: crate::game::GameState::to_rule_key(input_key),
            };
            self.collect_rule_commands_for_trigger(trigger, command_buffer);
        }
    }
}

impl RuleEvaluationService<'_> {
    pub(in crate::game) fn collect_rule_commands_for_trigger(
        &mut self,
        trigger: RuleTrigger,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.with_rule_engine(|engine| {
            engine.collect_rule_commands_for_trigger(trigger, command_buffer);
        });
    }

    pub(in crate::game) fn collect_rule_commands_for_interaction(
        &mut self,
        event: &InteractionEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.with_rule_engine(|engine| {
            engine.collect_rule_commands_for_interaction(event, command_buffer);
        });
    }

    pub(in crate::game) fn collect_rule_commands_for_dialog_completion(
        &mut self,
        event: &DialogCompletionEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.with_rule_engine(|engine| {
            engine.collect_rule_commands_for_dialog_completion(event, command_buffer);
        });
    }

    pub(in crate::game) fn collect_rule_commands_for_collision(
        &mut self,
        event: &CollisionEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.with_rule_engine(|engine| {
            engine.collect_rule_commands_for_collision(event, command_buffer);
        });
    }

    pub(in crate::game) fn collect_rule_commands_for_damage(
        &mut self,
        event: &DamageEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.with_rule_engine(|engine| {
            engine.collect_rule_commands_for_damage(event, command_buffer);
        });
    }

    pub(in crate::game) fn collect_rule_commands_for_death(
        &mut self,
        event: &DeathEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.with_rule_engine(|engine| {
            engine.collect_rule_commands_for_death(event, command_buffer);
        });
    }

    pub(in crate::game) fn collect_rule_commands_for_key_triggers(
        &mut self,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.with_rule_engine(|engine| {
            engine.collect_rule_commands_for_key_triggers(command_buffer);
        });
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl GameState {
    pub(in crate::game) fn collect_rule_commands_for_trigger(
        &mut self,
        trigger: RuleTrigger,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.rule_evaluation_service()
            .collect_rule_commands_for_trigger(trigger, command_buffer);
    }

    pub(in crate::game) fn collect_rule_commands_for_interaction(
        &mut self,
        event: &InteractionEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.rule_evaluation_service()
            .collect_rule_commands_for_interaction(event, command_buffer);
    }

    pub(in crate::game) fn collect_rule_commands_for_dialog_completion(
        &mut self,
        event: &DialogCompletionEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.rule_evaluation_service()
            .collect_rule_commands_for_dialog_completion(event, command_buffer);
    }

    pub(in crate::game) fn collect_rule_commands_for_collision(
        &mut self,
        event: &CollisionEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.rule_evaluation_service()
            .collect_rule_commands_for_collision(event, command_buffer);
    }

    pub(in crate::game) fn collect_rule_commands_for_damage(
        &mut self,
        event: &DamageEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.rule_evaluation_service()
            .collect_rule_commands_for_damage(event, command_buffer);
    }

    pub(in crate::game) fn collect_rule_commands_for_death(
        &mut self,
        event: &DeathEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.rule_evaluation_service()
            .collect_rule_commands_for_death(event, command_buffer);
    }

    pub(in crate::game) fn collect_rule_commands_for_key_triggers(
        &mut self,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.rule_evaluation_service()
            .collect_rule_commands_for_key_triggers(command_buffer);
    }
}
