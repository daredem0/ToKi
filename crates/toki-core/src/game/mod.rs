use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ai::AiSystem;
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::entity::{Entity, EntityDefinition, EntityId, EntityManager, MovementProfile};
use crate::events::{GameEvent, GameUpdateResult};
use crate::rules::{RuleSet, RuleTrigger};
use crate::scene_manager::SceneManager;

mod ai_runtime;
mod animation;
mod combat;
mod input;
mod input_state;
mod interaction;
mod inventory;
mod movement;
mod render_queries;
pub(crate) mod rules;
mod scene;
mod stat_effects;
mod transition;

#[cfg(test)]
mod rules_tests;

// Re-export event types for external use
pub use render_queries::GroundShadow;
pub use rules::{
    CollisionEvent, DamageEvent, DeathEvent, InteractionEvent, InteractionSpatial,
    TileTransitionEvent,
};

/// Default timestep in milliseconds for fixed 60 FPS game logic.
/// Used as the baseline for delta time scaling.
pub const DEFAULT_TIMESTEP_MS: f32 = 16.667;

/// Core input keys abstraction (platform-independent)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputKey {
    Up,
    Down,
    Left,
    Right,
    DebugToggle, // F4 key for toggling debug rendering
    Interact,
    AttackPrimary,
    AttackSecondary,
    Inventory,
    Pause,
}

/// Profile-scoped action buttons that can be mapped independently from movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputAction {
    Primary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityHealthBar {
    pub entity_id: EntityId,
    pub position: glam::IVec2,
    pub size: glam::UVec2,
    pub current: i32,
    pub max: i32,
}

/// Audio events that can be triggered by game logic
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioEvent {
    /// Play a one-shot sound effect on a logical channel.
    PlaySound {
        channel: AudioChannel,
        sound_id: String,
        source_position: Option<glam::IVec2>,
        hearing_radius: Option<u32>,
    },
    /// Start background music
    BackgroundMusic(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioChannel {
    Movement,
    Collision,
}

impl GameEvent for AudioEvent {}

/// Core game state that manages entities, scenes, input, and game logic.
///
/// This is platform-independent and contains pure game logic without
/// any runtime or windowing dependencies.
#[derive(Debug, Serialize, Deserialize)]
pub struct GameState {
    /// Scene manager holding all scenes
    scene_manager: SceneManager,

    /// Entity manager for all game objects in the current scene
    entity_manager: EntityManager,

    /// Authored entity definitions available to scene player-entry instantiation.
    #[serde(default)]
    entity_definitions: HashMap<String, EntityDefinition>,

    /// Player entity ID for quick access
    player_id: Option<EntityId>,

    /// Runtime input bookkeeping for keys, per-profile movement, and debounced actions.
    #[serde(default)]
    input_state: InputRuntimeState,

    /// Debug rendering flags
    #[serde(default)]
    debug_collision_rendering: bool,

    /// AI system for NPC behavior
    #[serde(skip, default)]
    ai_system: AiSystem,

    /// Data-driven gameplay rules evaluated each frame.
    #[serde(default)]
    rules: RuleSet,

    /// Runtime-only rule execution state.
    #[serde(skip, default)]
    rule_runtime: RuleRuntimeState,

    /// Pending generic stat changes gathered during update and resolved centrally.
    #[serde(skip, default)]
    pending_stat_changes: Vec<StatChangeRequest>,

    /// Entities that died and need to be despawned after death events are processed.
    #[serde(skip, default)]
    pending_despawns: Vec<EntityId>,
}

use input_state::InputRuntimeState;
use rules::RuleRuntimeState;
use stat_effects::StatChangeRequest;

impl GameState {
    fn effective_movement_profile(entity: &Entity) -> MovementProfile {
        entity.effective_movement_profile()
    }

    /// Update game state by one tick
    pub fn update(
        &mut self,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> GameUpdateResult<AudioEvent> {
        let mut result = GameUpdateResult::new();
        let mut rule_commands = Vec::new();
        self.rule_runtime.frame_collisions.clear();
        self.rule_runtime.frame_damage_events.clear();
        self.rule_runtime.frame_death_events.clear();
        self.rule_runtime.frame_interactions.clear();
        self.rule_runtime.frame_tile_transitions.clear();

        if !self.rule_runtime.started {
            self.collect_rule_commands_for_trigger(RuleTrigger::OnStart, &mut rule_commands);
            self.rule_runtime.started = true;
        }
        self.collect_rule_commands_for_trigger(RuleTrigger::OnUpdate, &mut rule_commands);
        self.collect_rule_commands_for_key_triggers(&mut rule_commands);
        let (mut pending_rule_animations, mut pending_scene_switch) =
            self.apply_rule_commands(rule_commands, &mut result, tilemap);

        let initial_player_position = self
            .player_id
            .and_then(|player_id| self.entity_manager.get_entity(player_id))
            .map(|entity| entity.position)
            .unwrap_or(glam::IVec2::ZERO);

        let input_result = self.process_input(world_bounds, tilemap, atlas);
        result.player_moved = input_result.player_moved;
        result.add_events(input_result.events);

        if self.apply_rule_velocities(world_bounds, tilemap, atlas, &mut result) {
            result.player_moved = true;
        }

        let intended_player_delta = self
            .player_id
            .and_then(|player_id| self.entity_manager.get_entity(player_id))
            .map(|entity| self.held_keys_for_profile(Self::effective_movement_profile(entity)))
            .map(|keys| Self::movement_delta_from_keys(&keys))
            .unwrap_or(glam::IVec2::ZERO);

        // Pick moving or idle animation
        if let Some(player_entity) = self.entity_manager.get_player_mut() {
            if let Some(animation_controller) = &mut player_entity.attributes.animation_controller {
                if !Self::action_animation_locks_locomotion(animation_controller) {
                    let actual_player_delta = player_entity.position - initial_player_position;
                    let player_delta = if actual_player_delta == glam::IVec2::ZERO {
                        intended_player_delta
                    } else {
                        actual_player_delta
                    };
                    // Use intent (direction keys held) for animation, not actual pixel movement.
                    // This ensures walking animation plays during sub-pixel accumulation.
                    let is_trying_to_move = intended_player_delta != glam::IVec2::ZERO;
                    let desired_player_animation = Self::resolve_animation_state(
                        animation_controller,
                        is_trying_to_move,
                        player_delta,
                    );
                    if animation_controller.current_clip_state != desired_player_animation {
                        tracing::debug!(
                            "Changing clip from  {:?} to {:?}",
                            animation_controller.current_clip_state,
                            desired_player_animation
                        );
                        animation_controller.play(desired_player_animation);
                    }
                }
            }
        }

        self.process_profile_actions();
        self.update_projectiles(tilemap, atlas);
        self.collect_overlapping_pickups();
        self.collect_interaction_events();
        self.resolve_pending_stat_changes();

        // Update NPC AI
        self.update_npc_ai(world_bounds, tilemap, atlas, &mut result);

        // Detect tile transitions after all movement is complete
        self.detect_tile_transitions(tilemap);

        let reactive_rule_commands =
            self.collect_reactive_rule_commands(result.player_moved, tilemap, atlas);
        let (mut reactive_animations, reactive_scene_switch) =
            self.apply_rule_commands(reactive_rule_commands, &mut result, tilemap);
        if pending_scene_switch.is_none() {
            pending_scene_switch = reactive_scene_switch;
        }
        pending_rule_animations.append(&mut reactive_animations);

        self.apply_rule_animations(pending_rule_animations);

        // Despawn entities that died after death events have been processed
        self.flush_pending_despawns();

        // Update entity animation timing and emit animation-loop-based movement sounds.
        let completed_animation_loops = self.entity_manager.update_animations(17.0);
        for (entity_id, completed_loops) in completed_animation_loops {
            self.emit_animation_loop_movement_audio(entity_id, completed_loops, &mut result);
        }

        if let Some((scene_name, spawn_point_id)) = pending_scene_switch {
            result.request_scene_switch(scene_name, spawn_point_id);
        }

        result
    }

    /// Update game state with delta time scaling.
    ///
    /// This method scales movement speed proportionally to the elapsed time,
    /// allowing for variable frame rate game logic while maintaining consistent
    /// perceived movement speed.
    ///
    /// # Arguments
    /// * `delta_ms` - Elapsed time since last update in milliseconds
    /// * `world_bounds` - World boundary constraints
    /// * `tilemap` - Current tilemap for collision detection
    /// * `atlas` - Atlas metadata for tile properties
    pub fn update_with_delta(
        &mut self,
        delta_ms: f32,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> GameUpdateResult<AudioEvent> {
        let time_scale = delta_ms / DEFAULT_TIMESTEP_MS;
        self.update_internal(time_scale, delta_ms, world_bounds, tilemap, atlas)
    }

    /// Internal update implementation that accepts time scaling parameters.
    fn update_internal(
        &mut self,
        time_scale: f32,
        animation_delta_ms: f32,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> GameUpdateResult<AudioEvent> {
        let mut result = GameUpdateResult::new();
        let mut rule_commands = Vec::new();
        self.rule_runtime.frame_collisions.clear();
        self.rule_runtime.frame_damage_events.clear();
        self.rule_runtime.frame_death_events.clear();
        self.rule_runtime.frame_interactions.clear();
        self.rule_runtime.frame_tile_transitions.clear();

        if !self.rule_runtime.started {
            self.collect_rule_commands_for_trigger(RuleTrigger::OnStart, &mut rule_commands);
            self.rule_runtime.started = true;
        }
        self.collect_rule_commands_for_trigger(RuleTrigger::OnUpdate, &mut rule_commands);
        self.collect_rule_commands_for_key_triggers(&mut rule_commands);
        let (mut pending_rule_animations, mut pending_scene_switch) =
            self.apply_rule_commands(rule_commands, &mut result, tilemap);

        let initial_player_position = self
            .player_id
            .and_then(|player_id| self.entity_manager.get_entity(player_id))
            .map(|entity| entity.position)
            .unwrap_or(glam::IVec2::ZERO);

        let input_result = self.process_input_scaled(world_bounds, tilemap, atlas, time_scale);
        result.player_moved = input_result.player_moved;
        result.add_events(input_result.events);

        if self.apply_rule_velocities(world_bounds, tilemap, atlas, &mut result) {
            result.player_moved = true;
        }

        let intended_player_delta = self
            .player_id
            .and_then(|player_id| self.entity_manager.get_entity(player_id))
            .map(|entity| self.held_keys_for_profile(Self::effective_movement_profile(entity)))
            .map(|keys| Self::movement_delta_from_keys(&keys))
            .unwrap_or(glam::IVec2::ZERO);

        self.update_player_animation(initial_player_position, intended_player_delta);
        self.process_profile_actions();
        self.update_projectiles(tilemap, atlas);
        self.collect_overlapping_pickups();
        self.collect_interaction_events();
        self.resolve_pending_stat_changes();

        // Update NPC AI
        self.update_npc_ai(world_bounds, tilemap, atlas, &mut result);

        // Detect tile transitions after all movement is complete
        self.detect_tile_transitions(tilemap);

        let reactive_rule_commands =
            self.collect_reactive_rule_commands(result.player_moved, tilemap, atlas);
        let (mut reactive_animations, reactive_scene_switch) =
            self.apply_rule_commands(reactive_rule_commands, &mut result, tilemap);
        if pending_scene_switch.is_none() {
            pending_scene_switch = reactive_scene_switch;
        }
        pending_rule_animations.append(&mut reactive_animations);

        self.apply_rule_animations(pending_rule_animations);

        // Despawn entities that died after death events have been processed
        self.flush_pending_despawns();

        // Update entity animation timing with actual delta
        let completed_animation_loops = self.entity_manager.update_animations(animation_delta_ms);
        for (entity_id, completed_loops) in completed_animation_loops {
            self.emit_animation_loop_movement_audio(entity_id, completed_loops, &mut result);
        }

        if let Some((scene_name, spawn_point_id)) = pending_scene_switch {
            result.request_scene_switch(scene_name, spawn_point_id);
        }

        result
    }

    /// Helper to update player animation based on movement intent.
    fn update_player_animation(
        &mut self,
        initial_player_position: glam::IVec2,
        intended_player_delta: glam::IVec2,
    ) {
        let Some(player_entity) = self.entity_manager.get_player_mut() else {
            return;
        };
        let Some(animation_controller) = &mut player_entity.attributes.animation_controller else {
            return;
        };
        if Self::action_animation_locks_locomotion(animation_controller) {
            return;
        }

        let actual_player_delta = player_entity.position - initial_player_position;
        let player_delta = if actual_player_delta == glam::IVec2::ZERO {
            intended_player_delta
        } else {
            actual_player_delta
        };
        let is_trying_to_move = intended_player_delta != glam::IVec2::ZERO;
        let desired_player_animation =
            Self::resolve_animation_state(animation_controller, is_trying_to_move, player_delta);
        if animation_controller.current_clip_state != desired_player_animation {
            tracing::debug!(
                "Changing clip from  {:?} to {:?}",
                animation_controller.current_clip_state,
                desired_player_animation
            );
            animation_controller.play(desired_player_animation);
        }
    }

    /// Update NPC AI using the AI system
    fn update_npc_ai(
        &mut self,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
        result: &mut GameUpdateResult<AudioEvent>,
    ) {
        let ai_updates = self.ai_system.update(
            &self.entity_manager,
            self.player_id,
            world_bounds,
            tilemap,
            atlas,
        );
        let effects = self.ai_runtime_applier().apply_updates(ai_updates);
        for (entity_id, movement_distance) in effects.movement_audio {
            self.emit_entity_movement_audio(entity_id, movement_distance, result);
        }
    }
}
