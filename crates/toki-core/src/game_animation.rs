use super::GameState;
use crate::animation::{AnimationController, AnimationState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FacingDirection {
    Down,
    Up,
    Left,
    Right,
}

impl GameState {
    pub(super) fn facing_from_delta(delta: glam::IVec2) -> Option<FacingDirection> {
        if delta == glam::IVec2::ZERO {
            return None;
        }
        if delta.x.abs() > delta.y.abs() {
            if delta.x < 0 {
                Some(FacingDirection::Left)
            } else {
                Some(FacingDirection::Right)
            }
        } else if delta.y < 0 {
            Some(FacingDirection::Up)
        } else {
            Some(FacingDirection::Down)
        }
    }

    pub(super) fn facing_from_animation_state(state: AnimationState) -> FacingDirection {
        match state {
            AnimationState::IdleUp | AnimationState::WalkUp | AnimationState::AttackUp => {
                FacingDirection::Up
            }
            AnimationState::IdleLeft | AnimationState::WalkLeft | AnimationState::AttackLeft => {
                FacingDirection::Left
            }
            AnimationState::IdleRight | AnimationState::WalkRight | AnimationState::AttackRight => {
                FacingDirection::Right
            }
            AnimationState::Idle
            | AnimationState::Walk
            | AnimationState::Attack
            | AnimationState::IdleDown
            | AnimationState::WalkDown
            | AnimationState::AttackDown => FacingDirection::Down,
        }
    }

    pub(super) fn directional_animation_state(
        moving: bool,
        facing: FacingDirection,
    ) -> AnimationState {
        match (moving, facing) {
            (false, FacingDirection::Down) => AnimationState::IdleDown,
            (false, FacingDirection::Up) => AnimationState::IdleUp,
            (false, FacingDirection::Left) => AnimationState::IdleLeft,
            (false, FacingDirection::Right) => AnimationState::IdleRight,
            (true, FacingDirection::Down) => AnimationState::WalkDown,
            (true, FacingDirection::Up) => AnimationState::WalkUp,
            (true, FacingDirection::Left) => AnimationState::WalkLeft,
            (true, FacingDirection::Right) => AnimationState::WalkRight,
        }
    }

    pub(super) fn animation_state_flip_x(state: AnimationState) -> bool {
        matches!(
            state,
            AnimationState::IdleLeft | AnimationState::WalkLeft | AnimationState::AttackLeft
        )
    }

    pub(super) fn directional_attack_state(facing: FacingDirection) -> AnimationState {
        match facing {
            FacingDirection::Down => AnimationState::AttackDown,
            FacingDirection::Up => AnimationState::AttackUp,
            FacingDirection::Left => AnimationState::AttackLeft,
            FacingDirection::Right => AnimationState::AttackRight,
        }
    }

    pub(super) fn is_action_animation_state(state: AnimationState) -> bool {
        matches!(
            state,
            AnimationState::Attack
                | AnimationState::AttackDown
                | AnimationState::AttackUp
                | AnimationState::AttackLeft
                | AnimationState::AttackRight
        )
    }

    pub(super) fn action_animation_locks_locomotion(
        animation_controller: &AnimationController,
    ) -> bool {
        Self::is_action_animation_state(animation_controller.current_clip_state)
            && !animation_controller.is_finished
    }

    pub(super) fn resolve_animation_state(
        animation_controller: &AnimationController,
        moving: bool,
        delta: glam::IVec2,
    ) -> AnimationState {
        let fallback = if moving {
            AnimationState::Walk
        } else {
            AnimationState::Idle
        };

        let facing = Self::facing_from_delta(delta).unwrap_or_else(|| {
            Self::facing_from_animation_state(animation_controller.current_clip_state)
        });
        let directional = Self::directional_animation_state(moving, facing);

        if animation_controller.has_clip(directional) {
            directional
        } else {
            fallback
        }
    }
}
