use super::*;

impl EditorApp {
    pub(super) fn build_map_editor_draft(
        project_assets: &ProjectAssets,
        name: &str,
        width: u32,
        height: u32,
    ) -> Result<MapEditorDraft> {
        if name.trim().is_empty() {
            return Err(anyhow::anyhow!("Map name cannot be empty"));
        }
        if name.contains('/') || name.contains('\\') {
            return Err(anyhow::anyhow!("Map name cannot contain path separators"));
        }

        let mut atlas_names = project_assets
            .sprite_atlases
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        atlas_names.sort();

        let chosen_atlas_name = if project_assets.sprite_atlases.contains_key("terrain") {
            "terrain".to_string()
        } else {
            atlas_names
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("No sprite atlases available for new map"))?
        };

        let atlas_asset = project_assets
            .sprite_atlases
            .get(&chosen_atlas_name)
            .ok_or_else(|| anyhow::anyhow!("Missing atlas asset '{}'", chosen_atlas_name))?;
        let atlas_meta = AtlasMeta::load_from_file(&atlas_asset.path)
            .map_err(|e| anyhow::anyhow!("Failed to load atlas '{}': {}", chosen_atlas_name, e))?;

        let mut tile_names = atlas_meta.tiles.keys().cloned().collect::<Vec<_>>();
        tile_names.sort();
        let fill_tile = tile_names.into_iter().next().ok_or_else(|| {
            anyhow::anyhow!("Atlas '{}' does not define any tiles", chosen_atlas_name)
        })?;

        let tilemap = TileMap {
            size: glam::UVec2::new(width.max(1), height.max(1)),
            tile_size: atlas_meta.tile_size,
            atlas: PathBuf::from(
                atlas_asset
                    .path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Atlas path has no file name"))?,
            ),
            tiles: vec![fill_tile; width.max(1) as usize * height.max(1) as usize],
            objects: vec![],
        };

        Ok(MapEditorDraft {
            name: name.trim().to_string(),
            tilemap,
        })
    }

    pub(super) fn tilemap_to_save_for_map_editor_draft(
        draft: &MapEditorDraft,
        viewport_tilemap: Option<&TileMap>,
    ) -> TileMap {
        viewport_tilemap
            .cloned()
            .unwrap_or_else(|| draft.tilemap.clone())
    }

    pub(super) fn handle_map_requests(&mut self) {
        // Handle Map Loading request
        if let Some((scene_name, map_name)) = self.ui.map_load_requested.take() {
            if let Some(config) = self.config.current_project_path() {
                let map_file = config
                    .join("assets")
                    .join("tilemaps")
                    .join(format!("{}.json", map_name));

                if let Some(viewport) = &mut self.scene_viewport {
                    match viewport.scene_manager_mut().load_tilemap(&map_file) {
                        Ok(()) => {
                            tracing::info!(
                                "Successfully loaded map '{}' from scene '{}' into viewport",
                                map_name,
                                scene_name
                            );
                            self.loaded_scene_maps
                                .insert(scene_name.clone(), map_name.clone());
                            // Mark viewport as needing re-render
                            viewport.mark_dirty();
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to load map '{}' from scene '{}': {}",
                                map_name,
                                scene_name,
                                e
                            );
                        }
                    }
                } else {
                    tracing::warn!(
                        "No scene viewport available for loading map '{}' from scene '{}'",
                        map_name,
                        scene_name
                    );
                }
            } else {
                tracing::warn!(
                    "No project loaded for map loading request: '{}' from scene '{}'",
                    map_name,
                    scene_name
                );
            }
        }
    }

    pub(super) fn handle_new_map_editor_requests(&mut self) {
        let Some(request) = self.ui.map_editor_new_map_requested.take() else {
            return;
        };

        let Some(project_assets) = self.project_manager.get_project_assets() else {
            tracing::warn!(
                "No project assets available for new map request '{}'",
                request.name
            );
            return;
        };

        match Self::build_map_editor_draft(
            project_assets,
            &request.name,
            request.width,
            request.height,
        ) {
            Ok(draft) => {
                let Some(viewport) = &mut self.map_editor_viewport else {
                    tracing::warn!(
                        "No map editor viewport available for new map '{}'",
                        request.name
                    );
                    return;
                };

                if let Err(error) = viewport
                    .scene_manager_mut()
                    .set_tilemap(draft.tilemap.clone())
                {
                    tracing::error!(
                        "Failed to load new map draft '{}' into map editor viewport: {}",
                        draft.name,
                        error
                    );
                    return;
                }

                self.ui.set_map_editor_draft(draft);
                viewport.mark_dirty();
            }
            Err(error) => {
                tracing::error!(
                    "Failed to create new map draft '{}': {}",
                    request.name,
                    error
                );
            }
        }
    }

    pub(super) fn handle_save_map_editor_request(&mut self) {
        if !self.ui.map_editor_save_requested {
            return;
        }

        if let Some(draft) = self.ui.map_editor_draft.clone() {
            let live_tilemap = self
                .map_editor_viewport
                .as_ref()
                .and_then(|viewport| viewport.scene_manager().tilemap());
            let tilemap_to_save = Self::tilemap_to_save_for_map_editor_draft(&draft, live_tilemap);
            match self
                .project_manager
                .save_tilemap_asset(&draft.name, &tilemap_to_save)
            {
                Ok(_) => {
                    tracing::info!("Saved map editor draft '{}'", draft.name);
                    self.ui.finalize_saved_map_editor_draft(draft.name);
                }
                Err(error) => {
                    tracing::error!(
                        "Failed to save map editor draft '{}': {}",
                        draft.name,
                        error
                    );
                    self.ui.map_editor_save_requested = false;
                }
            }
            return;
        }

        let Some(active_map_name) = self.ui.map_editor_active_map.clone() else {
            self.ui.map_editor_save_requested = false;
            return;
        };
        let Some(tilemap) = self
            .map_editor_viewport
            .as_ref()
            .and_then(|viewport| viewport.scene_manager().tilemap().cloned())
        else {
            self.ui.map_editor_save_requested = false;
            return;
        };

        match self
            .project_manager
            .save_tilemap_asset(&active_map_name, &tilemap)
        {
            Ok(_) => {
                tracing::info!("Saved map editor asset '{}'", active_map_name);
                self.ui.finalize_saved_existing_map();
            }
            Err(error) => {
                tracing::error!(
                    "Failed to save map editor asset '{}': {}",
                    active_map_name,
                    error
                );
                self.ui.map_editor_save_requested = false;
            }
        }
    }

    pub(super) fn handle_map_editor_map_requests(&mut self) {
        if self.ui.has_unsaved_map_editor_draft() {
            self.ui.map_editor_map_load_requested = None;
            return;
        }

        let Some(map_name) = self.ui.map_editor_map_load_requested.take() else {
            return;
        };

        let Some(project_path) = self.config.current_project_path().cloned() else {
            tracing::warn!(
                "No project loaded for map editor loading request: '{}'",
                map_name
            );
            return;
        };

        let Some(viewport) = &mut self.map_editor_viewport else {
            tracing::warn!(
                "No map editor viewport available for loading map '{}'",
                map_name
            );
            return;
        };

        let map_file = project_path
            .join("assets")
            .join("tilemaps")
            .join(format!("{}.json", map_name));

        viewport.scene_manager_mut().clear_tilemap();
        match viewport.scene_manager_mut().load_tilemap(&map_file) {
            Ok(()) => {
                tracing::info!("Loaded map '{}' into map editor viewport", map_name);
                self.ui.map_editor_active_map = Some(map_name);
                self.ui.clear_map_editor_dirty();
                self.ui.clear_map_editor_history();
                viewport.mark_dirty();
            }
            Err(e) => {
                tracing::error!(
                    "Failed to load map '{}' into map editor viewport: {}",
                    map_name,
                    e
                );
            }
        }
    }

    pub(super) fn handle_pending_map_editor_tilemap_sync(&mut self) {
        let Some(tilemap) = self.ui.take_pending_map_editor_tilemap_sync() else {
            return;
        };

        let Some(viewport) = &mut self.map_editor_viewport else {
            return;
        };

        match viewport.scene_manager_mut().set_tilemap(tilemap) {
            Ok(()) => viewport.mark_dirty(),
            Err(error) => tracing::error!(
                "Failed to apply pending map editor undo/redo snapshot to viewport: {}",
                error
            ),
        }
    }
}
