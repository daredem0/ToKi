//! Core component section rendering - Rendering and Attributes.

use crate::ui::EditorUI;

use super::widgets::{render_atlas_dropdown, show_field_error};

pub fn render_rendering_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let available_atlases = ui_state.entity_editor.available_atlases.clone();
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    egui::CollapsingHeader::new("Rendering")
        .default_open(true)
        .show(ui, |ui| {
            // Sprite Atlas dropdown
            ui.horizontal(|ui| {
                ui.label("Sprite Atlas:");
                render_atlas_dropdown(
                    ui,
                    "sprite_atlas",
                    &mut edit.definition.animations.atlas_name,
                    &available_atlases,
                    &mut edit.dirty,
                );
            });

            // Size
            ui.horizontal(|ui| {
                ui.label("Size:");
                let mut w = edit.definition.rendering.size[0] as i32;
                let mut h = edit.definition.rendering.size[1] as i32;
                if ui
                    .add(egui::DragValue::new(&mut w).range(1..=1024))
                    .changed()
                {
                    edit.definition.rendering.size[0] = w.max(1) as u32;
                    edit.mark_dirty();
                }
                ui.label("x");
                if ui
                    .add(egui::DragValue::new(&mut h).range(1..=1024))
                    .changed()
                {
                    edit.definition.rendering.size[1] = h.max(1) as u32;
                    edit.mark_dirty();
                }
            });
            show_field_error(ui, edit, "size");

            // Render layer
            ui.horizontal(|ui| {
                ui.label("Render Layer:");
                if ui
                    .add(egui::DragValue::new(
                        &mut edit.definition.rendering.render_layer,
                    ))
                    .changed()
                {
                    edit.mark_dirty();
                }
            });

            // Visible
            if ui
                .checkbox(&mut edit.definition.rendering.visible, "Visible")
                .changed()
            {
                edit.mark_dirty();
            }
        });
}

pub fn render_attributes_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    egui::CollapsingHeader::new("Attributes")
        .default_open(true)
        .show(ui, |ui| {
            // Speed
            ui.horizontal(|ui| {
                ui.label("Speed:");
                if ui
                    .add(egui::DragValue::new(&mut edit.definition.attributes.speed).speed(0.1))
                    .changed()
                {
                    edit.mark_dirty();
                }
            });

            // Boolean attributes
            if ui
                .checkbox(&mut edit.definition.attributes.solid, "Solid")
                .changed()
            {
                edit.mark_dirty();
            }
            if ui
                .checkbox(&mut edit.definition.attributes.active, "Active")
                .changed()
            {
                edit.mark_dirty();
            }
            if ui
                .checkbox(&mut edit.definition.attributes.can_move, "Can Move")
                .changed()
            {
                edit.mark_dirty();
            }
            if ui
                .checkbox(&mut edit.definition.attributes.interactable, "Interactable")
                .changed()
            {
                edit.mark_dirty();
            }

            // Interaction reach (only if interactable)
            if edit.definition.attributes.interactable {
                ui.horizontal(|ui| {
                    ui.label("Interaction Reach:");
                    let mut reach = edit.definition.attributes.interaction_reach as i32;
                    if ui
                        .add(egui::DragValue::new(&mut reach).range(0..=256))
                        .changed()
                    {
                        edit.definition.attributes.interaction_reach = reach.max(0) as u32;
                        edit.mark_dirty();
                    }
                });
            }
        });
}
