//! Entity details panel - property editing and component toggles.

use crate::ui::EditorUI;

use super::components::render_component_toggles;
use super::widgets::{render_save_section, show_field_error};

pub fn render_entity_details(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    if ui_state.entity_editor.edit_state.is_none() {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.label("No entity selected");
                ui.add_space(8.0);
                ui.label("Select an entity from the browser or create a new one.");
            });
        });
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("entity_details_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            render_core_properties(ui, ui_state);
            ui.add_space(8.0);
            render_component_toggles(ui, ui_state);
            ui.add_space(8.0);
            render_save_section(ui, ui_state);
        });
}

fn render_core_properties(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    ui.heading("Core Properties");
    ui.separator();

    // Name (identifier)
    ui.horizontal(|ui| {
        ui.label("Name:");
        if ui.text_edit_singleline(&mut edit.definition.name).changed() {
            edit.mark_dirty();
        }
    });
    show_field_error(ui, edit, "name");

    // Display name
    ui.horizontal(|ui| {
        ui.label("Display Name:");
        if ui
            .text_edit_singleline(&mut edit.definition.display_name)
            .changed()
        {
            edit.mark_dirty();
        }
    });
    show_field_error(ui, edit, "display_name");

    // Description
    ui.label("Description:");
    if ui
        .text_edit_multiline(&mut edit.definition.description)
        .changed()
    {
        edit.mark_dirty();
    }

    // Category
    ui.horizontal(|ui| {
        ui.label("Category:");
        if ui
            .text_edit_singleline(&mut edit.definition.category)
            .changed()
        {
            edit.mark_dirty();
        }
    });

    // Tags
    ui.horizontal(|ui| {
        ui.label("Tags:");
        if ui.text_edit_singleline(&mut edit.tags_input).changed() {
            edit.sync_tags();
            edit.mark_dirty();
        }
    });
    ui.label("(comma-separated)");
}
