//! Entity inspector - single or multi-entity selection editing.

use super::super::super::inspector_trait::{Inspector, InspectorContext};
use super::super::InspectorSystem;
use toki_core::entity::EntityId;

/// Inspector for single or multi-entity selection.
pub struct EntityInspector {
    entity_id: EntityId,
}

impl EntityInspector {
    pub fn new(entity_id: EntityId) -> Self {
        Self { entity_id }
    }
}

impl Inspector for EntityInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        let mut entity_changed = false;

        if ctx.ui_state.has_multi_entity_selection() {
            ui.heading(format!(
                "Entities: {}",
                ctx.ui_state.selected_entity_ids().len()
            ));
            ui.separator();
            entity_changed = InspectorSystem::render_multi_scene_entity_editor(ui, ctx.ui_state);
        } else {
            ui.separator();
            ui.heading(format!("Entity {}", self.entity_id));
            ui.separator();

            if let Some(scene_entity) =
                InspectorSystem::find_selected_scene_entity(ctx.ui_state, self.entity_id)
            {
                let mut draft = super::super::EntityPropertyDraft::from_entity(&scene_entity);
                if InspectorSystem::render_scene_entity_editor(ui, &mut draft, ctx.config) {
                    entity_changed = InspectorSystem::apply_entity_property_draft_with_undo(
                        ctx.ui_state,
                        self.entity_id,
                        &draft,
                    );
                }
            } else {
                ui.label("Runtime-only entity (read-only)");
                ui.separator();
                InspectorSystem::render_runtime_entity_read_only(
                    ui,
                    ctx.game_state,
                    self.entity_id,
                );
            }
        }

        if entity_changed {
            ctx.ui_state.scene_content_changed = true;
        }
        entity_changed
    }

    fn name(&self) -> &'static str {
        "Entity"
    }
}
