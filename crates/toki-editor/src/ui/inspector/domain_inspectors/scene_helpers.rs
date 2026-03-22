//! Helper rendering functions for scene inspector.

use super::super::super::inspector_trait::InspectorContext;
use super::scene::SceneInspector;
use super::scene_commands::build_delete_scene_command;
use crate::editor_services::commands as editor_commands;
use crate::scene::view_models::SceneSummaryView;
use crate::ui::editor_ui::EditorConfirmation;
use crate::ui::inspector::InspectorSystem;
use toki_core::scene::SceneAnchorKind;

pub fn render_scene_stats(ui: &mut egui::Ui, scene: &toki_core::Scene) {
    let summary = SceneSummaryView::from_scene(scene);
    ui.horizontal(|ui| {
        ui.label("Maps:");
        ui.label(format!("{}", summary.map_count));
    });

    ui.horizontal(|ui| {
        ui.label("Entities:");
        ui.label(format!("{}", summary.entity_count));
    });
}

pub fn render_delete_scene_button(
    ui: &mut egui::Ui,
    ctx: &mut InspectorContext<'_>,
    scene_name: &str,
    scene: &toki_core::Scene,
) -> bool {
    if ui.button("Delete Scene").clicked() {
        let scene_is_empty =
            scene.maps.is_empty() && scene.entities.is_empty() && scene.rules.rules.is_empty();
        if scene_is_empty {
            if let Some(project) = ctx.project.as_deref_mut() {
                match build_delete_scene_command(ctx.ui_state, project, scene_name) {
                    Ok(command) => {
                        let _ = editor_commands::execute_with_project(ctx.ui_state, project, command);
                    }
                    Err(error) => {
                        tracing::error!(
                            "Failed to build delete scene command for '{}': {}",
                            scene_name,
                            error
                        );
                    }
                }
            }
        } else {
            ctx.ui_state.project.pending_confirmation = Some(EditorConfirmation::DeleteScene {
                scene_name: scene_name.to_string(),
            });
        }
        return true;
    }
    false
}

pub fn render_background_music_editor(
    ui: &mut egui::Ui,
    ctx: &mut InspectorContext<'_>,
    scene_name: &str,
    scene: &toki_core::Scene,
) -> bool {
    let music_choices =
        SceneInspector::load_scene_music_choices(ctx, scene.background_music_track_id.as_deref());
    let mut selected_background_music_track_id = scene.background_music_track_id.clone();

    ui.horizontal(|ui| {
        ui.label("Background Music:");
        egui::ComboBox::from_id_salt(format!("scene_background_music_{}", scene_name))
            .selected_text(
                selected_background_music_track_id
                    .as_deref()
                    .unwrap_or("<none>"),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected_background_music_track_id, None, "<none>");
                for track_id in &music_choices {
                    ui.selectable_value(
                        &mut selected_background_music_track_id,
                        Some(track_id.clone()),
                        track_id,
                    );
                }
            });
    });

    if selected_background_music_track_id != scene.background_music_track_id {
        let before_scene = scene.clone();
        let mut after_scene = before_scene.clone();
        after_scene.background_music_track_id = selected_background_music_track_id;
        return editor_commands::execute(
            ctx.ui_state,
            crate::ui::undo_redo::EditorCommand::update_scene(
                scene_name.to_string(),
                before_scene,
                after_scene,
            ),
        );
    }

    false
}

pub fn render_scene_actions(ui: &mut egui::Ui) {
    ui.label("Scene Actions:");

    if ui.button("Add Map").clicked() {
        tracing::info!("Add Map clicked");
    }

    if ui.button("Add Entity").clicked() {
        tracing::info!("Add Entity clicked");
    }
}

pub fn render_scene_anchors_list(
    ui: &mut egui::Ui,
    ctx: &mut InspectorContext<'_>,
    scene_name: &str,
    scene: &toki_core::Scene,
) {
    ui.label("Scene Anchors:");

    for anchor in &scene.anchors {
        let selected = matches!(
            ctx.ui_state.selection,
            Some(crate::ui::editor_ui::Selection::SceneAnchor {
                scene_name: ref sn,
                anchor_id: ref aid
            }) if sn == scene_name && aid == &anchor.id
        );
        if ui
            .selectable_label(
                selected,
                format!(
                    "{} ({:?}) @ {}, {}",
                    anchor.id, anchor.kind, anchor.position.x, anchor.position.y
                ),
            )
            .clicked()
        {
            ctx.ui_state
                .set_selection(crate::ui::editor_ui::Selection::SceneAnchor {
                    scene_name: scene_name.to_string(),
                    anchor_id: anchor.id.clone(),
                });
        }
    }

    if ui.button("Place Spawn Point").clicked() {
        let next_id = crate::ui::interactions::PlacementInteraction::next_scene_anchor_id(
            &scene.anchors,
            SceneAnchorKind::SpawnPoint,
        );
        ctx.ui_state.enter_scene_anchor_placement_mode(
            crate::ui::editor_ui::SceneAnchorPlacementDraft {
                kind: SceneAnchorKind::SpawnPoint,
                suggested_id: next_id,
            },
        );
    }
}

pub fn render_rules_editor_section(
    ui: &mut egui::Ui,
    ctx: &mut InspectorContext<'_>,
    scene_name: &str,
) -> bool {
    let Some(scene_index) = ctx
        .ui_state
        .scenes
        .iter()
        .position(|scene| scene.name == scene_name)
    else {
        return false;
    };

    ui.separator();
    let before_rules = ctx.ui_state.scenes[scene_index].rules.clone();
    let mut edited_rules = before_rules.clone();

    let map_size = extract_map_size(ctx, scene_index);
    let rules_changed = InspectorSystem::render_scene_rules_editor(
        ui,
        scene_name,
        &mut edited_rules,
        &ctx.ui_state.scenes,
        ctx.config,
        map_size,
    );

    if rules_changed && edited_rules != before_rules {
        commit_rules_change(ctx, scene_name, before_rules, edited_rules);
        return true;
    }

    false
}

fn extract_map_size(ctx: &InspectorContext<'_>, scene_index: usize) -> Option<(u32, u32)> {
    ctx.ui_state
        .scenes
        .get(scene_index)
        .and_then(|scene| scene.maps.first())
        .and_then(|map_name| {
            if ctx.ui_state.map.active_map.as_ref() == Some(map_name) {
                ctx.ui_state
                    .map
                    .draft
                    .as_ref()
                    .map(|draft| (draft.tilemap.size.x, draft.tilemap.size.y))
                    .or_else(|| {
                        ctx.ui_state
                            .map
                            .pending_tilemap_sync
                            .as_ref()
                            .map(|tm| (tm.size.x, tm.size.y))
                    })
            } else {
                None
            }
        })
}

fn commit_rules_change(
    ctx: &mut InspectorContext<'_>,
    scene_name: &str,
    before_rules: toki_core::rules::RuleSet,
    edited_rules: toki_core::rules::RuleSet,
) {
    use crate::ui::editor_ui::SceneRulesGraphCommandData;
    use crate::ui::rule_graph::RuleGraph;

    let before_graph = ctx.ui_state.rule_graph_for_scene(scene_name).cloned();
    let after_graph = RuleGraph::from_rule_set(&edited_rules);
    let before_layout = ctx.ui_state.graph.layouts_by_scene.get(scene_name).cloned();
    let (zoom, pan) = ctx.ui_state.graph_view_for_scene(scene_name);
    let _ = ctx.ui_state.execute_scene_rules_graph_command(
        scene_name,
        SceneRulesGraphCommandData {
            before_rule_set: before_rules,
            after_rule_set: edited_rules,
            before_graph,
            after_graph,
            before_layout,
            zoom,
            pan,
        },
    );
}
