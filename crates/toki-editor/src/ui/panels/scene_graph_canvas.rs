use super::*;
use crate::rule_graph_ui::{
    rule_graph_action_summary, rule_graph_condition_summary, rule_graph_node_badge_color,
    rule_graph_node_kind_type, rule_graph_trigger_summary, RuleGraphSummaryStyle,
};
use crate::ui::graph_canvas::{
    render_graph_canvas, GraphCanvasAction, GraphCanvasBadge, GraphCanvasEdge, GraphCanvasNode,
    GraphCanvasNodeStyle, GraphCanvasPort, GraphCanvasState,
};

impl PanelSystem {
    pub(super) fn render_graph_canvas(
        ui: &mut egui::Ui,
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        canvas_state: &mut GraphCanvasState,
        selected_node_id: Option<u64>,
    ) -> Vec<GraphCanvasAction> {
        let (nodes, edges) = rule_graph_to_canvas(graph, node_badges, selected_node_id);
        let selected_str = selected_node_id.map(|id| id.to_string());
        render_graph_canvas(ui, canvas_state, &nodes, &edges, selected_str.as_deref())
    }
}

fn rule_graph_to_canvas(
    graph: &RuleGraph,
    node_badges: &HashMap<u64, String>,
    selected_node_id: Option<u64>,
) -> (Vec<GraphCanvasNode>, Vec<GraphCanvasEdge>) {
    let nodes = graph
        .nodes
        .iter()
        .map(|node| {
            let badge = node_badges
                .get(&node.id)
                .cloned()
                .unwrap_or_else(|| "?".to_string());
            let title = match &node.kind {
                RuleGraphNodeKind::Trigger(t) => {
                    rule_graph_trigger_summary(t.clone(), RuleGraphSummaryStyle::Compact)
                }
                RuleGraphNodeKind::Condition(c) => {
                    rule_graph_condition_summary(c, RuleGraphSummaryStyle::Compact)
                }
                RuleGraphNodeKind::Action(a) => {
                    rule_graph_action_summary(a, RuleGraphSummaryStyle::Compact)
                }
            };
            let (fill, stroke) = node_style_colors(&node.kind);
            GraphCanvasNode {
                id: node.id.to_string(),
                kind_label: rule_graph_node_kind_type(&node.kind).to_string(),
                title,
                summary: String::new(),
                position: node.position,
                badges: vec![GraphCanvasBadge {
                    label: badge,
                    color: rule_graph_node_badge_color(&node.kind),
                }],
                style: GraphCanvasNodeStyle { fill, stroke },
                output_ports: vec![GraphCanvasPort {
                    id: "next".to_string(),
                    label: "Next".to_string(),
                }],
            }
        })
        .collect();

    let edges = graph
        .edges
        .iter()
        .map(|edge| {
            let highlight =
                selected_node_id == Some(edge.from) || selected_node_id == Some(edge.to);
            GraphCanvasEdge {
                from_node_id: edge.from.to_string(),
                from_port_id: "next".to_string(),
                to_node_id: edge.to.to_string(),
                label: None,
                highlight,
            }
        })
        .collect();

    (nodes, edges)
}

fn node_style_colors(kind: &RuleGraphNodeKind) -> (egui::Color32, egui::Color32) {
    match kind {
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
    }
}
