//! Shared widget helpers for entity editor.

use crate::ui::editor_ui::EntityEditState;
use crate::ui::EditorUI;

use super::io::{revert_entity, save_entity};

pub fn render_save_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.separator();

    let is_dirty = ui_state.entity_editor.is_dirty();

    ui.horizontal(|ui| {
        let save_enabled = is_dirty;
        if ui
            .add_enabled(save_enabled, egui::Button::new("Save"))
            .clicked()
        {
            save_entity(ui_state);
        }

        if ui
            .add_enabled(is_dirty, egui::Button::new("Revert"))
            .clicked()
        {
            revert_entity(ui_state);
        }
    });

    // Show validation errors summary
    if let Some(edit) = &ui_state.entity_editor.edit_state {
        if !edit.validation_errors.is_empty() {
            ui.colored_label(
                egui::Color32::RED,
                format!("{} validation error(s)", edit.validation_errors.len()),
            );
        }
    }
}

pub fn show_field_error(ui: &mut egui::Ui, edit: &EntityEditState, field: &str) {
    if let Some(error) = edit.get_error(field) {
        ui.colored_label(egui::Color32::RED, error);
    }
}

/// Render a dropdown for selecting SFX sounds. Returns true if the value changed.
pub fn render_sfx_dropdown(
    ui: &mut egui::Ui,
    id: &str,
    current: &mut String,
    available: &[String],
    dirty: &mut bool,
) -> bool {
    let display_text = if current.is_empty() {
        "(None)"
    } else {
        current.as_str()
    };

    let mut changed = false;

    egui::ComboBox::from_id_salt(id)
        .selected_text(display_text)
        .show_ui(ui, |ui| {
            // None option
            if ui.selectable_label(current.is_empty(), "(None)").clicked() {
                current.clear();
                *dirty = true;
                changed = true;
            }

            // Available SFX
            for sfx in available {
                let is_selected = current == sfx;
                if ui.selectable_label(is_selected, sfx).clicked() {
                    *current = sfx.clone();
                    *dirty = true;
                    changed = true;
                }
            }
        });

    changed
}

/// Render a dropdown for selecting sprite atlases.
pub fn render_atlas_dropdown(
    ui: &mut egui::Ui,
    id: &str,
    current: &mut String,
    available: &[String],
    dirty: &mut bool,
) {
    let display_text = if current.is_empty() {
        "(None)"
    } else {
        current.as_str()
    };

    egui::ComboBox::from_id_salt(id)
        .selected_text(display_text)
        .show_ui(ui, |ui| {
            // None option
            if ui.selectable_label(current.is_empty(), "(None)").clicked() {
                current.clear();
                *dirty = true;
            }

            // Available atlases
            for atlas in available {
                let is_selected = current == atlas;
                if ui.selectable_label(is_selected, atlas).clicked() {
                    *current = atlas.clone();
                    *dirty = true;
                }
            }
        });
}
