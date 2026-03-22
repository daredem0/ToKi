//! Animation editor dialogs.

use crate::ui::EditorUI;

pub fn render_new_clip_dialog(ui_state: &mut EditorUI, ctx: &egui::Context) {
    let available_states = ui_state.animation.authoring.available_states();

    egui::Window::new("New Animation Clip")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("Select animation state:");

            // Quick add buttons for common states
            if !available_states.is_empty() {
                ui.label("Common states:");
                let mut created_state: Option<&str> = None;

                egui::ScrollArea::vertical()
                    .id_salt("anim_new_clip_states")
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for state in &available_states {
                            if ui.button(*state).clicked() {
                                created_state = Some(state);
                            }
                        }
                    });

                if let Some(state) = created_state {
                    ui_state.animation.authoring.create_clip(state);
                    ui_state.animation.show_new_clip_dialog = false;
                }

                ui.separator();
            }

            // Custom state input
            ui.label("Or enter custom state name:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut ui_state.animation.new_clip_state_input);
                let can_create = !ui_state.animation.new_clip_state_input.trim().is_empty()
                    && !ui_state
                        .animation
                        .authoring
                        .has_clip_for_state(ui_state.animation.new_clip_state_input.trim());

                if ui
                    .add_enabled(can_create, egui::Button::new("Create"))
                    .clicked()
                {
                    let state = ui_state.animation.new_clip_state_input.trim().to_string();
                    ui_state.animation.authoring.create_clip(&state);
                    ui_state.animation.new_clip_state_input.clear();
                    ui_state.animation.show_new_clip_dialog = false;
                }
            });

            ui.separator();
            if ui.button("Cancel").clicked() {
                ui_state.animation.new_clip_state_input.clear();
                ui_state.animation.show_new_clip_dialog = false;
            }
        });
}
