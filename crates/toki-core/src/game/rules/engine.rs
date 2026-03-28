use crate::entity::{Entity, EntityId, EntityManager};
use crate::flags::GameFlags;
use crate::game::{InputKey, RuleRuntimeState};
use crate::rules::{RuleSet, RuleTarget, TriggerContext};

pub(in crate::game) struct RuleEngine<'a> {
    pub(super) entity_manager: &'a EntityManager,
    pub(super) player_id: Option<EntityId>,
    pub(super) held_keys: Vec<InputKey>,
    pub(super) game_flags: &'a GameFlags,
    pub(super) rules: &'a RuleSet,
    pub(super) rule_runtime: &'a mut RuleRuntimeState,
}

impl<'a> RuleEngine<'a> {
    pub(super) fn new(
        entity_manager: &'a EntityManager,
        player_id: Option<EntityId>,
        held_keys: Vec<InputKey>,
        game_flags: &'a GameFlags,
        rules: &'a RuleSet,
        rule_runtime: &'a mut RuleRuntimeState,
    ) -> Self {
        Self {
            entity_manager,
            player_id,
            held_keys,
            game_flags,
            rules,
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
}

#[cfg(test)]
mod tests {
    use super::RuleEngine;
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
        let mut engine = RuleEngine::new(
            &entity_manager,
            Option::<EntityId>::None,
            Vec::new(),
            &game_flags,
            &rules,
            &mut runtime,
        );

        let mut commands = Vec::new();
        engine.collect_rule_commands_for_trigger(
            RuleTrigger::OnStart,
            &mut commands,
        );

        assert!(matches!(
            commands.as_slice(),
            [crate::game::rules::RuleCommand::StartDialog { dialog_id, .. }]
                if dialog_id.as_str() == "intro"
        ));
    }
}
