use crate::config::EditorConfig;
use crate::ui::editor_ui::EditorConfirmation;
use crate::ui::inspector::build_delete_scene_command;

/// Handles all menu bar rendering for the editor
pub struct MenuSystem;

impl MenuSystem {
    /// Renders the top menu bar with File, Edit, and View menus
    pub fn render_top_menu(
        ui_state: &mut super::EditorUI,
        ctx: &egui::Context,
        mut project: Option<&mut crate::project::Project>,
        config: Option<&EditorConfig>,
        busy_logo_texture: Option<&egui::TextureHandle>,
    ) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            let panel_rect = ui.max_rect();
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Empty Project...").clicked() {
                        tracing::info!("New Empty Project clicked");
                        ui_state.project.new_project_requested = true;
                    }
                    if ui.button("New Top-Down Starter...").clicked() {
                        tracing::info!("New Top-Down Starter clicked");
                        ui_state.project.new_top_down_project_requested = true;
                    }

                    // Auto-open the project from config
                    if let Some(config) = config {
                        if config.has_project_path() {
                            if ui.button("Open Project").clicked() {
                                tracing::info!(
                                    "Open Project clicked - opening project from config"
                                );
                                ui_state.project.open_project_requested = true;
                            }
                            if ui.button("Browse for Project...").clicked() {
                                tracing::info!("Browse for Project clicked");
                                ui_state.project.browse_for_project_requested = true;
                            }
                        } else if ui.button("Open Project...").clicked() {
                            tracing::info!("Open Project... clicked - no project path in config");
                            ui_state.project.browse_for_project_requested = true;
                        }
                    } else if ui.button("Open Project...").clicked() {
                        tracing::info!("Open Project... clicked - no config available");
                        ui_state.project.browse_for_project_requested = true;
                    }

                    ui.separator();
                    if ui.button("Save Project").clicked() {
                        tracing::info!("Save Project clicked");
                        ui_state.project.save_project_requested = true;
                    }
                    if ui
                        .add_enabled(
                            ui_state.has_unsaved_map_editor_changes(),
                            egui::Button::new("Save Map"),
                        )
                        .clicked()
                    {
                        tracing::info!("Save Map clicked");
                        ui_state.map.save_requested = true;
                    }
                    if ui
                        .add_enabled(
                            !ui_state.project.background_task_running,
                            egui::Button::new("Export Game..."),
                        )
                        .clicked()
                    {
                        tracing::info!("Export Game clicked");
                        ui_state.project.export_project_requested = true;
                    }
                    ui.separator();
                    if ui.button("Create Test Entities").clicked() {
                        tracing::info!("Create Test Entities clicked");
                        ui_state.visibility.create_test_entities = true;
                    }
                    ui.separator();
                    if ui.button("Init Config").clicked() {
                        tracing::info!("Init Config clicked");
                        ui_state.project.init_config_requested = true;
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        tracing::info!("Exit clicked");
                        ui_state.visibility.should_exit = true;
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui
                        .add_enabled(ui_state.can_undo(), egui::Button::new("Undo (Ctrl+Z)"))
                        .clicked()
                        && project
                            .as_mut()
                            .map(|project| ui_state.undo_with_project(project))
                            .unwrap_or_else(|| ui_state.undo())
                    {
                        tracing::info!("Undo command applied");
                    }
                    if ui
                        .add_enabled(
                            ui_state.can_redo(),
                            egui::Button::new("Redo (Ctrl+Y / Ctrl+Shift+Z)"),
                        )
                        .clicked()
                        && project
                            .as_mut()
                            .map(|project| ui_state.redo_with_project(project))
                            .unwrap_or_else(|| ui_state.redo())
                    {
                        tracing::info!("Redo command applied");
                    }
                    ui.separator();
                    if ui
                        .add_enabled(
                            !ui_state.project.background_task_running,
                            egui::Button::new("Validate Project Assets"),
                        )
                        .clicked()
                    {
                        tracing::info!("Validate Project Assets clicked");
                        ui_state.project.validate_assets_requested = true;
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut ui_state.visibility.show_hierarchy, "Hierarchy");
                    ui.checkbox(&mut ui_state.visibility.show_inspector, "Inspector");
                    ui.checkbox(&mut ui_state.visibility.show_maps, "Maps");
                    ui.checkbox(
                        &mut ui_state.visibility.show_runtime_entities,
                        "Show runtime entities",
                    );
                    ui.checkbox(&mut ui_state.visibility.show_console, "Console");
                });

                ui.separator();
                let can_play_scene = ui_state.active_scene.is_some()
                    && config.is_some_and(|editor_config| editor_config.has_project_path());
                if ui
                    .add_enabled(can_play_scene, egui::Button::new("▶ Play Scene"))
                    .clicked()
                {
                    ui_state.project.play_scene_requested = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui_state.project.background_task_running
                        && ui.button("Cancel Task").clicked()
                    {
                        ui_state.project.cancel_background_task_requested = true;
                    }
                    if let Some(status) = &ui_state.project.background_task_status {
                        ui.label(status);
                    }
                    if ui_state.project.background_task_running {
                        ui.separator();
                        Self::render_busy_logo(ui, ctx, busy_logo_texture);
                    }
                });
            });

            if let Some(window_title) = &ui_state.project.window_title {
                ui.painter().text(
                    panel_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    window_title,
                    egui::TextStyle::Button.resolve(ui.style()),
                    ui.visuals().text_color(),
                );
            }
        });

        if ui_state.project.show_new_project_dialog {
            Self::render_new_project_dialog(ui_state, ctx);
        }
        Self::render_pending_confirmations(ui_state, ctx, project);
    }

    fn render_pending_confirmations(
        ui_state: &mut super::EditorUI,
        ctx: &egui::Context,
        project: Option<&mut crate::project::Project>,
    ) {
        let Some(pending_confirmation) = ui_state.project.pending_confirmation.clone() else {
            return;
        };

        match pending_confirmation {
            EditorConfirmation::DeleteScene { scene_name } => {
                let mut open = true;
                let mut confirm_delete = false;
                let mut cancel = false;
                egui::Window::new("Delete Scene")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .open(&mut open)
                    .show(ctx, |ui| {
                        ui.label("The selected Scene is not empty. Do you really want to delete it? This cannot be undone.");
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Yes, delete anyway").clicked() {
                                confirm_delete = true;
                            }
                            if ui.button("No").clicked() {
                                cancel = true;
                            }
                        });
                    });

                if !open || cancel {
                    ui_state.project.pending_confirmation = None;
                    return;
                }

                if confirm_delete {
                    let Some(project) = project else {
                        ui_state.project.pending_confirmation = None;
                        return;
                    };
                    match build_delete_scene_command(ui_state, project, &scene_name) {
                        Ok(command) => {
                            let _ = ui_state.execute_command_with_project(project, command);
                        }
                        Err(error) => {
                            tracing::error!(
                                "Failed to build delete scene command for '{}': {}",
                                scene_name,
                                error
                            );
                        }
                    }
                    ui_state.project.pending_confirmation = None;
                }
            }
        }
    }

    fn render_new_project_dialog(ui_state: &mut super::EditorUI, ctx: &egui::Context) {
        let mut open = ui_state.project.show_new_project_dialog;
        let mut create_clicked = false;
        let mut cancel_clicked = false;
        egui::Window::new("New Project")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Template: {}",
                    ui_state.project.new_project_template.label()
                ));
                ui.separator();

                ui.label("Project Name");
                ui.text_edit_singleline(&mut ui_state.project.new_project_name);
                ui.separator();

                ui.label("Parent Folder");
                let parent_label = ui_state
                    .project
                    .new_project_parent_directory
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "No folder selected".to_string());
                ui.monospace(parent_label);
                if ui.button("Browse...").clicked() {
                    let mut dialog =
                        rfd::FileDialog::new().set_title("Select folder for new project");
                    if let Some(parent) = ui_state.project.new_project_parent_directory.as_deref() {
                        dialog = dialog.set_directory(parent);
                    }
                    if let Some(folder) = dialog.pick_folder() {
                        ui_state.project.new_project_parent_directory = Some(folder);
                    }
                }

                let can_create = ui_state.project.new_project_parent_directory.is_some()
                    && !ui_state.project.new_project_name.trim().is_empty();
                if !can_create {
                    ui.colored_label(
                        egui::Color32::from_rgb(215, 120, 120),
                        "Select a folder and enter a project name.",
                    );
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(can_create, egui::Button::new("Create"))
                        .clicked()
                    {
                        create_clicked = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel_clicked = true;
                    }
                });
            });

        if create_clicked {
            ui_state.submit_new_project_request();
            open = false;
        }
        if cancel_clicked {
            open = false;
        }
        ui_state.project.show_new_project_dialog = open;
    }

    fn render_busy_logo(
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        busy_logo_texture: Option<&egui::TextureHandle>,
    ) {
        let Some(texture) = busy_logo_texture else {
            ui.add(egui::Spinner::new());
            return;
        };

        let animation = Self::busy_logo_animation(ctx.input(|input| input.time));
        let size = egui::vec2(22.0, 22.0);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let image_rect = rect.translate(egui::vec2(0.0, animation.bob_offset));

        let glow_color = egui::Color32::from_rgba_unmultiplied(110, 210, 255, animation.glow_alpha);
        let glow_rect = image_rect.expand(animation.glow_spread);
        ui.painter()
            .rect_filled(glow_rect, glow_rect.width() * 0.35, glow_color);

        ui.painter().image(
            texture.id(),
            image_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    fn busy_logo_animation(time_seconds: f64) -> BusyLogoAnimation {
        let phase = time_seconds as f32 * 2.4;
        let bob_wave = phase.sin();
        let glow_wave = ((phase * 0.85).sin() + 1.0) * 0.5;
        BusyLogoAnimation {
            bob_offset: bob_wave * 2.0,
            glow_alpha: (28.0 + glow_wave * 38.0).round() as u8,
            glow_spread: 3.0 + glow_wave * 2.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BusyLogoAnimation {
    bob_offset: f32,
    glow_alpha: u8,
    glow_spread: f32,
}

#[cfg(test)]
#[path = "menus_tests.rs"]
mod tests;
