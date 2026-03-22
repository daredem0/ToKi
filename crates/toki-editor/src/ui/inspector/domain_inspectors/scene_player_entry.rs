//! Scene player entry inspector - player spawn configuration editing.

use super::super::super::inspector_trait::{Inspector, InspectorContext};
use super::super::InspectorSystem;
use super::scene::SceneInspector;

pub struct ScenePlayerEntryInspector {
    scene_name: String,
}

impl ScenePlayerEntryInspector {
    pub fn new(scene_name: String) -> Self {
        Self { scene_name }
    }
}

impl Inspector for ScenePlayerEntryInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading("Scene Player");
        ui.label(format!("Scene: {}", self.scene_name));
        ui.separator();

        let Some(scene) = ctx.ui_state.get_scene(&self.scene_name).cloned() else {
            ui.label("Scene not found.");
            return false;
        };

        let changed =
            SceneInspector::render_scene_player_entry_editor(ui, &self.scene_name, &scene, ctx);

        if changed {
            return true;
        }

        render_player_entity_definition(ui, ctx, &scene)
    }

    fn name(&self) -> &'static str {
        "ScenePlayerEntry"
    }
}

fn render_player_entity_definition(
    ui: &mut egui::Ui,
    ctx: &mut InspectorContext<'_>,
    scene: &toki_core::Scene,
) -> bool {
    let Some(player_entry) = scene.player_entry.as_ref() else {
        return false;
    };

    ui.separator();
    ui.heading(format!("Entity: {}", player_entry.entity_definition_name));
    ui.label("Player Entity Definition");
    ui.separator();

    let Some(config) = ctx.config else {
        return false;
    };

    let Some(project_path) = config.current_project_path() else {
        return false;
    };

    let entity_file = project_path
        .join("entities")
        .join(format!("{}.json", player_entry.entity_definition_name));

    match std::fs::read_to_string(&entity_file) {
        Ok(content) => match serde_json::from_str::<toki_core::entity::EntityDefinition>(&content) {
            Ok(mut definition) => {
                let mut draft =
                    super::super::EntityPropertyDraft::from_entity_definition(&definition);
                if InspectorSystem::render_entity_definition_property_editor(ui, &mut draft, ctx.config)
                    && InspectorSystem::apply_entity_property_draft_to_definition(
                        &mut definition,
                        &draft,
                    )
                {
                    if let Err(err) =
                        InspectorSystem::save_entity_definition(&definition, &entity_file)
                    {
                        ui.colored_label(egui::Color32::RED, err);
                    } else {
                        ctx.ui_state.scene_content_changed = true;
                        return true;
                    }
                }
            }
            Err(err) => {
                ui.colored_label(
                    egui::Color32::RED,
                    format!("Failed to parse entity definition: {err}"),
                );
            }
        },
        Err(err) => {
            ui.colored_label(
                egui::Color32::RED,
                format!("Failed to read entity definition: {err}"),
            );
        }
    }

    false
}
