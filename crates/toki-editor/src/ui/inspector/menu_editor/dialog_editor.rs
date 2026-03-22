//! Dialog editing UI.

use super::*;

impl InspectorSystem {
    pub(super) fn render_menu_dialog_editor(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: &mut Project,
        dialog_id: &str,
    ) {
        let before_settings = project.metadata.runtime.menu.clone();
        let Some(dialog_index) = Self::selected_menu_dialog_index(project, dialog_id) else {
            ui.label("Selected dialog no longer exists.");
            return;
        };

        let available_screen_ids = project
            .metadata
            .runtime
            .menu
            .screens
            .iter()
            .map(|screen| screen.id.clone())
            .collect::<Vec<_>>();
        let available_dialog_ids = project
            .metadata
            .runtime
            .menu
            .dialogs
            .iter()
            .map(|dialog| dialog.id.clone())
            .collect::<Vec<_>>();
        let available_surface_ids = available_screen_ids
            .iter()
            .chain(available_dialog_ids.iter())
            .cloned()
            .collect::<Vec<_>>();
        let mut changed = false;
        let mut renamed_to = None;
        let mut duplicate_dialog = false;
        let mut delete_dialog = false;

        egui::CollapsingHeader::new("Dialog")
            .default_open(false)
            .show(ui, |ui| {
                let dialog = &mut project.metadata.runtime.menu.dialogs[dialog_index];
                ui.label("Title");
                if ui.text_edit_singleline(&mut dialog.title).changed() {
                    changed = true;
                }

                ui.label("Dialog ID");
                let mut id = dialog.id.clone();
                if ui.text_edit_singleline(&mut id).changed() {
                    let normalized = Self::normalize_menu_screen_id(&id);
                    if !normalized.is_empty() && normalized != dialog.id {
                        dialog.id = normalized.clone();
                        renamed_to = Some(normalized);
                        changed = true;
                    }
                }

                ui.label("Body");
                if ui
                    .add(
                        egui::TextEdit::multiline(&mut dialog.body)
                            .desired_rows(3)
                            .lock_focus(true),
                    )
                    .changed()
                {
                    changed = true;
                }

                ui.separator();
                ui.label("Confirm Button");
                if ui.text_edit_singleline(&mut dialog.confirm_text).changed() {
                    changed = true;
                }
                ui.push_id("dialog_confirm_action", |ui| {
                    changed |= Self::render_menu_action_editor(
                        ui,
                        &available_surface_ids,
                        &mut dialog.confirm_action,
                    );
                });

                ui.separator();
                ui.label("Cancel Button");
                if ui.text_edit_singleline(&mut dialog.cancel_text).changed() {
                    changed = true;
                }
                ui.push_id("dialog_cancel_action", |ui| {
                    changed |= Self::render_menu_action_editor(
                        ui,
                        &available_surface_ids,
                        &mut dialog.cancel_action,
                    );
                });

                ui.separator();
                if ui
                    .checkbox(&mut dialog.hide_main_menu, "Hide Main Menu")
                    .on_hover_text("Hide the main menu behind this dialog")
                    .changed()
                {
                    changed = true;
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Duplicate Dialog").clicked() {
                        duplicate_dialog = true;
                    }
                    if ui.button("Delete Dialog").clicked() {
                        delete_dialog = true;
                    }
                });
            });

        if let Some(normalized) = renamed_to {
            Self::rewrite_ui_action_surface_targets(
                &mut project.metadata.runtime.menu,
                dialog_id,
                &normalized,
            );
            ui_state.select_menu_dialog(normalized);
        }
        if changed {
            Self::commit_menu_settings_change(ui_state, project, before_settings);
        }
        if duplicate_dialog {
            Self::duplicate_menu_dialog(ui_state, project, dialog_index);
        }
        if delete_dialog {
            Self::delete_menu_dialog(ui_state, project, dialog_index);
        }
    }
}
