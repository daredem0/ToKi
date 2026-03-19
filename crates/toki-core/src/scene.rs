use serde::{Deserialize, Serialize};

use crate::entity::{Entity, EntityId};
use crate::rules::RuleSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneAnchorKind {
    SpawnPoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneAnchorFacing {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneAnchor {
    pub id: String,
    pub kind: SceneAnchorKind,
    pub position: glam::IVec2,
    pub facing: Option<SceneAnchorFacing>,
}

/// Represents a game scene - a complete game environment with entities, maps, and metadata.
///
/// A scene is a self-contained game environment that can be loaded, saved, and edited.
/// Unlike GameState which is for runtime execution, Scene is for data persistence and editing.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Scene {
    /// Scene metadata
    pub name: String,
    pub description: Option<String>,

    /// Map configuration
    /// List of map names associated with this scene
    pub maps: Vec<String>,

    /// Entities in this scene
    pub entities: Vec<Entity>,

    /// Data-driven rules authored for this scene.
    #[serde(default)]
    pub rules: RuleSet,

    /// Scene-specific camera settings (optional override)
    pub camera_position: Option<glam::IVec2>,
    pub camera_scale: Option<u32>,

    /// Optional background music track id for this scene.
    #[serde(default)]
    pub background_music_track_id: Option<String>,

    /// Placeable authored scene anchors such as spawn points.
    #[serde(default)]
    pub anchors: Vec<SceneAnchor>,
}

impl Scene {
    /// Create a new empty scene with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            maps: Vec::new(),
            entities: Vec::new(),
            rules: RuleSet::default(),
            camera_position: None,
            camera_scale: None,
            background_music_track_id: None,
            anchors: Vec::new(),
        }
    }

    /// Create a scene with maps
    pub fn with_maps(name: String, maps: Vec<String>) -> Self {
        Self {
            name,
            description: None,
            maps,
            entities: Vec::new(),
            rules: RuleSet::default(),
            camera_position: None,
            camera_scale: None,
            background_music_track_id: None,
            anchors: Vec::new(),
        }
    }

    /// Add an entity to the scene
    pub fn add_entity(&mut self, entity: Entity) -> EntityId {
        let id = entity.id;
        self.entities.push(entity);
        id
    }

    /// Remove an entity from the scene
    pub fn remove_entity(&mut self, entity_id: EntityId) -> bool {
        let initial_len = self.entities.len();
        self.entities.retain(|e| e.id != entity_id);
        self.entities.len() != initial_len
    }

    /// Get an entity by ID
    pub fn get_entity(&self, entity_id: EntityId) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == entity_id)
    }

    /// Get a mutable reference to an entity by ID
    pub fn get_entity_mut(&mut self, entity_id: EntityId) -> Option<&mut Entity> {
        self.entities.iter_mut().find(|e| e.id == entity_id)
    }

    /// Add a map to this scene
    pub fn add_map(&mut self, map_name: String) {
        if !self.maps.contains(&map_name) {
            self.maps.push(map_name);
        }
    }

    /// Remove a map from this scene
    pub fn remove_map(&mut self, map_name: &str) -> bool {
        let initial_len = self.maps.len();
        self.maps.retain(|m| m != map_name);
        self.maps.len() != initial_len
    }

    /// Check if this scene has a specific map
    pub fn has_map(&self, map_name: &str) -> bool {
        self.maps.contains(&map_name.to_string())
    }

    /// Add a scene anchor.
    pub fn add_anchor(&mut self, anchor: SceneAnchor) {
        self.anchors.push(anchor);
    }

    /// Remove an anchor by id.
    pub fn remove_anchor(&mut self, anchor_id: &str) -> bool {
        let initial_len = self.anchors.len();
        self.anchors.retain(|anchor| anchor.id != anchor_id);
        self.anchors.len() != initial_len
    }

    /// Get an anchor by id.
    pub fn get_anchor(&self, anchor_id: &str) -> Option<&SceneAnchor> {
        self.anchors.iter().find(|anchor| anchor.id == anchor_id)
    }

    /// Get a mutable anchor by id.
    pub fn get_anchor_mut(&mut self, anchor_id: &str) -> Option<&mut SceneAnchor> {
        self.anchors
            .iter_mut()
            .find(|anchor| anchor.id == anchor_id)
    }
}
