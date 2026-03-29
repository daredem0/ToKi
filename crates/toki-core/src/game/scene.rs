use super::player_defs::{default_player_definition, player_like_npc_definition};
use super::transition::SceneTransitionPlanner;
use super::{GameState, ProgressState, RuntimeState, SceneState, WorldState};
use crate::entity::{EntityId, EntityManager};
use crate::scene::Scene;
use crate::sprite::SpriteInstance;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RestoreError {
    #[error("save data is missing an active scene name")]
    MissingActiveSceneName,
    #[error("scene '{0}' not found")]
    MissingScene(String),
    #[error("failed to prepare scene load: {0}")]
    PrepareSceneLoad(String),
    #[error("failed to apply scene load: {0}")]
    ApplySceneLoad(String),
}

pub struct SceneSystem;

impl SceneSystem {
    pub fn add_scene(state: &mut GameState, scene: Scene) {
        state.track_persistent_entities_for_scene(&scene);
        state.scene.scene_manager.add_scene(scene);
    }

    pub fn active_scene(state: &GameState) -> Option<&Scene> {
        state.scene.scene_manager.active_scene()
    }

    pub fn load(state: &mut GameState, scene_name: &str) -> Result<(), String> {
        let scene = state
            .scene
            .scene_manager
            .get_scene(scene_name)
            .ok_or_else(|| format!("Scene '{}' not found", scene_name))?
            .clone();

        let prepared = SceneTransitionPlanner::new(&state.world.entity_definitions)
            .prepare_scene_load(&scene, None, None)?;
        state.apply_prepared_scene_load(scene_name, prepared)
    }

    pub fn transition(
        state: &mut GameState,
        scene_name: &str,
        spawn_point_id: &str,
    ) -> Result<(), String> {
        let scene = state
            .scene
            .scene_manager
            .get_scene(scene_name)
            .ok_or_else(|| format!("Scene '{}' not found", scene_name))?
            .clone();
        let preserved_player = state
            .world()
            .player_id()
            .and_then(|player_id| state.world().entity_manager().stored_entity(player_id));
        let prepared = SceneTransitionPlanner::new(&state.world.entity_definitions)
            .prepare_scene_load(&scene, Some(spawn_point_id), preserved_player)?;

        state.apply_prepared_scene_load(scene_name, prepared)
    }

    pub fn sync_entities_to_active_scene(state: &mut GameState) {
        let rules = state.scene.active_rules.clone();
        if let Some(active_scene) = state.scene.scene_manager.active_scene_mut() {
            active_scene.clear_entities();

            for entity_id in state.world.entity_manager.active_entities_iter() {
                if let Some(stored) = state.world.entity_manager.stored_entity(entity_id) {
                    active_scene.add_stored_entity(stored);
                }
            }

            active_scene.rules = rules;
        }
    }

    pub fn sync_persistent_entities_to_active_scene(state: &mut GameState) {
        let Some(active_scene_name) = state
            .scene
            .scene_manager
            .active_scene_name()
            .map(str::to_string)
        else {
            return;
        };

        let tracked_ids = state
            .scene
            .persistent_scene_entities
            .iter()
            .filter(|(scene_name, _)| *scene_name == active_scene_name)
            .map(|(_, entity_id)| *entity_id)
            .collect::<Vec<_>>();

        if tracked_ids.is_empty() {
            return;
        }

        let current_entities = tracked_ids
            .iter()
            .filter_map(|entity_id| {
                state
                    .world
                    .entity_manager
                    .get_entity(*entity_id)
                    .cloned()
                    .map(|entity| (*entity_id, entity))
            })
            .collect::<HashMap<_, _>>();

        if let Some(active_scene) = state.scene.scene_manager.active_scene_mut() {
            for entity_id in tracked_ids {
                match current_entities.get(&entity_id) {
                    Some(entity) => {
                        if let Some(existing) = active_scene.entity_mut(entity_id) {
                            *existing = entity.clone();
                        } else {
                            active_scene.add_entity(entity.clone());
                        }
                        active_scene.components_mut().apply_optional_components(
                            entity_id,
                            state
                                .world
                                .entity_manager
                                .storage()
                                .components()
                                .optional_components(entity_id),
                        );
                    }
                    None => {
                        active_scene.remove_entity(entity_id);
                    }
                }
            }
        }
    }

    pub fn restore_from_save_data(
        state: &mut GameState,
        save_data: &crate::serialization::SaveData,
    ) -> Result<(), RestoreError> {
        super::scene_restore::restore_from_save_data(state, save_data)
    }

    pub fn spawn_player_at(state: &mut GameState, position: glam::IVec2) -> EntityId {
        let player_def = default_player_definition();
        let player_id = state
            .world
            .entity_manager
            .spawn_from_definition(&player_def, position)
            .expect("default player definition should always be valid");
        state
            .world
            .entity_manager
            .set_control_role(player_id, crate::entity::ControlRole::PlayerCharacter);
        if let Some(player) = state.world.entity_manager.get_entity_mut(player_id) {
            player.entity_kind = crate::entity::EntityKind::Player;
        }
        state.world.player_id = Some(player_id);
        player_id
    }

    pub fn spawn_player_like_npc(state: &mut GameState, position: glam::IVec2) -> EntityId {
        let npc_def = player_like_npc_definition();
        state
            .world
            .entity_manager
            .spawn_from_definition(&npc_def, position)
            .expect("default player-like npc definition should always be valid")
    }
}

impl GameState {
    /// Create a new GameState with the given player sprite
    pub fn new(player_sprite: SpriteInstance) -> Self {
        let mut entity_manager = EntityManager::new();

        let player_def = default_player_definition();
        let player_id = entity_manager
            .spawn_from_definition(&player_def, player_sprite.position)
            .expect("default player definition should always be valid");
        entity_manager.set_control_role(player_id, crate::entity::ControlRole::PlayerCharacter);
        if let Some(player) = entity_manager.get_entity_mut(player_id) {
            player.entity_kind = crate::entity::EntityKind::Player;
        }

        Self {
            world: WorldState {
                entity_manager,
                entity_definitions: HashMap::new(),
                player_id: Some(player_id),
            },
            scene: SceneState::default(),
            progress: ProgressState::default(),
            runtime: RuntimeState::default(),
        }
    }

    /// Create a new empty GameState with no entities
    pub fn new_empty() -> Self {
        Self {
            world: WorldState::default(),
            scene: SceneState::default(),
            progress: ProgressState::default(),
            runtime: RuntimeState::default(),
        }
    }

    /// Set the player entity ID directly (for testing purposes).
    #[cfg(test)]
    pub(crate) fn set_player_id(&mut self, id: EntityId) {
        self.world.player_id = Some(id);
    }

    pub(super) fn apply_prepared_scene_load(
        &mut self,
        scene_name: &str,
        prepared: super::transition::PreparedSceneLoad,
    ) -> Result<(), String> {
        self.scene.scene_manager.set_active_scene(scene_name)?;
        super::InputSystem::clear(&mut self.runtime);
        self.runtime.effects.pending_stat_changes.clear();
        self.runtime.effects.pending_despawns.clear();
        self.runtime.ai.delta_accumulator_ms = 0.0;
        self.world.entity_manager = prepared.entity_manager;
        self.world.player_id = prepared.player_id;
        super::RuleSystem::set_rules(self, prepared.rules);
        Ok(())
    }

    fn track_persistent_entities_for_scene(&mut self, scene: &Scene) {
        for entity in scene.entities() {
            if entity.persistent_across_saves {
                self.scene
                    .persistent_scene_entities
                    .insert((scene.name.clone(), entity.id));
            }
        }
    }

    pub(super) fn rebuild_persistent_scene_tracking(&mut self) {
        self.scene.persistent_scene_entities.clear();
        let scene_names = self
            .scene
            .scene_manager
            .scene_names()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        for scene_name in scene_names {
            let entities = self
                .scene
                .scene_manager
                .get_scene(&scene_name)
                .map(|scene| {
                    scene
                        .entities()
                        .iter()
                        .filter(|entity| entity.persistent_across_saves)
                        .map(|entity| entity.id)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            for entity_id in entities {
                self.scene
                    .persistent_scene_entities
                    .insert((scene_name.clone(), entity_id));
            }
        }
    }
}
