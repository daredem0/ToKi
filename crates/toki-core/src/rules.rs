use serde::{Deserialize, Serialize};
use strum::EnumIter;

use crate::animation::AnimationState;
use crate::entity::{EntityId, EntityKind};
use crate::expression::{Expression, ExpressionError};
use crate::flags::FlagValue;
use crate::ids::{DialogId, SceneId, UiLayoutId};
use crate::project_runtime::SceneTransitionEffect;
use crate::value_path::ValuePathContext;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleTrigger {
    OnStart,
    OnUpdate,
    OnPlayerMove,
    OnKey {
        key: RuleKey,
    },
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
    OnDialogComplete {
        dialog_id: DialogId,
        outcome_id: String,
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
                | RuleTrigger::OnDialogComplete { .. }
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

    pub fn dialog_completion_ids(&self) -> Option<(&str, &str)> {
        match self {
            RuleTrigger::OnDialogComplete {
                dialog_id,
                outcome_id,
            } => Some((dialog_id.as_str(), outcome_id.as_str())),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
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
    Expression {
        expression: String,
    },
    TargetExists {
        target: RuleTarget,
    },
    KeyHeld {
        key: RuleKey,
    },
    EntityActive {
        target: RuleTarget,
        is_active: bool,
    },
    /// True when the target entity's health is strictly below the threshold.
    HealthBelow {
        target: RuleTarget,
        threshold: i32,
    },
    /// True when the target entity's health is strictly above the threshold.
    HealthAbove {
        target: RuleTarget,
        threshold: i32,
    },
    /// True when the trigger_other entity from trigger context is the player.
    /// Fails safely (returns false) when no trigger context is available.
    TriggerOtherIsPlayer,
    /// True when the target entity's kind matches the specified kind.
    EntityIsKind {
        target: RuleTarget,
        kind: EntityKind,
    },
    /// True when the trigger_other entity from trigger context has the specified kind.
    /// Fails safely (returns false) when no trigger context is available.
    TriggerOtherIsKind {
        kind: EntityKind,
    },
    /// True when the target entity has the specified tag.
    EntityHasTag {
        target: RuleTarget,
        tag: String,
    },
    /// True when the trigger_other entity from trigger context has the specified tag.
    /// Fails safely (returns false) when no trigger context is available.
    TriggerOtherHasTag {
        tag: String,
    },
    /// True when the target entity's inventory contains at least `min_count` of the specified item.
    HasInventoryItem {
        target: RuleTarget,
        item_id: String,
        min_count: u32,
    },
    FlagEquals {
        flag: String,
        value: FlagValue,
    },
    FlagSet {
        flag: String,
    },
    FlagGreaterThan {
        flag: String,
        value: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuleIntSource {
    Literal(i32),
    Expression { expr: String },
}

impl RuleIntSource {
    pub fn literal(value: i32) -> Self {
        Self::Literal(value)
    }

    pub fn resolve(&self, context: ValuePathContext<'_, '_>) -> Result<i32, ExpressionError> {
        match self {
            Self::Literal(value) => Ok(*value),
            Self::Expression { expr } => match Expression::parse(expr)?.evaluate(context)? {
                crate::value_path::ResolvedValue::Int(value) => Ok(value),
                value => Err(ExpressionError::Evaluation {
                    span: crate::expression::TextSpan { start: 0, end: 0 },
                    message: format!("expected int expression result, got {:?}", value),
                }),
            },
        }
    }

    pub fn as_literal(&self) -> Option<i32> {
        match self {
            Self::Literal(value) => Some(*value),
            Self::Expression { .. } => None,
        }
    }

    pub fn expression(&self) -> Option<&str> {
        match self {
            Self::Literal(_) => None,
            Self::Expression { expr } => Some(expr.as_str()),
        }
    }

    pub fn set_literal(&mut self, value: i32) {
        *self = Self::Literal(value);
    }

    pub fn set_expression(&mut self, expr: impl Into<String>) {
        *self = Self::Expression { expr: expr.into() };
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuleBoolSource {
    Literal(bool),
    Expression { expr: String },
}

impl RuleBoolSource {
    pub fn resolve(&self, context: ValuePathContext<'_, '_>) -> Result<bool, ExpressionError> {
        match self {
            Self::Literal(value) => Ok(*value),
            Self::Expression { expr } => match Expression::parse(expr)?.evaluate(context)? {
                crate::value_path::ResolvedValue::Bool(value) => Ok(value),
                value => Err(ExpressionError::Evaluation {
                    span: crate::expression::TextSpan { start: 0, end: 0 },
                    message: format!("expected bool expression result, got {:?}", value),
                }),
            },
        }
    }

    pub fn as_literal(&self) -> Option<bool> {
        match self {
            Self::Literal(value) => Some(*value),
            Self::Expression { .. } => None,
        }
    }

    pub fn expression(&self) -> Option<&str> {
        match self {
            Self::Literal(_) => None,
            Self::Expression { expr } => Some(expr.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuleFlagValueSource {
    Literal(FlagValue),
    Expression { expr: String },
}

impl RuleFlagValueSource {
    pub fn resolve(&self, context: ValuePathContext<'_, '_>) -> Result<FlagValue, ExpressionError> {
        match self {
            Self::Literal(value) => Ok(value.clone()),
            Self::Expression { expr } => Ok(Expression::parse(expr)?
                .evaluate(context)?
                .into_flag_value()),
        }
    }

    pub fn as_literal(&self) -> Option<&FlagValue> {
        match self {
            Self::Literal(value) => Some(value),
            Self::Expression { .. } => None,
        }
    }

    pub fn expression(&self) -> Option<&str> {
        match self {
            Self::Literal(_) => None,
            Self::Expression { expr } => Some(expr.as_str()),
        }
    }

    pub fn set_literal(&mut self, value: FlagValue) {
        *self = Self::Literal(value);
    }

    pub fn set_expression(&mut self, expr: impl Into<String>) {
        *self = Self::Expression { expr: expr.into() };
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuleVec2IntSource {
    Literal([i32; 2]),
    Expression { x: RuleIntSource, y: RuleIntSource },
}

impl RuleVec2IntSource {
    pub fn literal(value: [i32; 2]) -> Self {
        Self::Literal(value)
    }

    pub fn resolve(&self, context: ValuePathContext<'_, '_>) -> Result<[i32; 2], ExpressionError> {
        match self {
            Self::Literal(value) => Ok(*value),
            Self::Expression { x, y } => Ok([x.resolve(context)?, y.resolve(context)?]),
        }
    }

    pub fn as_literal(&self) -> Option<[i32; 2]> {
        match self {
            Self::Literal(value) => Some(*value),
            Self::Expression { .. } => None,
        }
    }

    pub fn expression_components(&self) -> Option<(&RuleIntSource, &RuleIntSource)> {
        match self {
            Self::Literal(_) => None,
            Self::Expression { x, y } => Some((x, y)),
        }
    }

    pub fn expression_components_mut(
        &mut self,
    ) -> Option<(&mut RuleIntSource, &mut RuleIntSource)> {
        match self {
            Self::Literal(_) => None,
            Self::Expression { x, y } => Some((x, y)),
        }
    }

    pub fn set_literal(&mut self, value: [i32; 2]) {
        *self = Self::Literal(value);
    }

    pub fn set_expression(&mut self, x: RuleIntSource, y: RuleIntSource) {
        *self = Self::Expression { x, y };
    }
}

impl std::fmt::Display for RuleIntSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Literal(value) => write!(f, "{value}"),
            Self::Expression { expr } => write!(f, "={expr}"),
        }
    }
}

impl std::fmt::Display for RuleBoolSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Literal(value) => write!(f, "{value}"),
            Self::Expression { expr } => write!(f, "={expr}"),
        }
    }
}

impl std::fmt::Display for RuleFlagValueSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Literal(value) => write!(f, "{value:?}"),
            Self::Expression { expr } => write!(f, "={expr}"),
        }
    }
}

impl std::fmt::Display for RuleVec2IntSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Literal([x, y]) => write!(f, "[{x}, {y}]"),
            Self::Expression { x, y } => write!(f, "[{x}, {y}]"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter)]
pub enum RuleSoundChannel {
    Movement,
    Collision,
}

/// Spatial relationship required for OnInteract trigger to fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, EnumIter)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter)]
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
        velocity: RuleVec2IntSource,
    },
    Spawn {
        entity_type: RuleSpawnEntityType,
        position: RuleVec2IntSource,
    },
    DestroySelf {
        target: RuleTarget,
    },
    /// Runtime placeholder until scene-switch plumbing is integrated end-to-end.
    SwitchScene {
        scene_name: SceneId,
        spawn_point_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        transition: Option<SceneTransitionEffect>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u32>,
    },
    StartDialog {
        dialog_id: DialogId,
    },
    ShowUi {
        ui_id: UiLayoutId,
    },
    HideUi {
        ui_id: UiLayoutId,
    },
    UpdateUiBinding {
        ui_id: UiLayoutId,
        binding_key: String,
        value: RuleFlagValueSource,
    },
    /// Damages the target entity by the specified amount.
    /// Does not reduce health below zero. Death is handled by the game state.
    DamageEntity {
        target: RuleTarget,
        amount: RuleIntSource,
    },
    /// Heals the target entity by the specified amount.
    /// Does not exceed the entity's maximum health.
    HealEntity {
        target: RuleTarget,
        amount: RuleIntSource,
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
    /// Teleports the target entity to the specified tile position instantly.
    /// Uses tile coordinates (like OnTileEnter/OnTileExit), converted to pixels at runtime.
    TeleportEntity {
        target: RuleTarget,
        /// The tile x-coordinate (in tile units, not pixels).
        tile_x: RuleIntSource,
        /// The tile y-coordinate (in tile units, not pixels).
        tile_y: RuleIntSource,
    },
    SetFlag {
        flag: String,
        value: RuleFlagValueSource,
    },
    IncrementFlag {
        flag: String,
        amount: RuleIntSource,
    },
    ClearFlag {
        flag: String,
    },
    SaveGame {
        slot: u8,
    },
    LoadGame {
        slot: u8,
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
    #[serde(default, skip_serializing_if = "is_false")]
    pub log_enabled: bool,
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

const fn is_false(value: &bool) -> bool {
    !*value
}
