//! AI behavior handlers using the strategy pattern.

use crate::animation::AnimationState;
use crate::entity::{AiBehavior, Entity, EntityId};
use glam::IVec2;

use super::constants::{IDLE_WAIT_MAX_FRAMES, IDLE_WAIT_MIN_FRAMES, TILE_SIZE_PX};
use super::context::AiContext;
use super::movement::{
    build_movement_intent_result, build_wander_intent_result, compute_directions_away,
    compute_directions_toward, distance_between, preview_intended_position,
    random_cardinal_direction, try_intent_with_fallback,
};
use super::types::{AiRuntimeState, AiUpdateResult, WanderPhase};

/// Trait for AI behavior update logic.
/// Each behavior type implements this trait to provide polymorphic dispatch.
pub trait BehaviorUpdate {
    /// Update the entity's AI state and return movement/animation changes.
    fn update(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        ctx: &AiContext,
        ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult>;
}

/// Chase behavior handler - moves toward player when in detection radius.
#[derive(Debug, Clone, Copy)]
pub struct ChaseHandler;

impl BehaviorUpdate for ChaseHandler {
    fn update(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        ctx: &AiContext,
        ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult> {
        let player_pos = player_position?;
        let current_position = entity.position;
        let detection_radius = entity.attributes.ai_config.detection_radius;

        if distance_between(current_position, player_pos) > detection_radius as f32 {
            return IdleWanderHandler.update(entity, entity_id, player_position, ctx, ai_state);
        }

        let directions = compute_directions_toward(current_position, player_pos);
        try_intent_with_fallback(entity, entity_id, current_position, &directions, ctx)
    }
}

/// Run behavior handler - moves away from player when in detection radius.
#[derive(Debug, Clone, Copy)]
pub struct RunHandler;

impl BehaviorUpdate for RunHandler {
    fn update(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        ctx: &AiContext,
        ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult> {
        let player_pos = player_position?;
        let current_position = entity.position;
        let detection_radius = entity.attributes.ai_config.detection_radius;

        if distance_between(current_position, player_pos) > detection_radius as f32 {
            return IdleWanderHandler.update(entity, entity_id, player_position, ctx, ai_state);
        }

        let directions = compute_directions_away(current_position, player_pos);
        try_intent_with_fallback(entity, entity_id, current_position, &directions, ctx)
    }
}

/// Wander behavior handler - produces a random direction intent periodically.
#[derive(Debug, Clone, Copy)]
pub struct WanderHandler {
    frame_counter: u64,
}

impl WanderHandler {
    pub fn new(frame_counter: u64) -> Self {
        Self { frame_counter }
    }
}

impl BehaviorUpdate for WanderHandler {
    fn update(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        _player_position: Option<IVec2>,
        ctx: &AiContext,
        _ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult> {
        if !self
            .frame_counter
            .is_multiple_of(super::constants::WANDER_UPDATE_FREQUENCY)
        {
            return None;
        }

        let direction = match fastrand::u32(0..5) {
            0 => IVec2::new(0, -1),
            1 => IVec2::new(0, 1),
            2 => IVec2::new(-1, 0),
            3 => IVec2::new(1, 0),
            _ => IVec2::ZERO,
        };
        let can_move = preview_intended_position(entity, entity.position, direction, ctx)
            .is_some_and(|candidate| {
                candidate == entity.position || ctx.is_movement_valid(entity, entity_id, candidate)
            });
        build_wander_intent_result(entity_id, can_move, can_move.then_some(direction))
    }
}

/// Idle wander handler - state machine for waiting/walking when not pursuing.
#[derive(Debug, Clone, Copy)]
pub struct IdleWanderHandler;

impl BehaviorUpdate for IdleWanderHandler {
    fn update(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        _player_position: Option<IVec2>,
        ctx: &AiContext,
        ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult> {
        let current_position = entity.position;

        match &ai_state.wander_phase {
            WanderPhase::Waiting => {
                if ai_state.wait_frames_remaining > 0 {
                    ai_state.wait_frames_remaining -= 1;
                    return Some(AiUpdateResult {
                        entity_id,
                        movement_intent: None,
                        new_animation: Some(AnimationState::Idle),
                        spawn_request: None,
                    });
                }

                let direction = random_cardinal_direction();
                let tiles = fastrand::u32(
                    super::constants::WANDER_MIN_TILES..=super::constants::WANDER_MAX_TILES,
                );

                ai_state.wander_phase = WanderPhase::Walking {
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
            WanderPhase::Walking {
                direction,
                remaining_distance,
            } => self.handle_walking(
                entity,
                entity_id,
                current_position,
                *direction,
                *remaining_distance,
                ctx,
                ai_state,
            ),
        }
    }
}

impl IdleWanderHandler {
    #[allow(clippy::too_many_arguments)]
    fn handle_walking(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        current_position: IVec2,
        direction: IVec2,
        remaining_distance: f32,
        ctx: &AiContext,
        ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult> {
        let can_move = preview_intended_position(entity, current_position, direction, ctx)
            .is_some_and(|candidate| {
                candidate == current_position || ctx.is_movement_valid(entity, entity_id, candidate)
            });
        let new_remaining = remaining_distance - entity.attributes.speed.max(0.0);

        if can_move && new_remaining > 0.0 {
            ai_state.wander_phase = WanderPhase::Walking {
                direction,
                remaining_distance: new_remaining,
            };
            return Some(build_movement_intent_result(
                entity_id,
                Some(direction),
                true,
            ));
        }

        ai_state.wander_phase = WanderPhase::Waiting;
        ai_state.wait_frames_remaining = fastrand::u32(IDLE_WAIT_MIN_FRAMES..=IDLE_WAIT_MAX_FRAMES);

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
}

/// Enum wrapper for behavior handlers enabling factory method.
#[derive(Debug)]
pub enum BehaviorHandler {
    Chase(ChaseHandler),
    Run(RunHandler),
    Wander(WanderHandler),
}

impl BehaviorHandler {
    /// Create a handler for the given behavior type.
    /// Returns None for behaviors that don't need active updates (e.g., None).
    pub fn for_behavior(behavior: AiBehavior, frame_counter: u64) -> Option<Self> {
        match behavior {
            AiBehavior::Chase => Some(Self::Chase(ChaseHandler)),
            AiBehavior::Run => Some(Self::Run(RunHandler)),
            AiBehavior::Wander => Some(Self::Wander(WanderHandler::new(frame_counter))),
            AiBehavior::RunAndMultiply => None,
            AiBehavior::None => None,
        }
    }

    /// Delegate to the appropriate handler.
    pub fn update(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        ctx: &AiContext,
        ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult> {
        match self {
            Self::Chase(h) => h.update(entity, entity_id, player_position, ctx, ai_state),
            Self::Run(h) => h.update(entity, entity_id, player_position, ctx, ai_state),
            Self::Wander(h) => h.update(entity, entity_id, player_position, ctx, ai_state),
        }
    }
}
