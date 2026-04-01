//! Entity spawning from rules.
//!
//! Contains logic for spawning entities as rule actions.

use crate::entity::{EntityId, EntityKind, EntityRendering, MovementComponent, OptionalEntityComponents};
use crate::rules::RuleSpawnEntityType;

use super::GameState;
use crate::game::SceneSystem;

impl GameState {
    pub(super) fn spawn_entity_from_rule(
        &mut self,
        entity_type: RuleSpawnEntityType,
        position: glam::IVec2,
    ) -> EntityId {
        match entity_type {
            RuleSpawnEntityType::PlayerLikeNpc => {
                SceneSystem::spawn_player_like_npc(self, position)
            }
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
        self.world
            .entity_manager
            .spawn_entity(
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
