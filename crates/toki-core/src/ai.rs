//! AI system for entity behavior.
//!
//! This module provides the runtime AI system that updates entity positions
//! based on their authored AI configuration.

use crate::animation::AnimationState;
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::collision::can_entity_move_to_position;
use crate::entity::{AiBehavior, EntityId, EntityManager};
use glam::{IVec2, UVec2};
use std::collections::HashMap;

/// Runtime AI state for an entity.
/// This is separate from the authored `AiConfig` and tracks transient runtime data.
#[derive(Debug, Clone, Default)]
pub struct AiRuntimeState {
    /// Frame counter for update frequency control
    pub frame_counter: u64,
}

/// Manages AI state for all entities.
#[derive(Debug, Clone, Default)]
pub struct AiSystem {
    /// Per-entity runtime AI state
    entity_states: HashMap<EntityId, AiRuntimeState>,
    /// Global frame counter for periodic updates
    frame_counter: u64,
}

/// Result of an AI update for a single entity.
#[derive(Debug, Clone)]
pub struct AiUpdateResult {
    pub entity_id: EntityId,
    pub new_position: Option<IVec2>,
    pub new_animation: Option<AnimationState>,
    pub movement_distance: f32,
}

impl AiSystem {
    /// Create a new AI system.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all runtime AI state.
    pub fn reset(&mut self) {
        self.entity_states.clear();
        self.frame_counter = 0;
    }

    /// Update AI for all entities.
    /// Returns a list of updates to apply to entities.
    pub fn update(
        &mut self,
        entity_manager: &EntityManager,
        player_id: Option<EntityId>,
        world_bounds: UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<AiUpdateResult> {
        self.frame_counter += 1;

        // Only update AI every 60 frames (~1 second at 60fps)
        if !self.frame_counter.is_multiple_of(60) {
            return Vec::new();
        }

        let mut results = Vec::new();

        // Collect entities with Wander behavior
        let wander_entities: Vec<_> = entity_manager
            .active_entities()
            .iter()
            .filter_map(|&entity_id| {
                // Skip player
                if Some(entity_id) == player_id {
                    return None;
                }

                let entity = entity_manager.get_entity(entity_id)?;
                if matches!(entity.attributes.ai_config.behavior, AiBehavior::Wander) {
                    Some(entity_id)
                } else {
                    None
                }
            })
            .collect();

        for entity_id in wander_entities {
            if let Some(result) = self.update_wander_entity(
                entity_id,
                entity_manager,
                world_bounds,
                tilemap,
                atlas,
            ) {
                results.push(result);
            }
        }

        results
    }

    fn update_wander_entity(
        &mut self,
        entity_id: EntityId,
        entity_manager: &EntityManager,
        world_bounds: UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Option<AiUpdateResult> {
        let entity = entity_manager.get_entity(entity_id)?;
        let current_position = entity.position;

        // NPC wander uses entity speed * 5 for larger jumps
        let movement_step = (entity.attributes.speed * 5.0) as i32;
        let max_x = (world_bounds.x as i32 - entity.size.x as i32).max(0);
        let max_y = (world_bounds.y as i32 - entity.size.y as i32).max(0);

        // Choose random direction: 0=up, 1=down, 2=left, 3=right, 4=stay
        let random_direction = fastrand::u32(0..5);

        let new_position = match random_direction {
            0 => IVec2::new(current_position.x, (current_position.y - movement_step).max(0)),
            1 => IVec2::new(current_position.x, (current_position.y + movement_step).min(max_y)),
            2 => IVec2::new((current_position.x - movement_step).max(0), current_position.y),
            3 => IVec2::new((current_position.x + movement_step).min(max_x), current_position.y),
            _ => current_position, // Stay in place
        };

        let entity_moved = new_position != current_position
            && can_entity_move_to_position(entity, new_position, tilemap, atlas)
            && !entity_manager.would_collide_with_solid_entity(entity_id, new_position);

        let final_position = if entity_moved {
            new_position
        } else {
            current_position
        };

        let movement_distance = if entity_moved {
            let dx = (final_position.x - current_position.x) as f32;
            let dy = (final_position.y - current_position.y) as f32;
            (dx * dx + dy * dy).sqrt()
        } else {
            0.0
        };

        let desired_animation = if entity_moved {
            AnimationState::Walk
        } else {
            AnimationState::Idle
        };

        Some(AiUpdateResult {
            entity_id,
            new_position: if entity_moved {
                Some(final_position)
            } else {
                None
            },
            new_animation: Some(desired_animation),
            movement_distance,
        })
    }

    /// Get or create runtime state for an entity.
    pub fn get_or_create_state(&mut self, entity_id: EntityId) -> &mut AiRuntimeState {
        self.entity_states.entry(entity_id).or_default()
    }

    /// Remove runtime state for an entity.
    pub fn remove_state(&mut self, entity_id: EntityId) {
        self.entity_states.remove(&entity_id);
    }
}

#[cfg(test)]
#[path = "ai_tests.rs"]
mod tests;
