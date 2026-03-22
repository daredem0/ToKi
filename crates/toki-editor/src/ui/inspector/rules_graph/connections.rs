//! Connection management UI for rule graph nodes.

use super::context::NodeActionResult;
use super::*;

impl InspectorSystem {
    /// Collects outgoing and incoming edge node IDs for a given node.
    pub(super) fn collect_connection_ids(graph: &RuleGraph, node_id: u64) -> (Vec<u64>, Vec<u64>) {
        let mut outgoing: Vec<u64> = graph
            .edges
            .iter()
            .filter(|edge| edge.from == node_id)
            .map(|edge| edge.to)
            .collect();
        outgoing.sort_unstable();
        outgoing.dedup();

        let mut incoming: Vec<u64> = graph
            .edges
            .iter()
            .filter(|edge| edge.to == node_id)
            .map(|edge| edge.from)
            .collect();
        incoming.sort_unstable();
        incoming.dedup();

        (outgoing, incoming)
    }

    /// Builds list of nodes that can be connected to/from current node.
    pub(super) fn build_connectable_nodes(
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        node_id: u64,
        connected_ids: &[u64],
        connect_from: bool,
    ) -> Vec<(u64, String)> {
        graph
            .nodes
            .iter()
            .filter_map(|node| {
                if node.id == node_id || connected_ids.contains(&node.id) {
                    return None;
                }
                let can_connect = if connect_from {
                    graph.can_connect_nodes(node.id, node_id).is_ok()
                } else {
                    graph.can_connect_nodes(node_id, node.id).is_ok()
                };
                if !can_connect {
                    return None;
                }
                Self::rule_graph_node_label_for_inspector(graph, node_badges, node.id)
                    .map(|label| (node.id, label))
            })
            .collect()
    }

    /// Renders the action buttons grid (Disconnect Node, Delete Node, Connect From/To).
    pub(super) fn render_node_action_buttons(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_id: u64,
        graph: &mut RuleGraph,
        connectable_from: &[(u64, String)],
        connectable_to: &[(u64, String)],
    ) -> NodeActionResult {
        let mut result = NodeActionResult::default();

        ui.push_id(("graph_node_action_buttons", scene_name, node_id), |ui| {
            egui::Grid::new("graph_node_action_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    Self::render_disconnect_delete_buttons(ui, node_id, graph, &mut result);
                    ui.end_row();
                    Self::render_connect_menus(ui, connectable_from, connectable_to, &mut result);
                    ui.end_row();
                });
        });

        result
    }

    fn render_disconnect_delete_buttons(
        ui: &mut egui::Ui,
        node_id: u64,
        graph: &mut RuleGraph,
        result: &mut NodeActionResult,
    ) {
        if ui.button("Disconnect Node").clicked() {
            if let Err(error) = graph.disconnect_node(node_id) {
                result.error = Some(format!("Failed to disconnect node: {:?}", error));
            } else {
                result.mutated = true;
            }
        }
        if ui
            .add(egui::Button::new("Delete Node").fill(egui::Color32::from_rgb(120, 30, 30)))
            .clicked()
        {
            if let Err(error) = graph.remove_node(node_id) {
                result.error = Some(format!("Failed to delete node: {:?}", error));
            } else {
                result.mutated = true;
            }
        }
    }

    fn render_connect_menus(
        ui: &mut egui::Ui,
        connectable_from: &[(u64, String)],
        connectable_to: &[(u64, String)],
        result: &mut NodeActionResult,
    ) {
        ui.menu_button("Connect From", |ui| {
            if connectable_from.is_empty() {
                ui.label("No available nodes");
                return;
            }
            for (candidate_id, label) in connectable_from {
                if ui.button(label).clicked() {
                    result.pending_connect_from = Some(*candidate_id);
                    ui.close();
                }
            }
        });
        ui.menu_button("Connect To", |ui| {
            if connectable_to.is_empty() {
                ui.label("No available nodes");
                return;
            }
            for (candidate_id, label) in connectable_to {
                if ui.button(label).clicked() {
                    result.pending_connect_to = Some(*candidate_id);
                    ui.close();
                }
            }
        });
    }

    /// Renders the connections list (incoming and outgoing edges).
    pub(super) fn render_connections_list(
        ui: &mut egui::Ui,
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        node_id: u64,
    ) -> Option<(u64, u64)> {
        let outgoing: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| edge.from == node_id)
            .copied()
            .collect();
        let incoming: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| edge.to == node_id)
            .copied()
            .collect();

        ui.label("Connections");
        if outgoing.is_empty() && incoming.is_empty() {
            ui.label("None");
            return None;
        }

        let mut pending_disconnect = None;
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                Self::render_edge_list(
                    ui,
                    graph,
                    node_badges,
                    &outgoing,
                    true,
                    &mut pending_disconnect,
                );
                Self::render_edge_list(
                    ui,
                    graph,
                    node_badges,
                    &incoming,
                    false,
                    &mut pending_disconnect,
                );
            });
        pending_disconnect
    }

    fn render_edge_list(
        ui: &mut egui::Ui,
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        edges: &[RuleGraphEdge],
        is_outgoing: bool,
        pending_disconnect: &mut Option<(u64, u64)>,
    ) {
        if edges.is_empty() {
            return;
        }
        ui.label(if is_outgoing { "Outgoing" } else { "Incoming" });
        for edge in edges {
            let target_id = if is_outgoing { edge.to } else { edge.from };
            let label = Self::rule_graph_node_label_for_inspector(graph, node_badges, target_id)
                .unwrap_or_else(|| format!("node {}", target_id));
            let prefix = if is_outgoing { "->" } else { "<-" };
            ui.horizontal(|ui| {
                ui.label(format!("{} {}", prefix, label));
                if ui.small_button("Disconnect").clicked() {
                    *pending_disconnect = Some((edge.from, edge.to));
                }
            });
        }
    }

    /// Processes pending connection operations.
    pub(super) fn process_pending_operations(
        graph: &mut RuleGraph,
        node_id: u64,
        action_result: &NodeActionResult,
        pending_disconnect: Option<(u64, u64)>,
    ) -> (bool, Option<String>) {
        let mut mutated = action_result.mutated;
        let mut error = action_result.error.clone();

        if let Some((from, to)) = pending_disconnect {
            if graph.disconnect_nodes(from, to) {
                mutated = true;
            } else {
                error = Some("Failed to disconnect selected connection".to_string());
            }
        }
        if let Some(connect_from) = action_result.pending_connect_from {
            if let Err(e) = graph.connect_nodes(connect_from, node_id) {
                error = Some(format!("Failed to connect nodes: {:?}", e));
            } else {
                mutated = true;
            }
        }
        if let Some(connect_to) = action_result.pending_connect_to {
            if let Err(e) = graph.connect_nodes(node_id, connect_to) {
                error = Some(format!("Failed to connect nodes: {:?}", e));
            } else {
                mutated = true;
            }
        }

        (mutated, error)
    }
}
