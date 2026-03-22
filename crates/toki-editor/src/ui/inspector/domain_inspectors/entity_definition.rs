//! Entity definition inspector - entity palette item editing.

use super::super::super::inspector_trait::{Inspector, InspectorContext};
use super::super::InspectorSystem;

/// Inspector for entity definition from palette.
pub struct EntityDefinitionInspector {
    entity_name: String,
}

impl EntityDefinitionInspector {
    pub fn new(entity_name: String) -> Self {
        Self { entity_name }
    }
}

impl Inspector for EntityDefinitionInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading(format!("Entity: {}", self.entity_name));
        ui.label("Entity Definition");
        ui.separator();

        InspectorSystem::render_entity_definition_details(ui, &self.entity_name, ctx.config)
    }

    fn name(&self) -> &'static str {
        "EntityDefinition"
    }
}
