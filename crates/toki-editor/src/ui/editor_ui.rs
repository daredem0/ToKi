use crate::scene::SceneViewport;
use crate::ui::panels;
use std::path::PathBuf;
use toki_core::entity::EntityId;

/// Manages the editor's UI state and rendering
pub struct EditorUI {
    pub selected_entity_id: Option<EntityId>,
    pub show_hierarchy: bool,
    pub show_inspector: bool,
    pub should_exit: bool,
    pub show_console: bool,
    pub create_test_entities: bool,

    // Project management flags
    pub new_project_requested: bool,
    pub open_project_requested: bool,
    pub browse_for_project_requested: bool,
    pub save_project_requested: bool,
    pub save_as_project_requested: bool,
    pub init_config_requested: bool,
    pub open_recent_project_requested: Option<PathBuf>,
    pub open_last_project_requested: bool,
}

impl EditorUI {
    pub fn new() -> Self {
        Self {
            selected_entity_id: None,
            show_hierarchy: true,
            show_inspector: true,
            should_exit: false,
            show_console: true,
            create_test_entities: false,

            // Project management flags
            new_project_requested: false,
            open_project_requested: false,
            browse_for_project_requested: false,
            save_project_requested: false,
            save_as_project_requested: false,
            init_config_requested: false,
            open_recent_project_requested: None,
            open_last_project_requested: false,
        }
    }

    /// Render the entire UI
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        scene_viewport: Option<&mut SceneViewport>,
        config: Option<&crate::config::EditorConfig>,
        log_capture: Option<&crate::logging::LogCapture>,
    ) {
        self.render_top_menu(ctx, config);

        // Render log panel first to claim full width at bottom
        if self.show_console {
            self.render_log_panel(ctx, log_capture);
        }

        // Render hierarchy and inspector panels
        let game_state = scene_viewport
            .as_ref()
            .map(|v| v.scene_manager().game_state());

        if self.show_hierarchy {
            panels::render_hierarchy(ctx, game_state, &mut self.selected_entity_id);
        }

        if self.show_inspector {
            panels::render_inspector(ctx, game_state, self.selected_entity_id);
        }

        // Render viewport last (mutable access)
        self.render_viewport(ctx, scene_viewport);
    }

    /// Apply config settings to UI state
    pub fn apply_config(&mut self, config: &crate::config::EditorConfig) {
        self.show_hierarchy = config.editor_settings.panels.hierarchy_visible;
        self.show_inspector = config.editor_settings.panels.inspector_visible;
        self.show_console = config.editor_settings.panels.console_visible;
    }

    fn render_top_menu(
        &mut self,
        ctx: &egui::Context,
        config: Option<&crate::config::EditorConfig>,
    ) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project...").clicked() {
                        tracing::info!("New Project clicked");
                        self.new_project_requested = true;
                    }
                    // Smart open project - try recent first
                    if let Some(config) = config {
                        if !config.recent_projects.is_empty() {
                            if ui.button("Open Last Project").clicked() {
                                tracing::info!("Open Last Project clicked");
                                self.open_last_project_requested = true;
                            }
                        }
                    }

                    // Auto-open the project from config
                    if let Some(config) = config {
                        if config.has_project_path() {
                            if ui.button("Open Project").clicked() {
                                tracing::info!(
                                    "Open Project clicked - opening project from config"
                                );
                                self.open_project_requested = true;
                            }
                            if ui.button("Browse for Project...").clicked() {
                                tracing::info!("Browse for Project clicked");
                                self.browse_for_project_requested = true;
                            }
                        } else {
                            if ui.button("Open Project...").clicked() {
                                tracing::info!(
                                    "Open Project... clicked - no project path in config"
                                );
                                self.browse_for_project_requested = true;
                            }
                        }
                    } else {
                        if ui.button("Open Project...").clicked() {
                            tracing::info!("Open Project... clicked - no config available");
                            self.browse_for_project_requested = true;
                        }
                    }

                    // Recent projects submenu
                    if let Some(config) = config {
                        if !config.recent_projects.is_empty() {
                            ui.menu_button("Open Recent", |ui| {
                                for project_path in &config.recent_projects {
                                    let project_name = project_path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("Unknown Project");

                                    if ui.button(project_name).clicked() {
                                        tracing::info!(
                                            "Opening recent project: {:?}",
                                            project_path
                                        );
                                        self.open_recent_project_requested =
                                            Some(project_path.clone());
                                    }
                                }
                            });
                        }
                    }

                    ui.separator();
                    if ui.button("Save Project").clicked() {
                        tracing::info!("Save Project clicked");
                        self.save_project_requested = true;
                    }
                    if ui.button("Save As...").clicked() {
                        tracing::info!("Save As clicked");
                        self.save_as_project_requested = true;
                    }
                    ui.separator();
                    if ui.button("Create Test Entities").clicked() {
                        tracing::info!("Create Test Entities clicked");
                        self.create_test_entities = true;
                    }
                    ui.separator();
                    if ui.button("Init Config").clicked() {
                        tracing::info!("Init Config clicked");
                        self.init_config_requested = true;
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        tracing::info!("Exit clicked");
                        self.should_exit = true;
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_hierarchy, "Hierarchy");
                    ui.checkbox(&mut self.show_inspector, "Inspector");
                    ui.checkbox(&mut self.show_console, "Console");
                });
            });
        });
    }

    fn render_viewport(&mut self, ctx: &egui::Context, scene_viewport: Option<&mut SceneViewport>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Scene Viewport");
            ui.separator();

            // Collect stats before updating viewport to avoid borrowing conflicts
            let (entity_count, selected_entity) = if let Some(ref viewport) = scene_viewport {
                let count = viewport
                    .scene_manager()
                    .game_state()
                    .entity_manager()
                    .active_entities()
                    .len();
                let selected = viewport.selected_entity();
                (count, selected)
            } else {
                (0, None)
            };

            // Update and render the scene viewport
            if let Some(viewport) = scene_viewport {
                // Update the viewport systems
                if let Err(e) = viewport.update() {
                    tracing::error!("Scene viewport update error: {e}");
                }

                // Handle viewport interactions
                let available_size = ui.available_size();
                let (rect, response) =
                    ui.allocate_exact_size(available_size, egui::Sense::click_and_drag());

                // Handle click events for entity selection
                if response.clicked() {
                    if let Some(click_pos) = response.interact_pointer_pos() {
                        let screen_pos = glam::Vec2::new(click_pos.x, click_pos.y);
                        if let Some(entity_id) = viewport.handle_click(screen_pos, rect) {
                            self.selected_entity_id = Some(entity_id);
                        } else {
                            self.selected_entity_id = None;
                        }
                    }
                }

                // Render the scene content
                viewport.render(ui, rect);
            } else {
                // Show placeholder when no viewport
                let available_size = ui.available_size();
                ui.allocate_response(available_size, egui::Sense::click())
                    .on_hover_text("Scene viewport not initialized");
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.label("📊 Stats:");
                    ui.label(format!(
                        "Entities: {} | Selected: {:?}",
                        entity_count, selected_entity
                    ));
                    ui.label("Press F1/F2 to toggle panels");
                });
            });
        });
    }

    fn render_log_panel(
        &mut self,
        ctx: &egui::Context,
        log_capture: Option<&crate::logging::LogCapture>,
    ) {
        egui::TopBottomPanel::bottom("log_panel")
            .resizable(true)
            .default_height(200.0)
            .show(ctx, |ui| {
                ui.heading("📝 Console");
                ui.separator();

                if let Some(capture) = log_capture {
                    let logs = capture.get_logs();
                    let scroll_area = egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true);

                    scroll_area.show(ui, |ui| {
                        for log_entry in &logs {
                            ui.horizontal(|ui| {
                                ui.label(&log_entry.timestamp);
                                ui.label(&log_entry.level);
                                ui.label(&log_entry.message);
                            });
                        }
                    });
                } else {
                    ui.label("Logs are being sent to terminal (check log_to_terminal config)");
                }
            });
    }
}
