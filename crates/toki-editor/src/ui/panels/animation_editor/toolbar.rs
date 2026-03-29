//! Animation editor toolbar.

use super::io;
use crate::ui::EditorUI;

pub fn render_toolbar(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.horizontal(|ui| {
        ui.heading("Animation Editor");

        if crate::ui::editor_context::animation_state_mut(ui_state).has_entity() {
            ui.separator();

            // Save button
            let is_dirty = crate::ui::editor_context::animation_state_mut(ui_state)
                .authoring
                .dirty;
            if ui
                .add_enabled(is_dirty, egui::Button::new("Save"))
                .clicked()
            {
                io::save_current_entity(ui_state);
            }

            // Entity name label
            if let Some(name) =
                &crate::ui::editor_context::animation_state_mut(ui_state).active_entity
            {
                ui.separator();
                ui.label(format!("Entity: {}", name));
                if is_dirty {
                    ui.label("*");
                }
            }
        }
    });
}
