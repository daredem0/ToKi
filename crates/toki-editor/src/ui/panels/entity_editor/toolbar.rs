//! Entity editor toolbar rendering.

use crate::ui::EditorUI;

pub fn render_toolbar(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.horizontal(|ui| {
        ui.heading("Entity Editor");
        ui.separator();

        // New entity button
        if ui.button("+ New Entity").clicked() {
            ui_state.entity_editor.new_entity_dialog.open_for_new();
        }

        // Refresh button
        if ui.button("Refresh").clicked() {
            ui_state.entity_editor.needs_refresh = true;
        }

        // Show selected entity name and dirty indicator
        if let Some(name) = &ui_state.entity_editor.selected_entity {
            ui.separator();
            ui.label(format!("Selected: {}", name));
            if ui_state.entity_editor.is_dirty() {
                ui.label("*");
            }
        }
    });
}
