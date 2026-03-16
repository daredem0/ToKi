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
}
