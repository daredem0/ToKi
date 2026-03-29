use super::*;
use crate::project::ProjectAssets;
use crate::ui::editor_ui::{sync_dialog_registry, DialogEditorState};
use chrono::Utc;

impl InspectorSystem {
    pub(crate) fn render_dialog_editor_inspector(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: Option<&mut Project>,
        project_assets: Option<&mut ProjectAssets>,
    ) {
        let Some(project_assets) = project_assets else {
            ui.heading("Dialog Editor");
            ui.separator();
            ui.label("Open a project to edit dialog assets.");
            return;
        };

        sync_dialog_registry(ui_state, project_assets);

        ui.heading("Dialog Editor");
        ui.separator();
        Self::render_dialog_asset_sidebar(ui_state, ui, project_assets);
        ui.separator();
        Self::render_dialog_appearance_inspector(ui_state, ui, project);
    }

    fn render_dialog_asset_sidebar(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project_assets: &mut ProjectAssets,
    ) {
        ui.heading("Dialogs");
        if ui.button("New Dialog").clicked() {
            let existing = project_assets.get_dialog_names();
            ui_state.dialog = DialogEditorState::new_dialog(&existing);
        }

        if let Some(status) = &ui_state.dialog.status_message {
            ui.label(status);
        }

        let dialog_names = project_assets.get_dialog_names();
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                for dialog_id in dialog_names {
                    let is_selected =
                        ui_state.dialog.selected_dialog_id.as_deref() == Some(&dialog_id);
                    if ui.selectable_label(is_selected, &dialog_id).clicked() {
                        match project_assets.load_dialog(&dialog_id) {
                            Ok(Some(dialog)) => ui_state.dialog.load_dialog(dialog),
                            Ok(None) => {
                                ui_state.dialog.status_message =
                                    Some(format!("Dialog '{dialog_id}' no longer exists"));
                            }
                            Err(error) => {
                                ui_state.dialog.status_message =
                                    Some(format!("Failed to load dialog '{dialog_id}': {error}"));
                            }
                        }
                    }
                }
            });
    }

    fn render_dialog_appearance_inspector(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: Option<&mut Project>,
    ) {
        let Some(project) = project else {
            ui.heading("Dialog Style");
            ui.separator();
            ui.label("Open a project to edit dialog appearance.");
            return;
        };

        ui.heading("Dialog Style");
        let changed = Self::render_menu_appearance_editor(
            ui_state,
            ui,
            &mut project.metadata.runtime.dialog_appearance,
            true,
        );

        if changed {
            project.metadata.project.modified = Utc::now();
            project.is_dirty = true;
        }
    }
}
