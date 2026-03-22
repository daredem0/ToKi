//! Rule graph node inspector - editing rule nodes in the scene graph.

use super::super::super::inspector_trait::{Inspector, InspectorContext};
use super::super::InspectorSystem;

/// Inspector for rule graph node selection.
pub struct RuleGraphNodeInspector {
    scene_name: String,
    node_key: String,
}

impl RuleGraphNodeInspector {
    pub fn new(scene_name: String, node_key: String) -> Self {
        Self {
            scene_name,
            node_key,
        }
    }
}

impl Inspector for RuleGraphNodeInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading("Scene Rule Node");
        ui.label(format!("Scene: {}", self.scene_name));
        ui.monospace(&self.node_key);
        ui.separator();

        let changed = InspectorSystem::render_selected_rule_graph_node_editor(
            ui,
            ctx.ui_state,
            &self.scene_name,
            &self.node_key,
            ctx.config,
        );

        if changed {
            ctx.ui_state.scene_content_changed = true;
        }
        changed
    }

    fn name(&self) -> &'static str {
        "RuleGraphNode"
    }
}
