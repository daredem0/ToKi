//! Scene inspector - scene properties, music, anchors, and player entry editing.

use super::super::super::inspector_trait::{Inspector, InspectorContext};
use super::scene_helpers::{
    render_background_music_editor, render_delete_scene_button, render_rules_editor_section,
    render_scene_actions, render_scene_anchors_list, render_scene_stats,
};
use crate::editor_services::commands as editor_commands;
use toki_core::entity::ControlRole;
use toki_core::scene::{SceneAnchorKind, ScenePlayerEntry};

/// Inspector for scene selection.
pub struct SceneInspector {
    scene_name: String,
}

impl SceneInspector {
    pub fn new(scene_name: String) -> Self {
        Self { scene_name }
    }

    pub(super) fn load_scene_music_choices(
        ctx: &InspectorContext<'_>,
        current_track_id: Option<&str>,
    ) -> Vec<String> {
        let project_path = ctx
            .project
            .as_ref()
            .map(|project| project.path.clone())
            .or_else(|| {
                ctx.config
                    .and_then(|config| config.current_project_path().cloned())
            });

        let mut choices = project_path
            .map(|path| {
                crate::project::ProjectAssets::discover_project_audio_names(
                    &path,
                    crate::project::assets::ProjectAudioAssetKind::Music,
                )
            })
            .unwrap_or_default();

        if let Some(current_track_id) = current_track_id {
            if !current_track_id.trim().is_empty()
                && !choices.iter().any(|choice| choice == current_track_id)
            {
                choices.push(current_track_id.to_string());
                choices.sort();
                choices.dedup();
            }
        }

        choices
    }

    fn scene_has_authored_player_entity(scene: &toki_core::Scene) -> bool {
        scene
            .entities
            .iter()
            .any(|entity| entity.control_role == ControlRole::PlayerCharacter)
    }

    pub(super) fn render_scene_player_entry_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        scene: &toki_core::Scene,
        ctx: &mut InspectorContext<'_>,
    ) -> bool {
        ui.label("Scene Player Entry:");

        let entry_ctx = PlayerEntryContext::new(scene, ctx.project.as_deref());

        if Self::scene_has_authored_player_entity(scene) {
            ui.colored_label(
                egui::Color32::YELLOW,
                "This scene already contains a placed player entity. Scene Player Entry preview stays disabled until that authored player entity is removed.",
            );
        }

        match scene.player_entry.clone() {
            Some(entry) => {
                Self::render_existing_player_entry(ui, scene_name, scene, ctx, &entry_ctx, entry)
            }
            None => Self::render_add_player_entry_button(ui, scene_name, scene, ctx, &entry_ctx),
        }
    }

    fn render_existing_player_entry(
        ui: &mut egui::Ui,
        scene_name: &str,
        scene: &toki_core::Scene,
        ctx: &mut InspectorContext<'_>,
        entry_ctx: &PlayerEntryContext,
        current_entry: ScenePlayerEntry,
    ) -> bool {
        let mut edited_entry = current_entry.clone();
        let mut entry_changed = false;

        entry_changed |= Self::render_entry_definition_combo(
            ui,
            scene_name,
            &entry_ctx.entity_defs,
            &mut edited_entry.entity_definition_name,
        );
        entry_changed |= Self::render_entry_spawn_point_combo(
            ui,
            scene_name,
            &entry_ctx.spawn_points,
            &mut edited_entry.spawn_point_id,
        );

        if entry_changed
            && Self::commit_player_entry_change(scene_name, scene, Some(edited_entry), ctx)
        {
            return true;
        }

        Self::render_entry_validation_hints(ui, entry_ctx);

        if ui.button("Remove Scene Player Entry").clicked() {
            return Self::remove_player_entry(scene_name, scene, ctx);
        }

        false
    }

    fn render_entry_definition_combo(
        ui: &mut egui::Ui,
        scene_name: &str,
        choices: &[String],
        value: &mut String,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Entity Definition:");
            egui::ComboBox::from_id_salt(("scene_player_entity_definition", scene_name))
                .selected_text(value.as_str())
                .show_ui(ui, |ui| {
                    for name in choices {
                        changed |= ui.selectable_value(value, name.clone(), name).changed();
                    }
                });
        });
        changed
    }

    fn render_entry_spawn_point_combo(
        ui: &mut egui::Ui,
        scene_name: &str,
        choices: &[String],
        value: &mut String,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Spawn Point:");
            egui::ComboBox::from_id_salt(("scene_player_spawn_point", scene_name))
                .selected_text(value.as_str())
                .show_ui(ui, |ui| {
                    for id in choices {
                        changed |= ui.selectable_value(value, id.clone(), id).changed();
                    }
                });
        });
        changed
    }

    fn render_entry_validation_hints(ui: &mut egui::Ui, entry_ctx: &PlayerEntryContext) {
        if entry_ctx.entity_defs.is_empty() {
            ui.label("No entity definitions found in this project.");
        }
        if entry_ctx.spawn_points.is_empty() {
            ui.label("Add a spawn point before assigning a scene player entry.");
        }
    }

    fn commit_player_entry_change(
        scene_name: &str,
        scene: &toki_core::Scene,
        new_entry: Option<ScenePlayerEntry>,
        ctx: &mut InspectorContext<'_>,
    ) -> bool {
        let before_scene = scene.clone();
        let mut after_scene = before_scene.clone();
        after_scene.player_entry = new_entry;
        editor_commands::execute(
            ctx.ui_state,
            crate::ui::undo_redo::EditorCommand::update_scene(
                scene_name.to_string(),
                before_scene,
                after_scene,
            ),
        )
    }

    fn remove_player_entry(
        scene_name: &str,
        scene: &toki_core::Scene,
        ctx: &mut InspectorContext<'_>,
    ) -> bool {
        if Self::commit_player_entry_change(scene_name, scene, None, ctx) {
            if matches!(
                ctx.ui_state.selection,
                Some(crate::ui::editor_ui::Selection::ScenePlayerEntry(ref s)) if s == scene_name
            ) {
                ctx.ui_state
                    .set_selection(crate::ui::editor_ui::Selection::Scene(
                        scene_name.to_string(),
                    ));
            }
            return true;
        }
        false
    }

    fn render_add_player_entry_button(
        ui: &mut egui::Ui,
        scene_name: &str,
        scene: &toki_core::Scene,
        ctx: &mut InspectorContext<'_>,
        entry_ctx: &PlayerEntryContext,
    ) -> bool {
        if entry_ctx.entity_defs.is_empty() {
            ui.label("No entity definitions found in this project.");
        }
        if entry_ctx.spawn_points.is_empty() {
            ui.label("Add a spawn point before creating a scene player entry.");
        }

        let can_add = !entry_ctx.entity_defs.is_empty() && !entry_ctx.spawn_points.is_empty();
        if ui
            .add_enabled(can_add, egui::Button::new("Add Scene Player Entry"))
            .clicked()
        {
            let new_entry = ScenePlayerEntry {
                entity_definition_name: entry_ctx.entity_defs[0].clone(),
                spawn_point_id: entry_ctx.spawn_points[0].clone(),
            };
            return Self::commit_player_entry_change(scene_name, scene, Some(new_entry), ctx);
        }
        false
    }
}

/// Context for player entry editing
struct PlayerEntryContext {
    entity_defs: Vec<String>,
    spawn_points: Vec<String>,
}

impl PlayerEntryContext {
    fn new(scene: &toki_core::Scene, project: Option<&crate::project::Project>) -> Self {
        let entity_defs = project
            .map(|p| {
                crate::project::ProjectAssets::discover_project_entity_definition_names(&p.path)
            })
            .unwrap_or_default();
        let spawn_points = scene
            .anchors
            .iter()
            .filter(|a| a.kind == SceneAnchorKind::SpawnPoint)
            .map(|a| a.id.clone())
            .collect();
        Self {
            entity_defs,
            spawn_points,
        }
    }
}

impl Inspector for SceneInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading(format!("Scene: {}", self.scene_name));
        ui.separator();

        let Some(scene) = ctx.ui_state.get_scene(&self.scene_name).cloned() else {
            return false;
        };

        render_scene_stats(ui, &scene);
        ui.separator();

        if render_delete_scene_button(ui, ctx, &self.scene_name, &scene) {
            return false;
        }

        ui.separator();

        if render_background_music_editor(ui, ctx, &self.scene_name, &scene) {
            return true;
        }

        ui.separator();
        if Self::render_scene_player_entry_editor(ui, &self.scene_name, &scene, ctx) {
            return true;
        }

        ui.separator();
        render_scene_actions(ui);

        ui.separator();
        render_scene_anchors_list(ui, ctx, &self.scene_name, &scene);

        render_rules_editor_section(ui, ctx, &self.scene_name)
    }

    fn name(&self) -> &'static str {
        "Scene"
    }
}
