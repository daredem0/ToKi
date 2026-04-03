use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::ai::AiSystem;
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::entity::{EntityDefinition, EntityId, EntityManager};
use crate::events::{GameEvent, GameUpdateResult};
use crate::flags::{FlagValue, GameFlags};
use crate::ids::EntityDefName;
use crate::project_runtime::ProjectFlagDefinition;
use crate::rules::RuleSet;
use crate::scene_manager::SceneManager;
use crate::SceneId;

mod ai_runtime;
mod animation;
mod combat;
mod input;
mod input_state;
mod interaction;
mod inventory;
mod movement;
mod player_defs;
mod render_queries;
pub(crate) mod rules;
mod scene;
mod scene_restore;
mod simulation;
mod stat_effects;
mod transition;
mod world_context;

#[cfg(test)]
mod rules_tests;

// Re-export event types for external use
pub use combat::{CombatService, CombatSystem};
pub use input::InputSystem;
pub use interaction::{InteractionService, InteractionSystem};
pub use movement::{MovementService, MovementSystem};
pub use render_queries::GroundShadow;
pub use render_queries::RenderQueryService;
pub use rules::{RuleEvaluationService, RuleSystem};
pub use rules::{
    CollisionEvent, DamageEvent, DeathEvent, InteractionEvent, InteractionSpatial,
    TileTransitionEvent,
};
pub use scene::{RestoreError, SceneLoadError, SceneSystem};
pub(crate) use world_context::WorldContext;

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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorldState {
    #[serde(default)]
    entity_manager: EntityManager,
    #[serde(default)]
    entity_definitions: HashMap<EntityDefName, EntityDefinition>,
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
        self.entity_manager.get_player_id()
    }

    pub fn insert_entity_definition(&mut self, definition: EntityDefinition) {
        self.entity_definitions
            .insert(definition.name.clone(), definition);
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SceneState {
    #[serde(default)]
    scene_manager: SceneManager,
    #[serde(default)]
    active_rules: RuleSet,
    #[serde(skip, default)]
    persistent_scene_entities: HashSet<(SceneId, crate::entity::EntityId)>,
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

    pub fn persistent_scene_entities(&self) -> &HashSet<(SceneId, crate::entity::EntityId)> {
        &self.persistent_scene_entities
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProgressState {
    #[serde(default)]
    game_flags: GameFlags,
    #[serde(default)]
    play_time_ms: u64,
    #[serde(skip, default)]
    play_time_remainder_ms: f32,
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

#[derive(Debug, Default, Serialize, Deserialize)]
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

    pub fn persistent_scene_entity_keys(&self) -> Vec<(SceneId, crate::entity::EntityId)> {
        let mut keys = self
            .scene
            .persistent_scene_entities
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        keys
    }

    /// Update NPC AI using the AI system
    fn update_npc_ai_fixed(
        &mut self,
        world: WorldContext<'_>,
        result: &mut GameUpdateResult<AudioEvent>,
    ) {
        let ai_updates = self.runtime.ai.system.update(
            &self.world.entity_manager,
            self.world.player_id(),
            world.bounds,
            world.tilemap,
            world.atlas,
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

            self.movement_service().apply_accumulated_movement_scaled(
                ai_result.entity_id,
                direction,
                movement::MovementStepContext {
                    world_bounds: world.bounds,
                    tilemap: world.tilemap,
                    atlas: world.atlas,
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
            self.movement_service().emit_entity_movement_audio(
                ai_result.entity_id,
                movement::MovementService::movement_distance(initial_position, final_position),
                result,
            );
        }
        self.ai_runtime_applier().apply_updates(ai_updates);
    }

    fn update_npc_ai_with_delta(
        &mut self,
        delta_ms: f32,
        world: WorldContext<'_>,
        result: &mut GameUpdateResult<AudioEvent>,
    ) {
        self.runtime.ai.delta_accumulator_ms += delta_ms.max(0.0);

        let mut steps = 0;
        while self.runtime.ai.delta_accumulator_ms >= DEFAULT_TIMESTEP_MS {
            self.update_npc_ai_fixed(world, result);
            self.runtime.ai.delta_accumulator_ms -= DEFAULT_TIMESTEP_MS;
            steps += 1;

            if steps >= MAX_AI_FIXED_STEPS_PER_UPDATE {
                break;
            }
        }
    }
}
