//! Rule graph node editing inspector UI.
//!
//! This module provides the UI for editing rule graph nodes in the inspector panel.
//!
//! # Module Structure
//!
//! - `context`: Editor context and helper structs
//! - `summaries`: Summary generation for node display
//! - `shared_editors`: Common editor widgets (target, key, etc.)
//! - `trigger_editor`: Trigger node editing
//! - `condition_editor`: Condition node editing
//! - `action_editor`: Action node editing
//! - `connections`: Connection management UI

mod action_editor;
mod condition_editor;
mod connections;
mod context;
mod shared_editors;
mod summaries;
mod trigger_editor;

use super::*;

use context::{NodeEditParams, NodeEditorContext};

impl InspectorSystem {
    pub(in super::super) fn render_selected_rule_graph_node_editor(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
        scene_name: &str,
        node_key: &str,
        config: Option<&EditorConfig>,
    ) -> bool {
        let Some((mut graph, ctx)) =
            NodeEditorContext::new(ui_state, scene_name, node_key, config)
        else {
            ui.label("Scene not found.");
            return false;
        };

        let Some(node_id) = graph.node_id_for_stable_key(node_key) else {
            ui.colored_label(
                egui::Color32::from_rgb(255, 210, 80),
                "Selected node no longer exists.",
            );
            return false;
        };

        let Some(node_kind) = graph.nodes.iter().find(|n| n.id == node_id).map(|n| n.kind.clone())
        else {
            ui.colored_label(
                egui::Color32::from_rgb(255, 120, 120),
                "Failed to resolve selected node.",
            );
            return false;
        };

        let params = NodeEditParams {
            scene_name,
            node_key,
            node_id,
        };
        let mut graph_mutated =
            Self::render_node_kind_editor(ui, ui_state, &params, &node_kind, &mut graph, &ctx);

        ui.separator();
        let (outgoing_ids, incoming_ids) = Self::collect_connection_ids(&graph, node_id);
        let connectable_to =
            Self::build_connectable_nodes(&graph, &ctx.node_badges, node_id, &outgoing_ids, false);
        let connectable_from =
            Self::build_connectable_nodes(&graph, &ctx.node_badges, node_id, &incoming_ids, true);

        let action_result = Self::render_node_action_buttons(
            ui,
            scene_name,
            node_id,
            &mut graph,
            &connectable_from,
            &connectable_to,
        );

        ui.separator();
        let pending_disconnect =
            Self::render_connections_list(ui, &graph, &ctx.node_badges, node_id);
        let (ops_mutated, ops_error) =
            Self::process_pending_operations(&mut graph, node_id, &action_result, pending_disconnect);
        graph_mutated |= ops_mutated;

        if let Some(message) = ops_error {
            ui.colored_label(egui::Color32::from_rgb(255, 120, 120), message);
        }

        if !graph_mutated {
            return false;
        }

        Self::commit_graph_changes(ui, ui_state, scene_name, graph, &ctx)
    }

    fn render_node_kind_editor(
        ui: &mut egui::Ui,
        ui_state: &EditorUI,
        params: &NodeEditParams,
        node_kind: &RuleGraphNodeKind,
        graph: &mut RuleGraph,
        ctx: &NodeEditorContext,
    ) -> bool {
        match node_kind {
            RuleGraphNodeKind::Trigger(trigger) => {
                ui.label("Trigger");
                Self::edit_trigger_node(ui, ui_state, params, *trigger, graph, ctx)
            }
            RuleGraphNodeKind::Condition(condition) => {
                ui.label("Condition");
                Self::edit_condition_node(ui, params, condition, graph)
            }
            RuleGraphNodeKind::Action(action) => {
                ui.label("Action");
                Self::edit_action_node(ui, ui_state, params, action, graph, ctx)
            }
        }
    }

    fn edit_trigger_node(
        ui: &mut egui::Ui,
        ui_state: &EditorUI,
        params: &NodeEditParams,
        trigger: toki_core::rules::RuleTrigger,
        graph: &mut RuleGraph,
        ctx: &NodeEditorContext,
    ) -> bool {
        let mut edited_trigger = trigger;
        let map_size = Self::extract_map_size(ui_state, ctx.scene_index);
        let changed = Self::render_rule_graph_trigger_editor(
            ui,
            params.scene_name,
            params.node_key,
            &mut edited_trigger,
            map_size,
        );

        if changed && edited_trigger != trigger {
            if let Err(error) = graph.set_trigger_for_chain(params.node_id, edited_trigger) {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 120, 120),
                    format!("Failed to update trigger: {:?}", error),
                );
                return false;
            }
            return true;
        }
        false
    }

    fn edit_condition_node(
        ui: &mut egui::Ui,
        params: &NodeEditParams,
        condition: &toki_core::rules::RuleCondition,
        graph: &mut RuleGraph,
    ) -> bool {
        let mut edited_condition = condition.clone();
        let changed = Self::render_rule_graph_condition_editor(
            ui,
            params.scene_name,
            params.node_key,
            &mut edited_condition,
        );

        if changed && edited_condition != *condition {
            if let Err(error) = graph.set_condition_for_node(params.node_id, edited_condition) {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 120, 120),
                    format!("Failed to update condition: {:?}", error),
                );
                return false;
            }
            return true;
        }
        false
    }

    fn edit_action_node(
        ui: &mut egui::Ui,
        ui_state: &EditorUI,
        params: &NodeEditParams,
        action: &toki_core::rules::RuleAction,
        graph: &mut RuleGraph,
        ctx: &NodeEditorContext,
    ) -> bool {
        let mut edited_action = action.clone();
        let changed = Self::render_rule_graph_action_editor(
            ui,
            params.scene_name,
            params.node_key,
            &mut edited_action,
            &ctx.validation_issues,
            &ctx.audio_choices,
            &ui_state.scenes,
        );

        if changed && edited_action != *action {
            if let Err(error) = graph.set_action_for_node(params.node_id, edited_action) {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 120, 120),
                    format!("Failed to update action: {:?}", error),
                );
                return false;
            }
            return true;
        }
        false
    }

    fn extract_map_size(ui_state: &EditorUI, scene_index: usize) -> Option<(u32, u32)> {
        let scene = ui_state.scenes.get(scene_index)?;
        let map_name = scene.maps.first()?;
        if ui_state.map.active_map.as_ref() != Some(map_name) {
            return None;
        }
        ui_state
            .map
            .draft
            .as_ref()
            .map(|draft| (draft.tilemap.size.x, draft.tilemap.size.y))
            .or_else(|| {
                ui_state
                    .map
                    .pending_tilemap_sync
                    .as_ref()
                    .map(|tm| (tm.size.x, tm.size.y))
            })
    }

    fn commit_graph_changes(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
        scene_name: &str,
        graph: RuleGraph,
        ctx: &NodeEditorContext,
    ) -> bool {
        match graph.to_rule_set() {
            Ok(updated_rules) => {
                let (zoom, pan) = ui_state.graph_view_for_scene(scene_name);
                ui_state.execute_scene_rules_graph_command(
                    scene_name,
                    SceneRulesGraphCommandData {
                        before_rule_set: ctx.before_rules.clone(),
                        after_rule_set: updated_rules,
                        before_graph: ctx.before_graph.clone(),
                        after_graph: graph,
                        before_layout: ctx.before_layout.clone(),
                        zoom,
                        pan,
                    },
                )
            }
            Err(error) => {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 120, 120),
                    format!("Failed to rebuild rule set from graph: {:?}", error),
                );
                false
            }
        }
    }
}
