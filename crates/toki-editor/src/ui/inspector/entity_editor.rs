// Entity Editor tab inspector
// Shows info and controls when the Entity Editor center panel tab is active

use super::InspectorSystem;

impl InspectorSystem {
    /// Render inspector panel when the Entity Editor tab is active.
    /// Shows entity info, component toggles, and quick actions.
    pub(super) fn render_entity_editor_inspector(
        ui_state: &mut crate::ui::EditorUI,
        ui: &mut egui::Ui,
    ) {
        ui.heading("Entity Editor");
        ui.separator();

        if !ui_state.entity_editor.has_entity() {
            ui.label("No entity selected.");
            ui.label("Select an entity from the browser or create a new one.");
            return;
        }

        // Entity info
        if let Some(summary) = ui_state.entity_editor.selected_entity_summary() {
            ui.label(format!("Name: {}", summary.name));
            ui.label(format!("Display: {}", summary.display_name));
            ui.label(format!("Category: {}", summary.category));

            if !summary.tags.is_empty() {
                ui.label(format!("Tags: {}", summary.tags.join(", ")));
            }
        }

        ui.separator();

        // Quick actions
        ui.label("Quick Actions:");

        if ui.button("Open in Animation Editor").clicked() {
            // Switch to animation editor tab (entity stays selected)
            ui_state.center_panel_tab = crate::ui::editor_ui::CenterPanelTab::AnimationEditor;
        }

        ui.separator();

        // Statistics
        ui.label("Browser Statistics:");
        ui.label(format!(
            "Total entities: {}",
            ui_state.entity_editor.entities.len()
        ));

        let filtered_count = ui_state.entity_editor.filtered_entities().len();
        if ui_state.entity_editor.filter.is_active() {
            ui.label(format!("Filtered: {}", filtered_count));
        }

        let category_count = ui_state.entity_editor.all_categories().len();
        ui.label(format!("Categories: {}", category_count));

        let tag_count = ui_state.entity_editor.all_tags().len();
        ui.label(format!("Unique tags: {}", tag_count));

        // Help
        ui.separator();
        ui.label("Tips:");
        ui.label("- Right-click entities for context menu");
        ui.label("- Use filters to find entities");
        ui.label("- Property editing in Phase 4.5C");
    }
}
