use super::EditorUI;
impl EditorUI {
    pub fn render_hierarchy_and_maps_combined_panel(
        &mut self,
        ctx: &egui::Context,
        game_state: Option<&toki_core::GameState>,
        config: Option<&crate::config::EditorConfig>,
    ) {
        egui::SidePanel::left("hierarchy_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Scene Hierarchy");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("+ Add Scene").clicked() {
                            let new_scene_name = format!("Scene {}", self.scenes.len() + 1);
                            self.add_scene(new_scene_name.clone());
                            tracing::info!("Created new scene: {}", new_scene_name);
                        }
                    });
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("hierarchy_scroll")
                    .show(ui, |ui| {
                        self.render_scene_hierarchy_section(ui, game_state);

                        ui.add_space(10.0);
                        ui.heading("Assets");
                        ui.separator();

                        if self.show_maps {
                            self.render_standalone_maps_section(ui, config);
                        }

                        self.render_entity_palette_section(ui, config);
                    });
            });
    }
}
