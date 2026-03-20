use super::*;
use crate::ui::EditorUI;

impl PanelSystem {
    pub(super) fn render_scene_graph(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
        show_scene_rules: bool,
    ) {
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

        let mut connect_from = ui_state.graph.connect_from_node;
        let mut connect_to = ui_state.graph.connect_to_node;
        let (mut graph_zoom, mut graph_pan) = ui_state.graph_view_for_scene(&active_scene_name);
        let before_rule_set = ui_state.scenes[scene_index].rules.clone();
        let before_graph_snapshot = ui_state.rule_graph_for_scene(&active_scene_name).cloned();
        let before_layout_snapshot = ui_state
            .graph
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
            ui_state.sync_rule_graph_with_rule_set(&active_scene_name, &scene_rules);
            let mut graph = ui_state
                .rule_graph_for_scene(&active_scene_name)
                .cloned()
                .unwrap_or_else(|| RuleGraph::from_rule_set(&scene_rules));
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
                        pending_command = Some(GraphCommand::AddConditionNode);
                    }
                    if ui.button("➕ Add Action").clicked() {
                        pending_command = Some(GraphCommand::AddActionNode);
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
                        pending_command = Some(GraphCommand::AddConditionNode);
                    }
                    if ui.button("➕ Add Action").clicked() {
                        pending_command = Some(GraphCommand::AddActionNode);
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
                                                egui::ComboBox::from_id_salt((
                                                    "graph_trigger_kind",
                                                    &active_scene_name,
                                                    node_id,
                                                ))
                                                    .selected_text(Self::graph_trigger_kind_label(kind))
                                                    .show_ui(ui, |ui| {
                                                        for candidate in [
                                                            GraphTriggerKind::Start,
                                                            GraphTriggerKind::Update,
                                                            GraphTriggerKind::PlayerMove,
                                                            GraphTriggerKind::Key,
                                                            GraphTriggerKind::Collision,
                                                            GraphTriggerKind::Damaged,
                                                            GraphTriggerKind::Death,
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
                                                    let _ = Self::edit_rule_key(
                                                        ui,
                                                        key,
                                                        &format!(
                                                            "graph_trigger_key::{}::{}",
                                                            active_scene_name, node_id
                                                        ),
                                                    );
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
                                                    Self::condition_summary(condition)
                                                );
                                                let is_selected = selected_graph_node == Some(node_id);
                                                if ui.selectable_label(is_selected, node_label).clicked() {
                                                    selected_graph_node = Some(node_id);
                                                }
                                                let mut kind = Self::graph_condition_kind(condition);
                                                egui::ComboBox::from_id_salt((
                                                    "graph_condition_kind",
                                                    &active_scene_name,
                                                    node_id,
                                                ))
                                                    .selected_text(Self::graph_condition_kind_label(kind))
                                                    .show_ui(ui, |ui| {
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
                                                    });
                                                let mut edited_condition = condition.clone();
                                                if kind != Self::graph_condition_kind(condition) {
                                                    edited_condition =
                                                        Self::graph_default_condition(kind);
                                                }
                                                let payload_changed =
                                                    Self::edit_graph_condition_payload(
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
                                                egui::ComboBox::from_id_salt((
                                                    "graph_action_kind",
                                                    &active_scene_name,
                                                    node_id,
                                                ))
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
                                                    });
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
                let is_draft_only_command = matches!(
                    command,
                    GraphCommand::AddConditionNode | GraphCommand::AddActionNode
                );
                let remembered_layout = Self::remember_graph_layout(&graph);
                let command_result = match command {
                    GraphCommand::AddTrigger => graph.add_trigger_chain().map(|_| ()),
                    GraphCommand::ResetLayout => {
                        let auto_positions =
                            Self::compute_auto_layout_positions(ui, &graph, &node_badges);
                        auto_positions
                            .into_iter()
                            .try_for_each(|(node_id, position)| {
                                graph.set_node_position(node_id, position)
                            })
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
                        graph_changed = true;
                        if is_reset_layout {
                            // Keep a visible border gap when snapping to auto layout.
                            graph_pan = [16.0, 16.0];
                            Self::enforce_graph_border_gap(&graph, graph_zoom, &mut graph_pan);
                        }
                        if !is_layout_command && !is_reset_layout {
                            Self::restore_graph_layout(&mut graph, &remembered_layout);
                        }
                        if is_layout_command || is_reset_layout || is_draft_only_command {
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
                if !ui_state.execute_scene_rules_graph_command(
                    &active_scene_name,
                    SceneRulesGraphCommandData {
                        before_rule_set: before_rule_set.clone(),
                        after_rule_set,
                        before_graph: before_graph_snapshot.clone(),
                        after_graph: graph.clone(),
                        before_layout: before_layout_snapshot.clone(),
                        zoom: graph_zoom,
                        pan: graph_pan,
                    },
                ) {
                    operation_error =
                        Some("Failed to record scene graph change in undo history.".to_string());
                }
            } else if ui_state.rule_graph_for_scene(&active_scene_name).is_none() {
                ui_state.set_rule_graph_for_scene(active_scene_name.clone(), graph.clone());
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

        ui_state.graph.connect_from_node = connect_from;
        ui_state.graph.connect_to_node = connect_to;
        ui_state.graph.canvas_zoom = graph_zoom;
        ui_state.graph.canvas_pan = graph_pan;
        ui_state.set_graph_view_for_scene(&active_scene_name, graph_zoom, graph_pan);
        if scene_changed {
            ui_state.scene_content_changed = true;
        }
        if let Some(error) = operation_error {
            ui.colored_label(egui::Color32::from_rgb(255, 120, 120), error);
        }
    }
}
