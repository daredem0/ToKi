use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::animation::AnimationState;
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::entity::{AiBehavior, Entity, EntityId, EntityManager, MovementProfile};
use crate::events::{GameEvent, GameUpdateResult};
use crate::rules::{RuleSet, RuleTrigger};
use crate::scene_manager::SceneManager;

mod animation;
mod combat;
mod input;
mod inventory;
mod movement;
mod render_queries;
mod rules;
mod scene;

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
                 // Can extend with more keys as needed
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

    /// Player entity ID for quick access
    player_id: Option<EntityId>,

    /// Currently held input keys
    #[serde(default)]
    keys_held: HashSet<InputKey>,

    /// Runtime-held movement input scoped by movement profile.
    #[serde(skip, default)]
    profile_keys_held: HashMap<MovementProfile, HashSet<InputKey>>,

    /// Held profile-scoped action buttons used to debounce edge-triggered actions.
    #[serde(skip, default)]
    profile_actions_held: HashMap<MovementProfile, HashSet<InputAction>>,

    /// Pending one-shot profile-scoped action requests to be consumed during update.
    #[serde(skip, default)]
    pending_profile_actions: HashMap<MovementProfile, HashSet<InputAction>>,

    /// Debug rendering flags
    #[serde(default)]
    debug_collision_rendering: bool,

    /// Frame counter for NPC AI decisions
    #[serde(default)]
    npc_ai_frame_counter: u32,

    /// Data-driven gameplay rules evaluated each frame.
    #[serde(default)]
    rules: RuleSet,

    /// Runtime-only rule execution state.
    #[serde(skip, default)]
    rule_runtime: RuleRuntimeState,

    /// Pending generic stat changes gathered during update and resolved centrally.
    #[serde(skip, default)]
    pending_stat_changes: Vec<StatChangeRequest>,
}

use combat::StatChangeRequest;
use rules::RuleRuntimeState;

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
        self.rule_runtime.frame_collision_detected = false;
        self.rule_runtime.frame_damage_detected = false;
        self.rule_runtime.frame_death_detected = false;

        if !self.rule_runtime.started {
            self.collect_rule_commands_for_trigger(RuleTrigger::OnStart, &mut rule_commands);
            self.rule_runtime.started = true;
        }
        self.collect_rule_commands_for_trigger(RuleTrigger::OnUpdate, &mut rule_commands);
        self.collect_rule_commands_for_key_triggers(&mut rule_commands);
        let (mut pending_rule_animations, mut pending_scene_switch) =
            self.apply_rule_commands(rule_commands, &mut result);

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
        self.resolve_pending_stat_changes();

        // Update NPC AI
        self.update_npc_ai(world_bounds, tilemap, atlas, &mut result);

        let mut reactive_rule_commands = Vec::new();
        if result.player_moved {
            self.collect_rule_commands_for_trigger(
                RuleTrigger::OnPlayerMove,
                &mut reactive_rule_commands,
            );
        }
        if self.rule_runtime.frame_collision_detected {
            self.collect_rule_commands_for_trigger(
                RuleTrigger::OnCollision,
                &mut reactive_rule_commands,
            );
        }
        if self.rule_runtime.frame_damage_detected {
            self.collect_rule_commands_for_trigger(
                RuleTrigger::OnDamaged,
                &mut reactive_rule_commands,
            );
        }
        if self.rule_runtime.frame_death_detected {
            self.collect_rule_commands_for_trigger(
                RuleTrigger::OnDeath,
                &mut reactive_rule_commands,
            );
        }
        if self.any_entity_overlaps_trigger_tile(tilemap, atlas) {
            self.collect_rule_commands_for_trigger(
                RuleTrigger::OnTrigger,
                &mut reactive_rule_commands,
            );
        }
        let (mut reactive_animations, reactive_scene_switch) =
            self.apply_rule_commands(reactive_rule_commands, &mut result);
        if pending_scene_switch.is_none() {
            pending_scene_switch = reactive_scene_switch;
        }
        pending_rule_animations.append(&mut reactive_animations);

        self.apply_rule_animations(pending_rule_animations);

        // Update entity animation timing and emit animation-loop-based movement sounds.
        let completed_animation_loops = self.entity_manager.update_animations(17.0);
        for (entity_id, completed_loops) in completed_animation_loops {
            self.emit_animation_loop_movement_audio(entity_id, completed_loops, &mut result);
        }

        if let Some(scene_name) = pending_scene_switch {
            self.apply_rule_scene_switch(&scene_name);
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
        self.rule_runtime.frame_collision_detected = false;
        self.rule_runtime.frame_damage_detected = false;
        self.rule_runtime.frame_death_detected = false;

        if !self.rule_runtime.started {
            self.collect_rule_commands_for_trigger(RuleTrigger::OnStart, &mut rule_commands);
            self.rule_runtime.started = true;
        }
        self.collect_rule_commands_for_trigger(RuleTrigger::OnUpdate, &mut rule_commands);
        self.collect_rule_commands_for_key_triggers(&mut rule_commands);
        let (mut pending_rule_animations, mut pending_scene_switch) =
            self.apply_rule_commands(rule_commands, &mut result);

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
        self.resolve_pending_stat_changes();

        // Update NPC AI
        self.update_npc_ai(world_bounds, tilemap, atlas, &mut result);

        let mut reactive_rule_commands: Vec<rules::RuleCommand> = Vec::new();
        self.collect_reactive_rule_commands(&result, tilemap, atlas, &mut reactive_rule_commands);
        let (mut reactive_animations, reactive_scene_switch) =
            self.apply_rule_commands(reactive_rule_commands, &mut result);
        if pending_scene_switch.is_none() {
            pending_scene_switch = reactive_scene_switch;
        }
        pending_rule_animations.append(&mut reactive_animations);

        self.apply_rule_animations(pending_rule_animations);

        // Update entity animation timing with actual delta
        let completed_animation_loops = self.entity_manager.update_animations(animation_delta_ms);
        for (entity_id, completed_loops) in completed_animation_loops {
            self.emit_animation_loop_movement_audio(entity_id, completed_loops, &mut result);
        }

        if let Some(scene_name) = pending_scene_switch {
            self.apply_rule_scene_switch(&scene_name);
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

    /// Collect reactive rule commands based on game events this frame.
    fn collect_reactive_rule_commands(
        &mut self,
        result: &GameUpdateResult<AudioEvent>,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
        reactive_rule_commands: &mut Vec<rules::RuleCommand>,
    ) {
        if result.player_moved {
            self.collect_rule_commands_for_trigger(
                RuleTrigger::OnPlayerMove,
                reactive_rule_commands,
            );
        }
        if self.rule_runtime.frame_collision_detected {
            self.collect_rule_commands_for_trigger(
                RuleTrigger::OnCollision,
                reactive_rule_commands,
            );
        }
        if self.rule_runtime.frame_damage_detected {
            self.collect_rule_commands_for_trigger(RuleTrigger::OnDamaged, reactive_rule_commands);
        }
        if self.rule_runtime.frame_death_detected {
            self.collect_rule_commands_for_trigger(RuleTrigger::OnDeath, reactive_rule_commands);
        }
        if self.any_entity_overlaps_trigger_tile(tilemap, atlas) {
            self.collect_rule_commands_for_trigger(RuleTrigger::OnTrigger, reactive_rule_commands);
        }
    }

    /// Update NPC AI - makes NPCs move randomly every few frames
    fn update_npc_ai(
        &mut self,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
        result: &mut GameUpdateResult<AudioEvent>,
    ) {
        self.npc_ai_frame_counter += 1;

        // Only update NPC AI every 60 frames (roughly once per second at 60fps)
        if !self.npc_ai_frame_counter.is_multiple_of(60) {
            return;
        }

        let npc_entity_ids: Vec<_> = self
            .entity_manager
            .active_entities()
            .iter()
            .filter_map(|&entity_id| {
                if let Some(entity) = self.entity_manager.get_entity(entity_id) {
                    // Skip the player entity
                    if Some(entity_id) == self.player_id {
                        return None;
                    }
                    if matches!(entity.attributes.ai_behavior, AiBehavior::Wander) {
                        return Some(entity_id);
                    }
                }
                None
            })
            .collect();

        for npc_id in npc_entity_ids {
            let Some(npc_entity) = self.entity_manager.get_entity(npc_id) else {
                continue;
            };

            let current_position = npc_entity.position;
            // NPC wander uses entity speed * 5 for larger jumps
            let movement_step = (npc_entity.attributes.speed * 5.0) as i32;
            let max_x = (world_bounds.x as i32 - npc_entity.size.x as i32).max(0);
            let max_y = (world_bounds.y as i32 - npc_entity.size.y as i32).max(0);

            // Choose random direction: 0=up, 1=down, 2=left, 3=right, 4=stay
            let random_direction = fastrand::u32(0..5);

            let new_position = match random_direction {
                0 => glam::IVec2::new(
                    current_position.x,
                    (current_position.y - movement_step).max(0),
                ),
                1 => glam::IVec2::new(
                    current_position.x,
                    (current_position.y + movement_step).min(max_y),
                ),
                2 => glam::IVec2::new(
                    (current_position.x - movement_step).max(0),
                    current_position.y,
                ),
                3 => glam::IVec2::new(
                    (current_position.x + movement_step).min(max_x),
                    current_position.y,
                ),
                4 => current_position, // Stay in place
                _ => current_position,
            };

            let npc_moved = if new_position != current_position {
                if self.can_entity_move_to_position(npc_id, new_position, tilemap, atlas) {
                    if let Some(npc_entity) = self.entity_manager.get_entity_mut(npc_id) {
                        npc_entity.position = new_position;
                    }
                    self.emit_entity_movement_audio(
                        npc_id,
                        Self::movement_distance(current_position, new_position),
                        result,
                    );
                    true
                } else {
                    false
                }
            } else {
                false
            };

            if let Some(npc_entity) = self.entity_manager.get_entity_mut(npc_id) {
                // Update NPC animation based on movement
                if let Some(animation_controller) = &mut npc_entity.attributes.animation_controller
                {
                    let desired_animation = if npc_moved {
                        AnimationState::Walk
                    } else {
                        AnimationState::Idle
                    };

                    if animation_controller.current_clip_state != desired_animation {
                        animation_controller.play(desired_animation);
                    }
                }
            }
        }
    }
}
