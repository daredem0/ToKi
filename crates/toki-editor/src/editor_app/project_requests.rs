use super::*;
use crate::editor_services::graph_metadata;
use crate::ui::editor_ui::{DialogEditorState, ProjectRequest};

impl EditorApp {
    pub(super) fn refresh_project_assets_after_rescan(&mut self) {
        let (project_palettes, available_dialogs) = self
            .core
            .project_manager
            .get_project_assets_mut()
            .map(|assets| {
                let palettes = assets.load_project_palettes().unwrap_or_default();
                let dialogs = assets
                    .get_dialog_names()
                    .iter()
                    .filter_map(|dialog_id| assets.load_dialog(dialog_id).ok().flatten())
                    .collect::<Vec<_>>();
                (
                    palettes,
                    DialogEditorState::collect_available_dialogs(&dialogs),
                )
            })
            .unwrap_or_default();
        self.tabs
            .ui
            .project
            .set_available_palettes(&project_palettes);
        self.tabs
            .ui
            .project
            .set_available_dialogs(&available_dialogs);

        if let Some(current_project) = self.core.project_manager.current_project.as_ref() {
            let indexed_palette_override = current_project
                .metadata
                .runtime
                .display
                .indexed_palette_override
                .clone();
            self.tabs.ui.project.indexed_palette_override = indexed_palette_override.clone();
            if let Some(viewport) = self.viewport_manager.scene.as_mut() {
                viewport.set_available_palettes(&self.tabs.ui.project.available_palettes);
                viewport.set_indexed_palette_override(indexed_palette_override.clone());
                viewport.clear_asset_caches();
            }
            if let Some(viewport) = self.viewport_manager.map_editor.as_mut() {
                viewport.set_available_palettes(&self.tabs.ui.project.available_palettes);
                viewport.set_indexed_palette_override(indexed_palette_override);
                viewport.clear_asset_caches();
            }
        }

        self.panel_coordinator.preview_sprite_frames.clear();

        match self.core.project_manager.load_scenes() {
            Ok(loaded_scenes) => self.tabs.ui.load_scenes_from_project(loaded_scenes),
            Err(error) => tracing::error!("Failed to reload scenes after asset rescan: {}", error),
        }
    }

    pub(super) fn activate_loaded_project(
        &mut self,
        game_state: GameState,
        project_path: std::path::PathBuf,
        context: &str,
    ) {
        let (resolution_width, resolution_height) = self
            .core
            .project_manager
            .current_project
            .as_ref()
            .map(|p| {
                (
                    p.metadata.runtime.display.resolution_width,
                    p.metadata.runtime.display.resolution_height,
                )
            })
            .unwrap_or_else(|| {
                (
                    toki_core::project_runtime::default_resolution_width(),
                    toki_core::project_runtime::default_resolution_height(),
                )
            });

        match SceneViewport::with_game_state_and_resolution(
            game_state,
            resolution_width,
            resolution_height,
        ) {
            Ok(viewport) => {
                self.viewport_manager.scene =
                    self.initialize_viewport(|| Ok(viewport), "scene viewport");
                self.project_session.last_loaded_active_scene = None;
                self.project_session.loaded_scene_maps.clear();

                self.core.config.set_project_path(project_path);
                if let Err(error) = self.core.config.save() {
                    tracing::warn!(
                        "Failed to save config after activating {}: {}",
                        context,
                        error
                    );
                }

                let project_name = self
                    .core
                    .project_manager
                    .current_project
                    .as_ref()
                    .map(|project| project.name.clone());
                let (project_palettes, available_dialogs) = self
                    .core
                    .project_manager
                    .get_project_assets_mut()
                    .map(|assets| {
                        let palettes = assets.load_project_palettes().unwrap_or_default();
                        let dialogs = assets
                            .get_dialog_names()
                            .iter()
                            .filter_map(|dialog_id| assets.load_dialog(dialog_id).ok().flatten())
                            .collect::<Vec<_>>();
                        (
                            palettes,
                            DialogEditorState::collect_available_dialogs(&dialogs),
                        )
                    })
                    .unwrap_or_default();
                if let Some(project_name) = project_name {
                    self.tabs.ui.set_title(&project_name);
                    self.tabs
                        .ui
                        .project
                        .set_available_palettes(&project_palettes);
                    self.tabs
                        .ui
                        .project
                        .set_available_dialogs(&available_dialogs);
                }
                let indexed_palette_override = self
                    .core
                    .project_manager
                    .current_project
                    .as_ref()
                    .and_then(|project| {
                        project
                            .metadata
                            .runtime
                            .display
                            .indexed_palette_override
                            .clone()
                    });
                self.tabs.ui.project.indexed_palette_override = indexed_palette_override.clone();
                if let Some(viewport) = self.viewport_manager.scene.as_mut() {
                    viewport.set_available_palettes(&self.tabs.ui.project.available_palettes);
                    viewport.set_indexed_palette_override(indexed_palette_override.clone());
                }
                if let Some(viewport) = self.viewport_manager.map_editor.as_mut() {
                    viewport.set_available_palettes(&self.tabs.ui.project.available_palettes);
                    viewport.set_indexed_palette_override(indexed_palette_override.clone());
                }

                match self.core.project_manager.load_scenes() {
                    Ok(loaded_scenes) => {
                        self.tabs.ui.load_scenes_from_project(loaded_scenes);
                        tracing::info!("Loaded scenes into UI hierarchy");
                    }
                    Err(error) => {
                        tracing::error!("Failed to load scenes into UI: {}", error);
                    }
                }

                graph_metadata::load_into_ui(
                    &mut self.tabs.ui,
                    self.core.project_manager.current_project.as_ref(),
                );
            }
            Err(error) => {
                tracing::error!(
                    "Failed to initialize scene viewport for {}: {}",
                    context,
                    error
                );
            }
        }
    }

    pub(super) fn open_project_at_path(&mut self, project_path: std::path::PathBuf) {
        match self.core.project_manager.open_project(project_path.clone()) {
            Ok(game_state) => {
                self.activate_loaded_project(game_state, project_path, "opened project");
                tracing::info!("Opened project successfully");
            }
            Err(error) => {
                tracing::error!("Failed to open project: {}", error);
            }
        }
    }

    pub(super) fn handle_open_project_request(&mut self) {
        let project_path = if let Some(config_path) = &self.core.config.project_path {
            tracing::info!("Opening project from config: {:?}", config_path);
            Some(config_path.clone())
        } else {
            tracing::info!("No project path in config, asking user to select folder");
            rfd::FileDialog::new()
                .set_title("Open ToKi Project")
                .add_filter("ToKi Project", &["toki"])
                .pick_folder()
        };

        if let Some(project_path) = project_path {
            self.open_project_at_path(project_path);
        }
    }

    pub(super) fn handle_browse_for_project_request(&mut self) {
        if let Some(project_path) = rfd::FileDialog::new()
            .set_title("Browse for ToKi Project")
            .add_filter("ToKi Project", &["toki"])
            .pick_folder()
        {
            self.open_project_at_path(project_path);
            tracing::info!("Opened browsed project successfully");
        }
    }

    pub(super) fn handle_save_project_request(&mut self) {
        if let Err(error) = self.persist_dirty_ui_layout_draft() {
            tracing::error!(
                "Failed to save UI layout draft before project save: {}",
                error
            );
            return;
        }

        if let Some(project) = self.core.project_manager.current_project.as_mut() {
            graph_metadata::copy_ui_into_project(&self.tabs.ui, project);
        }

        let scenes = &self.tabs.ui.scenes;
        match self.core.project_manager.save_current_project(scenes) {
            Ok(_) => {
                tracing::info!("Project saved successfully");
                crate::ui::editor_ui::clear_graph_layout_dirty(&mut self.tabs.ui);
            }
            Err(error) => {
                tracing::error!("Failed to save project: {}", error);
            }
        }
    }

    pub(super) fn handle_reload_project_assets_request(&mut self) {
        if let Err(error) = self.core.project_manager.rescan_assets() {
            tracing::error!("Failed to reload project assets: {}", error);
            return;
        }

        tracing::info!("Reloaded project assets");
        self.refresh_project_assets_after_rescan();
    }

    pub(super) fn handle_init_project_request(&mut self) {
        match EditorConfig::init_default_config() {
            Ok(new_config) => {
                self.core.config = new_config;
                tracing::info!("Config initialized successfully");
            }
            Err(error) => {
                tracing::error!("Failed to initialize config: {}", error);
            }
        }
    }

    pub(super) fn handle_project_requests(&mut self, _event_loop: &ActiveEventLoop) {
        self.poll_background_task_updates();

        if self.tabs.ui.project.cancel_background_task_requested {
            self.tabs.ui.project.cancel_background_task_requested = false;
            if self.command_coordinator.background_tasks.request_cancel() {
                tracing::info!("Background task cancellation requested");
            }
        }

        if self
            .tabs
            .ui
            .project
            .take_request(ProjectRequest::NewProject)
        {
            let suggested_parent = self
                .core
                .config
                .current_project_path()
                .map(|path| Self::suggested_new_project_parent_path(path.as_path()));
            let suggested_name = Self::next_available_project_name(
                suggested_parent
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new(".")),
                "NewProject",
            );
            self.tabs.ui.begin_new_project_dialog(
                ProjectTemplateKind::Empty,
                suggested_parent,
                suggested_name,
            );
        }

        if self
            .tabs
            .ui
            .project
            .take_request(ProjectRequest::NewTopDownProject)
        {
            let suggested_parent = self
                .core
                .config
                .current_project_path()
                .map(|path| Self::suggested_new_project_parent_path(path.as_path()));
            let suggested_name = Self::next_available_project_name(
                suggested_parent
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new(".")),
                "NewProject",
            );
            self.tabs.ui.begin_new_project_dialog(
                ProjectTemplateKind::TopDownStarter,
                suggested_parent,
                suggested_name,
            );
        }

        if let Some(request) = self.tabs.ui.project.new_project_submit_requested.take() {
            self.handle_new_project_requested(request.template, request.parent_path, request.name);
        }

        if self
            .tabs
            .ui
            .project
            .take_request(ProjectRequest::OpenProject)
        {
            self.handle_open_project_request();
        }

        if self
            .tabs
            .ui
            .project
            .take_request(ProjectRequest::BrowseForProject)
        {
            self.handle_browse_for_project_request();
        }

        if self
            .tabs
            .ui
            .project
            .take_request(ProjectRequest::ReloadProjectAssets)
        {
            self.handle_reload_project_assets_request();
        }

        if self
            .tabs
            .ui
            .project
            .take_request(ProjectRequest::SaveProject)
        {
            self.handle_save_project_request();
        }

        if self
            .tabs
            .ui
            .project
            .take_request(ProjectRequest::ExportProject)
        {
            self.handle_export_project_request();
        }

        if self
            .tabs
            .ui
            .project
            .take_request(ProjectRequest::InitConfig)
        {
            self.handle_init_project_request();
        }

        if self
            .tabs
            .ui
            .project
            .take_request(ProjectRequest::ValidateAssets)
        {
            self.handle_validate_assets_request();
        }
    }

    pub(super) fn handle_validate_assets_request(&mut self) {
        if self.command_coordinator.background_tasks.is_running() {
            tracing::warn!("Cannot validate assets: another background task is running");
            return;
        }

        let Some(project_path) = self.core.config.current_project_path().cloned() else {
            tracing::warn!("No project loaded - cannot validate assets");
            return;
        };

        tracing::info!("Starting asset validation task");
        if let Err(error) = self
            .command_coordinator
            .background_tasks
            .start_validate_assets(ValidateAssetsJob { project_path })
        {
            tracing::error!("Failed to start asset validation task: {}", error);
        } else {
            self.poll_background_task_updates();
        }
    }
}
