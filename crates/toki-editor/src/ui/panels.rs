use super::editor_ui::CenterPanelTab;
use super::interactions::{CameraInteraction, PlacementInteraction, SelectionInteraction};
use super::rule_graph::{RuleGraph, RuleGraphNodeKind};
use crate::config::EditorConfig;
use crate::scene::SceneViewport;
use std::collections::{HashMap, HashSet};
use toki_core::animation::AnimationState;
use toki_core::rules::{
    RuleAction, RuleCondition, RuleKey, RuleSoundChannel, RuleSpawnEntityType, RuleTarget,
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

impl PanelSystem {
    /// Renders the main scene viewport panel in the center of the screen
    pub fn render_viewport(
        ui_state: &mut super::EditorUI,
        ctx: &egui::Context,
        scene_viewport: Option<&mut SceneViewport>,
        config: Option<&EditorConfig>,
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
            });
            ui.separator();

            if ui_state.center_panel_tab == CenterPanelTab::SceneGraph {
                Self::render_scene_graph(ui, ui_state);
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
                            config,
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
                    CameraInteraction::handle_drag(viewport, &response, config);
                } else {
                    viewport.stop_camera_drag();
                }

                // Handle placement mode hover logic
                PlacementInteraction::handle_hover(ui_state, viewport, &response, rect, config);

                // Handle viewport clicks (entity placement or selection)
                if response.clicked() {
                    if let Some(click_pos) = response.hover_pos() {
                        // Check if we're in placement mode
                        if ui_state.is_in_placement_mode() {
                            PlacementInteraction::handle_click(
                                ui_state, viewport, click_pos, rect, config,
                            );
                        } else {
                            // Normal entity selection
                            SelectionInteraction::handle_click(ui_state, viewport, click_pos, rect);
                        }
                    }
                }

                // Render the scene content
                let project_path = config.and_then(|c| c.current_project_path());
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

    fn render_scene_graph(ui: &mut egui::Ui, ui_state: &mut super::EditorUI) {
        enum GraphCommand {
            AddChain,
            AppendCondition(u64),
            AppendAction(u64),
            SetCondition(u64, RuleCondition),
            SetAction(u64, RuleAction),
            RemoveNode(u64),
            Connect(u64, u64),
            Disconnect(u64, u64),
        }

        ui.heading("Active Scene Graph");
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
        let mut scene_changed = false;
        let mut operation_error: Option<String> = None;

        {
            let scene = &mut ui_state.scenes[scene_index];
            let mut graph = RuleGraph::from_rule_set(&scene.rules);
            let mut pending_command: Option<GraphCommand> = None;

            ui.horizontal(|ui| {
                if ui.button("➕ Add Rule Chain").clicked() {
                    pending_command = Some(GraphCommand::AddChain);
                }
            });

            if connect_from.is_some_and(|id| !graph.nodes.iter().any(|node| node.id == id)) {
                connect_from = None;
            }
            if connect_to.is_some_and(|id| !graph.nodes.iter().any(|node| node.id == id)) {
                connect_to = None;
            }

            ui.horizontal(|ui| {
                ui.label("Connect:");

                egui::ComboBox::from_id_salt(format!("graph_connect_from_{}", scene_index))
                    .selected_text(
                        connect_from
                            .and_then(|id| Self::rule_graph_node_label(&graph, id))
                            .unwrap_or_else(|| "<source>".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for node in &graph.nodes {
                            ui.selectable_value(
                                &mut connect_from,
                                Some(node.id),
                                Self::rule_graph_node_label(&graph, node.id)
                                    .unwrap_or_else(|| format!("{}", node.id)),
                            );
                        }
                    });

                egui::ComboBox::from_id_salt(format!("graph_connect_to_{}", scene_index))
                    .selected_text(
                        connect_to
                            .and_then(|id| Self::rule_graph_node_label(&graph, id))
                            .unwrap_or_else(|| "<target>".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for node in &graph.nodes {
                            ui.selectable_value(
                                &mut connect_to,
                                Some(node.id),
                                Self::rule_graph_node_label(&graph, node.id)
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

            ui.label(format!(
                "Chains: {} | Nodes: {} | Edges: {}",
                graph.chains.len(),
                graph.nodes.len(),
                graph.edges.len()
            ));

            if graph.nodes.is_empty() {
                ui.label("No rules in active scene. Add a rule chain to start authoring.");
            }

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
                                            ui.monospace(format!("Trigger -> {:?}", trigger));
                                        }
                                        RuleGraphNodeKind::Condition(condition) => {
                                            ui.monospace("Condition");
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
                                            if kind != Self::graph_condition_kind(*condition) {
                                                pending_command = Some(GraphCommand::SetCondition(
                                                    node_id,
                                                    Self::graph_default_condition(kind),
                                                ));
                                            }
                                            if ui.small_button("✕").clicked() {
                                                pending_command = Some(GraphCommand::RemoveNode(node_id));
                                            }
                                        }
                                        RuleGraphNodeKind::Action(action) => {
                                            ui.monospace("Action");
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
                                            if kind != Self::graph_action_kind(action) {
                                                pending_command = Some(GraphCommand::SetAction(
                                                    node_id,
                                                    Self::graph_default_action(kind),
                                                ));
                                            }
                                            if ui.small_button("✕").clicked() {
                                                pending_command = Some(GraphCommand::RemoveNode(node_id));
                                            }
                                        }
                                    });
                                });
                            }

                            let edge_list = graph
                                .edges
                                .iter()
                                .filter(|edge| {
                                    sequence_set.contains(&edge.from) || sequence_set.contains(&edge.to)
                                })
                                .copied()
                                .collect::<Vec<_>>();

                            if !edge_list.is_empty() {
                                egui::CollapsingHeader::new("Edges")
                                    .id_salt(("graph_edges", chain.trigger_node_id))
                                    .show(ui, |ui| {
                                        for edge in edge_list {
                                            ui.horizontal(|ui| {
                                                ui.monospace(format!("{} -> {}", edge.from, edge.to));
                                                if ui.small_button("Disconnect").clicked() {
                                                    pending_command =
                                                        Some(GraphCommand::Disconnect(edge.from, edge.to));
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

            if let Some(command) = pending_command {
                let command_result = match command {
                    GraphCommand::AddChain => graph.add_default_chain().map(|_| ()),
                    GraphCommand::AppendCondition(trigger_node_id) => {
                        graph.append_condition_to_chain(trigger_node_id, RuleCondition::Always)
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
                    GraphCommand::RemoveNode(node_id) => graph.remove_node(node_id),
                    GraphCommand::Connect(from, to) => graph.connect_nodes(from, to),
                    GraphCommand::Disconnect(from, to) => {
                        graph.disconnect_nodes(from, to);
                        Ok(())
                    }
                };

                match command_result {
                    Ok(()) => {
                        scene_changed = true;
                    }
                    Err(error) => {
                        operation_error = Some(format!("Graph edit failed: {:?}", error));
                    }
                }
            }

            if scene_changed {
                match graph.to_rule_set() {
                    Ok(rule_set) => {
                        scene.rules = rule_set;
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
        }

        ui_state.graph_connect_from_node = connect_from;
        ui_state.graph_connect_to_node = connect_to;
        if scene_changed {
            ui_state.scene_content_changed = true;
        }
        if let Some(error) = operation_error {
            ui.colored_label(egui::Color32::from_rgb(255, 120, 120), error);
        }
    }

    fn rule_graph_node_label(graph: &RuleGraph, node_id: u64) -> Option<String> {
        let node = graph.nodes.iter().find(|node| node.id == node_id)?;
        let kind = match node.kind {
            RuleGraphNodeKind::Trigger(_) => "Trigger",
            RuleGraphNodeKind::Condition(_) => "Condition",
            RuleGraphNodeKind::Action(_) => "Action",
        };
        Some(format!("{} ({})", node_id, kind))
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
