//! Entity spawning from rules.
//!
//! Contains logic for spawning entities as rule actions.

use crate::entity::{
    EntityId, EntityKind, EntityRendering, MovementComponent, OptionalEntityComponents,
};
use crate::rules::RuleSpawnEntityType;

#[cfg(test)]
use super::GameState;
use super::RuleEvaluationService;
use crate::game::player_defs::player_like_npc_definition;

impl RuleEvaluationService<'_> {
    pub(super) fn spawn_entity_from_rule(
        &mut self,
        entity_type: RuleSpawnEntityType,
        position: glam::IVec2,
    ) -> EntityId {
        match entity_type {
            RuleSpawnEntityType::PlayerLikeNpc => self.spawn_player_like_npc_from_rule(position),
            RuleSpawnEntityType::Npc => self.spawn_basic_entity(EntityKind::Npc, position, true),
            RuleSpawnEntityType::Item => self.spawn_basic_entity(EntityKind::Item, position, false),
            RuleSpawnEntityType::Decoration => {
                self.spawn_basic_entity(EntityKind::Decoration, position, false)
            }
            RuleSpawnEntityType::Trigger => self.spawn_trigger_entity(position),
        }
    }

    fn spawn_player_like_npc_from_rule(&mut self, position: glam::IVec2) -> EntityId {
        let npc_def = player_like_npc_definition();
        self.world
            .entity_manager
            .spawn_from_definition(&npc_def, position)
            .expect("default player-like npc definition should always be valid")
    }

    fn spawn_basic_entity(
        &mut self,
        kind: EntityKind,
        position: glam::IVec2,
        can_move: bool,
    ) -> EntityId {
        self.world.entity_manager.spawn_entity(
            kind,
            position,
            glam::UVec2::new(16, 16),
            EntityRendering::default(),
            false,
            true,
            OptionalEntityComponents {
                movement: Some(MovementComponent {
                    can_move,
                    ..MovementComponent::default()
                }),
                ..OptionalEntityComponents::default()
            },
        )
    }

    fn spawn_trigger_entity(&mut self, position: glam::IVec2) -> EntityId {
        self.world.entity_manager.spawn_entity(
            EntityKind::Trigger,
            position,
            glam::UVec2::new(16, 16),
            EntityRendering {
                visible: false,
                ..EntityRendering::default()
            },
            false,
            true,
            OptionalEntityComponents {
                movement: Some(MovementComponent {
                    can_move: false,
                    ..MovementComponent::default()
                }),
                ..OptionalEntityComponents::default()
            },
        )
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl GameState {
    pub(super) fn spawn_entity_from_rule(
        &mut self,
        entity_type: RuleSpawnEntityType,
        position: glam::IVec2,
    ) -> EntityId {
        self.rule_evaluation_service()
            .spawn_entity_from_rule(entity_type, position)
    }
}
