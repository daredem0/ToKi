//! Movement computation helpers for AI.

use crate::animation::AnimationState;
use crate::entity::{Entity, EntityId};
use glam::IVec2;

use super::context::AiContext;
use super::types::AiUpdateResult;

fn update_axis_accumulator(accumulator: &mut f32, speed: f32, direction: i32) -> i32 {
    if direction == 0 {
        *accumulator = 0.0;
        return 0;
    }

    let direction_sign = direction.signum() as f32;
    let accumulator_sign = accumulator.signum();
    if accumulator_sign != 0.0 && accumulator_sign != direction_sign {
        *accumulator = 0.0;
    }

    *accumulator += speed * direction_sign;
    let whole_pixels = accumulator.trunc() as i32;
    *accumulator -= whole_pixels as f32;
    whole_pixels
}

fn clamp_entity_position_to_world_bounds(
    entity: &Entity,
    candidate_position: IVec2,
    ctx: &AiContext,
) -> IVec2 {
    let footprint = entity.resolved_footprint();
    let min_x = -footprint.offset[0];
    let min_y = -footprint.offset[1];
    let max_x = ctx.world_bounds.x as i32 - footprint.size[0] as i32 - footprint.offset[0];
    let max_y = ctx.world_bounds.y as i32 - footprint.size[1] as i32 - footprint.offset[1];

    IVec2::new(
        candidate_position
            .x
            .clamp(min_x.min(max_x), min_x.max(max_x)),
        candidate_position
            .y
            .clamp(min_y.min(max_y), min_y.max(max_y)),
    )
}

pub fn preview_intended_position(
    entity: &Entity,
    entity_id: EntityId,
    current_position: IVec2,
    direction: IVec2,
    ctx: &AiContext,
) -> Option<IVec2> {
    if direction == IVec2::ZERO {
        return None;
    }

    let mut accumulator = entity.movement_accumulator;
    let speed = ctx
        .entity_manager
        .movement(entity_id)
        .map(|movement| movement.speed.max(0.0))
        .unwrap_or(0.0);
    let pixels_x = update_axis_accumulator(&mut accumulator.x, speed, direction.x);
    let pixels_y = update_axis_accumulator(&mut accumulator.y, speed, direction.y);

    if pixels_x == 0 && pixels_y == 0 {
        return Some(current_position);
    }

    Some(clamp_entity_position_to_world_bounds(
        entity,
        current_position + IVec2::new(pixels_x, pixels_y),
        ctx,
    ))
}

/// Calculate distance between two positions.
pub fn distance_between(a: IVec2, b: IVec2) -> f32 {
    let dx = (b.x - a.x) as f32;
    let dy = (b.y - a.y) as f32;
    (dx * dx + dy * dy).sqrt()
}

/// Generate a random cardinal direction (up, down, left, right).
pub fn random_cardinal_direction() -> IVec2 {
    match fastrand::u32(0..4) {
        0 => IVec2::new(0, -1), // Up
        1 => IVec2::new(0, 1),  // Down
        2 => IVec2::new(-1, 0), // Left
        _ => IVec2::new(1, 0),  // Right
    }
}

/// Compute movement directions toward a target, ordered by priority.
/// Returns primary direction first, then perpendicular directions.
pub fn compute_directions_toward(current: IVec2, target: IVec2) -> Vec<IVec2> {
    let dx = target.x - current.x;
    let dy = target.y - current.y;

    let mut directions = Vec::with_capacity(3);

    // Primary direction (dominant axis)
    if dx.abs() >= dy.abs() {
        directions.push(IVec2::new(dx.signum(), 0));
        // Perpendicular directions
        if dy != 0 {
            directions.push(IVec2::new(0, dy.signum()));
        } else {
            directions.push(IVec2::new(0, 1));
            directions.push(IVec2::new(0, -1));
        }
    } else {
        directions.push(IVec2::new(0, dy.signum()));
        // Perpendicular directions
        if dx != 0 {
            directions.push(IVec2::new(dx.signum(), 0));
        } else {
            directions.push(IVec2::new(1, 0));
            directions.push(IVec2::new(-1, 0));
        }
    }

    directions
}

/// Compute movement directions away from a threat, ordered by priority.
/// Returns primary direction first, then perpendicular directions.
pub fn compute_directions_away(current: IVec2, threat: IVec2) -> Vec<IVec2> {
    let dx = current.x - threat.x;
    let dy = current.y - threat.y;

    let mut directions = Vec::with_capacity(3);

    // Primary direction (dominant axis away from threat)
    if dx.abs() >= dy.abs() {
        let dir = if dx == 0 { 1 } else { dx.signum() };
        directions.push(IVec2::new(dir, 0));
        // Perpendicular directions
        if dy != 0 {
            directions.push(IVec2::new(0, dy.signum()));
        } else {
            directions.push(IVec2::new(0, 1));
            directions.push(IVec2::new(0, -1));
        }
    } else {
        let dir = if dy == 0 { 1 } else { dy.signum() };
        directions.push(IVec2::new(0, dir));
        // Perpendicular directions
        if dx != 0 {
            directions.push(IVec2::new(dx.signum(), 0));
        } else {
            directions.push(IVec2::new(1, 0));
            directions.push(IVec2::new(-1, 0));
        }
    }

    directions
}

/// Try intent directions in priority order, falling back when the per-tick shared movement
/// preview is blocked.
pub fn try_intent_with_fallback(
    entity: &Entity,
    entity_id: EntityId,
    current_position: IVec2,
    directions: &[IVec2],
    ctx: &AiContext,
) -> Option<AiUpdateResult> {
    for &direction in directions {
        if direction == IVec2::ZERO {
            continue;
        }
        let Some(new_position) =
            preview_intended_position(entity, entity_id, current_position, direction, ctx)
        else {
            continue;
        };

        if new_position == current_position {
            return Some(build_movement_intent_result(
                entity_id,
                Some(direction),
                true,
            ));
        }

        if ctx.is_movement_valid(entity, entity_id, new_position) {
            return Some(build_movement_intent_result(
                entity_id,
                Some(direction),
                true,
            ));
        }
    }

    Some(build_movement_intent_result(entity_id, None, false))
}

/// Build the final AI update result after choosing movement intent.
pub fn build_movement_intent_result(
    entity_id: EntityId,
    movement_intent: Option<IVec2>,
    movement_valid: bool,
) -> AiUpdateResult {
    let desired_animation = if movement_valid && movement_intent.is_some() {
        AnimationState::Walk
    } else {
        AnimationState::Idle
    };

    AiUpdateResult {
        entity_id,
        movement_intent: movement_intent.filter(|intent| *intent != IVec2::ZERO),
        new_animation: Some(desired_animation),
        spawn_request: None,
    }
}

/// Build a wander-specific result with optional movement.
pub fn build_wander_intent_result(
    entity_id: EntityId,
    entity_moved: bool,
    movement_intent: Option<IVec2>,
) -> Option<AiUpdateResult> {
    let desired_animation = if entity_moved {
        AnimationState::Walk
    } else {
        AnimationState::Idle
    };

    Some(AiUpdateResult {
        entity_id,
        movement_intent: movement_intent.filter(|intent| *intent != IVec2::ZERO),
        new_animation: Some(desired_animation),
        spawn_request: None,
    })
}
