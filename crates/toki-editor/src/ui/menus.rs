use crate::config::EditorConfig;

/// Handles all menu bar rendering for the editor
pub struct MenuSystem;

impl MenuSystem {
    /// Renders the top menu bar with File, Edit, and View menus
    pub fn render_top_menu(
        ui_state: &mut super::EditorUI,
        ctx: &egui::Context,
        config: Option<&EditorConfig>,
        busy_logo_texture: Option<&egui::TextureHandle>,
    ) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            let panel_rect = ui.max_rect();
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Empty Project...").clicked() {
                        tracing::info!("New Empty Project clicked");
                        ui_state.new_project_requested = true;
                    }
                    if ui.button("New Top-Down Starter...").clicked() {
                        tracing::info!("New Top-Down Starter clicked");
                        ui_state.new_top_down_project_requested = true;
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
                    if ui
                        .add_enabled(
                            ui_state.has_unsaved_map_editor_draft(),
                            egui::Button::new("Save Map"),
                        )
                        .clicked()
                    {
                        tracing::info!("Save Map clicked");
                        ui_state.map_editor_save_requested = true;
                    }
                    if ui
                        .add_enabled(
                            !ui_state.background_task_running,
                            egui::Button::new("Export Game..."),
                        )
                        .clicked()
                    {
                        tracing::info!("Export Game clicked");
                        ui_state.export_project_requested = true;
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
                    if ui
                        .add_enabled(ui_state.can_undo(), egui::Button::new("Undo (Ctrl+Z)"))
                        .clicked()
                        && ui_state.undo()
                    {
                        tracing::info!("Undo command applied");
                    }
                    if ui
                        .add_enabled(
                            ui_state.can_redo(),
                            egui::Button::new("Redo (Ctrl+Y / Ctrl+Shift+Z)"),
                        )
                        .clicked()
                        && ui_state.redo()
                    {
                        tracing::info!("Redo command applied");
                    }
                    ui.separator();
                    if ui
                        .add_enabled(
                            !ui_state.background_task_running,
                            egui::Button::new("Validate Project Assets"),
                        )
                        .clicked()
                    {
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

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui_state.background_task_running && ui.button("Cancel Task").clicked() {
                        ui_state.cancel_background_task_requested = true;
                    }
                    if let Some(status) = &ui_state.background_task_status {
                        ui.label(status);
                    }
                    if ui_state.background_task_running {
                        ui.separator();
                        Self::render_busy_logo(ui, ctx, busy_logo_texture);
                    }
                });
            });

            if let Some(window_title) = &ui_state.window_title {
                ui.painter().text(
                    panel_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    window_title,
                    egui::TextStyle::Button.resolve(ui.style()),
                    ui.visuals().text_color(),
                );
            }
        });
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
mod tests {
    use super::MenuSystem;

    #[test]
    fn busy_logo_animation_stays_within_expected_visual_bounds() {
        let sample_points = [0.0, 0.5, 1.0, 2.0, 4.0, 8.0];
        for sample in sample_points {
            let animation = MenuSystem::busy_logo_animation(sample);
            assert!(animation.bob_offset >= -2.1 && animation.bob_offset <= 2.1);
            assert!(animation.glow_alpha >= 28 && animation.glow_alpha <= 66);
            assert!(animation.glow_spread >= 3.0 && animation.glow_spread <= 5.6);
        }
    }

    #[test]
    fn busy_logo_animation_changes_over_time() {
        let early = MenuSystem::busy_logo_animation(0.0);
        let later = MenuSystem::busy_logo_animation(1.0);
        assert_ne!(early, later);
    }
}
