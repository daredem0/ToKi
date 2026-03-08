use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::scene::Scene;

/// Manages all scenes within a GameState.
///
/// Provides a centralized way to load, store, and switch between scenes
/// while maintaining a single source of truth for scene data.
#[derive(Debug, Serialize, Deserialize)]
pub struct SceneManager {
    /// All available scenes by name
    scenes: HashMap<String, Scene>,

    /// Currently active scene name
    active_scene: Option<String>,
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
        let scene_name = scene.name.clone();
        self.scenes.insert(scene_name, scene);
    }

    /// Remove a scene
    pub fn remove_scene(&mut self, scene_name: &str) -> bool {
        if Some(scene_name) == self.active_scene.as_deref() {
            self.active_scene = None;
        }
        self.scenes.remove(scene_name).is_some()
    }

    /// Get a reference to a scene
    pub fn get_scene(&self, scene_name: &str) -> Option<&Scene> {
        self.scenes.get(scene_name)
    }

    /// Get a mutable reference to a scene
    pub fn get_scene_mut(&mut self, scene_name: &str) -> Option<&mut Scene> {
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
    pub fn set_active_scene(&mut self, scene_name: &str) -> Result<(), String> {
        if !self.scenes.contains_key(scene_name) {
            return Err(format!("Scene '{}' not found", scene_name));
        }
        self.active_scene = Some(scene_name.to_string());
        Ok(())
    }

    /// Get the name of the active scene
    pub fn active_scene_name(&self) -> Option<&str> {
        self.active_scene.as_deref()
    }

    /// Get all scene names
    pub fn scene_names(&self) -> Vec<&str> {
        self.scenes.keys().map(|s| s.as_str()).collect()
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
