use strum::EnumIter;
use toki_core::animation::AnimationState;
use toki_core::entity::EntityKind;
use toki_core::rules::{
    InteractionMode, RuleAction, RuleCondition, RuleKey, RuleSoundChannel, RuleSpawnEntityType,
    RuleTarget, RuleTrigger,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum RuleActionEditorKind {
    PlaySound,
    PlayMusic,
    PlayAnimation,
    SetVelocity,
    Spawn,
    DestroySelf,
    SwitchScene,
    DamageEntity,
    HealEntity,
    AddInventoryItem,
    RemoveInventoryItem,
    SetEntityActive,
    TeleportEntity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
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

pub fn rule_action_kind(action: &RuleAction) -> RuleActionEditorKind {
    match action {
        RuleAction::PlaySound { .. } => RuleActionEditorKind::PlaySound,
        RuleAction::PlayMusic { .. } => RuleActionEditorKind::PlayMusic,
        RuleAction::PlayAnimation { .. } => RuleActionEditorKind::PlayAnimation,
        RuleAction::SetVelocity { .. } => RuleActionEditorKind::SetVelocity,
        RuleAction::Spawn { .. } => RuleActionEditorKind::Spawn,
        RuleAction::DestroySelf { .. } => RuleActionEditorKind::DestroySelf,
        RuleAction::SwitchScene { .. } => RuleActionEditorKind::SwitchScene,
        RuleAction::DamageEntity { .. } => RuleActionEditorKind::DamageEntity,
        RuleAction::HealEntity { .. } => RuleActionEditorKind::HealEntity,
        RuleAction::AddInventoryItem { .. } => RuleActionEditorKind::AddInventoryItem,
        RuleAction::RemoveInventoryItem { .. } => RuleActionEditorKind::RemoveInventoryItem,
        RuleAction::SetEntityActive { .. } => RuleActionEditorKind::SetEntityActive,
        RuleAction::TeleportEntity { .. } => RuleActionEditorKind::TeleportEntity,
    }
}

pub fn rule_action_kind_label(kind: RuleActionEditorKind) -> &'static str {
    match kind {
        RuleActionEditorKind::PlaySound => "PlaySound",
        RuleActionEditorKind::PlayMusic => "PlayMusic",
        RuleActionEditorKind::PlayAnimation => "PlayAnimation",
        RuleActionEditorKind::SetVelocity => "SetVelocity",
        RuleActionEditorKind::Spawn => "Spawn",
        RuleActionEditorKind::DestroySelf => "DestroySelf",
        RuleActionEditorKind::SwitchScene => "SwitchScene",
        RuleActionEditorKind::DamageEntity => "DamageEntity",
        RuleActionEditorKind::HealEntity => "HealEntity",
        RuleActionEditorKind::AddInventoryItem => "AddInventoryItem",
        RuleActionEditorKind::RemoveInventoryItem => "RemoveInventoryItem",
        RuleActionEditorKind::SetEntityActive => "SetEntityActive",
        RuleActionEditorKind::TeleportEntity => "TeleportEntity",
    }
}

pub fn default_rule_action(kind: RuleActionEditorKind) -> RuleAction {
    match kind {
        RuleActionEditorKind::PlaySound => RuleAction::PlaySound {
            channel: RuleSoundChannel::Movement,
            sound_id: "sfx_placeholder".to_string(),
        },
        RuleActionEditorKind::PlayMusic => RuleAction::PlayMusic {
            track_id: "music_placeholder".to_string(),
        },
        RuleActionEditorKind::PlayAnimation => RuleAction::PlayAnimation {
            target: RuleTarget::Player,
            state: AnimationState::Idle,
        },
        RuleActionEditorKind::SetVelocity => RuleAction::SetVelocity {
            target: RuleTarget::Player,
            velocity: [0, 0],
        },
        RuleActionEditorKind::Spawn => RuleAction::Spawn {
            entity_type: RuleSpawnEntityType::Npc,
            position: [0, 0],
        },
        RuleActionEditorKind::DestroySelf => RuleAction::DestroySelf {
            target: RuleTarget::Player,
        },
        RuleActionEditorKind::SwitchScene => RuleAction::SwitchScene {
            scene_name: String::new(),
            spawn_point_id: String::new(),
        },
        RuleActionEditorKind::DamageEntity => RuleAction::DamageEntity {
            target: RuleTarget::Player,
            amount: 10,
        },
        RuleActionEditorKind::HealEntity => RuleAction::HealEntity {
            target: RuleTarget::Player,
            amount: 10,
        },
        RuleActionEditorKind::AddInventoryItem => RuleAction::AddInventoryItem {
            target: RuleTarget::Player,
            item_id: String::new(),
            count: 1,
        },
        RuleActionEditorKind::RemoveInventoryItem => RuleAction::RemoveInventoryItem {
            target: RuleTarget::Player,
            item_id: String::new(),
            count: 1,
        },
        RuleActionEditorKind::SetEntityActive => RuleAction::SetEntityActive {
            target: RuleTarget::Player,
            active: false,
        },
        RuleActionEditorKind::TeleportEntity => RuleAction::TeleportEntity {
            target: RuleTarget::Player,
            tile_x: 0,
            tile_y: 0,
        },
    }
}

pub fn rule_condition_kind(condition: &RuleCondition) -> RuleConditionEditorKind {
    match condition {
        RuleCondition::Always => RuleConditionEditorKind::Always,
        RuleCondition::TargetExists { .. } => RuleConditionEditorKind::TargetExists,
        RuleCondition::KeyHeld { .. } => RuleConditionEditorKind::KeyHeld,
        RuleCondition::EntityActive { .. } => RuleConditionEditorKind::EntityActive,
        RuleCondition::HealthBelow { .. } => RuleConditionEditorKind::HealthBelow,
        RuleCondition::HealthAbove { .. } => RuleConditionEditorKind::HealthAbove,
        RuleCondition::TriggerOtherIsPlayer => RuleConditionEditorKind::TriggerOtherIsPlayer,
        RuleCondition::EntityIsKind { .. } => RuleConditionEditorKind::EntityIsKind,
        RuleCondition::TriggerOtherIsKind { .. } => RuleConditionEditorKind::TriggerOtherIsKind,
        RuleCondition::EntityHasTag { .. } => RuleConditionEditorKind::EntityHasTag,
        RuleCondition::TriggerOtherHasTag { .. } => RuleConditionEditorKind::TriggerOtherHasTag,
        RuleCondition::HasInventoryItem { .. } => RuleConditionEditorKind::HasInventoryItem,
    }
}

pub fn rule_condition_kind_label(kind: RuleConditionEditorKind) -> &'static str {
    match kind {
        RuleConditionEditorKind::Always => "Always",
        RuleConditionEditorKind::TargetExists => "TargetExists",
        RuleConditionEditorKind::KeyHeld => "KeyHeld",
        RuleConditionEditorKind::EntityActive => "EntityActive",
        RuleConditionEditorKind::HealthBelow => "HealthBelow",
        RuleConditionEditorKind::HealthAbove => "HealthAbove",
        RuleConditionEditorKind::TriggerOtherIsPlayer => "TriggerOtherIsPlayer",
        RuleConditionEditorKind::EntityIsKind => "EntityIsKind",
        RuleConditionEditorKind::TriggerOtherIsKind => "TriggerOtherIsKind",
        RuleConditionEditorKind::EntityHasTag => "EntityHasTag",
        RuleConditionEditorKind::TriggerOtherHasTag => "TriggerOtherHasTag",
        RuleConditionEditorKind::HasInventoryItem => "HasInventoryItem",
    }
}

pub fn default_rule_condition(kind: RuleConditionEditorKind) -> RuleCondition {
    match kind {
        RuleConditionEditorKind::Always => RuleCondition::Always,
        RuleConditionEditorKind::TargetExists => RuleCondition::TargetExists {
            target: RuleTarget::Player,
        },
        RuleConditionEditorKind::KeyHeld => RuleCondition::KeyHeld { key: RuleKey::Up },
        RuleConditionEditorKind::EntityActive => RuleCondition::EntityActive {
            target: RuleTarget::Player,
            is_active: true,
        },
        RuleConditionEditorKind::HealthBelow => RuleCondition::HealthBelow {
            target: RuleTarget::Player,
            threshold: 50,
        },
        RuleConditionEditorKind::HealthAbove => RuleCondition::HealthAbove {
            target: RuleTarget::Player,
            threshold: 50,
        },
        RuleConditionEditorKind::TriggerOtherIsPlayer => RuleCondition::TriggerOtherIsPlayer,
        RuleConditionEditorKind::EntityIsKind => RuleCondition::EntityIsKind {
            target: RuleTarget::Player,
            kind: EntityKind::Player,
        },
        RuleConditionEditorKind::TriggerOtherIsKind => RuleCondition::TriggerOtherIsKind {
            kind: EntityKind::Npc,
        },
        RuleConditionEditorKind::EntityHasTag => RuleCondition::EntityHasTag {
            target: RuleTarget::Player,
            tag: String::new(),
        },
        RuleConditionEditorKind::TriggerOtherHasTag => {
            RuleCondition::TriggerOtherHasTag { tag: String::new() }
        }
        RuleConditionEditorKind::HasInventoryItem => RuleCondition::HasInventoryItem {
            target: RuleTarget::Player,
            item_id: String::new(),
            min_count: 1,
        },
    }
}

pub fn rule_trigger_kind(trigger: &RuleTrigger) -> RuleTriggerEditorKind {
    match trigger {
        RuleTrigger::OnStart => RuleTriggerEditorKind::Start,
        RuleTrigger::OnUpdate => RuleTriggerEditorKind::Update,
        RuleTrigger::OnPlayerMove => RuleTriggerEditorKind::PlayerMove,
        RuleTrigger::OnKey { .. } => RuleTriggerEditorKind::Key,
        RuleTrigger::OnCollision { .. } => RuleTriggerEditorKind::Collision,
        RuleTrigger::OnDamaged { .. } => RuleTriggerEditorKind::Damaged,
        RuleTrigger::OnDeath { .. } => RuleTriggerEditorKind::Death,
        RuleTrigger::OnTrigger => RuleTriggerEditorKind::Trigger,
        RuleTrigger::OnInteract { .. } => RuleTriggerEditorKind::Interact,
        RuleTrigger::OnTileEnter { .. } => RuleTriggerEditorKind::TileEnter,
        RuleTrigger::OnTileExit { .. } => RuleTriggerEditorKind::TileExit,
    }
}

pub fn rule_trigger_kind_label(kind: RuleTriggerEditorKind) -> &'static str {
    match kind {
        RuleTriggerEditorKind::Start => "OnStart",
        RuleTriggerEditorKind::Update => "OnUpdate",
        RuleTriggerEditorKind::PlayerMove => "OnPlayerMove",
        RuleTriggerEditorKind::Key => "OnKey",
        RuleTriggerEditorKind::Collision => "OnCollision",
        RuleTriggerEditorKind::Damaged => "OnDamaged",
        RuleTriggerEditorKind::Death => "OnDeath",
        RuleTriggerEditorKind::Trigger => "OnTrigger",
        RuleTriggerEditorKind::Interact => "OnInteract",
        RuleTriggerEditorKind::TileEnter => "OnTileEnter",
        RuleTriggerEditorKind::TileExit => "OnTileExit",
    }
}

pub fn default_rule_trigger(kind: RuleTriggerEditorKind) -> RuleTrigger {
    match kind {
        RuleTriggerEditorKind::Start => RuleTrigger::OnStart,
        RuleTriggerEditorKind::Update => RuleTrigger::OnUpdate,
        RuleTriggerEditorKind::PlayerMove => RuleTrigger::OnPlayerMove,
        RuleTriggerEditorKind::Key => RuleTrigger::OnKey { key: RuleKey::Up },
        RuleTriggerEditorKind::Collision => RuleTrigger::OnCollision { entity: None },
        RuleTriggerEditorKind::Damaged => RuleTrigger::OnDamaged { entity: None },
        RuleTriggerEditorKind::Death => RuleTrigger::OnDeath { entity: None },
        RuleTriggerEditorKind::Trigger => RuleTrigger::OnTrigger,
        RuleTriggerEditorKind::Interact => RuleTrigger::OnInteract {
            mode: InteractionMode::default(),
            entity: None,
        },
        RuleTriggerEditorKind::TileEnter => RuleTrigger::OnTileEnter { x: 0, y: 0 },
        RuleTriggerEditorKind::TileExit => RuleTrigger::OnTileExit { x: 0, y: 0 },
    }
}

pub fn rule_key_label(key: RuleKey) -> &'static str {
    match key {
        RuleKey::Up => "Up",
        RuleKey::Down => "Down",
        RuleKey::Left => "Left",
        RuleKey::Right => "Right",
        RuleKey::DebugToggle => "DebugToggle",
        RuleKey::Interact => "Interact",
        RuleKey::AttackPrimary => "AttackPrimary",
        RuleKey::AttackSecondary => "AttackSecondary",
        RuleKey::Inventory => "Inventory",
        RuleKey::Pause => "Pause",
    }
}

pub fn rule_sound_channel_label(channel: RuleSoundChannel) -> &'static str {
    match channel {
        RuleSoundChannel::Movement => "Movement",
        RuleSoundChannel::Collision => "Collision",
    }
}

pub fn rule_spawn_entity_type_label(entity_type: RuleSpawnEntityType) -> &'static str {
    match entity_type {
        RuleSpawnEntityType::PlayerLikeNpc => "PlayerLikeNpc",
        RuleSpawnEntityType::Npc => "Npc",
        RuleSpawnEntityType::Item => "Item",
        RuleSpawnEntityType::Decoration => "Decoration",
        RuleSpawnEntityType::Trigger => "Trigger",
    }
}

pub fn rule_target_label(target: RuleTarget) -> String {
    match target {
        RuleTarget::Player => "Player".to_string(),
        RuleTarget::Entity(entity_id) => format!("Entity({entity_id})"),
        RuleTarget::RuleOwner => "RuleOwner".to_string(),
        RuleTarget::TriggerSelf => "TriggerSelf".to_string(),
        RuleTarget::TriggerOther => "TriggerOther".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn rule_trigger_kind_defaults_round_trip() {
        for kind in RuleTriggerEditorKind::iter() {
            let trigger = default_rule_trigger(kind);
            assert_eq!(rule_trigger_kind(&trigger), kind);
            assert!(!rule_trigger_kind_label(kind).is_empty());
        }
    }

    #[test]
    fn rule_condition_kind_defaults_round_trip() {
        for kind in RuleConditionEditorKind::iter() {
            let condition = default_rule_condition(kind);
            assert_eq!(rule_condition_kind(&condition), kind);
            assert!(!rule_condition_kind_label(kind).is_empty());
        }
    }

    #[test]
    fn rule_action_kind_defaults_round_trip() {
        for kind in RuleActionEditorKind::iter() {
            let action = default_rule_action(kind);
            assert_eq!(rule_action_kind(&action), kind);
            assert!(!rule_action_kind_label(kind).is_empty());
        }
    }
}
