use super::*;
use toki_core::assets::tileset::TileSetMeta;

impl EditorApp {
    pub(super) fn build_map_editor_draft(
        project_assets: &ProjectAssets,
        name: &str,
        width: u32,
        height: u32,
        tile_width: u32,
        tile_height: u32,
    ) -> Result<MapEditorDraft> {
        if name.trim().is_empty() {
            return Err(anyhow::anyhow!("Map name cannot be empty"));
        }
        if name.contains('/') || name.contains('\\') {
            return Err(anyhow::anyhow!("Map name cannot contain path separators"));
        }

        if project_assets.sprite_atlases.is_empty() {
            return Err(anyhow::anyhow!("No sprite atlases available for new map"));
        }

        let tilemap = TileMap {
            size: glam::UVec2::new(width.max(1), height.max(1)),
            tile_size: glam::UVec2::new(tile_width.max(1), tile_height.max(1)),
            tileset: PathBuf::from(format!("{}.json", name.trim())),
            layers: vec![TileLayer::new_empty(
                "ground",
                width.max(1) as usize * height.max(1) as usize,
            )],
        };

        Ok(MapEditorDraft {
            name: name.trim().to_string(),
            tilemap,
        })
    }

    pub(super) fn build_map_editor_tileset_draft(
        project_assets: &ProjectAssets,
        map_name: &str,
    ) -> Result<TileSetMeta> {
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
        let atlas_file_name = atlas_asset
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("Atlas path has no valid file name"))?;
        let mut tileset = TileSetMeta::from_atlas(atlas_file_name, &atlas_meta);
        for entry in tileset.entries.values_mut() {
            if entry.display_name.is_none() {
                entry.display_name = Some(format!("{map_name}: {}", entry.source_name));
            }
        }
        Ok(tileset)
    }

    pub(super) fn tilemap_to_save_for_map_editor_draft(
        draft: &MapEditorDraft,
        viewport_tilemap: Option<&TileMap>,
    ) -> TileMap {
        viewport_tilemap
            .cloned()
            .unwrap_or_else(|| draft.tilemap.clone())
    }

    fn tileset_to_save(ui: &crate::ui::EditorUI, tilemap: &TileMap) -> TileSetMeta {
        crate::ui::editor_context::map_state(ui)
            .modified_tileset
            .clone()
            .unwrap_or(TileSetMeta {
                tile_size: tilemap.tile_size,
                entries: std::collections::HashMap::new(),
            })
    }

    pub(super) fn handle_map_requests(&mut self) {
        // Handle Map Loading request
        if let Some(request) = crate::ui::editor_context::map_state_mut(&mut self.tabs.ui)
            .load_requested
            .take()
        {
            let scene_name = request.scene_name;
            let map_name = request.map_name;
            if let Some(config) = self.core.config.current_project_path() {
                let map_file = config
                    .join("assets")
                    .join("tilemaps")
                    .join(format!("{}.json", map_name));

                if let Some(viewport) = &mut self.viewport_manager.scene {
                    match viewport.load_tilemap(&map_file) {
                        Ok(()) => {
                            tracing::info!(
                                "Successfully loaded map '{}' from scene '{}' into viewport",
                                map_name,
                                scene_name
                            );
                            self.project_session
                                .loaded_scene_maps
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
        let Some(request) = crate::ui::editor_context::map_state_mut(&mut self.tabs.ui)
            .new_map_requested
            .take()
        else {
            return;
        };

        let Some(project_assets) = self.core.project_manager.get_project_assets() else {
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
            request.tile_width,
            request.tile_height,
        ) {
            Ok(draft) => {
                let tileset =
                    match Self::build_map_editor_tileset_draft(project_assets, &request.name) {
                        Ok(tileset) => tileset,
                        Err(error) => {
                            tracing::error!(
                                "Failed to create tileset draft for new map '{}': {}",
                                request.name,
                                error
                            );
                            return;
                        }
                    };
                let Some(viewport) = &mut self.viewport_manager.map_editor else {
                    tracing::warn!(
                        "No map editor viewport available for new map '{}'",
                        request.name
                    );
                    return;
                };

                if let Err(error) = viewport.set_tilemap(draft.tilemap.clone()) {
                    tracing::error!(
                        "Failed to load new map draft '{}' into map editor viewport: {}",
                        draft.name,
                        error
                    );
                    return;
                }

                crate::ui::editor_ui::set_map_editor_draft(&mut self.tabs.ui, draft);
                crate::ui::editor_context::map_state_mut(&mut self.tabs.ui).modified_tileset =
                    Some(tileset.clone());
                if let Some(project_path) = self.core.config.current_project_path().cloned() {
                    if let Err(error) =
                        viewport.set_tileset_for_current_tilemap(&project_path, tileset)
                    {
                        tracing::error!(
                            "Failed to seed new map editor draft tileset into viewport: {}",
                            error
                        );
                    }
                }
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
        if !crate::ui::editor_context::map_state_mut(&mut self.tabs.ui).save_requested {
            return;
        }

        if let Some(draft) = crate::ui::editor_context::map_state_mut(&mut self.tabs.ui)
            .draft
            .clone()
        {
            let live_tilemap = self
                .viewport_manager
                .map_editor
                .as_ref()
                .and_then(|viewport| viewport.tilemap());
            let tilemap_to_save = Self::tilemap_to_save_for_map_editor_draft(&draft, live_tilemap);
            let tileset_to_save = Self::tileset_to_save(&self.tabs.ui, &tilemap_to_save);
            if let Err(error) = self
                .core
                .project_manager
                .save_tileset_asset(&draft.name, &tileset_to_save)
            {
                tracing::error!(
                    "Failed to save map editor tileset '{}': {}",
                    draft.name,
                    error
                );
                crate::ui::editor_context::map_state_mut(&mut self.tabs.ui).save_requested = false;
                return;
            }
            match self
                .core
                .project_manager
                .save_tilemap_asset(&draft.name, &tilemap_to_save)
            {
                Ok(_) => {
                    tracing::info!("Saved map editor draft '{}'", draft.name);
                    crate::ui::editor_ui::finalize_saved_map_editor_draft(
                        &mut self.tabs.ui,
                        draft.name,
                    );
                }
                Err(error) => {
                    tracing::error!(
                        "Failed to save map editor draft '{}': {}",
                        draft.name,
                        error
                    );
                    crate::ui::editor_context::map_state_mut(&mut self.tabs.ui).save_requested =
                        false;
                }
            }
            return;
        }

        let Some(active_map_name) = crate::ui::editor_context::map_state_mut(&mut self.tabs.ui)
            .active_map
            .clone()
        else {
            crate::ui::editor_context::map_state_mut(&mut self.tabs.ui).save_requested = false;
            return;
        };
        let Some(tilemap) = self
            .viewport_manager
            .map_editor
            .as_ref()
            .and_then(|viewport| viewport.tilemap().cloned())
        else {
            crate::ui::editor_context::map_state_mut(&mut self.tabs.ui).save_requested = false;
            return;
        };
        let tileset_to_save = Self::tileset_to_save(&self.tabs.ui, &tilemap);
        if let Err(error) = self
            .core
            .project_manager
            .save_tileset_asset(&active_map_name, &tileset_to_save)
        {
            tracing::error!(
                "Failed to save map editor tileset '{}': {}",
                active_map_name,
                error
            );
            crate::ui::editor_context::map_state_mut(&mut self.tabs.ui).save_requested = false;
            return;
        }

        match self
            .core
            .project_manager
            .save_tilemap_asset(&active_map_name, &tilemap)
        {
            Ok(_) => {
                tracing::info!("Saved map editor asset '{}'", active_map_name);
                Self::save_modified_atlas(&mut self.tabs.ui);
                crate::ui::editor_ui::finalize_saved_existing_map(&mut self.tabs.ui);
            }
            Err(error) => {
                tracing::error!(
                    "Failed to save map editor asset '{}': {}",
                    active_map_name,
                    error
                );
                crate::ui::editor_context::map_state_mut(&mut self.tabs.ui).save_requested = false;
            }
        }
    }

    pub(super) fn handle_map_editor_map_requests(&mut self) {
        if crate::ui::editor_ui::has_unsaved_map_editor_draft(&self.tabs.ui) {
            crate::ui::editor_context::map_state_mut(&mut self.tabs.ui).map_load_requested = None;
            return;
        }

        let Some(map_name) = crate::ui::editor_context::map_state_mut(&mut self.tabs.ui)
            .map_load_requested
            .take()
        else {
            return;
        };

        let Some(project_path) = self.core.config.current_project_path().cloned() else {
            tracing::warn!(
                "No project loaded for map editor loading request: '{}'",
                map_name
            );
            return;
        };

        let Some(viewport) = &mut self.viewport_manager.map_editor else {
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

        viewport.clear_tilemap();
        match viewport.load_tilemap(&map_file) {
            Ok(()) => {
                tracing::info!("Loaded map '{}' into map editor viewport", map_name);
                if let Ok(tilemap) = toki_core::assets::tilemap::TileMap::load_from_file(&map_file)
                {
                    let modified_tileset = Self::resolve_tileset_path(&project_path, &tilemap)
                        .and_then(|tileset_path| TileSetMeta::load_from_file(&tileset_path).ok());
                    let draft = crate::ui::editor_ui::MapEditorDraft {
                        name: map_name.clone(),
                        tilemap,
                    };
                    let state = crate::ui::editor_context::map_state_mut(&mut self.tabs.ui);
                    state.draft = Some(draft);
                    state.modified_tileset = modified_tileset;
                    state.atlas_path = None;
                    state.modified_atlas = None;
                }
                crate::ui::editor_context::map_state_mut(&mut self.tabs.ui).active_map =
                    Some(map_name);
                crate::ui::editor_ui::clear_map_editor_dirty(&mut self.tabs.ui);
                crate::ui::editor_ui::clear_map_editor_history(&mut self.tabs.ui);
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

    fn save_modified_atlas(ui: &mut crate::ui::EditorUI) {
        let state = crate::ui::editor_context::map_state_mut(ui);
        let Some(atlas) = state.modified_atlas.take() else {
            return;
        };
        let Some(atlas_path) = &state.atlas_path else {
            tracing::warn!("Modified atlas exists but no atlas_path set; cannot save");
            return;
        };
        let json = match serde_json::to_string_pretty(&atlas) {
            Ok(json) => json,
            Err(e) => {
                tracing::error!("Failed to serialize modified atlas: {e}");
                return;
            }
        };
        let path = atlas_path.clone();
        if let Err(e) = std::fs::write(&path, json) {
            tracing::error!("Failed to write atlas to {}: {e}", path.display());
        } else {
            tracing::info!("Saved modified atlas to {}", path.display());
        }
    }

    fn resolve_tileset_path(
        project_path: &std::path::Path,
        tilemap: &toki_core::assets::tilemap::TileMap,
    ) -> Option<std::path::PathBuf> {
        toki_core::project_assets::resolve_tilemap_tileset_path(
            project_path,
            &project_path
                .join("assets")
                .join("tilemaps")
                .join("__editor__.json"),
            tilemap,
        )
    }

    pub(super) fn handle_pending_map_editor_tilemap_sync(&mut self) {
        let Some(tilemap) =
            crate::ui::editor_ui::take_pending_map_editor_tilemap_sync(&mut self.tabs.ui)
        else {
            return;
        };

        let Some(viewport) = &mut self.viewport_manager.map_editor else {
            return;
        };

        match viewport.set_tilemap(tilemap) {
            Ok(()) => viewport.mark_dirty(),
            Err(error) => tracing::error!(
                "Failed to apply pending map editor undo/redo snapshot to viewport: {}",
                error
            ),
        }
    }

    pub(super) fn handle_pending_map_editor_tileset_sync(&mut self) {
        let Some(tileset) =
            crate::ui::editor_ui::take_pending_map_editor_tileset_sync(&mut self.tabs.ui)
        else {
            return;
        };

        let Some(project_path) = self.core.config.current_project_path().cloned() else {
            return;
        };
        let Some(viewport) = &mut self.viewport_manager.map_editor else {
            return;
        };

        if let Err(error) = viewport.set_tileset_for_current_tilemap(&project_path, tileset) {
            tracing::error!(
                "Failed to apply pending map editor tileset snapshot to viewport: {}",
                error
            );
        }
    }
}
