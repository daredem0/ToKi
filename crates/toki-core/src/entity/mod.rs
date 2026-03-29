//! Entity system - types, definitions, and management.
//!
//! This module is organized into:
//! - `types`: Core entity types (Entity, EntityKind, EntityAttributes, etc.)
//! - `definition`: Entity definition types for JSON deserialization
//! - `manager`: EntityManager for runtime entity lifecycle

#![allow(unused_imports)]

mod builder;
mod category;
mod components;
mod definition;
mod manager;
mod model;
mod wire;

// Re-export all public types
pub use builder::EntityBuilder;
pub use category::{default_category_for_kind, runtime_entity_kind_for_category};
pub use components::{
    EntityComponentStore, EntityOptionalComponents, EntitySpawnBundle, Inventory, PickupDef,
    PrimaryProjectileDef, ProjectileState,
};
pub use definition::{
    AnimationClipDef, AnimationsDef, AttributesDef, AudioDef, CollisionDef, EntityDefinition,
    EntityDefinitionError, RenderingDef,
};
pub use manager::EntityManager;
pub use model::{
    AiBehavior, AiConfig, ControlRole, Entity, EntityAttributes, EntityAudioComponent,
    EntityAudioSettings, EntityBehavior, EntityFootprint, EntityGameplay, EntityGrounding,
    EntityId, EntityKind, EntityRendering, EntityStats, MovementProfile,
    MovementSoundTrigger, StaticObjectRenderDef, ATTACK_POWER_STAT_ID, HEALTH_STAT_ID,
};
pub use wire::{EntityAttributesWire, EntityWire, StoredEntity};
