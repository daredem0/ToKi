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

/// Wandering state for idle AI behavior.
#[derive(Debug, Clone, Default)]
pub enum WanderPhase {
    /// Entity is waiting/idle for a number of frames
    #[default]
    Waiting,
    /// Entity is walking in a direction for remaining distance (in pixels)
    Walking {
        direction: IVec2,
        remaining_distance: i32,
    },
}

/// Runtime AI state for an entity.
/// This is separate from the authored `AiConfig` and tracks transient runtime data.
#[derive(Debug, Clone)]
pub struct AiRuntimeState {
    /// Frame counter for update frequency control
    pub frame_counter: u64,
    /// Current wandering phase for idle behavior
    pub wander_phase: WanderPhase,
    /// Frames remaining in current wait period
    pub wait_frames_remaining: u32,
}

impl Default for AiRuntimeState {
    fn default() -> Self {
        Self {
            frame_counter: 0,
            wander_phase: WanderPhase::Waiting,
            // Start with random wait so entities don't all move at once
            wait_frames_remaining: fastrand::u32(30..=90),
        }
    }
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

        let player_position = player_id
            .and_then(|id| entity_manager.get_entity(id))
            .map(|e| e.position);

        let mut results = Vec::new();

        // Collect entities with active AI behaviors
        let ai_entities: Vec<_> = entity_manager
            .active_entities()
            .iter()
            .filter_map(|&entity_id| {
                // Skip player
                if Some(entity_id) == player_id {
                    return None;
                }

                let entity = entity_manager.get_entity(entity_id)?;
                let behavior = entity.attributes.ai_config.behavior;
                if matches!(
                    behavior,
                    AiBehavior::Wander | AiBehavior::Chase | AiBehavior::Run
                ) {
                    Some((entity_id, behavior))
                } else {
                    None
                }
            })
            .collect();

        for (entity_id, behavior) in ai_entities {
            let result = match behavior {
                AiBehavior::Wander => self.update_wander_entity(
                    entity_id,
                    entity_manager,
                    world_bounds,
                    tilemap,
                    atlas,
                ),
                AiBehavior::Chase => self.update_chase_entity(
                    entity_id,
                    entity_manager,
                    player_position,
                    world_bounds,
                    tilemap,
                    atlas,
                ),
                AiBehavior::Run => self.update_run_entity(
                    entity_id,
                    entity_manager,
                    player_position,
                    world_bounds,
                    tilemap,
                    atlas,
                ),
                _ => None,
            };
            if let Some(r) = result {
                results.push(r);
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
        // Wander only updates every 60 frames to avoid chaotic movement
        if !self.frame_counter.is_multiple_of(60) {
            return None;
        }

        let entity = entity_manager.get_entity(entity_id)?;
        let current_position = entity.position;

        // Wander uses larger steps since it updates less frequently
        let movement_step = (entity.attributes.speed * 5.0).round() as i32;
        let max_x = (world_bounds.x as i32 - entity.size.x as i32).max(0);
        let max_y = (world_bounds.y as i32 - entity.size.y as i32).max(0);

        // Choose random direction: 0=up, 1=down, 2=left, 3=right, 4=stay
        let random_direction = fastrand::u32(0..5);

        let new_position = match random_direction {
            0 => IVec2::new(
                current_position.x,
                (current_position.y - movement_step).max(0),
            ),
            1 => IVec2::new(
                current_position.x,
                (current_position.y + movement_step).min(max_y),
            ),
            2 => IVec2::new(
                (current_position.x - movement_step).max(0),
                current_position.y,
            ),
            3 => IVec2::new(
                (current_position.x + movement_step).min(max_x),
                current_position.y,
            ),
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

    fn update_chase_entity(
        &mut self,
        entity_id: EntityId,
        entity_manager: &EntityManager,
        player_position: Option<IVec2>,
        world_bounds: UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Option<AiUpdateResult> {
        let entity = entity_manager.get_entity(entity_id)?;
        let player_pos = player_position?;

        let current_position = entity.position;
        let detection_radius = entity.attributes.ai_config.detection_radius;

        // Calculate distance to player
        let dx = player_pos.x - current_position.x;
        let dy = player_pos.y - current_position.y;
        let distance = ((dx * dx + dy * dy) as f32).sqrt();

        // If player is outside detection radius, wander randomly
        if distance > detection_radius as f32 {
            return self.idle_wander(
                entity,
                entity_id,
                world_bounds,
                entity_manager,
                tilemap,
                atlas,
            );
        }

        let movement_step = entity.attributes.speed.round() as i32;
        let directions =
            Self::compute_directions_toward(current_position, player_pos, movement_step);

        Self::try_movement_with_fallback(
            entity,
            entity_id,
            current_position,
            &directions,
            world_bounds,
            entity_manager,
            tilemap,
            atlas,
        )
    }

    fn update_run_entity(
        &mut self,
        entity_id: EntityId,
        entity_manager: &EntityManager,
        player_position: Option<IVec2>,
        world_bounds: UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Option<AiUpdateResult> {
        let entity = entity_manager.get_entity(entity_id)?;
        let player_pos = player_position?;

        let current_position = entity.position;
        let detection_radius = entity.attributes.ai_config.detection_radius;

        // Calculate distance to player
        let dx = player_pos.x - current_position.x;
        let dy = player_pos.y - current_position.y;
        let distance = ((dx * dx + dy * dy) as f32).sqrt();

        // If player is outside detection radius, wander randomly
        if distance > detection_radius as f32 {
            return self.idle_wander(
                entity,
                entity_id,
                world_bounds,
                entity_manager,
                tilemap,
                atlas,
            );
        }

        let movement_step = entity.attributes.speed.round() as i32;
        let directions = Self::compute_directions_away(current_position, player_pos, movement_step);

        Self::try_movement_with_fallback(
            entity,
            entity_id,
            current_position,
            &directions,
            world_bounds,
            entity_manager,
            tilemap,
            atlas,
        )
    }

    /// Idle wandering behavior for Chase/Run when player is outside detection radius.
    /// Uses a state machine: walk random tiles in one direction, then wait, repeat.
    #[allow(clippy::too_many_arguments)]
    fn idle_wander(
        &mut self,
        entity: &crate::entity::Entity,
        entity_id: EntityId,
        world_bounds: UVec2,
        entity_manager: &EntityManager,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Option<AiUpdateResult> {
        let state = self.entity_states.entry(entity_id).or_default();
        let current_position = entity.position;

        match &state.wander_phase {
            WanderPhase::Waiting => self.handle_wander_waiting(entity_id),
            WanderPhase::Walking {
                direction,
                remaining_distance,
            } => {
                let dir = *direction;
                let remaining = *remaining_distance;
                self.handle_wander_walking(
                    entity,
                    entity_id,
                    current_position,
                    dir,
                    remaining,
                    world_bounds,
                    entity_manager,
                    tilemap,
                    atlas,
                )
            }
        }
    }

    /// Handle the waiting phase of idle wandering.
    fn handle_wander_waiting(&mut self, entity_id: EntityId) -> Option<AiUpdateResult> {
        let state = self.entity_states.get_mut(&entity_id)?;

        if state.wait_frames_remaining > 0 {
            state.wait_frames_remaining -= 1;
            return Some(AiUpdateResult {
                entity_id,
                new_position: None,
                new_animation: Some(AnimationState::Idle),
                movement_distance: 0.0,
            });
        }

        // Done waiting - start walking in a random direction
        let direction = Self::random_cardinal_direction();
        let tiles = fastrand::u32(1..=3); // Walk 1-3 tiles
        let tile_size = 16; // Standard tile size in pixels

        state.wander_phase = WanderPhase::Walking {
            direction,
            remaining_distance: (tiles * tile_size) as i32,
        };

        // Return walking animation for this frame
        Some(AiUpdateResult {
            entity_id,
            new_position: None,
            new_animation: Some(AnimationState::Walk),
            movement_distance: 0.0,
        })
    }

    /// Handle the walking phase of idle wandering.
    #[allow(clippy::too_many_arguments)]
    fn handle_wander_walking(
        &mut self,
        entity: &crate::entity::Entity,
        entity_id: EntityId,
        current_position: IVec2,
        direction: IVec2,
        remaining_distance: i32,
        world_bounds: UVec2,
        entity_manager: &EntityManager,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Option<AiUpdateResult> {
        let movement_step = entity.attributes.speed.round() as i32;
        let max_x = (world_bounds.x as i32 - entity.size.x as i32).max(0);
        let max_y = (world_bounds.y as i32 - entity.size.y as i32).max(0);

        let scaled_direction = IVec2::new(direction.x * movement_step, direction.y * movement_step);
        let new_position = IVec2::new(
            (current_position.x + scaled_direction.x).clamp(0, max_x),
            (current_position.y + scaled_direction.y).clamp(0, max_y),
        );

        let can_move = new_position != current_position
            && can_entity_move_to_position(entity, new_position, tilemap, atlas)
            && !entity_manager.would_collide_with_solid_entity(entity_id, new_position);

        let state = self.entity_states.get_mut(&entity_id)?;
        let new_remaining = remaining_distance - movement_step;

        if can_move && new_remaining > 0 {
            // Continue walking
            state.wander_phase = WanderPhase::Walking {
                direction,
                remaining_distance: new_remaining,
            };
            return Some(Self::build_movement_result(
                entity_id,
                current_position,
                new_position,
                true,
            ));
        }

        // Either blocked or finished walking - transition to waiting
        let wait_frames = fastrand::u32(30..=180); // Wait 1-3 seconds at 60fps
        state.wander_phase = WanderPhase::Waiting;
        state.wait_frames_remaining = wait_frames;

        if can_move {
            // Final step before waiting
            Some(Self::build_movement_result(
                entity_id,
                current_position,
                new_position,
                true,
            ))
        } else {
            // Blocked - just start waiting
            Some(AiUpdateResult {
                entity_id,
                new_position: None,
                new_animation: Some(AnimationState::Idle),
                movement_distance: 0.0,
            })
        }
    }

    /// Generate a random cardinal direction (up, down, left, right).
    fn random_cardinal_direction() -> IVec2 {
        match fastrand::u32(0..4) {
            0 => IVec2::new(0, -1), // Up
            1 => IVec2::new(0, 1),  // Down
            2 => IVec2::new(-1, 0), // Left
            _ => IVec2::new(1, 0),  // Right
        }
    }

    /// Compute movement directions toward a target, ordered by priority.
    /// Returns primary direction first, then perpendicular directions.
    fn compute_directions_toward(current: IVec2, target: IVec2, step: i32) -> Vec<IVec2> {
        let dx = target.x - current.x;
        let dy = target.y - current.y;

        let mut directions = Vec::with_capacity(3);

        // Primary direction (dominant axis)
        if dx.abs() >= dy.abs() {
            directions.push(IVec2::new(dx.signum() * step, 0));
            // Perpendicular directions
            if dy != 0 {
                directions.push(IVec2::new(0, dy.signum() * step));
            } else {
                directions.push(IVec2::new(0, step));
                directions.push(IVec2::new(0, -step));
            }
        } else {
            directions.push(IVec2::new(0, dy.signum() * step));
            // Perpendicular directions
            if dx != 0 {
                directions.push(IVec2::new(dx.signum() * step, 0));
            } else {
                directions.push(IVec2::new(step, 0));
                directions.push(IVec2::new(-step, 0));
            }
        }

        directions
    }

    /// Compute movement directions away from a threat, ordered by priority.
    /// Returns primary direction first, then perpendicular directions.
    fn compute_directions_away(current: IVec2, threat: IVec2, step: i32) -> Vec<IVec2> {
        let dx = current.x - threat.x;
        let dy = current.y - threat.y;

        let mut directions = Vec::with_capacity(3);

        // Primary direction (dominant axis away from threat)
        if dx.abs() >= dy.abs() {
            let dir = if dx == 0 { 1 } else { dx.signum() };
            directions.push(IVec2::new(dir * step, 0));
            // Perpendicular directions
            if dy != 0 {
                directions.push(IVec2::new(0, dy.signum() * step));
            } else {
                directions.push(IVec2::new(0, step));
                directions.push(IVec2::new(0, -step));
            }
        } else {
            let dir = if dy == 0 { 1 } else { dy.signum() };
            directions.push(IVec2::new(0, dir * step));
            // Perpendicular directions
            if dx != 0 {
                directions.push(IVec2::new(dx.signum() * step, 0));
            } else {
                directions.push(IVec2::new(step, 0));
                directions.push(IVec2::new(-step, 0));
            }
        }

        directions
    }

    /// Try movement in multiple directions, falling back to perpendicular if blocked.
    #[allow(clippy::too_many_arguments)]
    fn try_movement_with_fallback(
        entity: &crate::entity::Entity,
        entity_id: EntityId,
        current_position: IVec2,
        directions: &[IVec2],
        world_bounds: UVec2,
        entity_manager: &EntityManager,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Option<AiUpdateResult> {
        let max_x = (world_bounds.x as i32 - entity.size.x as i32).max(0);
        let max_y = (world_bounds.y as i32 - entity.size.y as i32).max(0);

        for &direction in directions {
            let new_position = IVec2::new(
                (current_position.x + direction.x).clamp(0, max_x),
                (current_position.y + direction.y).clamp(0, max_y),
            );

            if new_position == current_position {
                continue; // Clamped to same position, try next direction
            }

            if Self::is_movement_valid(
                entity,
                entity_id,
                new_position,
                entity_manager,
                tilemap,
                atlas,
            ) {
                return Some(Self::build_movement_result(
                    entity_id,
                    current_position,
                    new_position,
                    true,
                ));
            }
        }

        // All directions blocked, stay in place
        Some(Self::build_movement_result(
            entity_id,
            current_position,
            current_position,
            false,
        ))
    }

    /// Build the final AI update result after computing movement.
    fn build_movement_result(
        entity_id: EntityId,
        current_position: IVec2,
        new_position: IVec2,
        movement_valid: bool,
    ) -> AiUpdateResult {
        let entity_moved = movement_valid && new_position != current_position;

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

        AiUpdateResult {
            entity_id,
            new_position: if entity_moved {
                Some(final_position)
            } else {
                None
            },
            new_animation: Some(desired_animation),
            movement_distance,
        }
    }

    /// Check if movement to new position is valid (no collisions).
    fn is_movement_valid(
        entity: &crate::entity::Entity,
        entity_id: EntityId,
        new_position: IVec2,
        entity_manager: &EntityManager,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> bool {
        can_entity_move_to_position(entity, new_position, tilemap, atlas)
            && !entity_manager.would_collide_with_solid_entity(entity_id, new_position)
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
