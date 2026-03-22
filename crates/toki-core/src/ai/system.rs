//! Core AI system implementation.

use std::collections::HashMap;

use crate::animation::AnimationState;
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::entity::{AiBehavior, Entity, EntityId, EntityManager};
use glam::{IVec2, UVec2};

use super::constants::{
    IDLE_WAIT_MAX_FRAMES, IDLE_WAIT_MIN_FRAMES, TILE_SIZE_PX, WANDER_MAX_TILES, WANDER_MIN_TILES,
    WANDER_SPEED_MULTIPLIER, WANDER_UPDATE_FREQUENCY,
};
use super::context::AiContext;
use super::movement::{
    build_movement_result, compute_directions_away, compute_directions_toward, distance_between,
    random_cardinal_direction, try_movement_with_fallback,
};
use super::types::{AiRuntimeState, AiUpdateResult, SeparationState, WanderPhase};

/// Manages AI state for all entities.
#[derive(Debug, Clone, Default)]
pub struct AiSystem {
    /// Per-entity runtime AI state
    pub(super) entity_states: HashMap<EntityId, AiRuntimeState>,
    /// Global frame counter for periodic updates
    pub(super) frame_counter: u64,
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

        let ctx = AiContext::new(entity_manager, world_bounds, tilemap, atlas);
        let mut results = Vec::new();

        // Collect entities with active AI behaviors
        let ai_entities: Vec<_> = entity_manager
            .active_entities_iter()
            .filter_map(|entity_id| {
                if Some(entity_id) == player_id {
                    return None;
                }

                let entity = entity_manager.get_entity(entity_id)?;
                let behavior = entity.attributes.ai_config.behavior;
                if matches!(
                    behavior,
                    AiBehavior::Wander
                        | AiBehavior::Chase
                        | AiBehavior::Run
                        | AiBehavior::RunAndMultiply
                ) {
                    Some((entity_id, behavior))
                } else {
                    None
                }
            })
            .collect();

        for (entity_id, behavior) in ai_entities {
            let result = match behavior {
                AiBehavior::Wander => self.update_wander_entity(entity_id, &ctx),
                AiBehavior::Chase => self.update_chase_entity(entity_id, player_position, &ctx),
                AiBehavior::Run => self.update_run_entity(entity_id, player_position, &ctx),
                AiBehavior::RunAndMultiply => {
                    self.update_run_and_multiply_entity(entity_id, player_position, &ctx)
                }
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
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        if !self.frame_counter.is_multiple_of(WANDER_UPDATE_FREQUENCY) {
            return None;
        }

        let entity = ctx.entity_manager.get_entity(entity_id)?;
        let current_position = entity.position;
        let movement_step = (entity.attributes.speed * WANDER_SPEED_MULTIPLIER).round() as i32;
        let (max_x, max_y) = ctx.max_position(entity.size);

        let random_direction = fastrand::u32(0..5);
        let new_position = match random_direction {
            0 => IVec2::new(current_position.x, (current_position.y - movement_step).max(0)),
            1 => IVec2::new(
                current_position.x,
                (current_position.y + movement_step).min(max_y),
            ),
            2 => IVec2::new((current_position.x - movement_step).max(0), current_position.y),
            3 => IVec2::new(
                (current_position.x + movement_step).min(max_x),
                current_position.y,
            ),
            _ => current_position,
        };

        let entity_moved = new_position != current_position
            && ctx.is_movement_valid(entity, entity_id, new_position);

        let final_position = if entity_moved {
            new_position
        } else {
            current_position
        };
        let movement_distance = if entity_moved {
            distance_between(current_position, final_position)
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
            spawn_request: None,
        })
    }

    fn update_chase_entity(
        &mut self,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let entity = ctx.entity_manager.get_entity(entity_id)?;
        let player_pos = player_position?;
        let current_position = entity.position;
        let detection_radius = entity.attributes.ai_config.detection_radius;

        let distance = distance_between(current_position, player_pos);

        if distance > detection_radius as f32 {
            return self.idle_wander(entity, entity_id, ctx);
        }

        let movement_step = entity.attributes.speed.round() as i32;
        let directions = compute_directions_toward(current_position, player_pos, movement_step);

        try_movement_with_fallback(entity, entity_id, current_position, &directions, ctx)
    }

    fn update_run_entity(
        &mut self,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let entity = ctx.entity_manager.get_entity(entity_id)?;
        let player_pos = player_position?;
        let current_position = entity.position;
        let detection_radius = entity.attributes.ai_config.detection_radius;

        let distance = distance_between(current_position, player_pos);

        if distance > detection_radius as f32 {
            return self.idle_wander(entity, entity_id, ctx);
        }

        let movement_step = entity.attributes.speed.round() as i32;
        let directions = compute_directions_away(current_position, player_pos, movement_step);

        try_movement_with_fallback(entity, entity_id, current_position, &directions, ctx)
    }

    /// Idle wandering behavior for Chase/Run when player is outside detection radius.
    pub(super) fn idle_wander(
        &mut self,
        entity: &Entity,
        entity_id: EntityId,
        ctx: &AiContext,
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
                self.handle_wander_walking(entity, entity_id, current_position, dir, remaining, ctx)
            }
        }
    }

    fn handle_wander_waiting(&mut self, entity_id: EntityId) -> Option<AiUpdateResult> {
        let state = self.entity_states.get_mut(&entity_id)?;

        if state.wait_frames_remaining > 0 {
            state.wait_frames_remaining -= 1;
            return Some(AiUpdateResult {
                entity_id,
                new_position: None,
                new_animation: Some(AnimationState::Idle),
                movement_distance: 0.0,
                spawn_request: None,
            });
        }

        let direction = random_cardinal_direction();
        let tiles = fastrand::u32(WANDER_MIN_TILES..=WANDER_MAX_TILES);

        state.wander_phase = WanderPhase::Walking {
            direction,
            remaining_distance: (tiles as i32) * TILE_SIZE_PX,
        };

        Some(AiUpdateResult {
            entity_id,
            new_position: None,
            new_animation: Some(AnimationState::Walk),
            movement_distance: 0.0,
            spawn_request: None,
        })
    }

    fn handle_wander_walking(
        &mut self,
        entity: &Entity,
        entity_id: EntityId,
        current_position: IVec2,
        direction: IVec2,
        remaining_distance: i32,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let movement_step = entity.attributes.speed.round() as i32;
        let (max_x, max_y) = ctx.max_position(entity.size);

        let scaled = IVec2::new(direction.x * movement_step, direction.y * movement_step);
        let new_position = IVec2::new(
            (current_position.x + scaled.x).clamp(0, max_x),
            (current_position.y + scaled.y).clamp(0, max_y),
        );

        let can_move = new_position != current_position
            && ctx.is_movement_valid(entity, entity_id, new_position);

        let state = self.entity_states.get_mut(&entity_id)?;
        let new_remaining = remaining_distance - movement_step;

        if can_move && new_remaining > 0 {
            state.wander_phase = WanderPhase::Walking {
                direction,
                remaining_distance: new_remaining,
            };
            return Some(build_movement_result(
                entity_id,
                current_position,
                new_position,
                true,
            ));
        }

        let wait_frames = fastrand::u32(IDLE_WAIT_MIN_FRAMES..=IDLE_WAIT_MAX_FRAMES);
        state.wander_phase = WanderPhase::Waiting;
        state.wait_frames_remaining = wait_frames;

        if can_move {
            Some(build_movement_result(
                entity_id,
                current_position,
                new_position,
                true,
            ))
        } else {
            Some(AiUpdateResult {
                entity_id,
                new_position: None,
                new_animation: Some(AnimationState::Idle),
                movement_distance: 0.0,
                spawn_request: None,
            })
        }
    }

    /// Enter separation state for an entity.
    pub fn enter_separation_state(
        &mut self,
        entity_id: EntityId,
        other_ids: Vec<EntityId>,
        required_distance: f32,
    ) {
        let state = self.entity_states.entry(entity_id).or_default();
        state.separation_state = Some(SeparationState {
            other_entity_ids: other_ids,
            required_distance,
        });
    }

    /// Check if an entity is currently in separation state.
    pub fn is_entity_separating(&self, entity_id: EntityId) -> bool {
        self.entity_states
            .get(&entity_id)
            .is_some_and(|state| state.separation_state.is_some())
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
