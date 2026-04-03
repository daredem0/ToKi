//! Core component section rendering - Rendering and Attributes.

use crate::ui::entity_kind_policy::{
    effective_kind_for_category, kind_supports_movement, uses_decoration_collision_policy,
};
use crate::ui::object_sheet_browser::{
    build_decoration_placement_draft, resolve_object_sheet_browser_source, sync_selected_object_name,
    sync_selected_sheet_name,
};
use crate::ui::EditorUI;
use std::path::Path;
use toki_core::entity::EntityFootprint;

use super::widgets::{render_atlas_dropdown, show_field_error};

fn resolved_ground_origin(edit: &crate::ui::editor_ui::EntityEditState) -> [i32; 2] {
    edit.definition.rendering.grounding.origin.unwrap_or([
        (edit.definition.rendering.size[0] / 2) as i32,
        edit.definition.rendering.size[1].saturating_sub(1) as i32,
    ])
}

fn resolved_ground_footprint(edit: &crate::ui::editor_ui::EntityEditState) -> EntityFootprint {
    edit.definition
        .rendering
        .grounding
        .footprint
        .unwrap_or(EntityFootprint::new(
            edit.definition.collision.offset,
            edit.definition.collision.size,
        ))
}

fn sync_collision_from_grounding(edit: &mut crate::ui::editor_ui::EntityEditState) {
    if let Some(footprint) = edit.definition.rendering.grounding.footprint {
        edit.definition.collision.offset = footprint.offset;
        edit.definition.collision.size = footprint.size;
    }
}

pub fn render_rendering_section(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    project_path: Option<&Path>,
) {
    let available_atlases = crate::ui::editor_context::entity_editor_state_mut(ui_state)
        .available_atlases
        .clone();
    let Some(edit) = crate::ui::editor_context::entity_editor_state_mut(ui_state)
        .edit_state
        .as_mut()
    else {
        return;
    };

    egui::CollapsingHeader::new("Rendering")
        .default_open(true)
        .show(ui, |ui| {
            // Sprite Atlas dropdown
            ui.horizontal(|ui| {
                ui.label("Animation Source:");
                render_atlas_dropdown(
                    ui,
                    "sprite_atlas",
                    &mut edit.definition.animations.atlas_name,
                    &available_atlases,
                    &mut edit.dirty,
                );
            });

            render_decoration_object_section(ui, edit, project_path);

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
            if ui
                .checkbox(&mut edit.definition.rendering.has_shadow, "Has Shadow")
                .changed()
            {
                edit.mark_dirty();
            }
            ui.horizontal(|ui| {
                ui.label("Palette Override:");
                if ui
                    .text_edit_singleline(
                        edit.definition
                            .rendering
                            .palette_override
                            .get_or_insert_with(String::new),
                    )
                    .changed()
                {
                    if edit
                        .definition
                        .rendering
                        .palette_override
                        .as_deref()
                        .is_some_and(|value| value.trim().is_empty())
                    {
                        edit.definition.rendering.palette_override = None;
                    }
                    edit.mark_dirty();
                }
            });

            egui::CollapsingHeader::new("Grounding")
                .default_open(false)
                .show(ui, |ui| {
                    let mut origin = resolved_ground_origin(edit);
                    ui.horizontal(|ui| {
                        ui.label("Origin:");
                        let origin_changed = ui.add(egui::DragValue::new(&mut origin[0])).changed()
                            | ui.add(egui::DragValue::new(&mut origin[1])).changed();
                        if origin_changed {
                            edit.definition.rendering.grounding.origin = Some(origin);
                            edit.mark_dirty();
                        }
                    });

                    let mut footprint = resolved_ground_footprint(edit);
                    ui.horizontal(|ui| {
                        ui.label("Footprint Offset:");
                        let offset_changed = ui
                            .add(egui::DragValue::new(&mut footprint.offset[0]))
                            .changed()
                            | ui.add(egui::DragValue::new(&mut footprint.offset[1]))
                                .changed();
                        if offset_changed {
                            edit.definition.rendering.grounding.footprint = Some(footprint);
                            sync_collision_from_grounding(edit);
                            edit.mark_dirty();
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Footprint Size:");
                        let mut width = footprint.size[0] as i32;
                        let mut height = footprint.size[1] as i32;
                        let size_changed = ui
                            .add(egui::DragValue::new(&mut width).range(1..=1024))
                            .changed()
                            | ui.add(egui::DragValue::new(&mut height).range(1..=1024))
                                .changed();
                        if size_changed {
                            footprint.size = [width.max(1) as u32, height.max(1) as u32];
                            edit.definition.rendering.grounding.footprint = Some(footprint);
                            sync_collision_from_grounding(edit);
                            edit.mark_dirty();
                        }
                    });
                });
        });
}

fn render_decoration_object_section(
    ui: &mut egui::Ui,
    edit: &mut crate::ui::editor_ui::EntityEditState,
    project_path: Option<&Path>,
) {
    if !uses_decoration_collision_policy(effective_kind_for_category(&edit.definition.category)) {
        return;
    }

    ui.separator();
    ui.label("Static Decoration");

    let Some(project_path) = project_path else {
        ui.small("Open a project to browse decoration object sheets.");
        return;
    };

    let selected_sheet = edit
        .definition
        .rendering
        .static_object
        .as_ref()
        .map(|render| render.sheet.clone());
    let Some(source) = resolve_object_sheet_browser_source(project_path, selected_sheet.as_deref())
    else {
        ui.small("No object sheets found in assets/sprites.");
        return;
    };

    let mut selected_sheet = edit
        .definition
        .rendering
        .static_object
        .as_ref()
        .map(|render| render.sheet.clone());
    let mut selected_object = edit
        .definition
        .rendering
        .static_object
        .as_ref()
        .map(|render| render.object_name.clone());

    sync_selected_sheet_name(&mut selected_sheet, &source.sheet_names);
    let active_source =
        resolve_object_sheet_browser_source(project_path, selected_sheet.as_deref()).unwrap_or(source);
    sync_selected_object_name(&mut selected_object, &active_source.object_names);

    let mut sheet_value = selected_sheet.unwrap_or_else(|| active_source.selected_sheet_name.clone());
    let mut object_value = selected_object
        .unwrap_or_else(|| active_source.object_names.first().cloned().unwrap_or_default());
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label("Object Sheet:");
        egui::ComboBox::from_id_salt("entity_definition_static_object_sheet")
            .selected_text(sheet_value.as_str())
            .show_ui(ui, |ui| {
                for sheet_name in &active_source.sheet_names {
                    changed |= ui
                        .selectable_value(&mut sheet_value, sheet_name.clone(), sheet_name.as_str())
                        .changed();
                }
            });
    });

    let object_source =
        resolve_object_sheet_browser_source(project_path, Some(sheet_value.as_str()))
            .unwrap_or(active_source);
    if !object_source.object_names.iter().any(|name| name == &object_value) {
        object_value = object_source
            .object_names
            .first()
            .cloned()
            .unwrap_or_default();
        changed = true;
    }

    ui.horizontal(|ui| {
        ui.label("Object:");
        egui::ComboBox::from_id_salt("entity_definition_static_object_name")
            .selected_text(object_value.as_str())
            .show_ui(ui, |ui| {
                for object_name in &object_source.object_names {
                    changed |= ui
                        .selectable_value(
                            &mut object_value,
                            object_name.clone(),
                            object_name.as_str(),
                        )
                        .changed();
                }
            });
    });

    if changed {
        edit.definition.rendering.static_object = Some(toki_core::entity::StaticObjectRenderDef {
            sheet: sheet_value.clone(),
            object_name: object_value.clone(),
        });

        if let Some(draft) = build_decoration_placement_draft(project_path, &sheet_value, &object_value)
        {
            let footprint = draft.grounding.footprint;
            edit.definition.rendering.size = [draft.size_px.x, draft.size_px.y];
            edit.definition.rendering.grounding = draft.grounding;
            edit.definition.collision.enabled = draft.solid;
            edit.definition.collision.offset = footprint.map(|f| f.offset).unwrap_or([0, 0]);
            edit.definition.collision.size = footprint
                .map(|f| f.size)
                .unwrap_or([draft.size_px.x, draft.size_px.y]);
            edit.definition.collision.trigger = false;
        }

        edit.mark_dirty();
    }
}

pub fn render_attributes_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = crate::ui::editor_context::entity_editor_state_mut(ui_state)
        .edit_state
        .as_mut()
    else {
        return;
    };

    egui::CollapsingHeader::new("Attributes")
        .default_open(true)
        .show(ui, |ui| {
            let kind = effective_kind_for_category(&edit.definition.category);
            let show_movement =
                edit.definition.components.movement.is_some() || kind_supports_movement(kind);

            if show_movement {
                ui.horizontal(|ui| {
                    ui.label("Speed:");
                    let speed = edit
                        .definition
                        .components
                        .movement
                        .as_ref()
                        .map(|movement| movement.speed)
                        .unwrap_or(0.0);
                    let mut speed = speed;
                    if ui
                        .add(egui::DragValue::new(&mut speed).speed(0.1))
                        .changed()
                    {
                        edit.definition
                            .components
                            .movement
                            .get_or_insert_with(toki_core::entity::MovementComponent::default)
                            .speed = speed.max(0.0);
                        edit.mark_dirty();
                    }
                });
            }

            // Boolean attributes
            if ui.checkbox(&mut edit.definition.solid, "Solid").changed() {
                edit.mark_dirty();
            }
            if ui.checkbox(&mut edit.definition.active, "Active").changed() {
                edit.mark_dirty();
            }
            if show_movement {
                let mut can_move = edit
                    .definition
                    .components
                    .movement
                    .as_ref()
                    .is_some_and(|movement| movement.can_move);
                if ui.checkbox(&mut can_move, "Can Move").changed() {
                    if can_move {
                        edit.definition
                            .components
                            .movement
                            .get_or_insert_with(toki_core::entity::MovementComponent::default)
                            .can_move = true;
                    } else if let Some(movement) = edit.definition.components.movement.as_mut() {
                        movement.can_move = false;
                    }
                    edit.mark_dirty();
                }
            }
            let mut interactable = edit.definition.components.interaction.is_some();
            if ui.checkbox(&mut interactable, "Interactable").changed() {
                edit.definition.components.interaction = if interactable {
                    Some(toki_core::entity::InteractionComponent {
                        interaction_reach: 32,
                    })
                } else {
                    None
                };
                edit.mark_dirty();
            }

            // Interaction reach (only if interactable)
            if let Some(current_reach) = edit
                .definition
                .components
                .interaction
                .as_ref()
                .map(|interaction| interaction.interaction_reach as i32)
            {
                let mut reach = current_reach;
                let changed = ui
                    .horizontal(|ui| {
                        ui.label("Interaction Reach:");
                        ui.add(egui::DragValue::new(&mut reach).range(0..=256))
                            .changed()
                    })
                    .inner;
                if changed {
                    if let Some(interaction) = edit.definition.components.interaction.as_mut() {
                        interaction.interaction_reach = reach.max(0) as u32;
                    }
                    edit.mark_dirty();
                }
            }
        });
}
