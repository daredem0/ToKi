use super::EditorUI;
use crate::ui::panel_layout::SIDE_PANEL_DEFAULT_WIDTH;
use crate::ui::{
    editor_ui::{sync_dialog_registry, sync_ui_layout_registry, CenterPanelTab},
};
impl EditorUI {
    pub fn render_hierarchy_and_maps_combined_panel(
        &mut self,
        ctx: &egui::Context,
        game_state: Option<&toki_core::GameState>,
        project_assets: Option<&mut crate::project::ProjectAssets>,
        config: Option<&crate::config::EditorConfig>,
    ) {
        egui::SidePanel::left("hierarchy_panel")
            .resizable(true)
            .default_width(SIDE_PANEL_DEFAULT_WIDTH)
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

                        if self.visibility.show_maps {
                            self.render_standalone_maps_section(ui, config);
                        }

                        if let Some(project_assets) = project_assets {
                            sync_dialog_registry(self, project_assets);
                            sync_ui_layout_registry(self, project_assets);
                            self.render_dialog_assets_section(ui, project_assets);
                            self.render_ui_assets_section(ui, project_assets);
                        }

                        self.render_entity_palette_section(ui, config);
                    });
            });
    }

    fn render_dialog_assets_section(
        &mut self,
        ui: &mut egui::Ui,
        project_assets: &mut crate::project::ProjectAssets,
    ) {
        ui.add_space(8.0);
        egui::CollapsingHeader::new("Dialogs")
            .id_salt("asset_palette_dialogs_section")
            .default_open(false)
            .show(ui, |ui| {
                ui.separator();
                if ui.button("+ New Dialog").clicked() {
                    let existing = project_assets.get_dialog_names();
                    self.dialog_editor_context_mut().dialog =
                        crate::ui::editor_ui::DialogEditorState::new_dialog(&existing);
                    self.workspace.center_panel_tab = CenterPanelTab::DialogEditor;
                }

                if let Some(status) = &self.dialog_editor_context().dialog.status_message {
                    ui.small(status);
                }

                for dialog_id in project_assets.get_dialog_names() {
                    let selected = self
                        .dialog_editor_context()
                        .dialog
                        .selected_dialog_id
                        .as_deref()
                        == Some(dialog_id.as_str());
                    if ui.selectable_label(selected, &dialog_id).clicked() {
                        match project_assets.load_dialog(&dialog_id) {
                            Ok(Some(dialog)) => {
                                self.dialog_editor_context_mut().dialog.load_dialog(dialog);
                                self.workspace.center_panel_tab = CenterPanelTab::DialogEditor;
                            }
                            Ok(None) => {
                                self.dialog_editor_context_mut().dialog.status_message =
                                    Some(format!("Dialog '{dialog_id}' no longer exists"));
                            }
                            Err(error) => {
                                self.dialog_editor_context_mut().dialog.status_message =
                                    Some(format!("Failed to load dialog '{dialog_id}': {error}"));
                            }
                        }
                    }
                }
            });
    }

    fn render_ui_assets_section(
        &mut self,
        ui: &mut egui::Ui,
        project_assets: &mut crate::project::ProjectAssets,
    ) {
        ui.add_space(8.0);
        egui::CollapsingHeader::new("UI")
            .id_salt("asset_palette_ui_section")
            .default_open(false)
            .show(ui, |ui| {
                ui.separator();
                if ui.button("+ New UI Layout").clicked() {
                    let existing = project_assets.get_ui_layout_names();
                    self.ui_editor_context_mut().ui =
                        crate::ui::editor_ui::UiEditorState::new_layout(&existing);
                    self.workspace.center_panel_tab = CenterPanelTab::UiEditor;
                }

                if let Some(status) = &self.ui_editor_context().ui.status_message {
                    ui.small(status);
                }

                for layout_id in project_assets.get_ui_layout_names() {
                    let selected = self.ui_editor_context().ui.selected_layout_id.as_deref()
                        == Some(layout_id.as_str());
                    if ui.selectable_label(selected, &layout_id).clicked() {
                        match project_assets.load_ui_layout(&layout_id) {
                            Ok(Some(layout)) => {
                                self.ui_editor_context_mut().ui.load_layout(layout);
                                self.workspace.center_panel_tab = CenterPanelTab::UiEditor;
                            }
                            Ok(None) => {
                                self.ui_editor_context_mut().ui.status_message =
                                    Some(format!("UI layout '{layout_id}' no longer exists"));
                            }
                            Err(error) => {
                                self.ui_editor_context_mut().ui.status_message = Some(format!(
                                    "Failed to load UI layout '{layout_id}': {error}"
                                ));
                            }
                        }
                    }
                }
            });
    }
}
