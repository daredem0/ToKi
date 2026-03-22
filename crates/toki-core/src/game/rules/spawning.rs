//! Entity spawning from rules.
//!
//! Contains logic for spawning entities as rule actions.

use crate::entity::{EntityAttributes, EntityId, EntityKind};
use crate::rules::RuleSpawnEntityType;

use super::GameState;

impl GameState {
    pub(super) fn spawn_entity_from_rule(
        &mut self,
        entity_type: RuleSpawnEntityType,
        position: glam::IVec2,
    ) -> EntityId {
        match entity_type {
            RuleSpawnEntityType::PlayerLikeNpc => self.spawn_player_like_npc(position),
            RuleSpawnEntityType::Npc => self.spawn_basic_entity(EntityKind::Npc, position, true),
            RuleSpawnEntityType::Item => self.spawn_basic_entity(EntityKind::Item, position, false),
            RuleSpawnEntityType::Decoration => {
                self.spawn_basic_entity(EntityKind::Decoration, position, false)
            }
            RuleSpawnEntityType::Trigger => self.spawn_trigger_entity(position),
        }
    }

    fn spawn_basic_entity(
        &mut self,
        kind: EntityKind,
        position: glam::IVec2,
        can_move: bool,
    ) -> EntityId {
        self.entity_manager.spawn_entity(
            kind,
            position,
            glam::UVec2::new(16, 16),
            EntityAttributes {
                solid: false,
                can_move,
                ..EntityAttributes::default()
            },
        )
    }

    fn spawn_trigger_entity(&mut self, position: glam::IVec2) -> EntityId {
        self.entity_manager.spawn_entity(
            EntityKind::Trigger,
            position,
            glam::UVec2::new(16, 16),
            EntityAttributes {
                solid: false,
                can_move: false,
                visible: false,
                ..EntityAttributes::default()
            },
        )
    }
}
