//! Dialog windows for entity editor - new entity and delete confirmation.

use crate::ui::editor_ui::EntityCategory;
use crate::ui::EditorUI;
use std::path::Path;

use super::io::create_new_entity;

pub fn render_new_entity_dialog(
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    project_path: Option<&Path>,
) {
    let mut close_dialog = false;
    let mut create_entity = false;

    egui::Window::new("New Entity")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name (identifier):");
                ui.text_edit_singleline(&mut ui_state.entity_editor.new_entity_dialog.name_input);
            });

            ui.horizontal(|ui| {
                ui.label("Display Name:");
                ui.text_edit_singleline(
                    &mut ui_state.entity_editor.new_entity_dialog.display_name_input,
                );
            });

            ui.horizontal(|ui| {
                ui.label("Description:");
            });
            ui.text_edit_multiline(&mut ui_state.entity_editor.new_entity_dialog.description_input);

            render_category_selector(ui, ui_state);

            // Show validation error
            if let Some(error) = &ui_state.entity_editor.new_entity_dialog.error_message {
                ui.colored_label(egui::Color32::RED, error);
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Create").clicked() {
                    create_entity = true;
                }
                if ui.button("Cancel").clicked() {
                    close_dialog = true;
                }
            });
        });

    if close_dialog {
        ui_state.entity_editor.new_entity_dialog.close();
    }

    if create_entity {
        let existing = ui_state.entity_editor.existing_names();
        if ui_state.entity_editor.new_entity_dialog.validate(&existing) {
            if let Some(path) = project_path {
                create_new_entity(ui_state, path);
                ui_state.entity_editor.new_entity_dialog.close();
            }
        }
    }
}

fn render_category_selector(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    // Build category list: existing categories + predefined suggestions
    let mut all_categories: std::collections::HashSet<String> =
        ui_state.entity_editor.all_categories();
    for category in EntityCategory::ALL {
        all_categories.insert(category.as_str().to_string());
    }
    let mut category_list: Vec<String> = all_categories.into_iter().collect();
    category_list.sort();

    ui.horizontal(|ui| {
        ui.label("Category:");
        ui.text_edit_singleline(&mut ui_state.entity_editor.new_entity_dialog.category);
    });

    ui.horizontal(|ui| {
        ui.label("Suggestions:");
        egui::ComboBox::from_id_salt("new_entity_category")
            .selected_text("Select...")
            .show_ui(ui, |ui| {
                for category in &category_list {
                    if ui.selectable_label(false, category).clicked() {
                        ui_state.entity_editor.new_entity_dialog.category = category.clone();
                    }
                }
            });
    });
}

pub fn render_delete_confirmation_dialog(
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    project_path: Option<&Path>,
) {
    let entity_name = ui_state
        .entity_editor
        .delete_confirmation
        .entity_name
        .clone();
    let mut close_dialog = false;
    let mut confirm_delete = false;

    egui::Window::new("Delete Entity")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label(format!(
                "Are you sure you want to delete '{}'?",
                entity_name
            ));
            ui.label("This action cannot be undone.");

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    confirm_delete = true;
                }
                if ui.button("Cancel").clicked() {
                    close_dialog = true;
                }
            });
        });

    if close_dialog {
        ui_state.entity_editor.delete_confirmation.close();
    }

    if confirm_delete {
        if let Some(path) = project_path {
            super::io::delete_entity(ui_state, path, &entity_name);
        }
        ui_state.entity_editor.delete_confirmation.close();
    }
}
