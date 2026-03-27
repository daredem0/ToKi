use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::ai::AiSystem;
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::entity::{Entity, EntityDefinition, EntityId, EntityManager, MovementProfile};
use crate::events::{GameEvent, GameUpdateResult};
use crate::flags::{FlagValue, GameFlags};
use crate::project_runtime::ProjectFlagDefinition;
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
const MAX_AI_FIXED_STEPS_PER_UPDATE: usize = 10;

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

    /// Runtime-only accumulator for feeding fixed-step AI updates in delta mode.
    #[serde(skip, default)]
    ai_delta_accumulator_ms: f32,

    /// Persistent game flags for authored progression and narrative state.
    #[serde(default)]
    game_flags: GameFlags,

    /// Total accumulated play time for stable save-slot metadata.
    #[serde(default)]
    play_time_ms: u64,

    /// Authored scene entities that should persist through save/load, tracked even when removed.
    #[serde(skip, default)]
    persistent_scene_entities: HashSet<(String, crate::entity::EntityId)>,
}

use input_state::InputRuntimeState;
use rules::RuleRuntimeState;
use stat_effects::StatChangeRequest;

impl GameState {
    pub fn game_flags(&self) -> &GameFlags {
        &self.game_flags
    }

    pub fn flag(&self, flag: &str) -> Option<&FlagValue> {
        self.game_flags.get(flag)
    }

    pub fn set_flag(&mut self, flag: impl Into<String>, value: FlagValue) {
        self.game_flags.set(flag, value);
    }

    pub fn clear_flag(&mut self, flag: &str) -> bool {
        self.game_flags.clear(flag)
    }

    pub fn increment_flag(&mut self, flag: impl Into<String>, amount: i32) -> bool {
        self.game_flags.increment(flag, amount)
    }

    pub fn play_time_ms(&self) -> u64 {
        self.play_time_ms
    }

    pub fn apply_flag_defaults(&mut self, declarations: &[ProjectFlagDefinition]) {
        for declaration in declarations {
            let flag = declaration.id.trim();
            if flag.is_empty() || self.game_flags.is_set(flag) {
                continue;
            }
            self.game_flags
                .set(flag.to_string(), declaration.default_value.clone());
        }
    }

    pub fn persistent_scene_entity_keys(&self) -> Vec<(String, crate::entity::EntityId)> {
        let mut keys = self.persistent_scene_entities.iter().cloned().collect::<Vec<_>>();
        keys.sort();
        keys
    }

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
        self.play_time_ms = self
            .play_time_ms
            .saturating_add(DEFAULT_TIMESTEP_MS.round() as u64);
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
        let (mut pending_rule_animations, mut pending_scene_switch, _, mut pending_persistence) =
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
        self.update_npc_ai_fixed(world_bounds, tilemap, atlas, &mut result);

        // Detect tile transitions after all movement is complete
        self.detect_tile_transitions(tilemap);

        let reactive_rule_commands =
            self.collect_reactive_rule_commands(result.player_moved, tilemap, atlas);
        let (mut reactive_animations, reactive_scene_switch, _, reactive_persistence) =
            self.apply_rule_commands(reactive_rule_commands, &mut result, tilemap);
        if pending_scene_switch.is_none() {
            pending_scene_switch = reactive_scene_switch;
        }
        if pending_persistence.is_none() {
            pending_persistence = reactive_persistence;
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

        if let Some(request) = pending_scene_switch {
            result.request_scene_switch(
                request.scene_name,
                request.spawn_point_id,
                request.transition,
                request.duration_ms,
            );
        }
        if let Some(crate::events::PersistenceRequest::SaveSlot { slot }) = pending_persistence {
            result.request_save_slot(slot);
        } else if let Some(crate::events::PersistenceRequest::LoadSlot { slot }) =
            pending_persistence
        {
            result.request_load_slot(slot);
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
        self.play_time_ms = self
            .play_time_ms
            .saturating_add(animation_delta_ms.max(0.0).round() as u64);
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
        let (mut pending_rule_animations, mut pending_scene_switch, _, mut pending_persistence) =
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

        // Update NPC AI on a fixed cadence so behavior stays consistent across frame rates.
        self.update_npc_ai_with_delta(
            animation_delta_ms,
            world_bounds,
            tilemap,
            atlas,
            &mut result,
        );

        // Detect tile transitions after all movement is complete
        self.detect_tile_transitions(tilemap);

        let reactive_rule_commands =
            self.collect_reactive_rule_commands(result.player_moved, tilemap, atlas);
        let (mut reactive_animations, reactive_scene_switch, _, reactive_persistence) =
            self.apply_rule_commands(reactive_rule_commands, &mut result, tilemap);
        if pending_scene_switch.is_none() {
            pending_scene_switch = reactive_scene_switch;
        }
        if pending_persistence.is_none() {
            pending_persistence = reactive_persistence;
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

        if let Some(request) = pending_scene_switch {
            result.request_scene_switch(
                request.scene_name,
                request.spawn_point_id,
                request.transition,
                request.duration_ms,
            );
        }
        if let Some(crate::events::PersistenceRequest::SaveSlot { slot }) = pending_persistence {
            result.request_save_slot(slot);
        } else if let Some(crate::events::PersistenceRequest::LoadSlot { slot }) =
            pending_persistence
        {
            result.request_load_slot(slot);
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
    fn update_npc_ai_fixed(
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
        for ai_result in &ai_updates {
            let Some(direction) = ai_result.movement_intent else {
                continue;
            };
            let Some(initial_position) = self
                .entity_manager
                .get_entity(ai_result.entity_id)
                .map(|entity| entity.position)
            else {
                continue;
            };

            self.apply_accumulated_movement_scaled(
                ai_result.entity_id,
                direction,
                movement::MovementStepContext {
                    world_bounds,
                    tilemap,
                    atlas,
                    result,
                    time_scale: 1.0,
                },
            );

            let Some(final_position) = self
                .entity_manager
                .get_entity(ai_result.entity_id)
                .map(|entity| entity.position)
            else {
                continue;
            };
            self.emit_entity_movement_audio(
                ai_result.entity_id,
                Self::movement_distance(initial_position, final_position),
                result,
            );
        }
        self.ai_runtime_applier().apply_updates(ai_updates);
    }

    fn update_npc_ai_with_delta(
        &mut self,
        delta_ms: f32,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
        result: &mut GameUpdateResult<AudioEvent>,
    ) {
        self.ai_delta_accumulator_ms += delta_ms.max(0.0);

        let mut steps = 0;
        while self.ai_delta_accumulator_ms >= DEFAULT_TIMESTEP_MS {
            self.update_npc_ai_fixed(world_bounds, tilemap, atlas, result);
            self.ai_delta_accumulator_ms -= DEFAULT_TIMESTEP_MS;
            steps += 1;

            if steps >= MAX_AI_FIXED_STEPS_PER_UPDATE {
                break;
            }
        }
    }
}
