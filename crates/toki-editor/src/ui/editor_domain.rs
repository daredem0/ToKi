use toki_core::animation::AnimationState;

pub fn animation_state_label(state: AnimationState) -> &'static str {
    match state {
        AnimationState::Idle => "Idle",
        AnimationState::Walk => "Walk",
        AnimationState::Attack => "Attack",
        AnimationState::IdleDown => "Idle Down",
        AnimationState::IdleUp => "Idle Up",
        AnimationState::IdleLeft => "Idle Left",
        AnimationState::IdleRight => "Idle Right",
        AnimationState::WalkDown => "Walk Down",
        AnimationState::WalkUp => "Walk Up",
        AnimationState::WalkLeft => "Walk Left",
        AnimationState::WalkRight => "Walk Right",
        AnimationState::AttackDown => "Attack Down",
        AnimationState::AttackUp => "Attack Up",
        AnimationState::AttackLeft => "Attack Left",
        AnimationState::AttackRight => "Attack Right",
    }
}

pub fn animation_state_options() -> [AnimationState; 15] {
    [
        AnimationState::Idle,
        AnimationState::Walk,
        AnimationState::Attack,
        AnimationState::IdleDown,
        AnimationState::IdleUp,
        AnimationState::IdleLeft,
        AnimationState::IdleRight,
        AnimationState::WalkDown,
        AnimationState::WalkUp,
        AnimationState::WalkLeft,
        AnimationState::WalkRight,
        AnimationState::AttackDown,
        AnimationState::AttackUp,
        AnimationState::AttackLeft,
        AnimationState::AttackRight,
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleActionEditorKind {
    PlaySound,
    PlayMusic,
    PlayAnimation,
    SetVelocity,
    Spawn,
    DestroySelf,
    SwitchScene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleConditionEditorKind {
    Always,
    TargetExists,
    KeyHeld,
    EntityActive,
    HealthBelow,
    HealthAbove,
    TriggerOtherIsPlayer,
    EntityIsKind,
    TriggerOtherIsKind,
    EntityHasTag,
    TriggerOtherHasTag,
    HasInventoryItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleTriggerEditorKind {
    Start,
    Update,
    PlayerMove,
    Key,
    Collision,
    Damaged,
    Death,
    Trigger,
    Interact,
    TileEnter,
    TileExit,
}
