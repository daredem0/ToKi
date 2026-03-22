//! Multi-entity batch editing functionality.

use super::super::InspectorSystem;
use super::types::{MultiEntityBatchEdit, MultiEntityCommonState};
use crate::editor_services::commands as editor_commands;
use crate::ui::editor_ui::EditorUI;
use crate::ui::undo_redo::EditorCommand;
use std::collections::HashSet;
use toki_core::entity::EntityId;

impl InspectorSystem {
    pub(in super::super) fn render_multi_scene_entity_editor(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
    ) -> bool {
        let Some(active_scene_name) = ui_state.active_scene.clone() else {
            ui.label("No active scene");
            return false;
        };

        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|scene| scene.name == active_scene_name)
        else {
            ui.label("Active scene not found");
            return false;
        };

        let selected_ids = ui_state.selected_entity_ids_vec();
        let selected_set: HashSet<_> = selected_ids.iter().copied().collect();
        let selected_entities = {
            let scene = &ui_state.scenes[scene_index];
            scene
                .entities
                .iter()
                .filter(|entity| selected_set.contains(&entity.id))
                .collect::<Vec<_>>()
        };

        if selected_entities.len() < 2 {
            ui.label("Select at least two scene entities for batch editing.");
            return false;
        }

        let common = Self::collect_multi_entity_common_state(&selected_entities);
        if ui_state.multi_entity_inspector_selection_signature != selected_ids {
            ui_state.multi_entity_inspector_selection_signature = selected_ids;
            ui_state.multi_entity_render_layer_input = common.render_layer.unwrap_or(0) as i64;
            ui_state.multi_entity_delta_x_input = 0;
            ui_state.multi_entity_delta_y_input = 0;
        }

        ui.label("Batch edit selected scene entities");
        ui.horizontal(|ui| {
            ui.label("Entities:");
            ui.label(selected_entities.len().to_string());
        });
        ui.separator();

        let mut edit = MultiEntityBatchEdit::default();
        render_bool_toggle_row(
            ui,
            "Visible",
            common.visible,
            &mut edit.set_visible,
            "Set Visible",
            "Set Hidden",
        );
        render_bool_toggle_row(
            ui,
            "Active",
            common.active,
            &mut edit.set_active,
            "Set Active",
            "Set Inactive",
        );
        render_bool_toggle_row(
            ui,
            "Collision",
            common.collision_enabled,
            &mut edit.set_collision_enabled,
            "Enable Collision",
            "Disable Collision",
        );

        render_render_layer_row(ui, ui_state, &common, &mut edit);
        render_position_delta_row(ui, ui_state, &mut edit);

        if edit.is_noop() {
            return false;
        }

        Self::apply_multi_entity_batch_edit_with_undo(
            ui_state,
            &active_scene_name,
            &selected_set,
            edit,
        )
    }

    pub(in super::super) fn collect_multi_entity_common_state(
        entities: &[&toki_core::entity::Entity],
    ) -> MultiEntityCommonState {
        fn common_bool(
            entities: &[&toki_core::entity::Entity],
            accessor: impl Fn(&toki_core::entity::Entity) -> bool,
        ) -> Option<bool> {
            let first = entities.first().map(|entity| accessor(entity))?;
            if entities.iter().all(|entity| accessor(entity) == first) {
                Some(first)
            } else {
                None
            }
        }

        fn common_i32(
            entities: &[&toki_core::entity::Entity],
            accessor: impl Fn(&toki_core::entity::Entity) -> i32,
        ) -> Option<i32> {
            let first = entities.first().map(|entity| accessor(entity))?;
            if entities.iter().all(|entity| accessor(entity) == first) {
                Some(first)
            } else {
                None
            }
        }

        MultiEntityCommonState {
            visible: common_bool(entities, |entity| entity.attributes.visible),
            active: common_bool(entities, |entity| entity.attributes.active),
            collision_enabled: common_bool(entities, |entity| entity.collision_box.is_some()),
            render_layer: common_i32(entities, |entity| entity.attributes.render_layer),
        }
    }

    pub(in super::super) fn apply_multi_entity_batch_edit_with_undo(
        ui_state: &mut EditorUI,
        scene_name: &str,
        selected_set: &HashSet<EntityId>,
        edit: MultiEntityBatchEdit,
    ) -> bool {
        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|scene| scene.name == scene_name)
        else {
            return false;
        };

        let before_entities = ui_state.scenes[scene_index]
            .entities
            .iter()
            .filter(|entity| selected_set.contains(&entity.id))
            .cloned()
            .collect::<Vec<_>>();

        if before_entities.is_empty() {
            return false;
        }

        let mut changed = false;
        let mut after_entities = Vec::with_capacity(before_entities.len());
        for before_entity in &before_entities {
            let mut after_entity = before_entity.clone();
            changed |= Self::apply_multi_entity_batch_edit_to_entity(&mut after_entity, edit);
            after_entities.push(after_entity);
        }

        if !changed {
            return false;
        }

        editor_commands::execute(ui_state, EditorCommand::update_entities(
            scene_name.to_string(),
            before_entities,
            after_entities,
        ))
    }

    pub(in super::super) fn apply_multi_entity_batch_edit_to_entity(
        entity: &mut toki_core::entity::Entity,
        edit: MultiEntityBatchEdit,
    ) -> bool {
        let mut changed = false;

        if let Some(visible) = edit.set_visible {
            if entity.attributes.visible != visible {
                entity.attributes.visible = visible;
                changed = true;
            }
        }

        if let Some(active) = edit.set_active {
            if entity.attributes.active != active {
                entity.attributes.active = active;
                changed = true;
            }
        }

        if let Some(render_layer) = edit.set_render_layer {
            if entity.attributes.render_layer != render_layer {
                entity.attributes.render_layer = render_layer;
                changed = true;
            }
        }

        if let Some(delta) = edit.position_delta {
            let new_position = entity.position + delta;
            if entity.position != new_position {
                entity.position = new_position;
                changed = true;
            }
        }

        if let Some(collision_enabled) = edit.set_collision_enabled {
            if collision_enabled {
                if entity.collision_box.is_none() {
                    entity.collision_box =
                        Some(toki_core::collision::CollisionBox::solid_box(entity.size));
                    changed = true;
                }
            } else if entity.collision_box.is_some() {
                entity.collision_box = None;
                changed = true;
            }
        }

        changed
    }
}

fn render_bool_toggle_row(
    ui: &mut egui::Ui,
    label: &str,
    common_value: Option<bool>,
    out_edit: &mut Option<bool>,
    true_button: &str,
    false_button: &str,
) {
    ui.horizontal(|ui| {
        let state_text = match common_value {
            Some(true) => "true",
            Some(false) => "false",
            None => "mixed",
        };
        ui.label(format!("{label}: {state_text}"));
        if ui.button(true_button).clicked() {
            *out_edit = Some(true);
        }
        if ui.button(false_button).clicked() {
            *out_edit = Some(false);
        }
    });
}

fn render_render_layer_row(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    common: &MultiEntityCommonState,
    edit: &mut MultiEntityBatchEdit,
) {
    ui.horizontal(|ui| {
        ui.label(format!(
            "Render Layer: {}",
            common
                .render_layer
                .map(|value| value.to_string())
                .unwrap_or_else(|| "Mixed".to_string())
        ));
        ui.add(egui::DragValue::new(&mut ui_state.multi_entity_render_layer_input).speed(1.0));
        if ui.button("Apply Layer").clicked() {
            edit.set_render_layer = Some(
                ui_state
                    .multi_entity_render_layer_input
                    .clamp(i32::MIN as i64, i32::MAX as i64) as i32,
            );
        }
    });
}

fn render_position_delta_row(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    edit: &mut MultiEntityBatchEdit,
) {
    ui.horizontal(|ui| {
        ui.label("Position Delta:");
        ui.add(egui::DragValue::new(&mut ui_state.multi_entity_delta_x_input).speed(1.0));
        ui.add(egui::DragValue::new(&mut ui_state.multi_entity_delta_y_input).speed(1.0));
        if ui.button("Apply Delta").clicked() {
            let delta = glam::IVec2::new(
                ui_state.multi_entity_delta_x_input,
                ui_state.multi_entity_delta_y_input,
            );
            if delta != glam::IVec2::ZERO {
                edit.position_delta = Some(delta);
            }
            ui_state.multi_entity_delta_x_input = 0;
            ui_state.multi_entity_delta_y_input = 0;
        }
    });
}
