use serde::{Deserialize, Serialize};

use crate::animation::AnimationState;
use crate::entity::EntityId;

/// Context provided by triggers that involve entity interactions.
///
/// # Architecture Note (for Phase 1.5B+ implementers)
///
/// This struct carries the "who" for triggers like `OnCollision`, `OnDamaged`, `OnDeath`.
/// - `trigger_self`: The primary subject (e.g., the entity whose rule fired, the victim)
/// - `trigger_other`: The secondary entity (e.g., the collider, the attacker)
///
/// Rules can use `RuleTarget::TriggerSelf` and `RuleTarget::TriggerOther` to reference
/// these entities in conditions and actions. These targets are only valid when the
/// active trigger provides context - validation should reject their use otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TriggerContext {
    /// The primary subject of the trigger (e.g., victim, rule-owning entity).
    pub trigger_self: Option<EntityId>,
    /// The secondary entity involved (e.g., attacker, collider).
    pub trigger_other: Option<EntityId>,
}

impl TriggerContext {
    /// Creates an empty context (no entities involved).
    pub const fn empty() -> Self {
        Self {
            trigger_self: None,
            trigger_other: None,
        }
    }

    /// Creates a context with both entities specified.
    pub const fn with_pair(trigger_self: EntityId, trigger_other: EntityId) -> Self {
        Self {
            trigger_self: Some(trigger_self),
            trigger_other: Some(trigger_other),
        }
    }

    /// Creates a context with only the primary subject (no secondary entity).
    pub const fn with_self_only(trigger_self: EntityId) -> Self {
        Self {
            trigger_self: Some(trigger_self),
            trigger_other: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleTrigger {
    OnStart,
    OnUpdate,
    OnPlayerMove,
    OnKey { key: RuleKey },
    OnCollision,
    OnDamaged,
    OnDeath,
    OnTrigger,
}

impl RuleTrigger {
    /// Returns true if this trigger type provides entity context.
    ///
    /// Triggers that return true here will populate `TriggerContext` with
    /// `trigger_self` and potentially `trigger_other` entity IDs.
    pub const fn provides_context(&self) -> bool {
        matches!(
            self,
            RuleTrigger::OnCollision | RuleTrigger::OnDamaged | RuleTrigger::OnDeath
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuleKey {
    Up,
    Down,
    Left,
    Right,
    DebugToggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleCondition {
    Always,
    TargetExists { target: RuleTarget },
    KeyHeld { key: RuleKey },
    EntityActive { target: RuleTarget, is_active: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleSoundChannel {
    Movement,
    Collision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleTarget {
    /// The player entity.
    Player,
    /// A specific entity by ID.
    Entity(EntityId),
    /// The entity that owns the rule (only valid for entity-owned rules).
    /// For scene-owned rules, this target is invalid.
    RuleOwner,
    /// The primary subject of the trigger context (e.g., victim, rule-owning entity in collision).
    /// Only valid when the active trigger provides context.
    TriggerSelf,
    /// The secondary entity from trigger context (e.g., attacker, collider).
    /// Only valid when the active trigger provides context.
    TriggerOther,
}

impl RuleTarget {
    /// Returns true if this target requires trigger context to resolve.
    pub const fn requires_trigger_context(&self) -> bool {
        matches!(self, RuleTarget::TriggerSelf | RuleTarget::TriggerOther)
    }

    /// Returns true if this target requires an entity owner (not valid for scene rules).
    pub const fn requires_entity_owner(&self) -> bool {
        matches!(self, RuleTarget::RuleOwner)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleSpawnEntityType {
    PlayerLikeNpc,
    Npc,
    Item,
    Decoration,
    Trigger,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleAction {
    PlaySound {
        channel: RuleSoundChannel,
        sound_id: String,
    },
    PlayMusic {
        track_id: String,
    },
    PlayAnimation {
        target: RuleTarget,
        state: AnimationState,
    },
    SetVelocity {
        target: RuleTarget,
        velocity: [i32; 2],
    },
    Spawn {
        entity_type: RuleSpawnEntityType,
        position: [i32; 2],
    },
    DestroySelf {
        target: RuleTarget,
    },
    /// Runtime placeholder until scene-switch plumbing is integrated end-to-end.
    SwitchScene {
        scene_name: String,
        spawn_point_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub once: bool,
    pub trigger: RuleTrigger,
    #[serde(default)]
    pub conditions: Vec<RuleCondition>,
    #[serde(default)]
    pub actions: Vec<RuleAction>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleSet {
    #[serde(default)]
    pub rules: Vec<Rule>,
}

fn default_true() -> bool {
    true
}
