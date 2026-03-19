use super::editor_ui::EditorUI;
use crate::config::EditorConfig;
use crate::project::Project;

/// Context passed to inspectors containing all commonly needed references.
/// This bundles the arguments that would otherwise be passed individually.
pub struct InspectorContext<'a> {
    pub ui_state: &'a mut EditorUI,
    /// The egui context for the current frame.
    #[allow(dead_code)] // Reserved for inspectors that need frame-level access
    pub ctx: &'a egui::Context,
    pub game_state: Option<&'a toki_core::GameState>,
    /// The project for inspectors that need to modify project settings.
    #[allow(dead_code)] // Reserved for project settings inspector
    pub project: Option<&'a mut Project>,
    #[allow(dead_code)] // Reserved for template editor and future schema-driven inspectors
    pub template_asset_choices: Option<&'a crate::ui::template_workflow::TemplateAssetChoices>,
    pub config: Option<&'a EditorConfig>,
}

/// Trait for domain-specific inspector panels.
/// Each inspector handles rendering for a specific selection type.
pub trait Inspector {
    /// Render the inspector contents for the current selection.
    /// Returns true if the inspector made changes that should mark content as dirty.
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool;

    /// Returns a human-readable name for this inspector (for debugging/logging).
    /// Used primarily for testing and diagnostics.
    #[allow(dead_code)] // Used in tests
    fn name(&self) -> &'static str;
}

/// Inspector that renders when nothing is selected.
pub struct NoSelectionInspector;

impl Inspector for NoSelectionInspector {
    fn render(&mut self, ui: &mut egui::Ui, _ctx: &mut InspectorContext<'_>) -> bool {
        ui.label("No selection");
        ui.separator();
        ui.label("Click on an item in the hierarchy to inspect it.");
        false
    }

    fn name(&self) -> &'static str {
        "NoSelection"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_selection_inspector_has_correct_name() {
        let inspector = NoSelectionInspector;
        assert_eq!(inspector.name(), "NoSelection");
    }
}
