//! Event types for the rule system.
//!
//! Contains collision, damage, death, interaction, and tile transition events
//! that trigger rule evaluation.

use crate::entity::EntityId;

/// A collision event between an entity and another entity or the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionEvent {
    /// The entity that was moving/checking collision.
    pub entity_a: EntityId,
    /// The entity that was collided with, if entity-entity collision.
    /// `None` for tile/world collisions.
    pub entity_b: Option<EntityId>,
}

/// A damage event recording who was damaged and by whom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageEvent {
    /// The entity that received damage.
    pub victim: EntityId,
    /// The entity that caused the damage, if known.
    pub attacker: Option<EntityId>,
}

/// A death event recording who died and who caused it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathEvent {
    /// The entity that died.
    pub victim: EntityId,
    /// The entity that caused the death, if known.
    pub attacker: Option<EntityId>,
}

/// The spatial relationship between player and interactable when interaction occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionSpatial {
    /// Player was overlapping the interactable (strict AABB intersection).
    Overlap,
    /// Player was adjacent to the interactable (within reach but not overlapping).
    Adjacent,
    /// Player was facing the interactable and within reach.
    InFront,
}

/// An interaction event recording when the player interacts with an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractionEvent {
    /// The entity that initiated the interaction (player).
    pub interactor: EntityId,
    /// The entity being interacted with.
    pub interactable: EntityId,
    /// The spatial relationship when interaction occurred.
    pub spatial: InteractionSpatial,
}

/// A tile transition event recording when an entity enters or exits a specific tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileTransitionEvent {
    /// The entity that entered or exited the tile.
    pub entity_id: EntityId,
    /// The tile x-coordinate.
    pub tile_x: u32,
    /// The tile y-coordinate.
    pub tile_y: u32,
    /// Whether this is an enter (true) or exit (false) event.
    pub is_enter: bool,
}
