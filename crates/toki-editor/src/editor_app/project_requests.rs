use super::*;
use crate::editor_services::graph_metadata;

impl EditorApp {
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
                self.viewports.scene = self.initialize_viewport(viewport);
                self.session.last_loaded_active_scene = None;
                self.session.loaded_scene_maps.clear();

                self.core.config.set_project_path(project_path);
                if let Err(error) = self.core.config.save() {
                    tracing::warn!(
                        "Failed to save config after activating {}: {}",
                        context,
                        error
                    );
                }

                if let Some(project) = self.core.project_manager.current_project.as_ref() {
                    self.core.ui.set_title(&project.name.to_string());
                }

                match self.core.project_manager.load_scenes() {
                    Ok(loaded_scenes) => {
                        self.core.ui.load_scenes_from_project(loaded_scenes);
                        tracing::info!("Loaded scenes into UI hierarchy");
                    }
                    Err(error) => {
                        tracing::error!("Failed to load scenes into UI: {}", error);
                    }
                }

                graph_metadata::load_into_ui(
                    &mut self.core.ui,
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
        self.core.ui.project.open_project_requested = false;

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
        self.core.ui.project.browse_for_project_requested = false;

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
        self.core.ui.project.save_project_requested = false;

        if let Some(project) = self.core.project_manager.current_project.as_mut() {
            graph_metadata::copy_ui_into_project(&self.core.ui, project);
        }

        let scenes = &self.core.ui.scenes;
        match self.core.project_manager.save_current_project(scenes) {
            Ok(_) => {
                tracing::info!("Project saved successfully");
                self.core.ui.clear_graph_layout_dirty();
            }
            Err(error) => {
                tracing::error!("Failed to save project: {}", error);
            }
        }
    }

    pub(super) fn handle_init_project_request(&mut self) {
        self.core.ui.project.init_config_requested = false;

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

        if self.core.ui.project.cancel_background_task_requested {
            self.core.ui.project.cancel_background_task_requested = false;
            if self.background_tasks.request_cancel() {
                tracing::info!("Background task cancellation requested");
            }
        }

        if self.core.ui.project.new_project_requested {
            self.core.ui.project.new_project_requested = false;
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
            self.core.ui.begin_new_project_dialog(
                ProjectTemplateKind::Empty,
                suggested_parent,
                suggested_name,
            );
        }

        if self.core.ui.project.new_top_down_project_requested {
            self.core.ui.project.new_top_down_project_requested = false;
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
            self.core.ui.begin_new_project_dialog(
                ProjectTemplateKind::TopDownStarter,
                suggested_parent,
                suggested_name,
            );
        }

        if let Some(request) = self.core.ui.project.new_project_submit_requested.take() {
            self.handle_new_project_requested(request.template, request.parent_path, request.name);
        }

        if self.core.ui.project.open_project_requested {
            self.handle_open_project_request();
        }

        if self.core.ui.project.browse_for_project_requested {
            self.handle_browse_for_project_request();
        }

        if self.core.ui.project.save_project_requested {
            self.handle_save_project_request();
        }

        if self.core.ui.project.export_project_requested {
            self.handle_export_project_request();
        }

        if self.core.ui.project.init_config_requested {
            self.handle_init_project_request();
        }

        if self.core.ui.project.validate_assets_requested {
            self.handle_validate_assets_request();
        }
    }

    pub(super) fn handle_validate_assets_request(&mut self) {
        self.core.ui.project.validate_assets_requested = false;

        if self.background_tasks.is_running() {
            tracing::warn!("Cannot validate assets: another background task is running");
            return;
        }

        let Some(project_path) = self.core.config.current_project_path().cloned() else {
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
