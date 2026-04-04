use super::*;
use crate::ui::editor_ui::SceneEditorSubView;
use crate::ui::graph_canvas::GraphCanvasAction;
use crate::ui::EditorUI;

#[derive(Default)]
struct GraphCommandFlags {
    scene_changed: bool,
    graph_changed: bool,
    layout_changed: bool,
    operation_error: Option<String>,
}

impl PanelSystem {
    pub(crate) fn render_scene_graph(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
        _config: Option<&EditorConfig>,
    ) {
        let show_scene_rules = {
            let sub_view = &mut crate::ui::editor_context::graph_state_mut(ui_state).sub_view;
            ui.horizontal(|ui| {
                ui.selectable_value(sub_view, SceneEditorSubView::Graph, "Graph");
                ui.selectable_value(sub_view, SceneEditorSubView::Rules, "Rules");
            });
            *sub_view == SceneEditorSubView::Rules
        };
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

        let mut connect_from =
            crate::ui::editor_context::graph_state_mut(ui_state).connect_from_node;
        let mut connect_to = crate::ui::editor_context::graph_state_mut(ui_state).connect_to_node;
        let mut canvas_state = crate::ui::editor_context::graph_state_mut(ui_state)
            .canvas_state
            .clone();
        let (persisted_zoom, persisted_pan) =
            crate::ui::editor_ui::graph_view_for_scene(ui_state, &active_scene_name);
        canvas_state.zoom = persisted_zoom;
        canvas_state.pan = persisted_pan;
        let before_rule_set = ui_state.scenes[scene_index].rules.clone();
        let before_graph_snapshot =
            crate::ui::editor_ui::rule_graph_for_scene(ui_state, &active_scene_name).cloned();
        let before_layout_snapshot = crate::ui::editor_context::graph_state_mut(ui_state)
            .layouts_by_scene
            .get(&active_scene_name)
            .cloned();
        let mut scene_changed = false;
        let mut graph_changed = false;
        let mut layout_changed = false;
        let mut operation_error: Option<String> = None;
        let mut selected_graph_node: Option<u64> = None;

        {
            let scene_rules = before_rule_set.clone();
            crate::ui::editor_ui::sync_rule_graph_with_rule_set(
                ui_state,
                &active_scene_name,
                &scene_rules,
            );
            let mut graph =
                crate::ui::editor_ui::rule_graph_for_scene(ui_state, &active_scene_name)
                    .cloned()
                    .unwrap_or_else(|| RuleGraph::from_rule_set(&scene_rules));
            let mut pending_command: Option<GraphCommand> = None;

            let node_ids = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
            for node_id in node_ids {
                let Some(node_key) = graph.stable_node_key(node_id) else {
                    continue;
                };
                let Some(position) = crate::ui::editor_ui::graph_layout_position(
                    ui_state,
                    &active_scene_name,
                    &node_key,
                ) else {
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

            ui.horizontal(|ui| {
                if ui.button("➕ Add Trigger").clicked() {
                    pending_command = Some(GraphCommand::AddTrigger);
                }
                if ui.button("➕ Add Condition").clicked() {
                    pending_command = Some(GraphCommand::AddConditionNode);
                }
                if ui.button("➕ Add Action").clicked() {
                    pending_command = Some(GraphCommand::AddActionNode);
                }
                if !show_scene_rules && ui.button("↺ Reset Auto Layout").clicked() {
                    pending_command = Some(GraphCommand::ResetLayout);
                }
            });

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
            let validation_issues = Self::collect_graph_validation_issues(&graph, &node_badges);
            if pending_command.is_none() {
                if let Some(fix_command) =
                    Self::render_graph_validation_summary(ui, &validation_issues)
                {
                    pending_command = Some(match fix_command {
                        GraphValidationFixCommand::DisconnectEdges(edges) => {
                            GraphCommand::DisconnectMany(edges)
                        }
                        GraphValidationFixCommand::RemoveNode(node_id) => {
                            GraphCommand::RemoveNode(node_id)
                        }
                    });
                }
            } else {
                let _ = Self::render_graph_validation_summary(ui, &validation_issues);
            }
            if !show_scene_rules {
                if pending_command.is_none() {
                    let actions = Self::render_graph_canvas(
                        ui,
                        &graph,
                        &node_badges,
                        &mut canvas_state,
                        selected_graph_node,
                    );
                    for action in actions {
                        match action {
                            GraphCanvasAction::SelectNode(id) => {
                                selected_graph_node = id.as_deref().and_then(|s| s.parse().ok());
                            }
                            GraphCanvasAction::MoveNode { node_id, position } => {
                                if let Ok(id) = node_id.parse::<u64>() {
                                    pending_command =
                                        Some(GraphCommand::SetNodePosition(id, position));
                                }
                            }
                            GraphCanvasAction::CreateNodeAt(_) => {
                                pending_command = Some(GraphCommand::AddTrigger);
                            }
                            GraphCanvasAction::Connect {
                                from_node_id,
                                to_node_id,
                                ..
                            } => {
                                if let (Ok(from), Ok(to)) =
                                    (from_node_id.parse::<u64>(), to_node_id.parse::<u64>())
                                {
                                    pending_command = Some(GraphCommand::Connect(from, to));
                                }
                            }
                        }
                    }
                }

                if graph.nodes.is_empty() {
                    ui.label("No rules in active scene. Add a rule chain to start authoring.");
                } else if let Some(node_id) = selected_graph_node {
                    ui.separator();
                    ui.strong("Selected Node");
                    if pending_command.is_none() {
                        pending_command = Self::render_graph_selected_node_editor(
                            ui,
                            &graph,
                            &node_badges,
                            node_id,
                            &active_scene_name,
                        );
                    } else {
                        let _ = Self::render_graph_selected_node_editor(
                            ui,
                            &graph,
                            &node_badges,
                            node_id,
                            &active_scene_name,
                        );
                    }
                }
            }

            if show_scene_rules {
                if pending_command.is_none() {
                    pending_command = Self::render_scene_rules_list(
                        ui,
                        &graph,
                        &node_badges,
                        &active_scene_name,
                        &mut selected_graph_node,
                    );
                } else {
                    let _ = Self::render_scene_rules_list(
                        ui,
                        &graph,
                        &node_badges,
                        &active_scene_name,
                        &mut selected_graph_node,
                    );
                }
            }

            if let Some(command) = pending_command {
                let flags = Self::apply_graph_command(
                    ui,
                    &mut graph,
                    &node_badges,
                    command,
                    canvas_state.zoom,
                    &mut canvas_state.pan,
                );
                scene_changed |= flags.scene_changed;
                graph_changed |= flags.graph_changed;
                layout_changed |= flags.layout_changed;
                if let Some(error) = flags.operation_error {
                    operation_error = Some(error);
                }
            }

            let mut after_rule_set = before_rule_set.clone();
            if scene_changed {
                match graph.to_rule_set() {
                    Ok(rule_set) => {
                        if rule_set != before_rule_set {
                            after_rule_set = rule_set;
                        } else {
                            scene_changed = false;
                        }
                    }
                    Err(error) => {
                        scene_changed = false;
                        let issue = Self::rule_graph_error_issue(&graph, &node_badges, &error);
                        operation_error = Some(format!(
                            "{} Scene JSON was not updated. Hint: {}",
                            issue.message, issue.hint
                        ));
                    }
                }
            }

            let state_changed = graph_changed || scene_changed || layout_changed;
            if state_changed {
                if !crate::ui::editor_ui::execute_scene_rules_graph_command(
                    ui_state,
                    &active_scene_name,
                    SceneRulesGraphCommandData {
                        before_rule_set: before_rule_set.clone(),
                        after_rule_set,
                        before_graph: before_graph_snapshot.clone(),
                        after_graph: graph.clone(),
                        before_layout: before_layout_snapshot.clone(),
                        zoom: canvas_state.zoom,
                        pan: canvas_state.pan,
                    },
                ) {
                    operation_error =
                        Some("Failed to record scene graph change in undo history.".to_string());
                }
            } else if crate::ui::editor_ui::rule_graph_for_scene(ui_state, &active_scene_name)
                .is_none()
            {
                crate::ui::editor_ui::set_rule_graph_for_scene(
                    ui_state,
                    active_scene_name.clone(),
                    graph.clone(),
                );
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

        crate::ui::editor_context::graph_state_mut(ui_state).connect_from_node = connect_from;
        crate::ui::editor_context::graph_state_mut(ui_state).connect_to_node = connect_to;
        crate::ui::editor_context::graph_state_mut(ui_state).canvas_state = canvas_state.clone();
        crate::ui::editor_ui::set_graph_view_for_scene(
            ui_state,
            &active_scene_name,
            canvas_state.zoom,
            canvas_state.pan,
        );
        if scene_changed {
            ui_state.scene_content_changed = true;
        }
        if let Some(error) = operation_error {
            ui.colored_label(egui::Color32::from_rgb(255, 120, 120), error);
        }
    }

    fn render_scene_rules_list(
        ui: &mut egui::Ui,
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        active_scene_name: &str,
        selected_graph_node: &mut Option<u64>,
    ) -> Option<GraphCommand> {
        let node_by_id = graph
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<HashMap<_, _>>();
        let mut outgoing = HashMap::<u64, Vec<u64>>::new();
        for edge in &graph.edges {
            outgoing.entry(edge.from).or_default().push(edge.to);
        }

        let mut pending_command = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (rule_index, chain) in graph.chains.iter().enumerate() {
                ui.push_id(("graph_chain", chain.trigger_node_id), |ui| {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.strong(format!("Rule {}: {}", rule_index + 1, chain.rule_id));
                            if !chain.enabled {
                                ui.label("(disabled)");
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
                                            Self::trigger_summary(trigger.clone())
                                        );
                                        let is_selected = *selected_graph_node == Some(node_id);
                                        if ui.selectable_label(is_selected, node_label).clicked() {
                                            *selected_graph_node = Some(node_id);
                                        }
                                        let mut trigger_value = trigger.clone();
                                        let mut kind = Self::graph_trigger_kind(trigger.clone());
                                        egui::ComboBox::from_id_salt((
                                            "graph_trigger_kind",
                                            active_scene_name,
                                            node_id,
                                        ))
                                        .selected_text(Self::graph_trigger_kind_label(kind))
                                        .show_ui(
                                            ui,
                                            |ui| {
                                                for candidate in [
                                                    GraphTriggerKind::Start,
                                                    GraphTriggerKind::Update,
                                                    GraphTriggerKind::PlayerMove,
                                                    GraphTriggerKind::Key,
                                                    GraphTriggerKind::Collision,
                                                    GraphTriggerKind::Damaged,
                                                    GraphTriggerKind::Death,
                                                    GraphTriggerKind::Trigger,
                                                    GraphTriggerKind::Interact,
                                                    GraphTriggerKind::DialogComplete,
                                                    GraphTriggerKind::TileEnter,
                                                    GraphTriggerKind::TileExit,
                                                ] {
                                                    ui.selectable_value(
                                                        &mut kind,
                                                        candidate,
                                                        Self::graph_trigger_kind_label(candidate),
                                                    );
                                                }
                                            },
                                        );
                                        if kind != Self::graph_trigger_kind(trigger.clone()) {
                                            trigger_value = Self::graph_default_trigger(kind);
                                        }
                                        if let RuleTrigger::OnKey { key } = &mut trigger_value {
                                            let _ = Self::edit_rule_key(
                                                ui,
                                                key,
                                                &format!(
                                                    "graph_trigger_key::{}::{}",
                                                    active_scene_name, node_id
                                                ),
                                            );
                                        }
                                        if let RuleTrigger::OnDialogComplete {
                                            dialog_id,
                                            outcome_id,
                                        } = &mut trigger_value
                                        {
                                            ui.label("Dialog:");
                                            let mut dialog_id_value = dialog_id.to_string();
                                            if ui
                                                .text_edit_singleline(&mut dialog_id_value)
                                                .changed()
                                            {
                                                *dialog_id = dialog_id_value.into();
                                            }
                                            ui.label("Outcome:");
                                            let _ = ui.text_edit_singleline(outcome_id);
                                        }
                                        if trigger_value != *trigger {
                                            pending_command = Some(GraphCommand::SetTrigger(
                                                node_id,
                                                trigger_value,
                                            ));
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
                                            Self::condition_summary(condition)
                                        );
                                        let is_selected = *selected_graph_node == Some(node_id);
                                        if ui.selectable_label(is_selected, node_label).clicked() {
                                            *selected_graph_node = Some(node_id);
                                        }
                                        let mut kind = Self::graph_condition_kind(condition);
                                        egui::ComboBox::from_id_salt((
                                            "graph_condition_kind",
                                            active_scene_name,
                                            node_id,
                                        ))
                                        .selected_text(Self::graph_condition_kind_label(kind))
                                        .show_ui(
                                            ui,
                                            |ui| {
                                                for candidate in [
                                                    GraphConditionKind::Always,
                                                    GraphConditionKind::TargetExists,
                                                    GraphConditionKind::KeyHeld,
                                                    GraphConditionKind::EntityActive,
                                                    GraphConditionKind::HealthBelow,
                                                    GraphConditionKind::HealthAbove,
                                                    GraphConditionKind::TriggerOtherIsPlayer,
                                                    GraphConditionKind::EntityIsKind,
                                                    GraphConditionKind::TriggerOtherIsKind,
                                                    GraphConditionKind::EntityHasTag,
                                                    GraphConditionKind::TriggerOtherHasTag,
                                                    GraphConditionKind::HasInventoryItem,
                                                ] {
                                                    ui.selectable_value(
                                                        &mut kind,
                                                        candidate,
                                                        Self::graph_condition_kind_label(candidate),
                                                    );
                                                }
                                            },
                                        );
                                        let mut edited_condition = condition.clone();
                                        if kind != Self::graph_condition_kind(condition) {
                                            edited_condition = Self::graph_default_condition(kind);
                                        }
                                        let payload_changed = Self::edit_graph_condition_payload(
                                            ui,
                                            &mut edited_condition,
                                            &format!(
                                                "graph_condition_payload::{}::{}",
                                                active_scene_name, node_id
                                            ),
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
                                        let node_label = format!(
                                            "{} Action: {}",
                                            badge,
                                            Self::action_summary(action)
                                        );
                                        let is_selected = *selected_graph_node == Some(node_id);
                                        if ui.selectable_label(is_selected, node_label).clicked() {
                                            *selected_graph_node = Some(node_id);
                                        }
                                        let mut kind = Self::graph_action_kind(action);
                                        egui::ComboBox::from_id_salt((
                                            "graph_action_kind",
                                            active_scene_name,
                                            node_id,
                                        ))
                                        .selected_text(Self::graph_action_kind_label(kind))
                                        .show_ui(
                                            ui,
                                            |ui| {
                                                for candidate in [
                                                    GraphActionKind::PlaySound,
                                                    GraphActionKind::PlayMusic,
                                                    GraphActionKind::PlayAnimation,
                                                    GraphActionKind::SetVelocity,
                                                    GraphActionKind::Spawn,
                                                    GraphActionKind::DestroySelf,
                                                    GraphActionKind::SwitchScene,
                                                    GraphActionKind::DamageEntity,
                                                    GraphActionKind::HealEntity,
                                                    GraphActionKind::AddInventoryItem,
                                                    GraphActionKind::RemoveInventoryItem,
                                                    GraphActionKind::SetEntityActive,
                                                    GraphActionKind::TeleportEntity,
                                                ] {
                                                    ui.selectable_value(
                                                        &mut kind,
                                                        candidate,
                                                        Self::graph_action_kind_label(candidate),
                                                    );
                                                }
                                            },
                                        );
                                        let mut edited_action = action.clone();
                                        if kind != Self::graph_action_kind(action) {
                                            edited_action = Self::graph_default_action(kind);
                                        }
                                        let payload_changed = Self::edit_graph_action_payload(
                                            ui,
                                            &mut edited_action,
                                            &format!(
                                                "graph_action_payload::{}::{}",
                                                active_scene_name, node_id
                                            ),
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
                                            let from_label = Self::rule_graph_node_label(
                                                graph,
                                                node_badges,
                                                edge.from,
                                            )
                                            .unwrap_or_else(|| format!("node {}", edge.from));
                                            let to_label = Self::rule_graph_node_label(
                                                graph,
                                                node_badges,
                                                edge.to,
                                            )
                                            .unwrap_or_else(|| format!("node {}", edge.to));
                                            ui.monospace(format!("{} -> {}", from_label, to_label));
                                            if ui.small_button("Disconnect").clicked() {
                                                pending_command = Some(GraphCommand::Disconnect(
                                                    edge.from, edge.to,
                                                ));
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
        pending_command
    }

    fn apply_graph_command(
        ui: &egui::Ui,
        graph: &mut RuleGraph,
        node_badges: &HashMap<u64, String>,
        command: GraphCommand,
        graph_zoom: f32,
        graph_pan: &mut [f32; 2],
    ) -> GraphCommandFlags {
        let is_layout_command = matches!(command, GraphCommand::SetNodePosition(_, _));
        let is_reset_layout = matches!(command, GraphCommand::ResetLayout);
        let is_draft_only_command = matches!(
            command,
            GraphCommand::AddConditionNode | GraphCommand::AddActionNode
        );
        let remembered_layout = Self::remember_graph_layout(graph);
        let command_result = match command {
            GraphCommand::AddTrigger => graph.add_trigger_chain().map(|_| ()),
            GraphCommand::ResetLayout => {
                let auto_positions = Self::compute_auto_layout_positions(ui, graph, node_badges);
                auto_positions
                    .into_iter()
                    .try_for_each(|(node_id, position)| graph.set_node_position(node_id, position))
            }
            GraphCommand::AddConditionNode => {
                graph.add_condition_node(RuleCondition::Always).map(|_| ())
            }
            GraphCommand::SetTrigger(trigger_node_id, trigger) => {
                graph.set_trigger_for_chain(trigger_node_id, trigger)
            }
            GraphCommand::AddActionNode => graph
                .add_action_node(RuleAction::PlaySound {
                    channel: RuleSoundChannel::Movement,
                    sound_id: "sfx_placeholder".to_string(),
                })
                .map(|_| ()),
            GraphCommand::SetCondition(node_id, condition) => {
                graph.set_condition_for_node(node_id, condition)
            }
            GraphCommand::SetAction(node_id, action) => graph.set_action_for_node(node_id, action),
            GraphCommand::SetNodePosition(node_id, position) => {
                graph.set_node_position(node_id, position)
            }
            GraphCommand::RemoveNode(node_id) => graph.remove_node(node_id),
            GraphCommand::Connect(from, to) => graph.connect_nodes(from, to),
            GraphCommand::Disconnect(from, to) => {
                graph.disconnect_nodes(from, to);
                Ok(())
            }
            GraphCommand::DisconnectMany(edges) => {
                for (from, to) in edges {
                    graph.disconnect_nodes(from, to);
                }
                Ok(())
            }
            GraphCommand::DisconnectNode(node_id) => graph.disconnect_node(node_id),
        };

        match command_result {
            Ok(()) => {
                let mut flags = GraphCommandFlags {
                    graph_changed: true,
                    ..Default::default()
                };
                if is_reset_layout {
                    *graph_pan = [16.0, 16.0];
                    Self::enforce_graph_border_gap(graph, graph_zoom, graph_pan);
                }
                if !is_layout_command && !is_reset_layout {
                    Self::restore_graph_layout(graph, &remembered_layout);
                }
                if is_layout_command || is_reset_layout || is_draft_only_command {
                    flags.layout_changed = true;
                } else {
                    flags.scene_changed = true;
                }
                flags
            }
            Err(error) => {
                let message = format!("Graph edit failed: {:?}", error);
                tracing::warn!("{message}");
                GraphCommandFlags {
                    operation_error: Some(message),
                    ..Default::default()
                }
            }
        }
    }
}
