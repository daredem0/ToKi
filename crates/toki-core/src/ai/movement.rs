//! Movement computation helpers for AI.

use crate::animation::AnimationState;
use crate::entity::{Entity, EntityId};
use glam::IVec2;

use super::context::AiContext;
use super::types::AiUpdateResult;

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
pub fn compute_directions_toward(current: IVec2, target: IVec2, step: i32) -> Vec<IVec2> {
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
pub fn compute_directions_away(current: IVec2, threat: IVec2, step: i32) -> Vec<IVec2> {
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
pub fn try_movement_with_fallback(
    entity: &Entity,
    entity_id: EntityId,
    current_position: IVec2,
    directions: &[IVec2],
    ctx: &AiContext,
) -> Option<AiUpdateResult> {
    let (max_x, max_y) = ctx.max_position(entity.size);

    for &direction in directions {
        let new_position = IVec2::new(
            (current_position.x + direction.x).clamp(0, max_x),
            (current_position.y + direction.y).clamp(0, max_y),
        );

        if new_position == current_position {
            continue; // Clamped to same position, try next direction
        }

        if ctx.is_movement_valid(entity, entity_id, new_position) {
            return Some(build_movement_result(
                entity_id,
                current_position,
                new_position,
                true,
            ));
        }
    }

    // All directions blocked, stay in place
    Some(build_movement_result(
        entity_id,
        current_position,
        current_position,
        false,
    ))
}

/// Build the final AI update result after computing movement.
pub fn build_movement_result(
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
        spawn_request: None,
    }
}

/// Build a wander-specific result with optional movement.
pub fn build_wander_result(
    entity_id: EntityId,
    current_position: IVec2,
    new_position: IVec2,
    entity_moved: bool,
) -> Option<AiUpdateResult> {
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
        spawn_request: None,
    })
}
