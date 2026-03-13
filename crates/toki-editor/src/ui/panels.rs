use super::editor_ui::{CenterPanelTab, Selection};
use super::interactions::{CameraInteraction, PlacementInteraction, SelectionInteraction};
use super::rule_graph::{RuleGraph, RuleGraphNodeKind};
use crate::config::EditorConfig;
use crate::scene::SceneViewport;
use std::collections::{HashMap, HashSet};
use toki_core::animation::AnimationState;
use toki_core::rules::{
    RuleAction, RuleCondition, RuleKey, RuleSoundChannel, RuleSpawnEntityType, RuleTarget,
    RuleTrigger,
};

/// Handles panel rendering for the editor (viewport and log panels)
pub struct PanelSystem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphConditionKind {
    Always,
    TargetExists,
    KeyHeld,
    EntityActive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphActionKind {
    PlaySound,
    PlayMusic,
    PlayAnimation,
    SetVelocity,
    Spawn,
    DestroySelf,
    SwitchScene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphTriggerKind {
    Start,
    Update,
    PlayerMove,
    Key,
    Collision,
    Trigger,
}

impl PanelSystem {
    /// Renders the main scene viewport panel in the center of the screen
    pub fn render_viewport(
        ui_state: &mut super::EditorUI,
        ctx: &egui::Context,
        scene_viewport: Option<&mut SceneViewport>,
        config: Option<&mut EditorConfig>,
        renderer: Option<&mut egui_wgpu::Renderer>,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ui_state.center_panel_tab,
                    CenterPanelTab::SceneViewport,
                    "Scene Viewport",
                );
                ui.selectable_value(
                    &mut ui_state.center_panel_tab,
                    CenterPanelTab::SceneGraph,
                    "Scene Graph",
                );
                ui.selectable_value(
                    &mut ui_state.center_panel_tab,
                    CenterPanelTab::SceneRules,
                    "Scene Rules",
                );
            });
            ui.separator();

            if ui_state.center_panel_tab == CenterPanelTab::SceneGraph {
                Self::render_scene_graph(ui, ui_state, false);
                return;
            }

            if ui_state.center_panel_tab == CenterPanelTab::SceneRules {
                Self::render_scene_graph(ui, ui_state, true);
                return;
            }

            // Update and render the scene viewport
            if let Some(viewport) = scene_viewport {
                // Collect stats before updating viewport to avoid borrowing conflicts
                let entity_count = viewport
                    .scene_manager()
                    .game_state()
                    .entity_manager()
                    .active_entities()
                    .len();
                let selected_entity = viewport.selected_entity();

                // Update the viewport systems
                if let Err(e) = viewport.update() {
                    tracing::error!("Scene viewport update error: {e}");
                }

                // Handle viewport interactions
                let available_size = ui.available_size();
                let (rect, response) = ui.allocate_exact_size(
                    available_size,
                    egui::Sense::click_and_drag().union(egui::Sense::hover()),
                );

                // Safety reset: don't keep entities hidden when no move drag is active.
                if !ui_state.is_entity_move_drag_active() {
                    viewport.clear_suppressed_entity_rendering();
                }

                // Start entity move drag if dragging began over an entity.
                if response.drag_started() {
                    if let Some(drag_start_pos) = response.interact_pointer_pos() {
                        SelectionInteraction::handle_drag_start(
                            ui_state,
                            viewport,
                            drag_start_pos,
                            rect,
                            config.as_deref(),
                        );
                    }
                }

                // Handle drag release for entity move operations.
                if response.drag_stopped() {
                    let drop_pos = response
                        .interact_pointer_pos()
                        .or_else(|| response.hover_pos());
                    SelectionInteraction::handle_drag_release(ui_state, viewport, drop_pos, rect);
                }

                // Handle camera panning with drag (disabled while moving an entity).
                if !ui_state.is_entity_move_drag_active() {
                    CameraInteraction::handle_drag(viewport, &response, config.as_deref());
                } else {
                    viewport.stop_camera_drag();
                }

                // Handle placement mode hover logic
                PlacementInteraction::handle_hover(
                    ui_state,
                    viewport,
                    &response,
                    rect,
                    config.as_deref(),
                );

                // Handle viewport clicks (entity placement or selection)
                if response.clicked() {
                    if let Some(click_pos) = response.hover_pos() {
                        // Check if we're in placement mode
                        if ui_state.is_in_placement_mode() {
                            PlacementInteraction::handle_click(
                                ui_state,
                                viewport,
                                click_pos,
                                rect,
                                config.as_deref(),
                            );
                        } else {
                            // Normal entity selection
                            SelectionInteraction::handle_click(ui_state, viewport, click_pos, rect);
                        }
                    }
                }

                // Render the scene content
                let project_path = config.as_deref().and_then(|c| c.current_project_path());
                viewport.render(ui, rect, project_path.map(|p| p.as_path()), renderer);

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("📊 Stats:");
                        ui.label(format!(
                            "Entities: {} | Selected: {:?}",
                            entity_count, selected_entity
                        ));
                        ui.label("Press F1/F2 to toggle panels");
                    });
                });
            } else {
                // Show placeholder when no viewport
                let available_size = ui.available_size();
                ui.allocate_response(available_size, egui::Sense::click())
                    .on_hover_text("Scene viewport not initialized");
            }
        });
    }

    fn render_scene_graph(
        ui: &mut egui::Ui,
        ui_state: &mut super::EditorUI,
        show_scene_rules: bool,
    ) {
        enum GraphCommand {
            AddTrigger,
            ResetLayout,
            SetTrigger(u64, RuleTrigger),
            AppendCondition(u64),
            AppendAction(u64),
            SetCondition(u64, RuleCondition),
            SetAction(u64, RuleAction),
            SetNodePosition(u64, [f32; 2]),
            RemoveNode(u64),
            Connect(u64, u64),
            Disconnect(u64, u64),
        }

        if show_scene_rules {
            ui.heading("Active Scene Rules");
        } else {
            ui.heading("Active Scene Graph");
        }
        ui.separator();

        let Some(active_scene_name) = ui_state.active_scene.clone() else {
            ui.label("No active scene selected.");
            return;
        };

        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|scene| scene.name == active_scene_name)
        else {
            ui.label(format!(
                "Active scene '{}' is not loaded.",
                active_scene_name
            ));
            return;
        };

        let mut connect_from = ui_state.graph_connect_from_node;
        let mut connect_to = ui_state.graph_connect_to_node;
        let (mut graph_zoom, mut graph_pan) = ui_state.graph_view_for_scene(&active_scene_name);
        let mut scene_changed = false;
        let mut layout_changed = false;
        let mut operation_error: Option<String> = None;
        let mut selected_graph_node: Option<u64> = None;

        {
            let scene_rules = ui_state.scenes[scene_index].rules.clone();
            let mut graph = RuleGraph::from_rule_set(&scene_rules);
            let mut pending_command: Option<GraphCommand> = None;

            let node_ids = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
            for node_id in node_ids {
                let Some(node_key) = graph.stable_node_key(node_id) else {
                    continue;
                };
                let Some(position) = ui_state.graph_layout_position(&active_scene_name, &node_key)
                else {
                    continue;
                };
                let _ = graph.set_node_position(node_id, position);
            }

            if let Some(Selection::RuleGraphNode {
                scene_name,
                node_key,
            }) = &ui_state.selection
            {
                if scene_name == &active_scene_name {
                    selected_graph_node = graph.node_id_for_stable_key(node_key);
                }
            }
            let node_badges = Self::rule_graph_node_badges(&graph);
            let selected_trigger_for_add = selected_graph_node
                .and_then(|node_id| graph.trigger_node_for_node(node_id))
                .or_else(|| graph.chains.first().map(|chain| chain.trigger_node_id));

            if !show_scene_rules {
                ui.horizontal(|ui| {
                    if !ui.ctx().wants_keyboard_input() {
                        if ui.input(|input| {
                            input.key_pressed(egui::Key::Plus)
                                || input.key_pressed(egui::Key::Equals)
                        }) {
                            graph_zoom = (graph_zoom * 1.1).clamp(0.4, 4.0);
                        }
                        if ui.input(|input| input.key_pressed(egui::Key::Minus)) {
                            graph_zoom = (graph_zoom / 1.1).clamp(0.4, 4.0);
                        }
                    }
                    ui.label(format!("Zoom: {:.0}%", graph_zoom * 100.0));
                    ui.label("Tip: Drag Empty Space To Pan");
                    if ui.button("➕ Add Trigger").clicked() {
                        pending_command = Some(GraphCommand::AddTrigger);
                    }
                    if ui.button("➕ Add Condition").clicked() {
                        if let Some(trigger_node_id) = selected_trigger_for_add {
                            pending_command = Some(GraphCommand::AppendCondition(trigger_node_id));
                        } else {
                            operation_error = Some(
                                "Select a node first to determine where to append condition."
                                    .to_string(),
                            );
                        }
                    }
                    if ui.button("➕ Add Action").clicked() {
                        if let Some(trigger_node_id) = selected_trigger_for_add {
                            pending_command = Some(GraphCommand::AppendAction(trigger_node_id));
                        } else {
                            operation_error = Some(
                                "Select a node first to determine where to append action."
                                    .to_string(),
                            );
                        }
                    }
                    if ui.button("↺ Reset Auto Layout").clicked() {
                        pending_command = Some(GraphCommand::ResetLayout);
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    if ui.button("➕ Add Trigger").clicked() {
                        pending_command = Some(GraphCommand::AddTrigger);
                    }
                    if ui.button("➕ Add Condition").clicked() {
                        if let Some(trigger_node_id) = selected_trigger_for_add {
                            pending_command = Some(GraphCommand::AppendCondition(trigger_node_id));
                        } else {
                            operation_error = Some(
                                "Select a node first to determine where to append condition."
                                    .to_string(),
                            );
                        }
                    }
                    if ui.button("➕ Add Action").clicked() {
                        if let Some(trigger_node_id) = selected_trigger_for_add {
                            pending_command = Some(GraphCommand::AppendAction(trigger_node_id));
                        } else {
                            operation_error = Some(
                                "Select a node first to determine where to append action."
                                    .to_string(),
                            );
                        }
                    }
                });
            }

            if connect_from.is_some_and(|id| !graph.nodes.iter().any(|node| node.id == id)) {
                connect_from = None;
            }
            if connect_to.is_some_and(|id| !graph.nodes.iter().any(|node| node.id == id)) {
                connect_to = None;
            }

            if !show_scene_rules {
                ui.horizontal(|ui| {
                    ui.label("Connect:");

                    egui::ComboBox::from_id_salt(format!("graph_connect_from_{}", scene_index))
                        .selected_text(
                            connect_from
                                .and_then(|id| {
                                    Self::rule_graph_node_label(&graph, &node_badges, id)
                                })
                                .unwrap_or_else(|| "<source>".to_string()),
                        )
                        .show_ui(ui, |ui| {
                            for node in &graph.nodes {
                                ui.selectable_value(
                                    &mut connect_from,
                                    Some(node.id),
                                    Self::rule_graph_node_label(&graph, &node_badges, node.id)
                                        .unwrap_or_else(|| format!("{}", node.id)),
                                );
                            }
                        });

                    egui::ComboBox::from_id_salt(format!("graph_connect_to_{}", scene_index))
                        .selected_text(
                            connect_to
                                .and_then(|id| {
                                    Self::rule_graph_node_label(&graph, &node_badges, id)
                                })
                                .unwrap_or_else(|| "<target>".to_string()),
                        )
                        .show_ui(ui, |ui| {
                            for node in &graph.nodes {
                                ui.selectable_value(
                                    &mut connect_to,
                                    Some(node.id),
                                    Self::rule_graph_node_label(&graph, &node_badges, node.id)
                                        .unwrap_or_else(|| format!("{}", node.id)),
                                );
                            }
                        });

                    if ui.button("Connect").clicked() {
                        if let (Some(from), Some(to)) = (connect_from, connect_to) {
                            pending_command = Some(GraphCommand::Connect(from, to));
                        }
                    }
                });
            }

            ui.label(format!(
                "Chains: {} | Nodes: {} | Edges: {}",
                graph.chains.len(),
                graph.nodes.len(),
                graph.edges.len()
            ));
            if !show_scene_rules {
                if pending_command.is_none() {
                    let (moved_node, clicked_node) = Self::render_graph_canvas(
                        ui,
                        &graph,
                        &node_badges,
                        graph_zoom,
                        &mut graph_pan,
                    );
                    if let Some((node_id, position)) = moved_node {
                        pending_command = Some(GraphCommand::SetNodePosition(node_id, position));
                    }
                    if let Some(node_id) = clicked_node {
                        selected_graph_node = Some(node_id);
                    }
                }

                if graph.nodes.is_empty() {
                    ui.label("No rules in active scene. Add a rule chain to start authoring.");
                }
            }

            if show_scene_rules {
                let node_by_id = graph
                    .nodes
                    .iter()
                    .map(|node| (node.id, node))
                    .collect::<HashMap<_, _>>();
                let mut outgoing = HashMap::<u64, Vec<u64>>::new();
                for edge in &graph.edges {
                    outgoing.entry(edge.from).or_default().push(edge.to);
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (rule_index, chain) in graph.chains.iter().enumerate() {
                        ui.push_id(("graph_chain", chain.trigger_node_id), |ui| {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.strong(format!("Rule {}: {}", rule_index + 1, chain.rule_id));
                                    if !chain.enabled {
                                        ui.label("(disabled)");
                                    }
                                    if ui.small_button("➕ Condition").clicked() {
                                        pending_command =
                                            Some(GraphCommand::AppendCondition(chain.trigger_node_id));
                                    }
                                    if ui.small_button("➕ Action").clicked() {
                                        pending_command =
                                            Some(GraphCommand::AppendAction(chain.trigger_node_id));
                                    }
                                    if ui.small_button("🗑 Rule").clicked() {
                                        pending_command =
                                            Some(GraphCommand::RemoveNode(chain.trigger_node_id));
                                    }
                                });

                                let sequence = match graph.chain_node_sequence(chain.trigger_node_id) {
                                    Ok(sequence) => sequence,
                                    Err(error) => {
                                        ui.colored_label(
                                            egui::Color32::from_rgb(255, 120, 120),
                                            format!("Invalid chain: {:?}", error),
                                        );
                                        Vec::new()
                                    }
                                };
                                let sequence_set = sequence.iter().copied().collect::<HashSet<_>>();

                                for node_id in sequence {
                                    let Some(node) = node_by_id.get(&node_id) else {
                                        continue;
                                    };
                                    ui.push_id(("graph_node", node_id), |ui| {
                                        ui.horizontal(|ui| match &node.kind {
                                            RuleGraphNodeKind::Trigger(trigger) => {
                                                let badge = node_badges
                                                    .get(&node_id)
                                                    .cloned()
                                                    .unwrap_or_else(|| "T?".to_string());
                                                let node_label = format!(
                                                    "{} Trigger: {}",
                                                    badge,
                                                    Self::trigger_summary(*trigger)
                                                );
                                                let is_selected = selected_graph_node == Some(node_id);
                                                if ui.selectable_label(is_selected, node_label).clicked() {
                                                    selected_graph_node = Some(node_id);
                                                }
                                                let mut trigger_value = *trigger;
                                                let mut kind = Self::graph_trigger_kind(*trigger);
                                                egui::ComboBox::from_id_salt("graph_trigger_kind")
                                                    .selected_text(Self::graph_trigger_kind_label(kind))
                                                    .show_ui(ui, |ui| {
                                                        for candidate in [
                                                            GraphTriggerKind::Start,
                                                            GraphTriggerKind::Update,
                                                            GraphTriggerKind::PlayerMove,
                                                            GraphTriggerKind::Key,
                                                            GraphTriggerKind::Collision,
                                                            GraphTriggerKind::Trigger,
                                                        ] {
                                                            ui.selectable_value(
                                                                &mut kind,
                                                                candidate,
                                                                Self::graph_trigger_kind_label(candidate),
                                                            );
                                                        }
                                                    });
                                                if kind != Self::graph_trigger_kind(*trigger) {
                                                    trigger_value = Self::graph_default_trigger(kind);
                                                }
                                                if let RuleTrigger::OnKey { key } = &mut trigger_value {
                                                    let _ =
                                                        Self::edit_rule_key(ui, key, "graph_trigger_key");
                                                }
                                                if trigger_value != *trigger {
                                                    pending_command =
                                                        Some(GraphCommand::SetTrigger(node_id, trigger_value));
                                                }
                                            }
                                            RuleGraphNodeKind::Condition(condition) => {
                                                let badge = node_badges
                                                    .get(&node_id)
                                                    .cloned()
                                                    .unwrap_or_else(|| "C?".to_string());
                                                let node_label = format!(
                                                    "{} Condition: {}",
                                                    badge,
                                                    Self::condition_summary(*condition)
                                                );
                                                let is_selected = selected_graph_node == Some(node_id);
                                                if ui.selectable_label(is_selected, node_label).clicked() {
                                                    selected_graph_node = Some(node_id);
                                                }
                                                let mut kind = Self::graph_condition_kind(*condition);
                                                egui::ComboBox::from_id_salt("graph_condition_kind")
                                                    .selected_text(Self::graph_condition_kind_label(kind))
                                                    .show_ui(ui, |ui| {
                                                        for candidate in [
                                                            GraphConditionKind::Always,
                                                            GraphConditionKind::TargetExists,
                                                            GraphConditionKind::KeyHeld,
                                                            GraphConditionKind::EntityActive,
                                                        ] {
                                                            ui.selectable_value(
                                                                &mut kind,
                                                                candidate,
                                                                Self::graph_condition_kind_label(candidate),
                                                            );
                                                        }
                                                    });
                                                let mut edited_condition = *condition;
                                                if kind != Self::graph_condition_kind(*condition) {
                                                    edited_condition =
                                                        Self::graph_default_condition(kind);
                                                }
                                                let payload_changed =
                                                    Self::edit_graph_condition_payload(
                                                        ui,
                                                        &mut edited_condition,
                                                    );
                                                if edited_condition != *condition || payload_changed {
                                                    pending_command = Some(GraphCommand::SetCondition(
                                                        node_id,
                                                        edited_condition,
                                                    ));
                                                }
                                                if ui.small_button("✕").clicked() {
                                                    pending_command =
                                                        Some(GraphCommand::RemoveNode(node_id));
                                                }
                                            }
                                            RuleGraphNodeKind::Action(action) => {
                                                let badge = node_badges
                                                    .get(&node_id)
                                                    .cloned()
                                                    .unwrap_or_else(|| "A?".to_string());
                                                let node_label =
                                                    format!(
                                                        "{} Action: {}",
                                                        badge,
                                                        Self::action_summary(action)
                                                    );
                                                let is_selected = selected_graph_node == Some(node_id);
                                                if ui.selectable_label(is_selected, node_label).clicked() {
                                                    selected_graph_node = Some(node_id);
                                                }
                                                let mut kind = Self::graph_action_kind(action);
                                                egui::ComboBox::from_id_salt("graph_action_kind")
                                                    .selected_text(Self::graph_action_kind_label(kind))
                                                    .show_ui(ui, |ui| {
                                                        for candidate in [
                                                            GraphActionKind::PlaySound,
                                                            GraphActionKind::PlayMusic,
                                                            GraphActionKind::PlayAnimation,
                                                            GraphActionKind::SetVelocity,
                                                            GraphActionKind::Spawn,
                                                            GraphActionKind::DestroySelf,
                                                            GraphActionKind::SwitchScene,
                                                        ] {
                                                            ui.selectable_value(
                                                                &mut kind,
                                                                candidate,
                                                                Self::graph_action_kind_label(candidate),
                                                            );
                                                        }
                                                    });
                                                let mut edited_action = action.clone();
                                                if kind != Self::graph_action_kind(action) {
                                                    edited_action = Self::graph_default_action(kind);
                                                }
                                                let payload_changed = Self::edit_graph_action_payload(
                                                    ui,
                                                    &mut edited_action,
                                                );
                                                if edited_action != *action || payload_changed {
                                                    pending_command = Some(GraphCommand::SetAction(
                                                        node_id,
                                                        edited_action,
                                                    ));
                                                }
                                                if ui.small_button("✕").clicked() {
                                                    pending_command =
                                                        Some(GraphCommand::RemoveNode(node_id));
                                                }
                                            }
                                        });
                                    });
                                }

                                let edge_list = graph
                                    .edges
                                    .iter()
                                    .filter(|edge| {
                                        sequence_set.contains(&edge.from)
                                            || sequence_set.contains(&edge.to)
                                    })
                                    .copied()
                                    .collect::<Vec<_>>();

                                if !edge_list.is_empty() {
                                    egui::CollapsingHeader::new("Edges")
                                        .id_salt(("graph_edges", chain.trigger_node_id))
                                        .show(ui, |ui| {
                                            for edge in edge_list {
                                                ui.horizontal(|ui| {
                                                    let from_label = Self::rule_graph_node_label(
                                                        &graph,
                                                        &node_badges,
                                                        edge.from,
                                                    )
                                                    .unwrap_or_else(|| format!("node {}", edge.from));
                                                    let to_label = Self::rule_graph_node_label(
                                                        &graph,
                                                        &node_badges,
                                                        edge.to,
                                                    )
                                                    .unwrap_or_else(|| format!("node {}", edge.to));
                                                    ui.monospace(format!(
                                                        "{} -> {}",
                                                        from_label, to_label
                                                    ));
                                                    if ui.small_button("Disconnect").clicked() {
                                                        pending_command = Some(
                                                            GraphCommand::Disconnect(edge.from, edge.to),
                                                        );
                                                    }
                                                });
                                            }
                                        });
                                }

                                if let Some(next_nodes) = outgoing.get(&chain.trigger_node_id) {
                                    if next_nodes.is_empty() {
                                        ui.colored_label(
                                            egui::Color32::from_rgb(255, 210, 80),
                                            "Trigger has no outgoing edge. Connect it to continue chain.",
                                        );
                                    }
                                }
                            });
                        });
                        ui.add_space(6.0);
                    }
                });
            }

            if let Some(command) = pending_command {
                let is_layout_command = matches!(command, GraphCommand::SetNodePosition(_, _));
                let is_reset_layout = matches!(command, GraphCommand::ResetLayout);
                let remembered_layout = Self::remember_graph_layout(&graph);
                let command_result = match command {
                    GraphCommand::AddTrigger => graph.add_trigger_chain().map(|_| ()),
                    GraphCommand::ResetLayout => {
                        let auto_layout_graph = RuleGraph::from_rule_set(&scene_rules);
                        let auto_node_badges = Self::rule_graph_node_badges(&auto_layout_graph);
                        let auto_positions = Self::compute_auto_layout_positions(
                            ui,
                            &auto_layout_graph,
                            &auto_node_badges,
                        );
                        auto_positions
                            .into_iter()
                            .try_for_each(|(node_id, position)| {
                                graph.set_node_position(node_id, position)
                            })
                    }
                    GraphCommand::AppendCondition(trigger_node_id) => {
                        graph.append_condition_to_chain(trigger_node_id, RuleCondition::Always)
                    }
                    GraphCommand::SetTrigger(trigger_node_id, trigger) => {
                        graph.set_trigger_for_chain(trigger_node_id, trigger)
                    }
                    GraphCommand::AppendAction(trigger_node_id) => graph.append_action_to_chain(
                        trigger_node_id,
                        RuleAction::PlaySound {
                            channel: RuleSoundChannel::Movement,
                            sound_id: "sfx_placeholder".to_string(),
                        },
                    ),
                    GraphCommand::SetCondition(node_id, condition) => {
                        graph.set_condition_for_node(node_id, condition)
                    }
                    GraphCommand::SetAction(node_id, action) => {
                        graph.set_action_for_node(node_id, action)
                    }
                    GraphCommand::SetNodePosition(node_id, position) => {
                        graph.set_node_position(node_id, position)
                    }
                    GraphCommand::RemoveNode(node_id) => graph.remove_node(node_id),
                    GraphCommand::Connect(from, to) => graph.connect_nodes(from, to),
                    GraphCommand::Disconnect(from, to) => {
                        graph.disconnect_nodes(from, to);
                        Ok(())
                    }
                };

                match command_result {
                    Ok(()) => {
                        if is_reset_layout {
                            // Keep a visible border gap when snapping to auto layout.
                            graph_pan = [16.0, 16.0];
                            Self::enforce_graph_border_gap(&graph, graph_zoom, &mut graph_pan);
                        }
                        if !is_layout_command && !is_reset_layout {
                            Self::restore_graph_layout(&mut graph, &remembered_layout);
                        }
                        if is_layout_command || is_reset_layout {
                            layout_changed = true;
                        } else {
                            scene_changed = true;
                        }
                    }
                    Err(error) => {
                        operation_error = Some(format!("Graph edit failed: {:?}", error));
                    }
                }
            }

            if scene_changed {
                match graph.to_rule_set() {
                    Ok(rule_set) => {
                        ui_state.scenes[scene_index].rules = rule_set;
                    }
                    Err(error) => {
                        scene_changed = false;
                        operation_error = Some(format!(
                            "Graph is invalid and could not be saved: {:?}",
                            error
                        ));
                    }
                }
            }

            if scene_changed || layout_changed {
                for node in &graph.nodes {
                    let Some(node_key) = graph.stable_node_key(node.id) else {
                        continue;
                    };
                    ui_state.set_graph_layout_position(
                        &active_scene_name,
                        &node_key,
                        node.position,
                    );
                }
            }

            if let Some(node_id) = selected_graph_node {
                if let Some(node_key) = graph.stable_node_key(node_id) {
                    ui_state.set_selection(Selection::RuleGraphNode {
                        scene_name: active_scene_name.clone(),
                        node_key,
                    });
                }
            }
        }

        ui_state.graph_connect_from_node = connect_from;
        ui_state.graph_connect_to_node = connect_to;
        ui_state.graph_canvas_zoom = graph_zoom;
        ui_state.graph_canvas_pan = graph_pan;
        ui_state.set_graph_view_for_scene(&active_scene_name, graph_zoom, graph_pan);
        if scene_changed {
            ui_state.scene_content_changed = true;
        }
        if let Some(error) = operation_error {
            ui.colored_label(egui::Color32::from_rgb(255, 120, 120), error);
        }
    }

    fn rule_graph_node_label(
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        node_id: u64,
    ) -> Option<String> {
        let node = graph.nodes.iter().find(|node| node.id == node_id)?;
        let badge = node_badges
            .get(&node_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        Some(format!(
            "{}: {}",
            badge,
            Self::rule_graph_node_kind_compact_label(&node.kind)
        ))
    }

    fn rule_graph_node_badges(graph: &RuleGraph) -> HashMap<u64, String> {
        let mut node_ids = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        node_ids.sort_unstable();

        let mut trigger_index = 0usize;
        let mut condition_index = 0usize;
        let mut action_index = 0usize;
        let mut badges = HashMap::new();
        for node_id in node_ids {
            let Some(node) = graph.nodes.iter().find(|candidate| candidate.id == node_id) else {
                continue;
            };
            let badge = match node.kind {
                RuleGraphNodeKind::Trigger(_) => {
                    trigger_index += 1;
                    format!("T{}", trigger_index)
                }
                RuleGraphNodeKind::Condition(_) => {
                    condition_index += 1;
                    format!("C{}", condition_index)
                }
                RuleGraphNodeKind::Action(_) => {
                    action_index += 1;
                    format!("A{}", action_index)
                }
            };
            badges.insert(node_id, badge);
        }
        badges
    }

    fn compute_auto_layout_positions(
        ui: &egui::Ui,
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
    ) -> HashMap<u64, [f32; 2]> {
        let node_by_id = graph
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<HashMap<_, _>>();

        let mut positions = HashMap::<u64, [f32; 2]>::new();
        let mut y = RuleGraph::auto_layout_start_y();

        for chain in &graph.chains {
            let Ok(sequence) = graph.chain_node_sequence(chain.trigger_node_id) else {
                continue;
            };
            if sequence.is_empty() {
                continue;
            }

            let mut node_sizes = Vec::<egui::Vec2>::with_capacity(sequence.len());
            for node_id in &sequence {
                let Some(node) = node_by_id.get(node_id) else {
                    node_sizes.push(egui::vec2(
                        RuleGraph::auto_layout_node_width(),
                        RuleGraph::auto_layout_node_height(),
                    ));
                    continue;
                };
                let badge = node_badges
                    .get(node_id)
                    .cloned()
                    .unwrap_or_else(|| "?".to_string());
                let label = format!(
                    "{}: {}",
                    badge,
                    Self::rule_graph_node_kind_compact_label(&node.kind)
                );
                node_sizes.push(Self::graph_node_size_for_label(ui, &label, 1.0));
            }

            let row_height = node_sizes
                .iter()
                .map(|size| size.y)
                .fold(0.0_f32, f32::max)
                .max(RuleGraph::auto_layout_node_height());

            let mut x = RuleGraph::auto_layout_start_x();
            for (index, node_id) in sequence.iter().enumerate() {
                positions.insert(*node_id, [x, y]);
                if let Some(next_size) = node_sizes.get(index + 1) {
                    let current_size = node_sizes[index];
                    x += current_size.x * 0.5
                        + RuleGraph::auto_layout_horizontal_edge_spacing()
                        + next_size.x * 0.5;
                }
            }

            y += row_height + RuleGraph::auto_layout_vertical_edge_spacing();
        }

        positions
    }

    fn render_graph_canvas(
        ui: &mut egui::Ui,
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        graph_zoom: f32,
        graph_pan: &mut [f32; 2],
    ) -> (Option<(u64, [f32; 2])>, Option<u64>) {
        let desired_size = egui::vec2(ui.available_width(), ui.available_height().max(220.0));
        let (rect, canvas_response) =
            ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, 6.0, egui::Color32::from_rgb(20, 24, 30));
        painter.rect_stroke(
            rect,
            6.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(70)),
            egui::StrokeKind::Inside,
        );

        if graph.nodes.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No Graph Nodes",
                egui::TextStyle::Body.resolve(ui.style()),
                egui::Color32::from_gray(170),
            );
            return (None, None);
        }

        let scale = graph_zoom.max(0.01);

        let to_canvas = |position: [f32; 2]| -> egui::Pos2 {
            egui::pos2(
                rect.left() + graph_pan[0] + position[0] * scale,
                rect.top() + graph_pan[1] + position[1] * scale,
            )
        };

        let node_positions = graph
            .nodes
            .iter()
            .map(|node| (node.id, to_canvas(node.position)))
            .collect::<HashMap<_, _>>();

        for edge in &graph.edges {
            let Some(from_pos) = node_positions.get(&edge.from).copied() else {
                continue;
            };
            let Some(to_pos) = node_positions.get(&edge.to).copied() else {
                continue;
            };
            painter.line_segment(
                [from_pos, to_pos],
                egui::Stroke::new(
                    Self::graph_edge_stroke_width(scale),
                    egui::Color32::from_rgb(130, 150, 185),
                ),
            );
        }

        let mut moved_node = None;
        let mut clicked_node = None;
        let mut any_node_dragged = false;
        let node_corner_radius = (6.0 * scale).clamp(2.0, 18.0);
        let node_stroke_width = (1.0 * scale).clamp(0.7, 2.5);
        let node_font_size = Self::graph_node_font_size(scale);
        for node in &graph.nodes {
            let Some(center) = node_positions.get(&node.id).copied() else {
                continue;
            };
            let badge = node_badges
                .get(&node.id)
                .cloned()
                .unwrap_or_else(|| "?".to_string());
            let label = format!(
                "{}: {}",
                badge,
                Self::rule_graph_node_kind_compact_label(&node.kind)
            );
            let node_size = Self::graph_node_size_for_label(ui, &label, scale);
            let (fill, stroke) = match node.kind {
                RuleGraphNodeKind::Trigger(_) => (
                    egui::Color32::from_rgb(45, 122, 199),
                    egui::Color32::from_rgb(140, 190, 245),
                ),
                RuleGraphNodeKind::Condition(_) => (
                    egui::Color32::from_rgb(139, 92, 46),
                    egui::Color32::from_rgb(214, 158, 106),
                ),
                RuleGraphNodeKind::Action(_) => (
                    egui::Color32::from_rgb(58, 140, 82),
                    egui::Color32::from_rgb(133, 208, 154),
                ),
            };
            let node_rect = egui::Rect::from_center_size(center, node_size);
            let response = ui.interact(
                node_rect,
                ui.make_persistent_id(("graph_canvas_node", node.id)),
                egui::Sense::click_and_drag(),
            );
            if response.clicked() {
                clicked_node = Some(node.id);
            }
            if response.dragged() {
                any_node_dragged = true;
                let delta = ui.ctx().input(|input| input.pointer.delta());
                if delta != egui::Vec2::ZERO && scale > 0.0 {
                    moved_node = Some((
                        node.id,
                        [
                            node.position[0] + (delta.x / scale),
                            node.position[1] + (delta.y / scale),
                        ],
                    ));
                }
            }
            let draw_fill = if response.dragged() {
                fill.gamma_multiply(1.2)
            } else {
                fill
            };
            painter.rect_filled(node_rect, node_corner_radius, draw_fill);
            painter.rect_stroke(
                node_rect,
                node_corner_radius,
                egui::Stroke::new(node_stroke_width, stroke),
                egui::StrokeKind::Inside,
            );
            painter.text(
                node_rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(node_font_size),
                egui::Color32::WHITE,
            );
        }

        if !any_node_dragged && canvas_response.dragged() {
            let delta = ui.ctx().input(|input| input.pointer.delta());
            if delta != egui::Vec2::ZERO {
                graph_pan[0] += delta.x;
                graph_pan[1] += delta.y;
            }
        }

        (moved_node, clicked_node)
    }

    fn enforce_graph_border_gap(graph: &RuleGraph, graph_zoom: f32, graph_pan: &mut [f32; 2]) {
        let Some(min_x) = graph
            .nodes
            .iter()
            .map(|node| node.position[0])
            .min_by(|a, b| a.total_cmp(b))
        else {
            return;
        };
        let Some(min_y) = graph
            .nodes
            .iter()
            .map(|node| node.position[1])
            .min_by(|a, b| a.total_cmp(b))
        else {
            return;
        };

        let scale = graph_zoom.max(0.01);
        let node_size = Self::graph_node_max_size(scale);
        let border_gap = 10.0;
        let min_center_x = border_gap + node_size.x * 0.5;
        let min_center_y = border_gap + node_size.y * 0.5;
        let required_pan_x = min_center_x - (min_x * scale);
        let required_pan_y = min_center_y - (min_y * scale);

        if graph_pan[0] < required_pan_x {
            graph_pan[0] = required_pan_x;
        }
        if graph_pan[1] < required_pan_y {
            graph_pan[1] = required_pan_y;
        }
    }

    fn graph_node_max_size(scale: f32) -> egui::Vec2 {
        egui::vec2(
            (320.0 * scale).clamp(120.0, 860.0),
            (36.0 * scale).clamp(18.0, 96.0),
        )
    }

    fn graph_node_min_size(scale: f32) -> egui::Vec2 {
        egui::vec2(
            (120.0 * scale).clamp(80.0, 300.0),
            (20.0 * scale).clamp(14.0, 48.0),
        )
    }

    fn graph_node_size_for_label(ui: &egui::Ui, label: &str, scale: f32) -> egui::Vec2 {
        let font_size = Self::graph_node_font_size(scale);
        let font_id = egui::FontId::proportional(font_size);
        let text_size = ui
            .painter()
            .layout_no_wrap(label.to_string(), font_id, egui::Color32::WHITE)
            .size();
        let padding_x = (16.0 * scale).clamp(8.0, 36.0);
        let padding_y = (8.0 * scale).clamp(4.0, 24.0);
        let desired = egui::vec2(text_size.x + padding_x * 2.0, text_size.y + padding_y * 2.0);
        let min_size = Self::graph_node_min_size(scale);
        let max_size = Self::graph_node_max_size(scale);
        egui::vec2(
            desired.x.clamp(min_size.x, max_size.x),
            desired.y.clamp(min_size.y, max_size.y),
        )
    }

    fn graph_node_font_size(scale: f32) -> f32 {
        (11.0 * scale).clamp(7.0, 24.0)
    }

    fn graph_edge_stroke_width(scale: f32) -> f32 {
        (1.5 * scale).clamp(0.7, 4.0)
    }

    fn remember_graph_layout(graph: &RuleGraph) -> HashMap<String, [f32; 2]> {
        graph
            .nodes
            .iter()
            .filter_map(|node| {
                graph
                    .stable_node_key(node.id)
                    .map(|node_key| (node_key, node.position))
            })
            .collect()
    }

    fn restore_graph_layout(graph: &mut RuleGraph, layout: &HashMap<String, [f32; 2]>) {
        let node_ids = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        for node_id in node_ids {
            let Some(node_key) = graph.stable_node_key(node_id) else {
                continue;
            };
            let Some(position) = layout.get(&node_key).copied() else {
                continue;
            };
            let _ = graph.set_node_position(node_id, position);
        }
    }

    fn rule_graph_node_kind_compact_label(kind: &RuleGraphNodeKind) -> String {
        match kind {
            RuleGraphNodeKind::Trigger(trigger) => {
                format!("Trigger {}", Self::trigger_summary(*trigger))
            }
            RuleGraphNodeKind::Condition(condition) => {
                format!("Condition {}", Self::condition_summary(*condition))
            }
            RuleGraphNodeKind::Action(action) => {
                format!("Action {}", Self::action_summary(action))
            }
        }
    }

    fn trigger_summary(trigger: RuleTrigger) -> String {
        match trigger {
            RuleTrigger::OnStart => "OnStart".to_string(),
            RuleTrigger::OnUpdate => "OnUpdate".to_string(),
            RuleTrigger::OnPlayerMove => "OnPlayerMove".to_string(),
            RuleTrigger::OnKey { key } => format!("OnKey({})", Self::key_label(key)),
            RuleTrigger::OnCollision => "OnCollision".to_string(),
            RuleTrigger::OnTrigger => "OnTrigger".to_string(),
        }
    }

    fn condition_summary(condition: RuleCondition) -> String {
        match condition {
            RuleCondition::Always => "Always".to_string(),
            RuleCondition::TargetExists { target } => {
                format!("TargetExists({})", Self::target_label(target))
            }
            RuleCondition::KeyHeld { key } => format!("KeyHeld({})", Self::key_label(key)),
            RuleCondition::EntityActive { target, is_active } => {
                format!(
                    "EntityActive({}, active={})",
                    Self::target_label(target),
                    is_active
                )
            }
        }
    }

    fn action_summary(action: &RuleAction) -> String {
        match action {
            RuleAction::PlaySound { channel, sound_id } => {
                format!(
                    "PlaySound({}, {})",
                    Self::sound_channel_label(*channel),
                    sound_id
                )
            }
            RuleAction::PlayMusic { track_id } => format!("PlayMusic({})", track_id),
            RuleAction::PlayAnimation { target, state } => {
                format!(
                    "PlayAnimation({}, {:?})",
                    Self::target_label(*target),
                    state
                )
            }
            RuleAction::SetVelocity { target, velocity } => format!(
                "SetVelocity({}, {}, {})",
                Self::target_label(*target),
                velocity[0],
                velocity[1]
            ),
            RuleAction::Spawn {
                entity_type,
                position,
            } => format!("Spawn({:?}, {}, {})", entity_type, position[0], position[1]),
            RuleAction::DestroySelf { target } => {
                format!("DestroySelf({})", Self::target_label(*target))
            }
            RuleAction::SwitchScene { scene_name } => format!("SwitchScene({})", scene_name),
        }
    }

    fn key_label(key: RuleKey) -> &'static str {
        match key {
            RuleKey::Up => "Up",
            RuleKey::Down => "Down",
            RuleKey::Left => "Left",
            RuleKey::Right => "Right",
            RuleKey::DebugToggle => "DebugToggle",
        }
    }

    fn sound_channel_label(channel: RuleSoundChannel) -> &'static str {
        match channel {
            RuleSoundChannel::Movement => "Movement",
            RuleSoundChannel::Collision => "Collision",
        }
    }

    fn target_label(target: RuleTarget) -> String {
        match target {
            RuleTarget::Player => "Player".to_string(),
            RuleTarget::Entity(entity_id) => format!("Entity({})", entity_id),
        }
    }

    fn graph_trigger_kind(trigger: RuleTrigger) -> GraphTriggerKind {
        match trigger {
            RuleTrigger::OnStart => GraphTriggerKind::Start,
            RuleTrigger::OnUpdate => GraphTriggerKind::Update,
            RuleTrigger::OnPlayerMove => GraphTriggerKind::PlayerMove,
            RuleTrigger::OnKey { .. } => GraphTriggerKind::Key,
            RuleTrigger::OnCollision => GraphTriggerKind::Collision,
            RuleTrigger::OnTrigger => GraphTriggerKind::Trigger,
        }
    }

    fn graph_trigger_kind_label(kind: GraphTriggerKind) -> &'static str {
        match kind {
            GraphTriggerKind::Start => "OnStart",
            GraphTriggerKind::Update => "OnUpdate",
            GraphTriggerKind::PlayerMove => "OnPlayerMove",
            GraphTriggerKind::Key => "OnKey",
            GraphTriggerKind::Collision => "OnCollision",
            GraphTriggerKind::Trigger => "OnTrigger",
        }
    }

    fn graph_default_trigger(kind: GraphTriggerKind) -> RuleTrigger {
        match kind {
            GraphTriggerKind::Start => RuleTrigger::OnStart,
            GraphTriggerKind::Update => RuleTrigger::OnUpdate,
            GraphTriggerKind::PlayerMove => RuleTrigger::OnPlayerMove,
            GraphTriggerKind::Key => RuleTrigger::OnKey { key: RuleKey::Up },
            GraphTriggerKind::Collision => RuleTrigger::OnCollision,
            GraphTriggerKind::Trigger => RuleTrigger::OnTrigger,
        }
    }

    fn graph_condition_kind(condition: RuleCondition) -> GraphConditionKind {
        match condition {
            RuleCondition::Always => GraphConditionKind::Always,
            RuleCondition::TargetExists { .. } => GraphConditionKind::TargetExists,
            RuleCondition::KeyHeld { .. } => GraphConditionKind::KeyHeld,
            RuleCondition::EntityActive { .. } => GraphConditionKind::EntityActive,
        }
    }

    fn graph_condition_kind_label(kind: GraphConditionKind) -> &'static str {
        match kind {
            GraphConditionKind::Always => "Always",
            GraphConditionKind::TargetExists => "TargetExists",
            GraphConditionKind::KeyHeld => "KeyHeld",
            GraphConditionKind::EntityActive => "EntityActive",
        }
    }

    fn graph_default_condition(kind: GraphConditionKind) -> RuleCondition {
        match kind {
            GraphConditionKind::Always => RuleCondition::Always,
            GraphConditionKind::TargetExists => RuleCondition::TargetExists {
                target: RuleTarget::Player,
            },
            GraphConditionKind::KeyHeld => RuleCondition::KeyHeld { key: RuleKey::Up },
            GraphConditionKind::EntityActive => RuleCondition::EntityActive {
                target: RuleTarget::Player,
                is_active: true,
            },
        }
    }

    fn graph_action_kind(action: &RuleAction) -> GraphActionKind {
        match action {
            RuleAction::PlaySound { .. } => GraphActionKind::PlaySound,
            RuleAction::PlayMusic { .. } => GraphActionKind::PlayMusic,
            RuleAction::PlayAnimation { .. } => GraphActionKind::PlayAnimation,
            RuleAction::SetVelocity { .. } => GraphActionKind::SetVelocity,
            RuleAction::Spawn { .. } => GraphActionKind::Spawn,
            RuleAction::DestroySelf { .. } => GraphActionKind::DestroySelf,
            RuleAction::SwitchScene { .. } => GraphActionKind::SwitchScene,
        }
    }

    fn graph_action_kind_label(kind: GraphActionKind) -> &'static str {
        match kind {
            GraphActionKind::PlaySound => "PlaySound",
            GraphActionKind::PlayMusic => "PlayMusic",
            GraphActionKind::PlayAnimation => "PlayAnimation",
            GraphActionKind::SetVelocity => "SetVelocity",
            GraphActionKind::Spawn => "Spawn",
            GraphActionKind::DestroySelf => "DestroySelf",
            GraphActionKind::SwitchScene => "SwitchScene",
        }
    }

    fn graph_default_action(kind: GraphActionKind) -> RuleAction {
        match kind {
            GraphActionKind::PlaySound => RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_placeholder".to_string(),
            },
            GraphActionKind::PlayMusic => RuleAction::PlayMusic {
                track_id: "music_placeholder".to_string(),
            },
            GraphActionKind::PlayAnimation => RuleAction::PlayAnimation {
                target: RuleTarget::Player,
                state: AnimationState::Idle,
            },
            GraphActionKind::SetVelocity => RuleAction::SetVelocity {
                target: RuleTarget::Player,
                velocity: [0, 0],
            },
            GraphActionKind::Spawn => RuleAction::Spawn {
                entity_type: RuleSpawnEntityType::Npc,
                position: [0, 0],
            },
            GraphActionKind::DestroySelf => RuleAction::DestroySelf {
                target: RuleTarget::Player,
            },
            GraphActionKind::SwitchScene => RuleAction::SwitchScene {
                scene_name: String::new(),
            },
        }
    }

    fn edit_graph_condition_payload(ui: &mut egui::Ui, condition: &mut RuleCondition) -> bool {
        match condition {
            RuleCondition::Always => false,
            RuleCondition::TargetExists { target } => {
                Self::edit_rule_target(ui, target, "graph_condition_target")
            }
            RuleCondition::KeyHeld { key } => Self::edit_rule_key(ui, key, "graph_condition_key"),
            RuleCondition::EntityActive { target, is_active } => {
                let mut changed =
                    Self::edit_rule_target(ui, target, "graph_condition_entity_target");
                changed |= ui.checkbox(is_active, "Active").changed();
                changed
            }
        }
    }

    fn edit_graph_action_payload(ui: &mut egui::Ui, action: &mut RuleAction) -> bool {
        match action {
            RuleAction::PlaySound { channel, sound_id } => {
                let mut changed = false;
                egui::ComboBox::from_id_salt("graph_action_channel")
                    .selected_text(match channel {
                        RuleSoundChannel::Movement => "Movement",
                        RuleSoundChannel::Collision => "Collision",
                    })
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(channel, RuleSoundChannel::Movement, "Movement")
                            .changed();
                        changed |= ui
                            .selectable_value(channel, RuleSoundChannel::Collision, "Collision")
                            .changed();
                    });
                changed |= ui.text_edit_singleline(sound_id).changed();
                changed
            }
            RuleAction::PlayMusic { track_id } => ui.text_edit_singleline(track_id).changed(),
            RuleAction::PlayAnimation { target, state } => {
                let mut changed = Self::edit_rule_target(ui, target, "graph_action_anim_target");
                egui::ComboBox::from_id_salt("graph_action_anim_state")
                    .selected_text(match state {
                        AnimationState::Idle => "Idle",
                        AnimationState::Walk => "Walk",
                    })
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(state, AnimationState::Idle, "Idle")
                            .changed();
                        changed |= ui
                            .selectable_value(state, AnimationState::Walk, "Walk")
                            .changed();
                    });
                changed
            }
            RuleAction::SetVelocity { target, velocity } => {
                let mut changed = Self::edit_rule_target(ui, target, "graph_action_vel_target");
                changed |= ui
                    .add(egui::DragValue::new(&mut velocity[0]).speed(1.0))
                    .changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut velocity[1]).speed(1.0))
                    .changed();
                changed
            }
            RuleAction::Spawn {
                entity_type,
                position,
            } => {
                let mut changed = false;
                egui::ComboBox::from_id_salt("graph_action_spawn_type")
                    .selected_text(match entity_type {
                        RuleSpawnEntityType::PlayerLikeNpc => "PlayerLikeNpc",
                        RuleSpawnEntityType::Npc => "Npc",
                        RuleSpawnEntityType::Item => "Item",
                        RuleSpawnEntityType::Decoration => "Decoration",
                        RuleSpawnEntityType::Trigger => "Trigger",
                    })
                    .show_ui(ui, |ui| {
                        for candidate in [
                            RuleSpawnEntityType::PlayerLikeNpc,
                            RuleSpawnEntityType::Npc,
                            RuleSpawnEntityType::Item,
                            RuleSpawnEntityType::Decoration,
                            RuleSpawnEntityType::Trigger,
                        ] {
                            changed |= ui
                                .selectable_value(
                                    entity_type,
                                    candidate,
                                    match candidate {
                                        RuleSpawnEntityType::PlayerLikeNpc => "PlayerLikeNpc",
                                        RuleSpawnEntityType::Npc => "Npc",
                                        RuleSpawnEntityType::Item => "Item",
                                        RuleSpawnEntityType::Decoration => "Decoration",
                                        RuleSpawnEntityType::Trigger => "Trigger",
                                    },
                                )
                                .changed();
                        }
                    });
                changed |= ui
                    .add(egui::DragValue::new(&mut position[0]).speed(1.0))
                    .changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut position[1]).speed(1.0))
                    .changed();
                changed
            }
            RuleAction::DestroySelf { target } => {
                Self::edit_rule_target(ui, target, "graph_action_destroy_target")
            }
            RuleAction::SwitchScene { scene_name } => ui.text_edit_singleline(scene_name).changed(),
        }
    }

    fn edit_rule_target(ui: &mut egui::Ui, target: &mut RuleTarget, id_salt: &str) -> bool {
        let mut changed = false;
        egui::ComboBox::from_id_salt((id_salt, "kind"))
            .selected_text(match target {
                RuleTarget::Player => "Player",
                RuleTarget::Entity(_) => "Entity",
            })
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(target, RuleTarget::Player, "Player")
                    .changed();
                let entity_label = match target {
                    RuleTarget::Entity(entity_id) => format!("Entity({})", entity_id),
                    RuleTarget::Player => "Entity(0)".to_string(),
                };
                if ui
                    .selectable_label(matches!(target, RuleTarget::Entity(_)), entity_label)
                    .clicked()
                    && !matches!(target, RuleTarget::Entity(_))
                {
                    *target = RuleTarget::Entity(0);
                    changed = true;
                }
            });

        if let RuleTarget::Entity(entity_id) = target {
            let mut entity_id_i64 = *entity_id as i64;
            let id_changed = ui
                .add(
                    egui::DragValue::new(&mut entity_id_i64)
                        .speed(1.0)
                        .range(0_i64..=u32::MAX as i64),
                )
                .changed();
            if id_changed {
                *entity_id = entity_id_i64 as u32;
                changed = true;
            }
        }
        changed
    }

    fn edit_rule_key(ui: &mut egui::Ui, key: &mut RuleKey, id_salt: &str) -> bool {
        let mut changed = false;
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(match key {
                RuleKey::Up => "Up",
                RuleKey::Down => "Down",
                RuleKey::Left => "Left",
                RuleKey::Right => "Right",
                RuleKey::DebugToggle => "DebugToggle",
            })
            .show_ui(ui, |ui| {
                for candidate in [
                    RuleKey::Up,
                    RuleKey::Down,
                    RuleKey::Left,
                    RuleKey::Right,
                    RuleKey::DebugToggle,
                ] {
                    changed |= ui
                        .selectable_value(
                            key,
                            candidate,
                            match candidate {
                                RuleKey::Up => "Up",
                                RuleKey::Down => "Down",
                                RuleKey::Left => "Left",
                                RuleKey::Right => "Right",
                                RuleKey::DebugToggle => "DebugToggle",
                            },
                        )
                        .changed();
                }
            });
        changed
    }

    /// Renders the log/console panel at the bottom of the screen
    pub fn render_log_panel(
        _ui_state: &mut super::EditorUI,
        ctx: &egui::Context,
        log_capture: Option<&crate::logging::LogCapture>,
    ) {
        egui::TopBottomPanel::bottom("log_panel")
            .resizable(true)
            .default_height(200.0)
            .show(ctx, |ui| {
                ui.heading("📝 Console");
                ui.separator();

                if let Some(capture) = log_capture {
                    let logs = capture.get_logs();
                    let scroll_area = egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true);

                    scroll_area.show(ui, |ui| {
                        for log_entry in &logs {
                            ui.horizontal(|ui| {
                                ui.label(&log_entry.timestamp);
                                ui.label(&log_entry.level);
                                ui.label(&log_entry.message);
                            });
                        }
                    });
                } else {
                    ui.label("Logs are being sent to terminal (check log_to_terminal config)");
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::PanelSystem;
    use crate::ui::rule_graph::RuleGraph;
    use toki_core::rules::{
        Rule, RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel, RuleTarget,
        RuleTrigger,
    };

    #[test]
    fn trigger_summary_is_semantic() {
        assert_eq!(
            PanelSystem::trigger_summary(RuleTrigger::OnStart),
            "OnStart"
        );
        assert_eq!(
            PanelSystem::trigger_summary(RuleTrigger::OnKey { key: RuleKey::Left }),
            "OnKey(Left)"
        );
    }

    #[test]
    fn condition_summary_is_semantic() {
        assert_eq!(
            PanelSystem::condition_summary(RuleCondition::Always),
            "Always"
        );
        assert_eq!(
            PanelSystem::condition_summary(RuleCondition::TargetExists {
                target: RuleTarget::Player
            }),
            "TargetExists(Player)"
        );
    }

    #[test]
    fn action_summary_is_semantic() {
        assert_eq!(
            PanelSystem::action_summary(&RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_step".to_string(),
            }),
            "PlaySound(Movement, sfx_step)"
        );
        assert_eq!(
            PanelSystem::action_summary(&RuleAction::PlayMusic {
                track_id: "bgm_forest".to_string(),
            }),
            "PlayMusic(bgm_forest)"
        );
    }

    #[test]
    fn restore_graph_layout_preserves_existing_nodes_after_add_chain() {
        let rules = RuleSet {
            rules: vec![Rule {
                id: "rule_1".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnStart,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::PlayMusic {
                    track_id: "bgm_1".to_string(),
                }],
            }],
        };
        let mut graph = RuleGraph::from_rule_set(&rules);
        let initial_nodes = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();

        for (index, node_id) in initial_nodes.iter().enumerate() {
            graph
                .set_node_position(*node_id, [400.0 + index as f32 * 37.0, 250.0])
                .expect("existing node should accept custom position");
        }

        let remembered_layout = PanelSystem::remember_graph_layout(&graph);
        graph
            .add_trigger_chain()
            .expect("adding trigger chain should succeed");
        PanelSystem::restore_graph_layout(&mut graph, &remembered_layout);

        for node_id in initial_nodes {
            let node = graph
                .nodes
                .iter()
                .find(|node| node.id == node_id)
                .expect("original node should still exist");
            let node_key = graph
                .stable_node_key(node_id)
                .expect("original node should still have stable key");
            assert_eq!(
                Some(node.position),
                remembered_layout.get(&node_key).copied(),
                "layout should be preserved for existing node key {node_key}"
            );
        }
    }

    #[test]
    fn graph_zoom_scales_node_visuals() {
        let zoomed_out = PanelSystem::graph_node_max_size(0.5);
        let zoomed_in = PanelSystem::graph_node_max_size(2.0);
        assert!(
            zoomed_out.x < zoomed_in.x && zoomed_out.y < zoomed_in.y,
            "node size should increase with zoom"
        );

        let font_out = PanelSystem::graph_node_font_size(0.5);
        let font_in = PanelSystem::graph_node_font_size(2.0);
        assert!(font_out < font_in, "font size should increase with zoom");

        let edge_out = PanelSystem::graph_edge_stroke_width(0.5);
        let edge_in = PanelSystem::graph_edge_stroke_width(2.0);
        assert!(edge_out < edge_in, "edge stroke should increase with zoom");
    }

    #[test]
    fn enforce_graph_border_gap_moves_pan_when_left_or_top_nodes_touch_border() {
        let mut graph = RuleGraph::from_rule_set(&RuleSet {
            rules: vec![Rule {
                id: "rule_1".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnStart,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::PlayMusic {
                    track_id: "bgm_1".to_string(),
                }],
            }],
        });
        for node in &mut graph.nodes {
            node.position[0] = 0.0;
            node.position[1] = 0.0;
        }

        let mut pan = [0.0, 0.0];
        PanelSystem::enforce_graph_border_gap(&graph, 1.0, &mut pan);

        assert!(
            pan[0] > 0.0,
            "x pan should be increased to preserve border gap"
        );
        assert!(
            pan[1] > 0.0,
            "y pan should be increased to preserve border gap"
        );
    }
}
