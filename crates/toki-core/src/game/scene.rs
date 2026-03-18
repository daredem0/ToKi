use super::{GameState, RuleRuntimeState};
use crate::entity::{Entity, EntityId, EntityManager};
use crate::rules::RuleSet;
use crate::scene::Scene;
use crate::scene_manager::SceneManager;
use crate::sprite::SpriteInstance;
use std::collections::{HashMap, HashSet};

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
            player_id: Some(player_id),
            keys_held: HashSet::new(),
            profile_keys_held: HashMap::new(),
            profile_actions_held: HashMap::new(),
            pending_profile_actions: HashMap::new(),
            debug_collision_rendering: false,
            npc_ai_frame_counter: 0,
            rules: RuleSet::default(),
            rule_runtime: RuleRuntimeState::default(),
            pending_stat_changes: Vec::new(),
        }
    }

    /// Create a new empty GameState with no entities
    pub fn new_empty() -> Self {
        Self {
            scene_manager: SceneManager::new(),
            entity_manager: EntityManager::new(),
            player_id: None,
            keys_held: HashSet::new(),
            profile_keys_held: HashMap::new(),
            profile_actions_held: HashMap::new(),
            pending_profile_actions: HashMap::new(),
            debug_collision_rendering: false,
            npc_ai_frame_counter: 0,
            rules: RuleSet::default(),
            rule_runtime: RuleRuntimeState::default(),
            pending_stat_changes: Vec::new(),
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
                ai_behavior: crate::entity::AiBehavior::None,
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
                        frame_duration_ms: 300.0,
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
                        frame_duration_ms: 150.0,
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
                ai_behavior: crate::entity::AiBehavior::Wander,
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
                        frame_duration_ms: 300.0,
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
                        frame_duration_ms: 150.0,
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

        self.scene_manager.set_active_scene(scene_name)?;

        self.entity_manager = EntityManager::new();
        self.player_id = None;
        self.profile_keys_held.clear();
        self.profile_actions_held.clear();
        self.pending_profile_actions.clear();
        self.pending_stat_changes.clear();
        self.set_rules(scene.rules.clone());

        for entity in scene.entities {
            self.entity_manager.add_existing_entity(entity);
        }
        self.player_id = self.entity_manager.get_player_id();

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
}
