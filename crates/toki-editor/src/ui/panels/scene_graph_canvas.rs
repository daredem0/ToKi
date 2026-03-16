use super::*;

impl PanelSystem {
    pub(super) fn rule_graph_node_label(
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

    pub(super) fn rule_graph_node_badges(graph: &RuleGraph) -> HashMap<u64, String> {
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

    pub(super) fn collect_graph_validation_issues(
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
    ) -> Vec<GraphValidationIssue> {
        let mut issues = Vec::<GraphValidationIssue>::new();
        let graph_serialization_error = graph.to_rule_set().err();
        if let Some(error) = graph_serialization_error.as_ref() {
            issues.push(Self::rule_graph_error_issue(graph, node_badges, error));
        }

        let mut serialized_nodes = HashSet::<u64>::new();
        for chain in &graph.chains {
            if let Ok(sequence) = graph.chain_node_sequence(chain.trigger_node_id) {
                serialized_nodes.extend(sequence);
            }
        }

        for node in &graph.nodes {
            if matches!(
                node.kind,
                RuleGraphNodeKind::Condition(_) | RuleGraphNodeKind::Action(_)
            ) && !serialized_nodes.contains(&node.id)
            {
                let node_label = Self::rule_graph_node_label(graph, node_badges, node.id)
                    .unwrap_or_else(|| format!("node {}", node.id));
                issues.push(GraphValidationIssue {
                    severity: GraphValidationSeverity::Warning,
                    message: format!("{node_label} is detached from all trigger chains."),
                    hint: "Connect it into a trigger chain, or delete it if it is no longer needed. Detached nodes stay in editor drafts but are not exported to scene JSON/runtime.".to_string(),
                    fixes: vec![GraphValidationFix {
                        label: format!("Delete {}", node_label),
                        command: GraphValidationFixCommand::RemoveNode(node.id),
                    }],
                });
            }
        }

        issues
    }

    pub(super) fn rule_graph_error_issue(
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        error: &RuleGraphError,
    ) -> GraphValidationIssue {
        match error {
            RuleGraphError::MissingTriggerNode { rule_id, node_id } => {
                let node_label = Self::rule_graph_node_label(graph, node_badges, *node_id)
                    .unwrap_or_else(|| format!("node {}", node_id));
                GraphValidationIssue {
                    severity: GraphValidationSeverity::Error,
                    message: format!(
                        "Rule '{}' references missing trigger {}.",
                        rule_id, node_label
                    ),
                    hint: "Delete and recreate the affected trigger chain.".to_string(),
                    fixes: Vec::new(),
                }
            }
            RuleGraphError::TriggerNodeKindMismatch { rule_id, node_id } => {
                let node_label = Self::rule_graph_node_label(graph, node_badges, *node_id)
                    .unwrap_or_else(|| format!("node {}", node_id));
                GraphValidationIssue {
                    severity: GraphValidationSeverity::Error,
                    message: format!(
                        "Rule '{}' trigger node has invalid kind at {}.",
                        rule_id, node_label
                    ),
                    hint: "Replace the node with a proper trigger node for this chain.".to_string(),
                    fixes: Vec::new(),
                }
            }
            RuleGraphError::MissingNode { rule_id, node_id } => {
                let node_label = Self::rule_graph_node_label(graph, node_badges, *node_id)
                    .unwrap_or_else(|| format!("node {}", node_id));
                GraphValidationIssue {
                    severity: GraphValidationSeverity::Error,
                    message: format!("Rule '{}' references missing {}.", rule_id, node_label),
                    hint: "Disconnect stale edges or remove/recreate the broken chain segment."
                        .to_string(),
                    fixes: Vec::new(),
                }
            }
            RuleGraphError::NonLinearChain { rule_id, node_id } => {
                let node_label = Self::rule_graph_node_label(graph, node_badges, *node_id)
                    .unwrap_or_else(|| format!("node {}", node_id));
                let extra_edges = Self::non_linear_extra_edges(graph, *node_id);
                let mut fixes = Vec::new();
                if !extra_edges.is_empty() {
                    fixes.push(GraphValidationFix {
                        label: format!("Disconnect extra branch edge(s) from {}", node_label),
                        command: GraphValidationFixCommand::DisconnectEdges(extra_edges),
                    });
                }
                GraphValidationIssue {
                    severity: GraphValidationSeverity::Error,
                    message: format!(
                        "Rule '{}' branches at {} (multiple outgoing edges).",
                        rule_id, node_label
                    ),
                    hint:
                        "Disconnect extra outgoing edges from this node, or split logic into separate trigger chains."
                            .to_string(),
                    fixes,
                }
            }
            RuleGraphError::CycleDetected { rule_id, node_id } => {
                let node_label = Self::rule_graph_node_label(graph, node_badges, *node_id)
                    .unwrap_or_else(|| format!("node {}", node_id));
                GraphValidationIssue {
                    severity: GraphValidationSeverity::Error,
                    message: format!("Rule '{}' contains a cycle at {}.", rule_id, node_label),
                    hint: "Disconnect one edge in the loop so each chain has a forward-only path."
                        .to_string(),
                    fixes: Vec::new(),
                }
            }
        }
    }

    pub(super) fn non_linear_extra_edges(graph: &RuleGraph, node_id: u64) -> Vec<(u64, u64)> {
        let node_by_id = graph
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<HashMap<_, _>>();
        let mut targets = graph
            .edges
            .iter()
            .filter(|edge| edge.from == node_id)
            .map(|edge| edge.to)
            .collect::<Vec<_>>();
        targets.sort_by_key(|target| {
            node_by_id
                .get(target)
                .map(|node| (Self::graph_node_kind_rank(&node.kind), *target))
                .unwrap_or((usize::MAX, *target))
        });
        if targets.len() <= 1 {
            return Vec::new();
        }
        targets
            .into_iter()
            .skip(1)
            .map(|target| (node_id, target))
            .collect()
    }

    pub(super) fn render_graph_validation_summary(
        ui: &mut egui::Ui,
        issues: &[GraphValidationIssue],
    ) -> Option<GraphValidationFixCommand> {
        let mut clicked_fix = None;
        if issues.is_empty() {
            ui.colored_label(
                egui::Color32::from_rgb(150, 210, 150),
                "Validation: Serializable (runtime/export ready)",
            );
            return None;
        }

        let error_count = issues
            .iter()
            .filter(|issue| issue.severity == GraphValidationSeverity::Error)
            .count();
        let warning_count = issues
            .iter()
            .filter(|issue| issue.severity == GraphValidationSeverity::Warning)
            .count();

        let header_color = if error_count > 0 {
            egui::Color32::from_rgb(255, 130, 130)
        } else {
            egui::Color32::from_rgb(255, 210, 120)
        };
        ui.group(|ui| {
            ui.colored_label(
                header_color,
                format!(
                    "Validation: {} error(s), {} warning(s)",
                    error_count, warning_count
                ),
            );
            for issue in issues {
                let (prefix, color) = match issue.severity {
                    GraphValidationSeverity::Error => {
                        ("Error", egui::Color32::from_rgb(255, 140, 140))
                    }
                    GraphValidationSeverity::Warning => {
                        ("Warning", egui::Color32::from_rgb(255, 210, 120))
                    }
                };
                ui.colored_label(color, format!("{prefix}: {}", issue.message));
                ui.label(format!("Hint: {}", issue.hint));
                if !issue.fixes.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for fix in &issue.fixes {
                            if ui.small_button(&fix.label).clicked() {
                                clicked_fix = Some(fix.command.clone());
                            }
                        }
                    });
                }
            }
        });
        clicked_fix
    }

    pub(super) fn compute_auto_layout_positions(
        ui: &egui::Ui,
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
    ) -> HashMap<u64, [f32; 2]> {
        let mut node_sizes = HashMap::<u64, egui::Vec2>::new();
        for node in &graph.nodes {
            let badge = node_badges
                .get(&node.id)
                .cloned()
                .unwrap_or_else(|| "?".to_string());
            let label = format!(
                "{}: {}",
                badge,
                Self::rule_graph_node_kind_compact_label(&node.kind)
            );
            node_sizes.insert(node.id, Self::graph_node_size_for_label(ui, &label, 1.0));
        }
        Self::compute_auto_layout_positions_from_sizes(graph, &node_sizes)
    }

    pub(super) fn compute_auto_layout_positions_from_sizes(
        graph: &RuleGraph,
        node_sizes: &HashMap<u64, egui::Vec2>,
    ) -> HashMap<u64, [f32; 2]> {
        let node_by_id = graph
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<HashMap<_, _>>();
        let mut node_ids = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        node_ids.sort_by_key(|node_id| {
            (
                Self::graph_node_kind_rank(&node_by_id[node_id].kind),
                *node_id,
            )
        });

        let mut incoming_count = HashMap::<u64, usize>::new();
        let mut outgoing = HashMap::<u64, Vec<u64>>::new();
        let mut incoming = HashMap::<u64, Vec<u64>>::new();
        for node_id in &node_ids {
            incoming_count.insert(*node_id, 0);
            outgoing.insert(*node_id, Vec::new());
            incoming.insert(*node_id, Vec::new());
        }
        for edge in &graph.edges {
            if !node_by_id.contains_key(&edge.from) || !node_by_id.contains_key(&edge.to) {
                continue;
            }
            outgoing.entry(edge.from).or_default().push(edge.to);
            incoming.entry(edge.to).or_default().push(edge.from);
            *incoming_count.entry(edge.to).or_default() += 1;
        }

        for targets in outgoing.values_mut() {
            targets.sort_by_key(|node_id| {
                (
                    Self::graph_node_kind_rank(&node_by_id[node_id].kind),
                    *node_id,
                )
            });
        }

        let mut ready = node_ids
            .iter()
            .copied()
            .filter(|node_id| incoming_count.get(node_id).copied().unwrap_or_default() == 0)
            .collect::<Vec<_>>();
        ready.sort_by_key(|node_id| {
            (
                Self::graph_node_kind_rank(&node_by_id[node_id].kind),
                *node_id,
            )
        });
        ready.reverse();

        let mut layer = node_ids
            .iter()
            .copied()
            .map(|node_id| (node_id, 0_usize))
            .collect::<HashMap<_, _>>();
        let mut processed = HashSet::<u64>::new();
        let mut topo_order = HashMap::<u64, usize>::new();
        let mut topo_index = 0_usize;

        while let Some(node_id) = ready.pop() {
            if !processed.insert(node_id) {
                continue;
            }
            topo_order.insert(node_id, topo_index);
            topo_index += 1;

            let current_layer = layer.get(&node_id).copied().unwrap_or_default();
            let targets = outgoing.get(&node_id).cloned().unwrap_or_default();
            for to in targets {
                let next_layer = current_layer + 1;
                let layer_entry = layer.entry(to).or_default();
                if *layer_entry < next_layer {
                    *layer_entry = next_layer;
                }
                if let Some(incoming) = incoming_count.get_mut(&to) {
                    *incoming = incoming.saturating_sub(1);
                    if *incoming == 0 {
                        ready.push(to);
                    }
                }
            }
            ready.sort_by_key(|candidate| {
                (
                    Self::graph_node_kind_rank(&node_by_id[candidate].kind),
                    *candidate,
                )
            });
            ready.reverse();
        }

        for node_id in node_ids.iter().copied() {
            if processed.contains(&node_id) {
                continue;
            }
            topo_order.insert(node_id, topo_index);
            topo_index += 1;
            processed.insert(node_id);
        }

        let mut layers = BTreeMap::<usize, Vec<u64>>::new();
        for node_id in node_ids {
            let node_layer = layer.get(&node_id).copied().unwrap_or_default();
            layers.entry(node_layer).or_default().push(node_id);
        }
        for layer_nodes in layers.values_mut() {
            layer_nodes.sort_by_key(|node_id| topo_order[node_id]);
        }

        let default_size = egui::vec2(
            RuleGraph::auto_layout_node_width(),
            RuleGraph::auto_layout_node_height(),
        );
        let mut positions = HashMap::<u64, [f32; 2]>::new();
        for (_layer_index, layer_nodes) in layers {
            let mut y_top = RuleGraph::auto_layout_start_y();
            for node_id in layer_nodes {
                let size = node_sizes.get(&node_id).copied().unwrap_or(default_size);
                let center_y = y_top + size.y * 0.5;
                positions.insert(node_id, [RuleGraph::auto_layout_start_x(), center_y]);
                y_top += size.y + RuleGraph::auto_layout_vertical_edge_spacing();
            }
        }

        let mut topo_nodes = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        topo_nodes.sort_by_key(|node_id| topo_order[node_id]);

        let horizontal_gap = RuleGraph::auto_layout_horizontal_edge_spacing();
        for node_id in &topo_nodes {
            let node_size = node_sizes.get(node_id).copied().unwrap_or(default_size);
            let predecessors = incoming.get(node_id).cloned().unwrap_or_default();
            let x = if predecessors.is_empty() {
                RuleGraph::auto_layout_start_x()
            } else {
                predecessors
                    .into_iter()
                    .filter_map(|from| {
                        let from_pos = positions.get(&from).copied()?;
                        let from_size = node_sizes.get(&from).copied().unwrap_or(default_size);
                        Some(from_pos[0] + from_size.x * 0.5 + horizontal_gap + node_size.x * 0.5)
                    })
                    .fold(RuleGraph::auto_layout_start_x(), f32::max)
            };
            if let Some(position) = positions.get_mut(node_id) {
                position[0] = x;
            }
        }

        // Enforce non-overlap strictly for node pairs that overlap vertically.
        let max_passes = topo_nodes.len().max(1).pow(2);
        for _ in 0..max_passes {
            let mut changed = false;
            for left_index in 0..topo_nodes.len() {
                for right_index in (left_index + 1)..topo_nodes.len() {
                    let left_id = topo_nodes[left_index];
                    let right_id = topo_nodes[right_index];
                    let Some(left_pos) = positions.get(&left_id).copied() else {
                        continue;
                    };
                    let Some(right_pos) = positions.get(&right_id).copied() else {
                        continue;
                    };
                    let left_size = node_sizes.get(&left_id).copied().unwrap_or(default_size);
                    let right_size = node_sizes.get(&right_id).copied().unwrap_or(default_size);

                    let required_dy = left_size.y * 0.5 + right_size.y * 0.5;
                    let actual_dy = (right_pos[1] - left_pos[1]).abs();
                    if actual_dy >= required_dy {
                        continue;
                    }

                    let (right_id, left_pos, right_pos, left_size, right_size) =
                        if left_pos[0] <= right_pos[0] {
                            (right_id, left_pos, right_pos, left_size, right_size)
                        } else {
                            (left_id, right_pos, left_pos, right_size, left_size)
                        };
                    let required_dx = left_size.x * 0.5 + right_size.x * 0.5 + horizontal_gap;
                    let actual_dx = right_pos[0] - left_pos[0];
                    if actual_dx >= required_dx {
                        continue;
                    }
                    if let Some(right_pos_mut) = positions.get_mut(&right_id) {
                        right_pos_mut[0] += required_dx - actual_dx;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }

        positions
    }

    pub(super) fn graph_node_kind_rank(kind: &RuleGraphNodeKind) -> usize {
        match kind {
            RuleGraphNodeKind::Trigger(_) => 0,
            RuleGraphNodeKind::Condition(_) => 1,
            RuleGraphNodeKind::Action(_) => 2,
        }
    }

    pub(super) fn render_graph_canvas(
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

        let mut node_positions = HashMap::<u64, egui::Pos2>::new();
        let mut node_labels = HashMap::<u64, String>::new();
        let mut node_rects = HashMap::<u64, egui::Rect>::new();
        for node in &graph.nodes {
            let center = to_canvas(node.position);
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
            let node_rect = egui::Rect::from_center_size(center, node_size);
            node_positions.insert(node.id, center);
            node_labels.insert(node.id, label);
            node_rects.insert(node.id, node_rect);
        }

        for edge in &graph.edges {
            let Some(from_rect) = node_rects.get(&edge.from).copied() else {
                continue;
            };
            let Some(to_rect) = node_rects.get(&edge.to).copied() else {
                continue;
            };
            let Some((start, end, direction)) = Self::graph_edge_points(from_rect, to_rect) else {
                continue;
            };
            let stroke = egui::Stroke::new(
                Self::graph_edge_stroke_width(scale),
                egui::Color32::from_rgb(130, 150, 185),
            );
            let arrow_length = (10.0 * scale).clamp(6.0, 18.0);
            let arrow_width = (5.0 * scale).clamp(3.0, 12.0);
            let body_end = end - direction * (arrow_length * 0.85);
            painter.line_segment([start, body_end], stroke);
            let perp = egui::vec2(-direction.y, direction.x);
            let arrow_left = end - direction * arrow_length + perp * arrow_width;
            let arrow_right = end - direction * arrow_length - perp * arrow_width;
            painter.line_segment([end, arrow_left], stroke);
            painter.line_segment([end, arrow_right], stroke);
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
            let Some(label) = node_labels.get(&node.id).cloned() else {
                continue;
            };
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
                &label,
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

    pub(super) fn render_graph_selected_node_editor(
        ui: &mut egui::Ui,
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        node_id: u64,
        scene_name: &str,
    ) -> Option<GraphCommand> {
        let Some(node_kind) = graph
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .map(|node| node.kind.clone())
        else {
            ui.label("Selected node no longer exists.");
            return None;
        };

        let display_label = Self::rule_graph_node_label(graph, node_badges, node_id)
            .unwrap_or_else(|| format!("node {node_id}"));
        ui.horizontal(|ui| {
            ui.monospace(display_label);
        });

        let mut command = None;
        ui.horizontal(|ui| {
            if ui.button("Disconnect All").clicked() {
                command = Some(GraphCommand::DisconnectNode(node_id));
            }
            if ui.button("Delete Node").clicked() {
                command = Some(GraphCommand::RemoveNode(node_id));
            }
        });

        ui.separator();

        match node_kind {
            RuleGraphNodeKind::Trigger(trigger) => {
                let mut trigger_value = trigger;
                let mut kind = Self::graph_trigger_kind(trigger);
                let kind_salt = format!("graph_canvas_trigger_kind::{scene_name}::{node_id}");
                egui::ComboBox::from_id_salt(kind_salt)
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
                if kind != Self::graph_trigger_kind(trigger) {
                    trigger_value = Self::graph_default_trigger(kind);
                }
                if let RuleTrigger::OnKey { key } = &mut trigger_value {
                    let key_salt = format!("graph_canvas_trigger_key::{scene_name}::{node_id}");
                    let _ = Self::edit_rule_key(ui, key, &key_salt);
                }
                if command.is_none() && trigger_value != trigger {
                    command = Some(GraphCommand::SetTrigger(node_id, trigger_value));
                }
            }
            RuleGraphNodeKind::Condition(condition) => {
                let mut edited_condition = condition;
                let mut kind = Self::graph_condition_kind(condition);
                let kind_salt = format!("graph_canvas_condition_kind::{scene_name}::{node_id}");
                egui::ComboBox::from_id_salt(kind_salt)
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
                if kind != Self::graph_condition_kind(condition) {
                    edited_condition = Self::graph_default_condition(kind);
                }
                let _ = Self::edit_graph_condition_payload(
                    ui,
                    &mut edited_condition,
                    &format!("graph_canvas_condition_payload::{scene_name}::{node_id}"),
                );
                if command.is_none() && edited_condition != condition {
                    command = Some(GraphCommand::SetCondition(node_id, edited_condition));
                }
            }
            RuleGraphNodeKind::Action(action) => {
                let mut edited_action = action.clone();
                let mut kind = Self::graph_action_kind(&action);
                let kind_salt = format!("graph_canvas_action_kind::{scene_name}::{node_id}");
                egui::ComboBox::from_id_salt(kind_salt)
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
                if kind != Self::graph_action_kind(&action) {
                    edited_action = Self::graph_default_action(kind);
                }
                let _ = Self::edit_graph_action_payload(
                    ui,
                    &mut edited_action,
                    &format!("graph_canvas_action_payload::{scene_name}::{node_id}"),
                );
                if command.is_none() && edited_action != action {
                    command = Some(GraphCommand::SetAction(node_id, edited_action));
                }
            }
        }

        command
    }

    pub(super) fn graph_edge_points(
        from_rect: egui::Rect,
        to_rect: egui::Rect,
    ) -> Option<(egui::Pos2, egui::Pos2, egui::Vec2)> {
        let from_center = from_rect.center();
        let to_center = to_rect.center();
        let center_delta = to_center - from_center;
        if center_delta.length_sq() <= f32::EPSILON {
            return None;
        }
        let start = Self::rect_border_point_toward(from_rect, to_center);
        let end = Self::rect_border_point_toward(to_rect, from_center);
        let line_delta = end - start;
        let line_length = line_delta.length();
        if line_length <= f32::EPSILON {
            return None;
        }
        Some((start, end, line_delta / line_length))
    }

    pub(super) fn rect_border_point_toward(rect: egui::Rect, toward: egui::Pos2) -> egui::Pos2 {
        let center = rect.center();
        let delta = toward - center;
        let half_size = rect.size() * 0.5;
        if half_size.x <= f32::EPSILON || half_size.y <= f32::EPSILON {
            return center;
        }
        let scale = (delta.x.abs() / half_size.x).max(delta.y.abs() / half_size.y);
        if scale <= f32::EPSILON {
            return center;
        }
        center + delta / scale
    }

    pub(super) fn enforce_graph_border_gap(
        graph: &RuleGraph,
        graph_zoom: f32,
        graph_pan: &mut [f32; 2],
    ) {
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

    pub(super) fn graph_node_max_size(scale: f32) -> egui::Vec2 {
        egui::vec2(
            (320.0 * scale).clamp(120.0, 860.0),
            (36.0 * scale).clamp(18.0, 96.0),
        )
    }

    pub(super) fn graph_node_min_size(scale: f32) -> egui::Vec2 {
        egui::vec2(
            (120.0 * scale).clamp(80.0, 300.0),
            (20.0 * scale).clamp(14.0, 48.0),
        )
    }

    pub(super) fn graph_node_size_for_label(ui: &egui::Ui, label: &str, scale: f32) -> egui::Vec2 {
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

    pub(super) fn graph_node_font_size(scale: f32) -> f32 {
        (11.0 * scale).clamp(7.0, 24.0)
    }

    pub(super) fn graph_edge_stroke_width(scale: f32) -> f32 {
        (1.5 * scale).clamp(0.7, 4.0)
    }

    pub(super) fn remember_graph_layout(graph: &RuleGraph) -> HashMap<String, [f32; 2]> {
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

    pub(super) fn restore_graph_layout(graph: &mut RuleGraph, layout: &HashMap<String, [f32; 2]>) {
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

    pub(super) fn rule_graph_node_kind_compact_label(kind: &RuleGraphNodeKind) -> String {
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
}
