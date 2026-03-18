use super::*;

impl EditorApp {
    pub(super) fn resolve_scene_map_to_load(
        scene: &toki_core::Scene,
        preferred_map: Option<&str>,
    ) -> Option<String> {
        if let Some(preferred_map) = preferred_map {
            if scene.maps.iter().any(|map| map == preferred_map) {
                return Some(preferred_map.to_string());
            }
        }

        scene.maps.first().cloned()
    }

    pub(super) fn handle_active_scene_map_loading(&mut self) {
        let current_active_scene = self.core.ui.active_scene.clone();

        if !self.should_reload_scene(&current_active_scene) {
            return;
        }

        self.update_scene_state(&current_active_scene);

        match &current_active_scene {
            Some(scene_name) => self.load_active_scene(scene_name),
            None => self.clear_viewport_scene(),
        }
    }

    pub(super) fn should_reload_scene(&self, current_scene: &Option<String>) -> bool {
        *current_scene != self.session.last_loaded_active_scene || self.core.ui.scene_content_changed
    }

    pub(super) fn update_scene_state(&mut self, current_scene: &Option<String>) {
        self.session.last_loaded_active_scene = current_scene.clone();
        self.core.ui.scene_content_changed = false;

        tracing::info!(
            "Active scene or content changed, reloading map for scene: {:?}",
            current_scene
        );

        if let Some(viewport) = &mut self.viewports.scene {
            viewport.mark_dirty();
        }
    }

    pub(super) fn load_active_scene(&mut self, scene_name: &str) {
        let Some(active_scene) = self.find_scene_by_name(scene_name).cloned() else {
            tracing::warn!("Active scene '{}' not found in scenes list", scene_name);
            return;
        };

        tracing::info!(
            "Found active scene '{}' with {} maps: {:?}",
            scene_name,
            active_scene.maps.len(),
            active_scene.maps
        );

        let Some(viewport) = &mut self.viewports.scene else {
            return;
        };

        let project_path = self.core.config.current_project_path().cloned();
        let preferred_map = self.session.loaded_scene_maps.get(scene_name).map(String::as_str);
        let map_to_load = Self::resolve_scene_map_to_load(&active_scene, preferred_map);

        Self::load_scene_into_gamestate(viewport, &active_scene, scene_name);
        Self::load_scene_tilemap(
            viewport,
            scene_name,
            map_to_load.as_deref(),
            project_path.as_deref(),
        );

        if preferred_map.is_some() && map_to_load.as_deref() != preferred_map {
            self.session.loaded_scene_maps.remove(scene_name);
        }
    }

    pub(super) fn find_scene_by_name(&self, scene_name: &str) -> Option<&toki_core::Scene> {
        self.core.ui.scenes.iter().find(|s| s.name == scene_name)
    }

    pub(super) fn load_scene_into_gamestate(
        viewport: &mut crate::scene::SceneViewport,
        scene: &toki_core::Scene,
        scene_name: &str,
    ) {
        viewport
            .scene_manager_mut()
            .game_state_mut()
            .add_scene(scene.clone());

        match viewport
            .scene_manager_mut()
            .game_state_mut()
            .load_scene(scene_name)
        {
            Ok(()) => {
                tracing::info!(
                    "Loaded active scene '{}' with {} entities into GameState",
                    scene_name,
                    scene.entities.len()
                );
            }
            Err(e) => {
                tracing::error!(
                    "Failed to load active scene '{}' into GameState: {}",
                    scene_name,
                    e
                );
            }
        }
    }

    pub(super) fn load_scene_tilemap(
        viewport: &mut crate::scene::SceneViewport,
        scene_name: &str,
        map_name: Option<&str>,
        project_path: Option<&std::path::Path>,
    ) {
        let Some(map_name) = map_name else {
            viewport.mark_dirty();
            return;
        };

        let Some(project_path) = project_path else {
            tracing::warn!("No project path available for loading tilemap");
            return;
        };

        let map_file = project_path
            .join("assets")
            .join("tilemaps")
            .join(format!("{}.json", map_name));

        match viewport.scene_manager_mut().load_tilemap(&map_file) {
            Ok(()) => {
                tracing::info!(
                    "Loaded active scene '{}' map '{}' into viewport",
                    scene_name,
                    map_name
                );
                viewport.mark_dirty();
            }
            Err(e) => {
                tracing::error!(
                    "Failed to load active scene '{}' map '{}': {}",
                    scene_name,
                    map_name,
                    e
                );
            }
        }
    }

    pub(super) fn clear_viewport_scene(&mut self) {
        if let Some(viewport) = &mut self.viewports.scene {
            viewport.scene_manager_mut().clear_tilemap();
        }
        tracing::debug!("No active scene set, cleared viewport");
    }
}
