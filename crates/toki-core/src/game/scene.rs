use super::transition::SceneTransitionPlanner;
use super::{GameState, ProgressState, RuntimeState, SceneState, WorldState};
use crate::entity::{ControlRole, EntityId, EntityKind, EntityManager};
use crate::ids::EntityDefName;
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

    pub fn active_scene<'a>(state: &'a GameState) -> Option<&'a Scene> {
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
        let scene_name = save_data.active_scene_name.trim();
        if scene_name.is_empty() {
            return Err(RestoreError::MissingActiveSceneName);
        }

        if !save_data.scene_snapshots.is_empty() {
            for scene_snapshot in &save_data.scene_snapshots {
                state.scene.scene_manager.add_scene(scene_snapshot.clone());
            }
            state.rebuild_persistent_scene_tracking();
        } else {
            for persisted in &save_data.persisted_entities {
                state
                    .scene
                    .persistent_scene_entities
                    .insert((persisted.scene_name.to_string(), persisted.entity_id));
                let Some(scene) = state.scene.scene_manager.get_scene_mut(&persisted.scene_name)
                else {
                    continue;
                };

                match &persisted.entity {
                    Some(stored_entity) => {
                        if let Some(existing) = scene.entity_mut(persisted.entity_id) {
                            *existing = stored_entity.entity.clone();
                        } else {
                            scene.add_entity(stored_entity.entity.clone());
                        }
                        scene.components_mut().apply_optional_components(
                            persisted.entity_id,
                            stored_entity.components.clone(),
                        );
                    }
                    None => {
                        scene.remove_entity(persisted.entity_id);
                    }
                }
            }
        }

        if let Some(scene) = state.scene.scene_manager.get_scene_mut(scene_name) {
            scene.camera_position = save_data.camera.position;
            scene.camera_scale = save_data.camera.scale;
        }

        let scene = state
            .scene
            .scene_manager
            .get_scene(scene_name)
            .ok_or_else(|| RestoreError::MissingScene(scene_name.to_string()))?
            .clone();

        let prepared = SceneTransitionPlanner::new(&state.world.entity_definitions)
            .prepare_scene_load(&scene, None, save_data.player.clone())
            .map_err(RestoreError::PrepareSceneLoad)?;

        state
            .apply_prepared_scene_load(scene_name, prepared)
            .map_err(RestoreError::ApplySceneLoad)?;
        if let (Some(saved_player), Some(player_id)) =
            (save_data.player.as_ref(), state.world.player_id)
        {
            if let Some(player) = state.world.entity_manager.get_entity_mut(player_id) {
                let mut restored_player = saved_player.entity.clone();
                restored_player.id = player_id;
                restored_player.control_role = ControlRole::PlayerCharacter;
                restored_player.entity_kind = EntityKind::Player;
                *player = restored_player;
            }
            if let Some(audio) = state
                .world
                .entity_manager
                .storage_mut()
                .audio_component_mut(player_id)
            {
                *audio = saved_player.entity.audio.to_component();
            }
            let components = state.world.entity_manager_mut().storage_mut().components_mut();
            components.set_inventory(player_id, saved_player.components.inventory.clone());
            components.set_primary_projectile(
                player_id,
                saved_player.components.primary_projectile.clone(),
            );
            components.set_projectile(player_id, saved_player.components.projectile.clone());
            components.set_pickup(player_id, saved_player.components.pickup.clone());
        }
        state.progress.game_flags = save_data.flags.clone();
        state.progress.play_time_ms = save_data.metadata.play_time_ms;
        Ok(())
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

    fn apply_prepared_scene_load(
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

    fn rebuild_persistent_scene_tracking(&mut self) {
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

fn default_player_definition() -> crate::entity::EntityDefinition {
    crate::entity::EntityDefinition {
        name: EntityDefName::from("player"),
        display_name: "Player".to_string(),
        description: "Default player entity".to_string(),
        rendering: crate::entity::RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            palette_override: None,
            static_object: None,
            grounding: Default::default(),
        },
        attributes: crate::entity::AttributesDef {
            health: Some(100),
            stats: std::collections::HashMap::new(),
            speed: 2.0,
            solid: true,
            active: true,
            can_move: true,
            interactable: false,
            interaction_reach: 0,
            ai_config: crate::entity::AiConfig::default(),
            movement_profile: crate::entity::MovementProfile::PlayerWasd,
            primary_projectile: None,
            pickup: None,
            has_inventory: true,
        },
        collision: crate::entity::CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: crate::entity::AudioDef {
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound_trigger: crate::entity::MovementSoundTrigger::Distance,
            movement_sound: "sfx_slime_bounce".to_string(),
            collision_sound: Some("sfx_hit2".to_string()),
        },
        animations: crate::entity::AnimationsDef {
            atlas_name: "creatures".to_string(),
            clips: vec![
                crate::entity::AnimationClipDef {
                    state: "idle".to_string(),
                    frame_tiles: vec!["slime/idle_0".to_string(), "slime/idle_1".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 300.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
                crate::entity::AnimationClipDef {
                    state: "walk".to_string(),
                    frame_tiles: vec![
                        "slime/walk_0".to_string(),
                        "slime/walk_1".to_string(),
                        "slime/walk_2".to_string(),
                        "slime/walk_3".to_string(),
                    ],
                    frame_positions: None,
                    frame_duration_ms: 150.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
            ],
            default_state: "idle".to_string(),
        },
        category: "human".to_string(),
        tags: vec!["player".to_string()],
    }
}

fn player_like_npc_definition() -> crate::entity::EntityDefinition {
    crate::entity::EntityDefinition {
        name: EntityDefName::from("player_like_npc"),
        display_name: "Player-like NPC".to_string(),
        description: "NPC using the player visual style".to_string(),
        rendering: crate::entity::RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            palette_override: None,
            static_object: None,
            grounding: Default::default(),
        },
        attributes: crate::entity::AttributesDef {
            health: Some(50),
            stats: std::collections::HashMap::new(),
            speed: 1.0,
            solid: true,
            active: true,
            can_move: false,
            interactable: false,
            interaction_reach: 0,
            ai_config: crate::entity::AiConfig::from_legacy_behavior(
                crate::entity::AiBehavior::Wander,
            ),
            movement_profile: crate::entity::MovementProfile::None,
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
        },
        collision: crate::entity::CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: crate::entity::AudioDef {
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound_trigger: crate::entity::MovementSoundTrigger::Distance,
            movement_sound: "sfx_slime_bounce".to_string(),
            collision_sound: Some("sfx_hit2".to_string()),
        },
        animations: crate::entity::AnimationsDef {
            atlas_name: "creatures".to_string(),
            clips: vec![
                crate::entity::AnimationClipDef {
                    state: "idle".to_string(),
                    frame_tiles: vec!["slime/idle_0".to_string(), "slime/idle_1".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 300.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
                crate::entity::AnimationClipDef {
                    state: "walk".to_string(),
                    frame_tiles: vec![
                        "slime/walk_0".to_string(),
                        "slime/walk_1".to_string(),
                        "slime/walk_2".to_string(),
                        "slime/walk_3".to_string(),
                    ],
                    frame_positions: None,
                    frame_duration_ms: 150.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
            ],
            default_state: "idle".to_string(),
        },
        category: "human".to_string(),
        tags: vec!["npc".to_string()],
    }
}
