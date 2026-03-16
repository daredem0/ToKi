use super::*;

impl EditorApp {
    pub(super) fn activate_loaded_project(
        &mut self,
        game_state: GameState,
        project_path: std::path::PathBuf,
        context: &str,
    ) {
        match SceneViewport::with_game_state(game_state) {
            Ok(viewport) => {
                self.scene_viewport = self.initialize_viewport(viewport);
                self.last_loaded_active_scene = None;
                self.loaded_scene_maps.clear();

                self.config.set_project_path(project_path);
                if let Err(error) = self.config.save() {
                    tracing::warn!(
                        "Failed to save config after activating {}: {}",
                        context,
                        error
                    );
                }

                if let Some(project) = self.project_manager.current_project.as_ref() {
                    self.ui.set_title(&project.name.to_string());
                }

                match self.project_manager.load_scenes() {
                    Ok(loaded_scenes) => {
                        self.ui.load_scenes_from_project(loaded_scenes);
                        tracing::info!("Loaded scenes into UI hierarchy");
                    }
                    Err(error) => {
                        tracing::error!("Failed to load scenes into UI: {}", error);
                    }
                }

                self.migrate_legacy_graph_layouts_into_project();
                self.sync_ui_graph_layouts_from_project();
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
        match self.project_manager.open_project(project_path.clone()) {
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
        self.ui.open_project_requested = false;

        let project_path = if let Some(config_path) = &self.config.project_path {
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
        self.ui.browse_for_project_requested = false;

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
        self.ui.save_project_requested = false;

        if let Some(project) = self.project_manager.current_project.as_mut() {
            project.metadata.editor.graph_layouts = self.ui.export_graph_layouts_for_project();
            project.metadata.editor.rule_graph_drafts =
                self.ui.export_rule_graph_drafts_for_project();
        }

        let scenes = &self.ui.scenes;
        match self.project_manager.save_current_project(scenes) {
            Ok(_) => {
                tracing::info!("Project saved successfully");
                self.ui.clear_graph_layout_dirty();
            }
            Err(error) => {
                tracing::error!("Failed to save project: {}", error);
            }
        }
    }

    pub(super) fn handle_init_project_request(&mut self) {
        self.ui.init_config_requested = false;

        match EditorConfig::init_default_config() {
            Ok(new_config) => {
                self.config = new_config;
                tracing::info!("Config initialized successfully");
            }
            Err(error) => {
                tracing::error!("Failed to initialize config: {}", error);
            }
        }
    }

    pub(super) fn handle_project_requests(&mut self, _event_loop: &ActiveEventLoop) {
        self.poll_background_task_updates();

        if self.ui.cancel_background_task_requested {
            self.ui.cancel_background_task_requested = false;
            if self.background_tasks.request_cancel() {
                tracing::info!("Background task cancellation requested");
            }
        }

        if self.ui.new_project_requested {
            self.handle_new_project_requested(ProjectTemplateKind::Empty);
        }

        if self.ui.new_top_down_project_requested {
            self.handle_new_project_requested(ProjectTemplateKind::TopDownStarter);
        }

        if self.ui.open_project_requested {
            self.handle_open_project_request();
        }

        if self.ui.browse_for_project_requested {
            self.handle_browse_for_project_request();
        }

        if self.ui.save_project_requested {
            self.handle_save_project_request();
        }

        if self.ui.export_project_requested {
            self.handle_export_project_request();
        }

        if self.ui.init_config_requested {
            self.handle_init_project_request();
        }

        if self.ui.validate_assets_requested {
            self.handle_validate_assets_request();
        }
    }

    pub(super) fn handle_validate_assets_request(&mut self) {
        self.ui.validate_assets_requested = false;

        if self.background_tasks.is_running() {
            tracing::warn!("Cannot validate assets: another background task is running");
            return;
        }

        let Some(project_path) = self.config.current_project_path().cloned() else {
            tracing::warn!("No project loaded - cannot validate assets");
            return;
        };

        tracing::info!("Starting asset validation task");
        if let Err(error) = self
            .background_tasks
            .start_validate_assets(ValidateAssetsJob { project_path })
        {
            tracing::error!("Failed to start asset validation task: {}", error);
        } else {
            self.poll_background_task_updates();
        }
    }
}
