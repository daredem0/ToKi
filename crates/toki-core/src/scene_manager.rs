use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ids::SceneId;
use crate::scene::Scene;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SceneManagerError {
    #[error("scene '{scene_name}' not found")]
    MissingScene { scene_name: SceneId },
}

/// Manages all scenes within a GameState.
///
/// Provides a centralized way to load, store, and switch between scenes
/// while maintaining a single source of truth for scene data.
#[derive(Debug, Serialize, Deserialize)]
pub struct SceneManager {
    /// All available scenes by name
    scenes: HashMap<SceneId, Scene>,

    /// Currently active scene name
    active_scene: Option<SceneId>,
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneManager {
    /// Create a new empty SceneManager
    pub fn new() -> Self {
        Self {
            scenes: HashMap::new(),
            active_scene: None,
        }
    }

    /// Add or update a scene
    pub fn add_scene(&mut self, scene: Scene) {
        let scene_id = SceneId::from(scene.name.as_str());
        self.scenes.insert(scene_id, scene);
    }

    /// Remove a scene
    pub fn remove_scene(&mut self, scene_name: &str) -> bool {
        if self.active_scene.as_deref() == Some(scene_name) {
            self.active_scene = None;
        }
        self.scenes.remove(scene_name).is_some()
    }

    /// Get a reference to a scene
    pub fn get_scene(&self, scene_name: &str) -> Option<&Scene> {
        self.scenes.get(scene_name)
    }

    pub fn get_scene_by_id(&self, scene_name: &SceneId) -> Option<&Scene> {
        self.scenes.get(scene_name)
    }

    /// Get a mutable reference to a scene
    pub fn get_scene_mut(&mut self, scene_name: &str) -> Option<&mut Scene> {
        self.scenes.get_mut(scene_name)
    }

    pub fn get_scene_mut_by_id(&mut self, scene_name: &SceneId) -> Option<&mut Scene> {
        self.scenes.get_mut(scene_name)
    }

    /// Get reference to the currently active scene
    pub fn active_scene(&self) -> Option<&Scene> {
        self.active_scene
            .as_ref()
            .and_then(|name| self.scenes.get(name))
    }

    /// Get mutable reference to the currently active scene
    pub fn active_scene_mut(&mut self) -> Option<&mut Scene> {
        let active_name = self.active_scene.as_ref()?;
        self.scenes.get_mut(active_name)
    }

    /// Set the active scene
    pub fn set_active_scene(&mut self, scene_name: &SceneId) -> Result<(), SceneManagerError> {
        if !self.scenes.contains_key(scene_name) {
            return Err(SceneManagerError::MissingScene {
                scene_name: scene_name.clone(),
            });
        }
        self.active_scene = Some(scene_name.clone());
        Ok(())
    }

    pub fn set_active_scene_by_name(&mut self, scene_name: &str) -> Result<(), SceneManagerError> {
        self.set_active_scene(&scene_name.into())
    }

    /// Get the name of the active scene
    pub fn active_scene_name(&self) -> Option<&str> {
        self.active_scene.as_deref()
    }

    pub fn active_scene_id(&self) -> Option<&SceneId> {
        self.active_scene.as_ref()
    }

    /// Get all scene names
    pub fn scene_names(&self) -> Vec<&SceneId> {
        self.scenes.keys().collect()
    }

    pub fn scene_entries(&self) -> impl Iterator<Item = (&SceneId, &Scene)> {
        self.scenes.iter()
    }

    /// Check if a scene exists
    pub fn has_scene(&self, scene_name: &str) -> bool {
        self.scenes.contains_key(scene_name)
    }

    /// Clear the active scene
    pub fn clear_active_scene(&mut self) {
        self.active_scene = None;
    }

    /// Get the number of scenes
    pub fn scene_count(&self) -> usize {
        self.scenes.len()
    }

    /// Check if there's an active scene
    pub fn has_active_scene(&self) -> bool {
        self.active_scene.is_some()
    }
}
