//! Animation editor toolbar.

use super::io;
use crate::ui::EditorUI;

pub fn render_toolbar(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.horizontal(|ui| {
        ui.heading("Animation Editor");

        if ui_state.animation.has_entity() {
            ui.separator();

            // Save button
            let is_dirty = ui_state.animation.authoring.dirty;
            if ui
                .add_enabled(is_dirty, egui::Button::new("Save"))
                .clicked()
            {
                io::save_current_entity(ui_state);
            }

            // Entity name label
            if let Some(name) = &ui_state.animation.active_entity {
                ui.separator();
                ui.label(format!("Entity: {}", name));
                if is_dirty {
                    ui.label("*");
                }
            }
        }
    });
}
