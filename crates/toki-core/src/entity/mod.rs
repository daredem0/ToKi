//! Entity system - types, definitions, and management.
//!
//! This module is organized into:
//! - `types`: Core entity types (Entity, EntityKind, EntityAttributes, etc.)
//! - `definition`: Entity definition types for JSON deserialization
//! - `manager`: EntityManager for runtime entity lifecycle

#![allow(unused_imports)]

mod builder;
mod category;
mod definition;
mod manager;
mod types;

// Re-export all public types
pub use builder::EntityBuilder;
pub use category::{default_category_for_kind, runtime_entity_kind_for_category};
pub use definition::{
    AnimationClipDef, AnimationsDef, AttributesDef, AudioDef, CollisionDef, EntityDefinition,
    EntityDefinitionError, RenderingDef,
};
pub use manager::EntityManager;
pub use types::{
    AiBehavior, AiConfig, ControlRole, Entity, EntityAttributes, EntityAudioComponent,
    EntityAudioSettings, EntityFootprint, EntityGrounding, EntityId, EntityKind, EntityStats,
    Inventory, MovementProfile, MovementSoundTrigger, PickupDef, PrimaryProjectileDef,
    ProjectileState, StaticObjectRenderDef, ATTACK_POWER_STAT_ID, HEALTH_STAT_ID,
};
