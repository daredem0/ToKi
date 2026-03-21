// Animation Editor tab inspector
// Shows info and controls when the Animation Editor center panel tab is active

use super::InspectorSystem;

impl InspectorSystem {
    /// Render inspector panel when the Animation Editor tab is active.
    /// Shows clip settings, atlas info, and preview controls.
    pub(super) fn render_animation_editor_inspector(
        ui_state: &mut crate::ui::EditorUI,
        ui: &mut egui::Ui,
    ) {
        ui.heading("Animation Editor");
        ui.separator();

        if !ui_state.animation.has_entity() {
            ui.label("No entity selected.");
            ui.label("Select an entity in the hierarchy to edit its animations.");
            return;
        }

        // Entity info
        if let Some(name) = &ui_state.animation.active_entity {
            ui.label(format!("Entity: {}", name));
        }

        ui.separator();

        // Atlas info
        ui.label("Atlas:");
        let atlas_name = &ui_state.animation.authoring.atlas_name;
        if atlas_name.is_empty() {
            ui.label("(none selected)");
        } else {
            ui.label(atlas_name);
        }

        ui.separator();

        // Preview settings
        ui.label("Preview Settings:");
        ui.horizontal(|ui| {
            ui.label("Zoom:");
            ui.add(
                egui::DragValue::new(&mut ui_state.animation.preview_zoom)
                    .speed(0.1)
                    .range(0.5..=8.0)
                    .suffix("x"),
            );
        });

        ui.checkbox(&mut ui_state.animation.show_grid, "Show Grid Overlay");

        ui.separator();

        // Playback info
        ui.label("Playback:");
        ui.label(format!(
            "Speed: {:.1}x",
            ui_state.animation.preview.speed
        ));

        if let Some(clip) = ui_state.animation.selected_clip() {
            ui.label(format!("Clip: {}", clip.state));
            ui.label(format!("Frames: {}", clip.frames.len()));
            ui.label(format!("Loop: {}", clip.loop_mode));
            ui.label(format!("Duration: {}ms", clip.default_duration_ms));
        } else {
            ui.label("No clip selected");
        }

        // Keyboard shortcuts help
        ui.separator();
        ui.label("Shortcuts:");
        ui.label("Delete - Remove selected frame");
    }
}
