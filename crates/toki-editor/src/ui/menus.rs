use crate::config::EditorConfig;

/// Handles all menu bar rendering for the editor
pub struct MenuSystem;

impl MenuSystem {
    /// Renders the top menu bar with File, Edit, and View menus
    pub fn render_top_menu(
        ui_state: &mut super::EditorUI,
        ctx: &egui::Context,
        config: Option<&EditorConfig>,
    ) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project...").clicked() {
                        tracing::info!("New Project clicked");
                        ui_state.new_project_requested = true;
                    }

                    // Auto-open the project from config
                    if let Some(config) = config {
                        if config.has_project_path() {
                            if ui.button("Open Project").clicked() {
                                tracing::info!(
                                    "Open Project clicked - opening project from config"
                                );
                                ui_state.open_project_requested = true;
                            }
                            if ui.button("Browse for Project...").clicked() {
                                tracing::info!("Browse for Project clicked");
                                ui_state.browse_for_project_requested = true;
                            }
                        } else if ui.button("Open Project...").clicked() {
                            tracing::info!("Open Project... clicked - no project path in config");
                            ui_state.browse_for_project_requested = true;
                        }
                    } else if ui.button("Open Project...").clicked() {
                        tracing::info!("Open Project... clicked - no config available");
                        ui_state.browse_for_project_requested = true;
                    }

                    ui.separator();
                    if ui.button("Save Project").clicked() {
                        tracing::info!("Save Project clicked");
                        ui_state.save_project_requested = true;
                    }
                    ui.separator();
                    if ui.button("Create Test Entities").clicked() {
                        tracing::info!("Create Test Entities clicked");
                        ui_state.create_test_entities = true;
                    }
                    ui.separator();
                    if ui.button("Init Config").clicked() {
                        tracing::info!("Init Config clicked");
                        ui_state.init_config_requested = true;
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        tracing::info!("Exit clicked");
                        ui_state.should_exit = true;
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui.button("Validate Project Assets").clicked() {
                        tracing::info!("Validate Project Assets clicked");
                        ui_state.validate_assets_requested = true;
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut ui_state.show_hierarchy, "Hierarchy");
                    ui.checkbox(&mut ui_state.show_inspector, "Inspector");
                    ui.checkbox(&mut ui_state.show_maps, "Maps");
                    ui.checkbox(&mut ui_state.show_console, "Console");
                });

                ui.separator();
                let can_play_scene = ui_state.active_scene.is_some()
                    && config.is_some_and(|editor_config| editor_config.has_project_path());
                if ui
                    .add_enabled(can_play_scene, egui::Button::new("▶ Play Scene"))
                    .clicked()
                {
                    ui_state.play_scene_requested = true;
                }

                ui.with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        ui.label(ui_state.window_title.as_ref().unwrap());
                    },
                );
            });
        });
    }
}
