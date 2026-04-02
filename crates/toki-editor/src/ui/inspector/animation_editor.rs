// Animation Editor tab inspector
// Shows info and controls when the Animation Editor center panel tab is active

use super::InspectorSystem;

impl InspectorSystem {
    pub(crate) fn render_animation_editor_toolbox(
        ui_state: &mut crate::ui::EditorUI,
        ui: &mut egui::Ui,
    ) {
        ui.heading("Animation Toolbox");
        ui.separator();

        if !crate::ui::editor_context::animation_state(ui_state).has_entity() {
            ui.label("No animation tools available.");
            ui.label("Select an entity to edit its animation preview.");
            return;
        }

        ui.label("Preview Settings:");
        ui.horizontal(|ui| {
            ui.label("Zoom:");
            ui.add(
                egui::DragValue::new(
                    &mut crate::ui::editor_context::animation_state_mut(ui_state).preview_zoom,
                )
                .speed(0.1)
                .range(0.5..=8.0)
                .suffix("x"),
            );
        });

        ui.checkbox(
            &mut crate::ui::editor_context::animation_state_mut(ui_state).show_grid,
            "Show Grid Overlay",
        );
    }

    /// Render inspector panel when the Animation Editor tab is active.
    /// Shows clip settings and asset state when the Animation Editor tab is active.
    pub(crate) fn render_animation_editor_inspector(
        ui_state: &mut crate::ui::EditorUI,
        ui: &mut egui::Ui,
    ) {
        ui.heading("Animation Editor");
        ui.separator();

        if !crate::ui::editor_context::animation_state_mut(ui_state).has_entity() {
            ui.label("No entity selected.");
            ui.label("Select an entity in the hierarchy to edit its animations.");
            return;
        }

        // Entity info
        if let Some(name) = &crate::ui::editor_context::animation_state_mut(ui_state).active_entity
        {
            ui.label(format!("Entity: {}", name));
        }

        ui.separator();

        // Atlas info
        ui.label("Atlas:");
        let atlas_name = &crate::ui::editor_context::animation_state_mut(ui_state)
            .authoring
            .atlas_name;
        if atlas_name.is_empty() {
            ui.label("(none selected)");
        } else {
            ui.label(atlas_name);
        }

        ui.separator();

        // Playback info
        ui.label("Playback:");
        ui.label(format!(
            "Speed: {:.1}x",
            crate::ui::editor_context::animation_state_mut(ui_state)
                .preview
                .speed()
        ));

        if let Some(clip) = crate::ui::editor_context::animation_state_mut(ui_state).selected_clip()
        {
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
