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

use self::engine::{RuleEngine, RuleEngineContext};
use crate::animation::AnimationState;
use crate::entity::EntityId;
use crate::flags::FlagValue;
use crate::ids::{DialogId, SceneId};
use crate::project_runtime::SceneTransitionEffect;
use crate::rules::{RuleSet, RuleSpawnEntityType};

// Re-export submodules
pub mod events;

// Private implementation modules
mod actions;
mod animations;
mod collectors;
mod commands;
mod engine;
mod evaluation;
mod reactive;
mod spawning;
mod tiles;
mod transitions;

// Re-export event types for public use
pub use events::{
    CollisionEvent, DamageEvent, DeathEvent, DialogCompletionEvent, InteractionEvent,
    InteractionSpatial, TileTransitionEvent,
};

use super::{AudioChannel, AudioEvent, GameState};

pub(super) struct AppliedRuleCommandResult {
    pub(super) pending_animations: Vec<(EntityId, AnimationState)>,
    pub(super) pending_scene_switch: Option<PendingSceneSwitch>,
    pub(super) pending_persistence: Option<crate::events::PersistenceRequest>,
}

pub struct RuleSystem;

impl RuleSystem {
    pub(in crate::game) fn begin_frame(state: &mut GameState) {
        state.runtime.rules.frame_collisions.clear();
        state.runtime.rules.frame_damage_events.clear();
        state.runtime.rules.frame_death_events.clear();
        state.runtime.rules.frame_interactions.clear();
        state.runtime.rules.frame_tile_transitions.clear();
    }

    pub(in crate::game) fn collect_frame_commands(
        state: &mut GameState,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        if !state.runtime.rules.started {
            state.collect_rule_commands_for_trigger(
                crate::rules::RuleTrigger::OnStart,
                command_buffer,
            );
            state.runtime.rules.started = true;
        }
        state
            .collect_rule_commands_for_trigger(crate::rules::RuleTrigger::OnUpdate, command_buffer);
        state.collect_rule_commands_for_key_triggers(command_buffer);
    }

    pub(in crate::game) fn apply_commands(
        state: &mut GameState,
        commands: Vec<RuleCommand>,
        result: &mut crate::events::GameUpdateResult<AudioEvent>,
        tilemap: &crate::assets::tilemap::TileMap,
    ) -> AppliedRuleCommandResult {
        state.apply_rule_commands(commands, result, tilemap)
    }

    pub(in crate::game) fn collect_reactive_commands(
        state: &mut GameState,
        player_moved: bool,
        tilemap: &crate::assets::tilemap::TileMap,
        atlas: &crate::assets::atlas::AtlasMeta,
    ) -> Vec<RuleCommand> {
        state.collect_reactive_rule_commands(player_moved, tilemap, atlas)
    }

    pub fn set_rules(state: &mut GameState, rules: RuleSet) {
        state.scene.active_rules = rules;
        state.runtime.rules = RuleRuntimeState::default();
    }

    pub fn record_dialog_completion(
        state: &mut GameState,
        dialog_id: impl Into<crate::DialogId>,
        outcome_id: impl Into<String>,
    ) {
        state
            .runtime
            .rules
            .frame_dialog_completions
            .push(DialogCompletionEvent {
                dialog_id: dialog_id.into(),
                outcome_id: outcome_id.into(),
            });
    }

    pub fn rule_velocity(state: &GameState, entity_id: EntityId) -> Option<glam::IVec2> {
        state.runtime.rules.velocities.get(&entity_id).copied()
    }

    pub fn set_rule_velocity(state: &mut GameState, entity_id: EntityId, velocity: glam::IVec2) {
        state.runtime.rules.velocities.insert(entity_id, velocity);
    }
}

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
    /// Dialog completion events that occurred outside the gameplay tick.
    pub(super) frame_dialog_completions: Vec<DialogCompletionEvent>,
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
        scene_name: SceneId,
        spawn_point_id: String,
        transition: Option<SceneTransitionEffect>,
        duration_ms: Option<u32>,
    },
    StartDialog {
        dialog_id: DialogId,
        context: crate::dialog::DialogRuntimeContext,
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
    SetFlag {
        flag: String,
        value: FlagValue,
    },
    IncrementFlag {
        flag: String,
        amount: i32,
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

/// A pending scene switch.
pub(super) type PendingSceneSwitch = crate::events::SceneSwitchRequest;
/// A pending dialog start request.
pub(super) type PendingDialogStart = crate::events::DialogStartRequest;

impl GameState {
    pub(super) fn with_rule_engine<R>(
        &mut self,
        build: impl FnOnce(&mut RuleEngine<'_>) -> R,
    ) -> R {
        let held_keys = self.all_held_keys();
        let mut engine = RuleEngine::new(
            RuleEngineContext {
                entity_manager: &self.world.entity_manager,
                player_id: self.world.player_id(),
                held_keys: &held_keys,
                game_flags: &self.progress.game_flags,
                rules: &self.scene.active_rules,
            },
            &mut self.runtime.rules,
        );
        build(&mut engine)
    }
}
