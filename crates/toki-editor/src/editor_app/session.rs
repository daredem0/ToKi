use super::*;
use crate::project::ProjectAssets;
use toki_core::project_assets::tilemap_file_path;
use toki_core::project_content::build_game_state_from_scene;

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
        *current_scene != self.session.last_loaded_active_scene
            || self.core.ui.scene_content_changed
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
        let preferred_map = self
            .session
            .loaded_scene_maps
            .get(scene_name)
            .map(String::as_str);
        let map_to_load = Self::resolve_scene_map_to_load(&active_scene, preferred_map);
        let preview_game_state = {
            let project_assets = self.core.project_manager.get_project_assets_mut();
            Self::build_scene_preview_game_state(&active_scene, project_assets)
        };

        Self::load_scene_into_gamestate(viewport, scene_name, preview_game_state);
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

    pub(super) fn build_scene_preview_game_state(
        scene: &toki_core::Scene,
        project_assets: Option<&mut ProjectAssets>,
    ) -> Result<toki_core::GameState, String> {
        let mut entity_definitions = Vec::new();

        if let Some(player_entry) = scene.player_entry.as_ref() {
            let project_assets = project_assets.ok_or_else(|| {
                format!(
                    "Scene '{}' has a player entry but no project assets are available",
                    scene.name
                )
            })?;
            let definition = project_assets
                .load_entity_definition(&player_entry.entity_definition_name)
                .map_err(|error| {
                    format!(
                        "Failed to load player entity definition '{}' for scene '{}': {}",
                        player_entry.entity_definition_name, scene.name, error
                    )
                })?
                .ok_or_else(|| {
                    format!(
                        "Scene '{}' references missing player entity definition '{}'",
                        scene.name, player_entry.entity_definition_name
                    )
                })?;
            entity_definitions.push(definition);
        }

        build_game_state_from_scene(scene.clone(), entity_definitions)
    }

    pub(super) fn load_scene_into_gamestate(
        viewport: &mut crate::scene::SceneViewport,
        scene_name: &str,
        preview_game_state: Result<toki_core::GameState, String>,
    ) {
        match preview_game_state {
            Ok(game_state) => {
                *viewport.game_state_mut() = game_state;
                viewport.mark_dirty();
                tracing::info!(
                    "Loaded active scene '{}' with {} entities into GameState",
                    scene_name,
                    viewport.game_state().entities().len()
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

        let map_file = tilemap_file_path(project_path, map_name);

        match viewport.load_tilemap(&map_file) {
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
            viewport.clear_tilemap();
        }
        tracing::debug!("No active scene set, cleared viewport");
    }
}
