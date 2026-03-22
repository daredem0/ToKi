//! Menu selection inspector - placeholder for menu editor integration.

use super::super::super::inspector_trait::{Inspector, InspectorContext};

/// Inspector for menu selections (placeholder).
pub struct MenuSelectionInspector;

impl Inspector for MenuSelectionInspector {
    fn render(&mut self, ui: &mut egui::Ui, _ctx: &mut InspectorContext<'_>) -> bool {
        ui.label("Menu selection available only in Menu Editor.");
        false
    }

    fn name(&self) -> &'static str {
        "MenuSelection"
    }
}
