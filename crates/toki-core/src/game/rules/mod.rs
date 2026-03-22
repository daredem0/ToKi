//! Rule system for the game engine.
//!
//! This module contains the rule evaluation engine, which processes
//! data-driven rules that respond to game events like collisions,
//! interactions, damage, and tile transitions.
//!
//! # Module Structure
//!
//! - `events`: Event types that trigger rules (CollisionEvent, DamageEvent, etc.)
//! - `evaluation`: Condition evaluation logic
//! - `collectors`: Rule command collection for different triggers
//! - `actions`: Action buffering (converting actions to commands)
//! - `commands`: Command application (applying buffered commands to state)
//! - `transitions`: Tile transition detection and handling
//! - `tiles`: Tile overlap utilities
//! - `spawning`: Entity spawning from rules
//! - `target`: Rule target resolution
//! - `animations`: Animation application

use std::collections::{HashMap, HashSet};

use crate::animation::AnimationState;
use crate::entity::EntityId;
use crate::rules::{Rule, RuleSet, RuleSpawnEntityType};

// Re-export submodules
pub mod events;

// Private implementation modules
mod actions;
mod animations;
mod collectors;
mod commands;
mod evaluation;
mod spawning;
mod target;
mod tiles;
mod transitions;

// Re-export event types for public use
pub use events::{
    CollisionEvent, DamageEvent, DeathEvent, InteractionEvent, InteractionSpatial,
    TileTransitionEvent,
};

use super::{AudioChannel, AudioEvent, GameState};

/// Runtime state for the rule system.
#[derive(Debug, Default)]
pub(super) struct RuleRuntimeState {
    pub(super) started: bool,
    pub(super) fired_once_rules: HashSet<String>,
    pub(super) velocities: HashMap<EntityId, glam::IVec2>,
    /// Collision events that occurred this frame.
    pub(super) frame_collisions: Vec<CollisionEvent>,
    /// Damage events that occurred this frame.
    pub(super) frame_damage_events: Vec<DamageEvent>,
    /// Death events that occurred this frame.
    pub(super) frame_death_events: Vec<DeathEvent>,
    /// Interaction events that occurred this frame.
    pub(super) frame_interactions: Vec<InteractionEvent>,
    /// Previous tile positions for entities, used to detect tile transitions.
    /// Key: EntityId, Value: (tile_x, tile_y)
    pub(super) entity_tile_positions: HashMap<EntityId, (u32, u32)>,
    /// Tile transition events that occurred this frame.
    pub(super) frame_tile_transitions: Vec<TileTransitionEvent>,
}

/// A buffered command to be executed by the rule system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RuleCommand {
    PlaySound {
        channel: AudioChannel,
        sound_id: String,
    },
    PlayMusic {
        track_id: String,
    },
    SetVelocity {
        entity_id: EntityId,
        velocity: glam::IVec2,
    },
    PlayAnimation {
        entity_id: EntityId,
        state: AnimationState,
    },
    Spawn {
        entity_type: RuleSpawnEntityType,
        position: glam::IVec2,
    },
    DestroySelf {
        entity_id: EntityId,
    },
    SwitchScene {
        scene_name: String,
        spawn_point_id: String,
    },
    DamageEntity {
        entity_id: EntityId,
        amount: i32,
    },
    HealEntity {
        entity_id: EntityId,
        amount: i32,
    },
    AddInventoryItem {
        entity_id: EntityId,
        item_id: String,
        count: u32,
    },
    RemoveInventoryItem {
        entity_id: EntityId,
        item_id: String,
        count: u32,
    },
    SetEntityActive {
        entity_id: EntityId,
        active: bool,
    },
    TeleportEntity {
        entity_id: EntityId,
        tile_x: u32,
        tile_y: u32,
    },
}

/// A pending scene switch (scene_name, spawn_point_id).
pub(super) type PendingSceneSwitch = (String, String);

// Public API on GameState for rule management
impl GameState {
    pub fn rules(&self) -> &RuleSet {
        &self.rules
    }

    pub fn rules_mut(&mut self) -> &mut RuleSet {
        &mut self.rules
    }

    pub fn set_rules(&mut self, rules: RuleSet) {
        self.rules = rules;
        self.rule_runtime = RuleRuntimeState::default();
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.rules.push(rule);
    }

    /// Gets the rule-assigned velocity for an entity, if any.
    /// Used by tests to verify rule actions.
    pub fn get_rule_velocity(&self, entity_id: EntityId) -> Option<glam::IVec2> {
        self.rule_runtime.velocities.get(&entity_id).copied()
    }

    /// Sets the rule-assigned velocity for an entity directly.
    /// Used by tests to set up specific scenarios.
    pub fn set_rule_velocity(&mut self, entity_id: EntityId, velocity: glam::IVec2) {
        self.rule_runtime.velocities.insert(entity_id, velocity);
    }
}
