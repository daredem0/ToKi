//! AI behavior handlers using the strategy pattern.

use crate::animation::AnimationState;
use crate::entity::{AiBehavior, Entity, EntityId};
use glam::IVec2;

use super::constants::{
    IDLE_WAIT_MAX_FRAMES, IDLE_WAIT_MIN_FRAMES, TILE_SIZE_PX, WANDER_MAX_TILES, WANDER_MIN_TILES,
    WANDER_SPEED_MULTIPLIER, WANDER_UPDATE_FREQUENCY,
};
use super::context::AiContext;
use super::movement::{
    build_wander_result, compute_directions_away, compute_directions_toward, distance_between,
    random_cardinal_direction, try_movement_with_fallback,
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

        let distance = distance_between(current_position, player_pos);

        // If player is outside detection radius, wander randomly
        if distance > detection_radius as f32 {
            return IdleWanderHandler.update(entity, entity_id, player_position, ctx, ai_state);
        }

        let movement_step = entity.attributes.speed.round() as i32;
        let directions = compute_directions_toward(current_position, player_pos, movement_step);

        try_movement_with_fallback(entity, entity_id, current_position, &directions, ctx)
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

        let distance = distance_between(current_position, player_pos);

        // If player is outside detection radius, wander randomly
        if distance > detection_radius as f32 {
            return IdleWanderHandler.update(entity, entity_id, player_position, ctx, ai_state);
        }

        let movement_step = entity.attributes.speed.round() as i32;
        let directions = compute_directions_away(current_position, player_pos, movement_step);

        try_movement_with_fallback(entity, entity_id, current_position, &directions, ctx)
    }
}

/// Wander behavior handler - random movement every 60 frames.
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
        // Wander only updates periodically to avoid chaotic movement
        if !self.frame_counter.is_multiple_of(WANDER_UPDATE_FREQUENCY) {
            return None;
        }

        let current_position = entity.position;
        let movement_step = (entity.attributes.speed * WANDER_SPEED_MULTIPLIER).round() as i32;
        let (max_x, max_y) = ctx.max_position(entity.size);

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
            _ => current_position,
        };

        let entity_moved = new_position != current_position
            && ctx.is_movement_valid(entity, entity_id, new_position);

        build_wander_result(entity_id, current_position, new_position, entity_moved)
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
                        new_position: None,
                        new_animation: Some(AnimationState::Idle),
                        movement_distance: 0.0,
                        spawn_request: None,
                    });
                }

                // Done waiting - start walking
                let direction = random_cardinal_direction();
                let tiles = fastrand::u32(WANDER_MIN_TILES..=WANDER_MAX_TILES);

                ai_state.wander_phase = WanderPhase::Walking {
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
            WanderPhase::Walking {
                direction,
                remaining_distance,
            } => {
                let dir = *direction;
                let remaining = *remaining_distance;
                self.handle_walking(
                    entity,
                    entity_id,
                    current_position,
                    dir,
                    remaining,
                    ctx,
                    ai_state,
                )
            }
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
        remaining_distance: i32,
        ctx: &AiContext,
        ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult> {
        let movement_step = entity.attributes.speed.round() as i32;
        let (max_x, max_y) = ctx.max_position(entity.size);

        let scaled_direction = IVec2::new(direction.x * movement_step, direction.y * movement_step);
        let new_position = IVec2::new(
            (current_position.x + scaled_direction.x).clamp(0, max_x),
            (current_position.y + scaled_direction.y).clamp(0, max_y),
        );

        let can_move = new_position != current_position
            && ctx.is_movement_valid(entity, entity_id, new_position);

        let new_remaining = remaining_distance - movement_step;

        if can_move && new_remaining > 0 {
            ai_state.wander_phase = WanderPhase::Walking {
                direction,
                remaining_distance: new_remaining,
            };
            return Some(super::movement::build_movement_result(
                entity_id,
                current_position,
                new_position,
                true,
            ));
        }

        // Transition to waiting
        ai_state.wander_phase = WanderPhase::Waiting;
        ai_state.wait_frames_remaining = fastrand::u32(IDLE_WAIT_MIN_FRAMES..=IDLE_WAIT_MAX_FRAMES);

        if can_move {
            Some(super::movement::build_movement_result(
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
            AiBehavior::RunAndMultiply => None, // Complex behavior still handled in AiSystem
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
