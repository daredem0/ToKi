use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::ai::AiSystem;
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::entity::{Entity, EntityDefinition, EntityId, EntityManager, MovementProfile};
use crate::events::{GameEvent, GameUpdateResult};
use crate::flags::{FlagValue, GameFlags};
use crate::ids::EntityDefName;
use crate::project_runtime::ProjectFlagDefinition;
use crate::rules::RuleSet;
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
pub use render_queries::RenderQueryService;
pub use movement::MovementSystem;
pub use combat::CombatSystem;
pub use interaction::InteractionSystem;
pub use rules::{
    CollisionEvent, DamageEvent, DeathEvent, InteractionEvent, InteractionSpatial,
    TileTransitionEvent,
};
pub use rules::RuleSystem;
pub use scene::RestoreError;
pub use scene::SceneSystem;
pub use input::InputSystem;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct WorldState {
    #[serde(default)]
    entity_manager: EntityManager,
    #[serde(default)]
    entity_definitions: HashMap<EntityDefName, EntityDefinition>,
    #[serde(default)]
    player_id: Option<EntityId>,
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            entity_manager: EntityManager::new(),
            entity_definitions: HashMap::new(),
            player_id: None,
        }
    }
}

impl WorldState {
    pub fn entity_manager(&self) -> &EntityManager {
        &self.entity_manager
    }

    pub fn entity_manager_mut(&mut self) -> &mut EntityManager {
        &mut self.entity_manager
    }

    pub fn entity_definitions(&self) -> &HashMap<EntityDefName, EntityDefinition> {
        &self.entity_definitions
    }

    pub fn entity_definitions_mut(&mut self) -> &mut HashMap<EntityDefName, EntityDefinition> {
        &mut self.entity_definitions
    }

    pub fn player_id(&self) -> Option<EntityId> {
        self.player_id
    }

    pub fn insert_entity_definition(&mut self, definition: EntityDefinition) {
        self.entity_definitions.insert(definition.name.clone(), definition);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SceneState {
    #[serde(default)]
    scene_manager: SceneManager,
    #[serde(default)]
    active_rules: RuleSet,
    #[serde(skip, default)]
    persistent_scene_entities: HashSet<(String, crate::entity::EntityId)>,
}

impl Default for SceneState {
    fn default() -> Self {
        Self {
            scene_manager: SceneManager::new(),
            active_rules: RuleSet::default(),
            persistent_scene_entities: HashSet::new(),
        }
    }
}

impl SceneState {
    pub fn scene_manager(&self) -> &SceneManager {
        &self.scene_manager
    }

    pub fn scene_manager_mut(&mut self) -> &mut SceneManager {
        &mut self.scene_manager
    }

    pub fn active_rules(&self) -> &RuleSet {
        &self.active_rules
    }

    pub fn active_rules_mut(&mut self) -> &mut RuleSet {
        &mut self.active_rules
    }

    pub fn persistent_scene_entities(&self) -> &HashSet<(String, crate::entity::EntityId)> {
        &self.persistent_scene_entities
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProgressState {
    #[serde(default)]
    game_flags: GameFlags,
    #[serde(default)]
    play_time_ms: u64,
}

impl ProgressState {
    pub fn game_flags(&self) -> &GameFlags {
        &self.game_flags
    }

    pub fn game_flags_mut(&mut self) -> &mut GameFlags {
        &mut self.game_flags
    }

    pub fn play_time_ms(&self) -> u64 {
        self.play_time_ms
    }
}

#[derive(Debug)]
pub struct AiRuntimeState {
    system: AiSystem,
    delta_accumulator_ms: f32,
}

impl Default for AiRuntimeState {
    fn default() -> Self {
        Self {
            system: AiSystem::new(),
            delta_accumulator_ms: 0.0,
        }
    }
}

#[derive(Debug, Default)]
pub struct EffectRuntimeState {
    pending_stat_changes: Vec<StatChangeRequest>,
    pending_despawns: Vec<EntityId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeState {
    #[serde(default)]
    input: InputRuntimeState,
    #[serde(default)]
    debug_collision_rendering: bool,
    #[serde(skip, default)]
    ai: AiRuntimeState,
    #[serde(skip, default)]
    rules: RuleRuntimeState,
    #[serde(skip, default)]
    effects: EffectRuntimeState,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            input: InputRuntimeState::default(),
            debug_collision_rendering: false,
            ai: AiRuntimeState::default(),
            rules: RuleRuntimeState::default(),
            effects: EffectRuntimeState::default(),
        }
    }
}

impl RuntimeState {
    pub fn debug_collision_rendering(&self) -> bool {
        self.debug_collision_rendering
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UpdateContext<'a> {
    pub time_scale: f32,
    pub world_bounds: glam::UVec2,
    pub tilemap: &'a TileMap,
    pub atlas: &'a AtlasMeta,
}

pub struct GameSimulation;

/// Core game state that manages entities, scenes, input, and game logic.
///
/// This is platform-independent and contains pure game logic without
/// any runtime or windowing dependencies.
#[derive(Debug, Serialize, Deserialize)]
pub struct GameState {
    world: WorldState,
    scene: SceneState,
    progress: ProgressState,
    runtime: RuntimeState,
}

use input_state::InputRuntimeState;
use rules::RuleRuntimeState;
use stat_effects::StatChangeRequest;

impl GameSimulation {
    pub fn tick(state: &mut GameState, ctx: UpdateContext<'_>) -> GameUpdateResult<AudioEvent> {
        let animation_delta_ms = (DEFAULT_TIMESTEP_MS * ctx.time_scale).max(0.0);
        state.progress.play_time_ms = state
            .progress
            .play_time_ms
            .saturating_add(animation_delta_ms.max(0.0).round() as u64);
        let mut result = GameUpdateResult::new();
        let mut rule_commands = Vec::new();
        RuleSystem::begin_frame(state);
        RuleSystem::collect_frame_commands(state, &mut rule_commands);
        let (mut pending_rule_animations, mut pending_scene_switch, _, mut pending_persistence) =
            RuleSystem::apply_commands(state, rule_commands, &mut result, ctx.tilemap);

        let initial_player_position = state
            .world
            .player_id
            .and_then(|player_id| state.world.entity_manager.get_entity(player_id))
            .map(|entity| entity.position)
            .unwrap_or(glam::IVec2::ZERO);

        let input_result = MovementSystem::process_input_scaled(
            state,
            ctx.world_bounds,
            ctx.tilemap,
            ctx.atlas,
            ctx.time_scale,
        );
        result.player_moved = input_result.player_moved;
        result.add_events(input_result.events);

        if MovementSystem::apply_rule_velocities(
            state,
            ctx.world_bounds,
            ctx.tilemap,
            ctx.atlas,
            &mut result,
        ) {
            result.player_moved = true;
        }

        let intended_player_delta = state
            .world
            .player_id
            .and_then(|player_id| state.world.entity_manager.get_entity(player_id))
            .map(|entity| state.held_keys_for_profile(GameState::effective_movement_profile(entity)))
            .map(|keys| GameState::movement_delta_from_keys(&keys))
            .unwrap_or(glam::IVec2::ZERO);

        MovementSystem::update_player_animation(state, initial_player_position, intended_player_delta);
        CombatSystem::process_profile_actions(state);
        CombatSystem::update_projectiles(state, ctx.tilemap, ctx.atlas);
        InteractionSystem::collect_overlapping_pickups(state);
        InteractionSystem::collect_interaction_events(state);
        state.resolve_pending_stat_changes();

        state.update_npc_ai_with_delta(
            animation_delta_ms,
            ctx.world_bounds,
            ctx.tilemap,
            ctx.atlas,
            &mut result,
        );

        state.detect_tile_transitions(ctx.tilemap);

        let reactive_rule_commands =
            RuleSystem::collect_reactive_commands(state, result.player_moved, ctx.tilemap, ctx.atlas);
        let (mut reactive_animations, reactive_scene_switch, _, reactive_persistence) =
            RuleSystem::apply_commands(state, reactive_rule_commands, &mut result, ctx.tilemap);
        if pending_scene_switch.is_none() {
            pending_scene_switch = reactive_scene_switch;
        }
        if pending_persistence.is_none() {
            pending_persistence = reactive_persistence;
        }
        pending_rule_animations.append(&mut reactive_animations);

        state.apply_rule_animations(pending_rule_animations);
        state.flush_pending_despawns();

        let completed_animation_loops = state
            .world
            .entity_manager
            .update_animations(animation_delta_ms);
        for (entity_id, completed_loops) in completed_animation_loops {
            MovementSystem::emit_animation_loop_audio(state, entity_id, completed_loops, &mut result);
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

    pub fn tick_fixed(
        state: &mut GameState,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> GameUpdateResult<AudioEvent> {
        Self::tick(
            state,
            UpdateContext {
                time_scale: 1.0,
                world_bounds,
                tilemap,
                atlas,
            },
        )
    }

    pub fn tick_with_delta(
        state: &mut GameState,
        delta_ms: f32,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> GameUpdateResult<AudioEvent> {
        Self::tick(
            state,
            UpdateContext {
                time_scale: delta_ms / DEFAULT_TIMESTEP_MS,
                world_bounds,
                tilemap,
                atlas,
            },
        )
    }
}

impl GameState {
    pub fn world(&self) -> &WorldState {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut WorldState {
        &mut self.world
    }

    pub fn scene(&self) -> &SceneState {
        &self.scene
    }

    pub fn scene_mut(&mut self) -> &mut SceneState {
        &mut self.scene
    }

    pub fn progress(&self) -> &ProgressState {
        &self.progress
    }

    pub fn progress_mut(&mut self) -> &mut ProgressState {
        &mut self.progress
    }

    pub fn runtime(&self) -> &RuntimeState {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut RuntimeState {
        &mut self.runtime
    }

    pub fn game_flags(&self) -> &GameFlags {
        &self.progress.game_flags
    }

    pub fn flag(&self, flag: &str) -> Option<&FlagValue> {
        self.progress.game_flags.get(flag)
    }

    pub fn set_flag(&mut self, flag: impl Into<String>, value: FlagValue) {
        self.progress.game_flags.set(flag, value);
    }

    pub fn clear_flag(&mut self, flag: &str) -> bool {
        self.progress.game_flags.clear(flag)
    }

    pub fn increment_flag(&mut self, flag: impl Into<String>, amount: i32) -> bool {
        self.progress.game_flags.increment(flag, amount)
    }

    pub fn play_time_ms(&self) -> u64 {
        self.progress.play_time_ms
    }

    pub fn apply_flag_defaults(&mut self, declarations: &[ProjectFlagDefinition]) {
        for declaration in declarations {
            let flag = declaration.id.trim();
            if flag.is_empty() || self.progress.game_flags.is_set(flag) {
                continue;
            }
            self.progress
                .game_flags
                .set(flag.to_string(), declaration.default_value.clone());
        }
    }

    pub fn persistent_scene_entity_keys(&self) -> Vec<(String, crate::entity::EntityId)> {
        let mut keys = self
            .scene
            .persistent_scene_entities
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        keys
    }

    fn effective_movement_profile(entity: &Entity) -> MovementProfile {
        entity.effective_movement_profile()
    }

    /// Helper to update player animation based on movement intent.
    fn update_player_animation(
        &mut self,
        initial_player_position: glam::IVec2,
        intended_player_delta: glam::IVec2,
    ) {
        let Some(player_entity) = self.world.entity_manager.get_player_mut() else {
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
        let ai_updates = self.runtime.ai.system.update(
            &self.world.entity_manager,
            self.world.player_id,
            world_bounds,
            tilemap,
            atlas,
        );
        for ai_result in &ai_updates {
            let Some(direction) = ai_result.movement_intent else {
                continue;
            };
            let Some(initial_position) = self
                .world
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
                .world
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
        self.runtime.ai.delta_accumulator_ms += delta_ms.max(0.0);

        let mut steps = 0;
        while self.runtime.ai.delta_accumulator_ms >= DEFAULT_TIMESTEP_MS {
            self.update_npc_ai_fixed(world_bounds, tilemap, atlas, result);
            self.runtime.ai.delta_accumulator_ms -= DEFAULT_TIMESTEP_MS;
            steps += 1;

            if steps >= MAX_AI_FIXED_STEPS_PER_UPDATE {
                break;
            }
        }
    }
}
