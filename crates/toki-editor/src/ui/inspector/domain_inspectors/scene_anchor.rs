//! Scene anchor inspector - spawn point editing.

use super::super::super::inspector_trait::{Inspector, InspectorContext};
use crate::editor_services::commands as editor_commands;
use toki_core::scene::{SceneAnchorFacing, SceneAnchorKind};

pub struct SceneAnchorInspector {
    scene_name: String,
    anchor_id: String,
}

impl SceneAnchorInspector {
    pub fn new(scene_name: String, anchor_id: String) -> Self {
        Self {
            scene_name,
            anchor_id,
        }
    }
}

impl Inspector for SceneAnchorInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading(format!("Anchor: {}", self.anchor_id));
        ui.label(format!("Scene: {}", self.scene_name));
        ui.separator();

        let Some(before_scene) = ctx.ui_state.get_scene(&self.scene_name).cloned() else {
            ui.label("Scene not found.");
            return false;
        };
        let Some(anchor_index) = before_scene
            .anchors
            .iter()
            .position(|anchor| anchor.id == self.anchor_id)
        else {
            ui.label("Selected anchor no longer exists.");
            return false;
        };

        let mut edited_anchor = before_scene.anchors[anchor_index].clone();
        let mut changed = false;
        let mut delete_requested = false;

        changed |= render_anchor_id_editor(ui, &mut edited_anchor.id);
        changed |= render_anchor_kind_editor(
            ui,
            &self.scene_name,
            &self.anchor_id,
            &mut edited_anchor.kind,
        );
        changed |= render_anchor_position_editor(ui, &mut edited_anchor.position);
        changed |= render_anchor_facing_editor(
            ui,
            &self.scene_name,
            &self.anchor_id,
            &mut edited_anchor.facing,
        );

        if has_duplicate_anchor_id(&before_scene.anchors, anchor_index, &edited_anchor.id) {
            ui.colored_label(
                egui::Color32::from_rgb(255, 210, 80),
                "Anchor id must be unique within the scene.",
            );
            changed = false;
        }

        if ui.button("Delete Anchor").clicked() {
            delete_requested = true;
        }

        if delete_requested {
            return handle_anchor_delete(ctx, &self.scene_name, before_scene, anchor_index);
        }

        if changed {
            return handle_anchor_update(
                ctx,
                &self.scene_name,
                before_scene,
                anchor_index,
                edited_anchor,
                &mut self.anchor_id,
            );
        }

        false
    }

    fn name(&self) -> &'static str {
        "SceneAnchor"
    }
}

fn render_anchor_id_editor(ui: &mut egui::Ui, id: &mut String) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Id:");
        changed |= ui.text_edit_singleline(id).changed();
    });
    changed
}

fn render_anchor_kind_editor(
    ui: &mut egui::Ui,
    scene_name: &str,
    anchor_id: &str,
    kind: &mut SceneAnchorKind,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Kind:");
        egui::ComboBox::from_id_salt(("scene_anchor_kind", scene_name, anchor_id))
            .selected_text(match kind {
                SceneAnchorKind::SpawnPoint => "SpawnPoint",
            })
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(kind, SceneAnchorKind::SpawnPoint, "SpawnPoint")
                    .changed();
            });
    });
    changed
}

fn render_anchor_position_editor(ui: &mut egui::Ui, position: &mut glam::IVec2) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Position:");
        changed |= ui
            .add(egui::DragValue::new(&mut position.x).speed(1.0))
            .changed();
        changed |= ui
            .add(egui::DragValue::new(&mut position.y).speed(1.0))
            .changed();
    });
    changed
}

fn render_anchor_facing_editor(
    ui: &mut egui::Ui,
    scene_name: &str,
    anchor_id: &str,
    facing: &mut Option<SceneAnchorFacing>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Facing:");
        let mut current_facing = *facing;
        egui::ComboBox::from_id_salt(("scene_anchor_facing", scene_name, anchor_id))
            .selected_text(match current_facing {
                None => "<none>",
                Some(SceneAnchorFacing::Up) => "Up",
                Some(SceneAnchorFacing::Down) => "Down",
                Some(SceneAnchorFacing::Left) => "Left",
                Some(SceneAnchorFacing::Right) => "Right",
            })
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(&mut current_facing, None, "<none>")
                    .changed();
                changed |= ui
                    .selectable_value(&mut current_facing, Some(SceneAnchorFacing::Up), "Up")
                    .changed();
                changed |= ui
                    .selectable_value(&mut current_facing, Some(SceneAnchorFacing::Down), "Down")
                    .changed();
                changed |= ui
                    .selectable_value(&mut current_facing, Some(SceneAnchorFacing::Left), "Left")
                    .changed();
                changed |= ui
                    .selectable_value(&mut current_facing, Some(SceneAnchorFacing::Right), "Right")
                    .changed();
            });
        *facing = current_facing;
    });
    changed
}

fn has_duplicate_anchor_id(
    anchors: &[toki_core::scene::SceneAnchor],
    current_index: usize,
    new_id: &str,
) -> bool {
    anchors
        .iter()
        .enumerate()
        .any(|(index, anchor)| index != current_index && anchor.id == new_id)
}

fn handle_anchor_delete(
    ctx: &mut InspectorContext<'_>,
    scene_name: &str,
    before_scene: toki_core::Scene,
    anchor_index: usize,
) -> bool {
    let mut after_scene = before_scene.clone();
    after_scene.anchors.remove(anchor_index);
    if editor_commands::execute(
        ctx.ui_state,
        crate::ui::undo_redo::EditorCommand::update_scene(
            scene_name.to_string(),
            before_scene,
            after_scene,
        ),
    )
    {
        ctx.ui_state
            .set_selection(crate::ui::editor_ui::Selection::Scene(
                scene_name.to_string(),
            ));
        return true;
    }
    false
}

fn handle_anchor_update(
    ctx: &mut InspectorContext<'_>,
    scene_name: &str,
    before_scene: toki_core::Scene,
    anchor_index: usize,
    edited_anchor: toki_core::scene::SceneAnchor,
    anchor_id: &mut String,
) -> bool {
    let mut after_scene = before_scene.clone();
    after_scene.anchors[anchor_index] = edited_anchor.clone();
    if editor_commands::execute(
        ctx.ui_state,
        crate::ui::undo_redo::EditorCommand::update_scene(
            scene_name.to_string(),
            before_scene,
            after_scene,
        ),
    )
    {
        *anchor_id = edited_anchor.id.clone();
        ctx.ui_state
            .set_selection(crate::ui::editor_ui::Selection::SceneAnchor {
                scene_name: scene_name.to_string(),
                anchor_id: edited_anchor.id,
            });
        return true;
    }
    false
}
