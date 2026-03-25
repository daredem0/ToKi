//! Map inspectors - map properties within a scene or standalone.

use super::super::super::inspector_trait::{Inspector, InspectorContext};
use super::super::InspectorSystem;

pub struct MapInspector {
    scene_name: Option<String>,
    map_name: String,
}

impl MapInspector {
    pub fn new(scene_name: String, map_name: String) -> Self {
        Self {
            scene_name: Some(scene_name),
            map_name,
        }
    }

    pub fn standalone(map_name: String) -> Self {
        Self {
            scene_name: None,
            map_name,
        }
    }
}

impl Inspector for MapInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading(format!("Map: {}", self.map_name));
        if let Some(scene_name) = &self.scene_name {
            ui.label(format!("Scene: {}", scene_name));
        } else {
            ui.label("(Standalone map - not in scene)");
        }
        ui.separator();

        InspectorSystem::render_map_details(
            ui,
            &self.map_name,
            ctx.config,
            self.scene_name.as_deref(),
            &mut ctx.ui_state.map.load_requested,
        );
        false
    }

    fn name(&self) -> &'static str {
        if self.scene_name.is_some() {
            "Map"
        } else {
            "StandaloneMap"
        }
    }
}
