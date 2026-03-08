use serde::{Deserialize, Serialize};

use crate::entity::{Entity, EntityId};

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

    /// Scene-specific camera settings (optional override)
    pub camera_position: Option<glam::IVec2>,
    pub camera_scale: Option<u32>,
}

impl Scene {
    /// Create a new empty scene with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            maps: Vec::new(),
            entities: Vec::new(),
            camera_position: None,
            camera_scale: None,
        }
    }

    /// Create a scene with maps
    pub fn with_maps(name: String, maps: Vec<String>) -> Self {
        Self {
            name,
            description: None,
            maps,
            entities: Vec::new(),
            camera_position: None,
            camera_scale: None,
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
}
