use crate::entity::{Entity, EntityId, EntityManager};
use crate::flags::GameFlags;
use crate::game::{InputKey, RuleRuntimeState};
use crate::rules::{RuleSet, RuleTarget, TriggerContext};
use crate::value_path::ValuePathContext;

pub(super) struct RuleEngineContext<'a> {
    pub(super) entity_manager: &'a EntityManager,
    pub(super) player_id: Option<EntityId>,
    pub(super) held_keys: &'a [InputKey],
    pub(super) game_flags: &'a GameFlags,
    pub(super) rules: &'a RuleSet,
}

pub(in crate::game) struct RuleEngine<'a> {
    pub(super) entity_manager: &'a EntityManager,
    pub(super) player_id: Option<EntityId>,
    pub(super) held_keys: &'a [InputKey],
    pub(super) game_flags: &'a GameFlags,
    pub(super) rules: &'a RuleSet,
    pub(super) rule_runtime: &'a mut RuleRuntimeState,
}

impl<'a> RuleEngine<'a> {
    pub(super) fn new(
        context: RuleEngineContext<'a>,
        rule_runtime: &'a mut RuleRuntimeState,
    ) -> Self {
        Self {
            entity_manager: context.entity_manager,
            player_id: context.player_id,
            held_keys: context.held_keys,
            game_flags: context.game_flags,
            rules: context.rules,
            rule_runtime,
        }
    }

    pub(super) fn resolve_rule_target(
        &self,
        target: RuleTarget,
        context: &TriggerContext,
    ) -> Option<EntityId> {
        match target {
            RuleTarget::Player => self.player_id,
            RuleTarget::Entity(entity_id) => Some(entity_id),
            RuleTarget::TriggerSelf => context.trigger_self,
            RuleTarget::TriggerOther => context.trigger_other,
            RuleTarget::RuleOwner => None,
        }
    }

    pub(super) fn resolve_entity(
        &self,
        target: RuleTarget,
        context: &TriggerContext,
    ) -> Option<&Entity> {
        self.resolve_rule_target(target, context)
            .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
    }

    pub(super) fn value_path_context<'b>(
        &self,
        context: &'b TriggerContext,
    ) -> ValuePathContext<'a, 'b> {
        ValuePathContext {
            entity_manager: self.entity_manager,
            game_flags: self.game_flags,
            player_id: self.player_id,
            trigger_context: context,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RuleEngine, RuleEngineContext};
    use crate::entity::EntityId;
    use crate::flags::GameFlags;
    use crate::game::RuleRuntimeState;
    use crate::rules::{Rule, RuleAction, RuleSet, RuleTrigger};

    #[test]
    fn rule_engine_collects_commands_directly() {
        let entity_manager = crate::entity::EntityManager::new();
        let mut runtime = RuleRuntimeState::default();
        let game_flags = GameFlags::default();
        let rules = RuleSet {
            rules: vec![Rule {
                id: "start".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                log_enabled: false,
                trigger: RuleTrigger::OnStart,
                conditions: Vec::new(),
                actions: vec![RuleAction::StartDialog {
                    dialog_id: "intro".into(),
                }],
            }],
        };
        let held_keys = Vec::new();
        let mut engine = RuleEngine::new(
            RuleEngineContext {
                entity_manager: &entity_manager,
                player_id: Option::<EntityId>::None,
                held_keys: &held_keys,
                game_flags: &game_flags,
                rules: &rules,
            },
            &mut runtime,
        );

        let mut commands = Vec::new();
        engine.collect_rule_commands_for_trigger(RuleTrigger::OnStart, &mut commands);

        assert!(matches!(
            commands.as_slice(),
            [crate::game::rules::RuleCommand::Scene(
                crate::game::rules::SceneCommand::StartDialog { dialog_id, .. }
            )]
                if dialog_id.as_str() == "intro"
        ));
    }
}
