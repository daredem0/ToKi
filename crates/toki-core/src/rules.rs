use serde::{Deserialize, Serialize};

use crate::animation::AnimationState;
use crate::entity::{EntityId, EntityKind};

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
    OnCollision {
        /// Optional entity filter. If set, trigger only fires when this entity collides.
        /// If None, fires for all collision events.
        #[serde(default)]
        entity: Option<RuleTarget>,
    },
    OnDamaged {
        /// Optional entity filter. If set, trigger only fires when this entity is damaged.
        /// If None, fires for all damage events.
        #[serde(default)]
        entity: Option<RuleTarget>,
    },
    OnDeath {
        /// Optional entity filter. If set, trigger only fires when this entity dies.
        /// If None, fires for all death events.
        #[serde(default)]
        entity: Option<RuleTarget>,
    },
    OnTrigger,
    OnInteract {
        #[serde(default)]
        mode: InteractionMode,
        /// Optional entity filter. If set, trigger only fires when this entity interacts.
        /// If None, fires for all interaction events.
        #[serde(default)]
        entity: Option<RuleTarget>,
    },
    OnTileEnter {
        /// The tile x-coordinate (in tile units, not pixels).
        x: u32,
        /// The tile y-coordinate (in tile units, not pixels).
        y: u32,
    },
    OnTileExit {
        /// The tile x-coordinate (in tile units, not pixels).
        x: u32,
        /// The tile y-coordinate (in tile units, not pixels).
        y: u32,
    },
}

impl RuleTrigger {
    /// Returns true if this trigger type provides entity context.
    ///
    /// Triggers that return true here will populate `TriggerContext` with
    /// `trigger_self` and potentially `trigger_other` entity IDs.
    pub const fn provides_context(&self) -> bool {
        matches!(
            self,
            RuleTrigger::OnCollision { .. }
                | RuleTrigger::OnDamaged { .. }
                | RuleTrigger::OnDeath { .. }
                | RuleTrigger::OnInteract { .. }
                | RuleTrigger::OnTileEnter { .. }
                | RuleTrigger::OnTileExit { .. }
        )
    }

    /// Returns the entity filter for OnCollision trigger, if any.
    pub const fn collision_entity_filter(&self) -> Option<RuleTarget> {
        match self {
            RuleTrigger::OnCollision { entity } => *entity,
            _ => None,
        }
    }

    /// Returns the entity filter for OnDamaged trigger, if any.
    pub const fn damaged_entity_filter(&self) -> Option<RuleTarget> {
        match self {
            RuleTrigger::OnDamaged { entity } => *entity,
            _ => None,
        }
    }

    /// Returns the entity filter for OnDeath trigger, if any.
    pub const fn death_entity_filter(&self) -> Option<RuleTarget> {
        match self {
            RuleTrigger::OnDeath { entity } => *entity,
            _ => None,
        }
    }

    /// Returns the interaction mode if this is an OnInteract trigger.
    pub const fn interaction_mode(&self) -> Option<InteractionMode> {
        match self {
            RuleTrigger::OnInteract { mode, .. } => Some(*mode),
            _ => None,
        }
    }

    /// Returns the entity filter for OnInteract trigger, if any.
    pub const fn interact_entity_filter(&self) -> Option<RuleTarget> {
        match self {
            RuleTrigger::OnInteract { entity, .. } => *entity,
            _ => None,
        }
    }

    /// Returns the tile coordinates for OnTileEnter or OnTileExit triggers, if any.
    pub const fn tile_coordinates(&self) -> Option<(u32, u32)> {
        match self {
            RuleTrigger::OnTileEnter { x, y } | RuleTrigger::OnTileExit { x, y } => Some((*x, *y)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuleKey {
    Up,
    Down,
    Left,
    Right,
    DebugToggle,
    Interact,
    AttackPrimary,
    AttackSecondary,
    Inventory,
    Pause,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleCondition {
    Always,
    TargetExists { target: RuleTarget },
    KeyHeld { key: RuleKey },
    EntityActive { target: RuleTarget, is_active: bool },
    /// True when the target entity's health is strictly below the threshold.
    HealthBelow { target: RuleTarget, threshold: i32 },
    /// True when the target entity's health is strictly above the threshold.
    HealthAbove { target: RuleTarget, threshold: i32 },
    /// True when the trigger_other entity from trigger context is the player.
    /// Fails safely (returns false) when no trigger context is available.
    TriggerOtherIsPlayer,
    /// True when the target entity's kind matches the specified kind.
    EntityIsKind { target: RuleTarget, kind: EntityKind },
    /// True when the trigger_other entity from trigger context has the specified kind.
    /// Fails safely (returns false) when no trigger context is available.
    TriggerOtherIsKind { kind: EntityKind },
    /// True when the target entity has the specified tag.
    EntityHasTag { target: RuleTarget, tag: String },
    /// True when the trigger_other entity from trigger context has the specified tag.
    /// Fails safely (returns false) when no trigger context is available.
    TriggerOtherHasTag { tag: String },
    /// True when the target entity's inventory contains at least `min_count` of the specified item.
    HasInventoryItem {
        target: RuleTarget,
        item_id: String,
        min_count: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleSoundChannel {
    Movement,
    Collision,
}

/// Spatial relationship required for OnInteract trigger to fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum InteractionMode {
    /// Player must be overlapping the interactable entity (strict AABB intersection).
    Overlap,
    /// Player can be adjacent to the interactable (touching at edges, 1px reach).
    #[default]
    Adjacent,
    /// Player must be facing the interactable and within reach.
    /// Uses the player's facing direction to determine valid interaction targets.
    InFront,
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
    /// Damages the target entity by the specified amount.
    /// Does not reduce health below zero. Death is handled by the game state.
    DamageEntity {
        target: RuleTarget,
        amount: i32,
    },
    /// Heals the target entity by the specified amount.
    /// Does not exceed the entity's maximum health.
    HealEntity {
        target: RuleTarget,
        amount: i32,
    },
    /// Adds the specified item to the target entity's inventory.
    /// If the item already exists, increases the count.
    AddInventoryItem {
        target: RuleTarget,
        item_id: String,
        count: u32,
    },
    /// Removes the specified item from the target entity's inventory.
    /// Removes up to the available count; never produces negative inventory.
    RemoveInventoryItem {
        target: RuleTarget,
        item_id: String,
        count: u32,
    },
    /// Sets the active state of the target entity.
    /// Inactive entities are not updated, rendered, or collidable.
    SetEntityActive {
        target: RuleTarget,
        active: bool,
    },
    /// Teleports the target entity to the specified world position instantly.
    /// Uses world coordinates (pixels), not tile coordinates.
    TeleportEntity {
        target: RuleTarget,
        position: [i32; 2],
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
