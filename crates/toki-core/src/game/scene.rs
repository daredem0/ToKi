use super::{GameState, RuleRuntimeState};
use crate::ai::AiSystem;
use crate::animation::AnimationState;
use crate::entity::{ControlRole, Entity, EntityDefinition, EntityId, EntityKind, EntityManager};
use crate::rules::RuleSet;
use crate::scene::{Scene, SceneAnchorFacing};
use crate::scene_manager::SceneManager;
use crate::sprite::SpriteInstance;
use std::collections::{HashMap, HashSet};

struct PreparedSceneLoad {
    entity_manager: EntityManager,
    player_id: Option<EntityId>,
    rules: RuleSet,
}

impl GameState {
    /// Create a new GameState with the given player sprite
    pub fn new(player_sprite: SpriteInstance) -> Self {
        let mut entity_manager = EntityManager::new();

        let player_def = Self::default_player_definition();
        let player_id = entity_manager
            .spawn_from_definition(&player_def, player_sprite.position)
            .expect("default player definition should always be valid");
        entity_manager.set_control_role(player_id, crate::entity::ControlRole::PlayerCharacter);
        if let Some(player) = entity_manager.get_entity_mut(player_id) {
            player.entity_kind = crate::entity::EntityKind::Player;
        }

        Self {
            scene_manager: SceneManager::new(),
            entity_manager,
            entity_definitions: HashMap::new(),
            player_id: Some(player_id),
            keys_held: HashSet::new(),
            profile_keys_held: HashMap::new(),
            profile_actions_held: HashMap::new(),
            pending_profile_actions: HashMap::new(),
            debug_collision_rendering: false,
            ai_system: AiSystem::new(),
            rules: RuleSet::default(),
            rule_runtime: RuleRuntimeState::default(),
            pending_stat_changes: Vec::new(),
            pending_despawns: Vec::new(),
        }
    }

    /// Create a new empty GameState with no entities
    pub fn new_empty() -> Self {
        Self {
            scene_manager: SceneManager::new(),
            entity_manager: EntityManager::new(),
            entity_definitions: HashMap::new(),
            player_id: None,
            keys_held: HashSet::new(),
            profile_keys_held: HashMap::new(),
            profile_actions_held: HashMap::new(),
            pending_profile_actions: HashMap::new(),
            debug_collision_rendering: false,
            ai_system: AiSystem::new(),
            rules: RuleSet::default(),
            rule_runtime: RuleRuntimeState::default(),
            pending_stat_changes: Vec::new(),
            pending_despawns: Vec::new(),
        }
    }

    /// Initialize the game with a player at the specified position
    pub fn spawn_player_at(&mut self, position: glam::IVec2) -> EntityId {
        let player_def = Self::default_player_definition();
        let player_id = self
            .entity_manager
            .spawn_from_definition(&player_def, position)
            .expect("default player definition should always be valid");
        self.entity_manager
            .set_control_role(player_id, crate::entity::ControlRole::PlayerCharacter);
        if let Some(player) = self.entity_manager.get_entity_mut(player_id) {
            player.entity_kind = crate::entity::EntityKind::Player;
        }
        self.player_id = Some(player_id);
        player_id
    }

    /// Spawn an NPC that looks identical to the player
    pub fn spawn_player_like_npc(&mut self, position: glam::IVec2) -> EntityId {
        let npc_def = Self::player_like_npc_definition();
        self.entity_manager
            .spawn_from_definition(&npc_def, position)
            .expect("default player-like npc definition should always be valid")
    }

    pub fn add_entity_definition(&mut self, definition: EntityDefinition) {
        self.entity_definitions
            .insert(definition.name.clone(), definition);
    }

    fn default_player_definition() -> crate::entity::EntityDefinition {
        crate::entity::EntityDefinition {
            name: "player".to_string(),
            display_name: "Player".to_string(),
            description: "Default player entity".to_string(),
            rendering: crate::entity::RenderingDef {
                size: [16, 16],
                render_layer: 0,
                visible: true,
                static_object: None,
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
            name: "player_like_npc".to_string(),
            display_name: "Player-like NPC".to_string(),
            description: "NPC using the player visual style".to_string(),
            rendering: crate::entity::RenderingDef {
                size: [16, 16],
                render_layer: 0,
                visible: true,
                static_object: None,
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

    /// Get reference to all entities (legacy method - preserved for compatibility)
    pub fn entities(&self) -> Vec<&Entity> {
        self.entity_manager
            .active_entities()
            .iter()
            .filter_map(|&id| self.entity_manager.get_entity(id))
            .collect()
    }

    /// Get access to the entity manager
    pub fn entity_manager(&self) -> &EntityManager {
        &self.entity_manager
    }

    /// Get mutable access to the entity manager
    pub fn entity_manager_mut(&mut self) -> &mut EntityManager {
        &mut self.entity_manager
    }

    /// Set the player entity ID directly (for testing purposes).
    #[cfg(test)]
    pub fn set_player_id(&mut self, id: EntityId) {
        self.player_id = Some(id);
    }

    /// Get access to the scene manager
    pub fn scene_manager(&self) -> &SceneManager {
        &self.scene_manager
    }

    /// Get mutable access to the scene manager
    pub fn scene_manager_mut(&mut self) -> &mut SceneManager {
        &mut self.scene_manager
    }

    /// Load a scene and make it the active scene
    /// This will clear current entities and load entities from the scene
    pub fn load_scene(&mut self, scene_name: &str) -> Result<(), String> {
        let scene = self
            .scene_manager
            .get_scene(scene_name)
            .ok_or_else(|| format!("Scene '{}' not found", scene_name))?
            .clone();

        let prepared = self.prepare_scene_load(&scene, None, None)?;
        self.scene_manager.set_active_scene(scene_name)?;
        self.clear_runtime_inputs();
        self.pending_stat_changes.clear();
        self.entity_manager = prepared.entity_manager;
        self.player_id = prepared.player_id;
        self.set_rules(prepared.rules);

        Ok(())
    }

    pub fn transition_to_scene(
        &mut self,
        scene_name: &str,
        spawn_point_id: &str,
    ) -> Result<(), String> {
        let scene = self
            .scene_manager
            .get_scene(scene_name)
            .ok_or_else(|| format!("Scene '{}' not found", scene_name))?
            .clone();
        let preserved_player = self.player_entity().cloned();
        let prepared = self.prepare_scene_load(&scene, Some(spawn_point_id), preserved_player)?;

        self.scene_manager.set_active_scene(scene_name)?;
        self.clear_runtime_inputs();
        self.pending_stat_changes.clear();
        self.entity_manager = prepared.entity_manager;
        self.player_id = prepared.player_id;
        self.set_rules(prepared.rules);

        Ok(())
    }

    /// Add a scene to the scene manager
    pub fn add_scene(&mut self, scene: Scene) {
        self.scene_manager.add_scene(scene);
    }

    /// Get reference to the current active scene
    pub fn active_scene(&self) -> Option<&Scene> {
        self.scene_manager.active_scene()
    }

    /// Sync current entities back to the active scene
    /// Useful for saving changes made during runtime back to scene data
    pub fn sync_entities_to_active_scene(&mut self) {
        let rules = self.rules.clone();
        if let Some(active_scene) = self.scene_manager.active_scene_mut() {
            active_scene.entities.clear();

            for entity_id in self.entity_manager.active_entities() {
                if let Some(entity) = self.entity_manager.get_entity(entity_id) {
                    active_scene.entities.push(entity.clone());
                }
            }

            active_scene.rules = rules;
        }
    }

    /// Get the player entity ID
    pub fn player_id(&self) -> Option<EntityId> {
        self.player_id
    }

    /// Get reference to player entity
    pub fn player_entity(&self) -> Option<&Entity> {
        self.player_id
            .and_then(|id| self.entity_manager.get_entity(id))
    }

    /// Get entities as owned Vec for camera system compatibility
    pub fn entities_owned(&self) -> Vec<Entity> {
        self.entity_manager
            .active_entities()
            .iter()
            .filter_map(|&id| self.entity_manager.get_entity(id))
            .cloned()
            .collect()
    }

    fn prepare_scene_load(
        &self,
        scene: &Scene,
        transition_spawn_point_id: Option<&str>,
        preserved_player: Option<Entity>,
    ) -> Result<PreparedSceneLoad, String> {
        let mut entity_manager = EntityManager::new();
        let preserve_player_across_transition =
            transition_spawn_point_id.is_some() && preserved_player.is_some();

        for entity in &scene.entities {
            let authored_player = matches!(
                entity.effective_control_role(),
                ControlRole::PlayerCharacter
            );
            if authored_player
                && (scene.player_entry.is_some() || preserve_player_across_transition)
            {
                continue;
            }
            entity_manager.add_existing_entity(entity.clone());
        }

        let player_id = if let Some(player_entry) = &scene.player_entry {
            let spawn_point_id = transition_spawn_point_id.unwrap_or(&player_entry.spawn_point_id);
            if let Some(preserved_player) = preserved_player.as_ref() {
                let mut player = self.instantiate_scene_player_entry(
                    scene,
                    &player_entry.entity_definition_name,
                    spawn_point_id,
                    preserved_player,
                )?;
                let player_id = player.id;
                self.reset_spawned_player_transient_state(&mut player, scene, spawn_point_id);
                entity_manager.add_existing_entity(player);
                entity_manager.set_control_role(player_id, ControlRole::PlayerCharacter);
                if let Some(player) = entity_manager.get_entity_mut(player_id) {
                    player.entity_kind = EntityKind::Player;
                }
                Some(player_id)
            } else {
                let (position, facing) = self.resolve_spawn_anchor(scene, spawn_point_id)?;
                let definition = self
                    .entity_definitions
                    .get(&player_entry.entity_definition_name)
                    .ok_or_else(|| {
                        format!(
                            "Scene '{}' references missing player entity definition '{}'",
                            scene.name, player_entry.entity_definition_name
                        )
                    })?;
                let player_id = entity_manager.spawn_from_definition(definition, position)?;
                entity_manager.set_control_role(player_id, ControlRole::PlayerCharacter);
                if let Some(player) = entity_manager.get_entity_mut(player_id) {
                    player.entity_kind = EntityKind::Player;
                    Self::reset_player_transient_state(player, facing);
                }
                Some(player_id)
            }
        } else if let (Some(spawn_point_id), Some(mut player)) =
            (transition_spawn_point_id, preserved_player)
        {
            self.reposition_preserved_player(&mut player, scene, spawn_point_id)?;
            let player_id = player.id;
            entity_manager.add_existing_entity(player);
            entity_manager.set_control_role(player_id, ControlRole::PlayerCharacter);
            if let Some(player) = entity_manager.get_entity_mut(player_id) {
                player.entity_kind = EntityKind::Player;
            }
            Some(player_id)
        } else {
            entity_manager.get_player_id()
        };

        Ok(PreparedSceneLoad {
            entity_manager,
            player_id,
            rules: scene.rules.clone(),
        })
    }

    fn instantiate_scene_player_entry(
        &self,
        scene: &Scene,
        entity_definition_name: &str,
        spawn_point_id: &str,
        preserved_player: &Entity,
    ) -> Result<Entity, String> {
        let definition = self
            .entity_definitions
            .get(entity_definition_name)
            .ok_or_else(|| {
                format!(
                    "Scene '{}' references missing player entity definition '{}'",
                    scene.name, entity_definition_name
                )
            })?;
        let (position, _) = self.resolve_spawn_anchor(scene, spawn_point_id)?;
        let mut player = definition.create_entity(position, preserved_player.id)?;
        player.control_role = ControlRole::PlayerCharacter;
        player.entity_kind = EntityKind::Player;
        Self::apply_durable_player_state(&mut player, preserved_player);
        Ok(player)
    }

    fn reposition_preserved_player(
        &self,
        player: &mut Entity,
        scene: &Scene,
        spawn_point_id: &str,
    ) -> Result<(), String> {
        let (position, facing) = self.resolve_spawn_anchor(scene, spawn_point_id)?;
        player.position = position;
        Self::reset_player_transient_state(player, facing);
        Ok(())
    }

    fn reset_spawned_player_transient_state(
        &self,
        player: &mut Entity,
        scene: &Scene,
        spawn_point_id: &str,
    ) {
        let anchor_facing = scene
            .get_anchor(spawn_point_id)
            .and_then(|anchor| anchor.facing);
        Self::reset_player_transient_state(player, anchor_facing);
    }

    fn reset_player_transient_state(player: &mut Entity, anchor_facing: Option<SceneAnchorFacing>) {
        player.movement_accumulator = glam::Vec2::ZERO;
        if let Some(animation_controller) = player.attributes.animation_controller.as_mut() {
            let facing = anchor_facing
                .map(Self::scene_anchor_facing_to_animation_state)
                .or_else(|| {
                    Some(Self::directional_animation_state(
                        false,
                        Self::facing_from_animation_state(animation_controller.current_clip_state),
                    ))
                })
                .unwrap_or(AnimationState::Idle);
            let desired_state = if animation_controller.has_clip(facing) {
                facing
            } else if animation_controller.has_clip(AnimationState::Idle) {
                AnimationState::Idle
            } else {
                animation_controller.current_clip_state
            };
            let _ = animation_controller.play(desired_state);
        }
    }

    fn apply_durable_player_state(target: &mut Entity, source: &Entity) {
        target.attributes.health = source.attributes.health;
        target.attributes.stats = source.attributes.stats.clone();
        target.attributes.inventory = source.attributes.inventory.clone();
        target.attributes.has_inventory = target.attributes.has_inventory
            || source.attributes.has_inventory
            || !source.attributes.inventory.is_empty();
    }

    fn scene_anchor_facing_to_animation_state(facing: SceneAnchorFacing) -> AnimationState {
        match facing {
            SceneAnchorFacing::Up => AnimationState::IdleUp,
            SceneAnchorFacing::Down => AnimationState::IdleDown,
            SceneAnchorFacing::Left => AnimationState::IdleLeft,
            SceneAnchorFacing::Right => AnimationState::IdleRight,
        }
    }

    fn resolve_spawn_anchor(
        &self,
        scene: &Scene,
        spawn_point_id: &str,
    ) -> Result<(glam::IVec2, Option<SceneAnchorFacing>), String> {
        let anchor = scene.get_anchor(spawn_point_id).ok_or_else(|| {
            format!(
                "Scene '{}' could not resolve spawn point '{}'",
                scene.name, spawn_point_id
            )
        })?;
        Ok((anchor.position, anchor.facing))
    }
}
