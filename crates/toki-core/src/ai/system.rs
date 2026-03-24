//! Core AI system implementation.

use std::collections::HashMap;

use crate::animation::AnimationState;
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::entity::{AiBehavior, Entity, EntityId, EntityManager};
use glam::{IVec2, UVec2};

use super::constants::{
    IDLE_WAIT_MAX_FRAMES, IDLE_WAIT_MIN_FRAMES, TILE_SIZE_PX, WANDER_MAX_TILES, WANDER_MIN_TILES,
    WANDER_UPDATE_FREQUENCY,
};
use super::context::AiContext;
use super::movement::{
    build_movement_intent_result, compute_directions_away, compute_directions_toward,
    distance_between, preview_intended_position, random_cardinal_direction,
    try_intent_with_fallback,
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
        let random_direction = match fastrand::u32(0..5) {
            0 => IVec2::new(0, -1),
            1 => IVec2::new(0, 1),
            2 => IVec2::new(-1, 0),
            3 => IVec2::new(1, 0),
            _ => IVec2::ZERO,
        };
        let can_move = preview_intended_position(entity, entity.position, random_direction, ctx)
            .is_some_and(|candidate| {
                candidate == entity.position || ctx.is_movement_valid(entity, entity_id, candidate)
            });
        Some(build_movement_intent_result(
            entity_id,
            can_move.then_some(random_direction),
            can_move,
        ))
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

        let directions = compute_directions_toward(current_position, player_pos);

        try_intent_with_fallback(entity, entity_id, current_position, &directions, ctx)
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

        let directions = compute_directions_away(current_position, player_pos);

        try_intent_with_fallback(entity, entity_id, current_position, &directions, ctx)
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
                movement_intent: None,
                new_animation: Some(AnimationState::Idle),
                spawn_request: None,
            });
        }

        let direction = random_cardinal_direction();
        let tiles = fastrand::u32(WANDER_MIN_TILES..=WANDER_MAX_TILES);

        state.wander_phase = WanderPhase::Walking {
            direction,
            remaining_distance: ((tiles as i32) * TILE_SIZE_PX) as f32,
        };

        Some(AiUpdateResult {
            entity_id,
            movement_intent: None,
            new_animation: Some(AnimationState::Walk),
            spawn_request: None,
        })
    }

    fn handle_wander_walking(
        &mut self,
        entity: &Entity,
        entity_id: EntityId,
        current_position: IVec2,
        direction: IVec2,
        remaining_distance: f32,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let can_move = preview_intended_position(entity, current_position, direction, ctx)
            .is_some_and(|candidate| {
                candidate == current_position || ctx.is_movement_valid(entity, entity_id, candidate)
            });

        let state = self.entity_states.get_mut(&entity_id)?;
        let new_remaining = remaining_distance - entity.attributes.speed.max(0.0);

        if can_move && new_remaining > 0.0 {
            state.wander_phase = WanderPhase::Walking {
                direction,
                remaining_distance: new_remaining,
            };
            return Some(build_movement_intent_result(
                entity_id,
                Some(direction),
                true,
            ));
        }

        let wait_frames = fastrand::u32(IDLE_WAIT_MIN_FRAMES..=IDLE_WAIT_MAX_FRAMES);
        state.wander_phase = WanderPhase::Waiting;
        state.wait_frames_remaining = wait_frames;

        if can_move {
            Some(build_movement_intent_result(
                entity_id,
                Some(direction),
                true,
            ))
        } else {
            Some(AiUpdateResult {
                entity_id,
                movement_intent: None,
                new_animation: Some(AnimationState::Idle),
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
