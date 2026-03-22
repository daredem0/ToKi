//! Map inspectors - map properties within a scene or standalone.

use super::super::super::inspector_trait::{Inspector, InspectorContext};
use super::super::InspectorSystem;

/// Inspector for map selection (within a scene).
pub struct MapInspector {
    scene_name: String,
    map_name: String,
}

impl MapInspector {
    pub fn new(scene_name: String, map_name: String) -> Self {
        Self {
            scene_name,
            map_name,
        }
    }
}

impl Inspector for MapInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading(format!("Map: {}", self.map_name));
        ui.label(format!("Scene: {}", self.scene_name));
        ui.separator();

        InspectorSystem::render_map_details(
            ui,
            &self.map_name,
            ctx.config,
            Some(&self.scene_name),
            &mut ctx.ui_state.map.load_requested,
        );
        false
    }

    fn name(&self) -> &'static str {
        "Map"
    }
}

/// Inspector for standalone map (not in scene context).
pub struct StandaloneMapInspector {
    map_name: String,
}

impl StandaloneMapInspector {
    pub fn new(map_name: String) -> Self {
        Self { map_name }
    }
}

impl Inspector for StandaloneMapInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading(format!("Map: {}", self.map_name));
        ui.label("(Standalone map - not in scene)");
        ui.separator();

        InspectorSystem::render_map_details(
            ui,
            &self.map_name,
            ctx.config,
            None,
            &mut ctx.ui_state.map.load_requested,
        );
        false
    }

    fn name(&self) -> &'static str {
        "StandaloneMap"
    }
}
