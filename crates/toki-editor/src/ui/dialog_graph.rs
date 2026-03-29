use crate::project::DialogGraphLayout;
use crate::ui::graph_canvas::{
    GraphCanvasBadge, GraphCanvasEdge, GraphCanvasNode, GraphCanvasNodeStyle, GraphCanvasPort,
};
use egui::Color32;
use std::collections::{HashMap, HashSet, VecDeque};
use toki_core::dialog::{
    DialogBranch, DialogChoice, DialogCondition, DialogNode, DialogNodeKind, DialogTree,
};

const AUTO_LAYOUT_START_X: f32 = 80.0;
const AUTO_LAYOUT_START_Y: f32 = 80.0;
const AUTO_LAYOUT_STEP_X: f32 = 320.0;
const AUTO_LAYOUT_STEP_Y: f32 = 160.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DialogGraphEdgeKind {
    LineNext,
    Choice { index: usize },
    Branch { index: usize },
    DefaultBranch,
}

impl DialogGraphEdgeKind {
    pub(crate) fn to_port_id(&self) -> String {
        match self {
            Self::LineNext => "line_next".to_string(),
            Self::Choice { index } => format!("choice:{index}"),
            Self::Branch { index } => format!("branch:{index}"),
            Self::DefaultBranch => "branch:default".to_string(),
        }
    }

    pub(crate) fn from_port_id(port_id: &str) -> Option<Self> {
        if port_id == "line_next" {
            return Some(Self::LineNext);
        }
        if port_id == "branch:default" {
            return Some(Self::DefaultBranch);
        }
        if let Some(index) = port_id.strip_prefix("choice:") {
            return index
                .parse::<usize>()
                .ok()
                .map(|index| Self::Choice { index });
        }
        if let Some(index) = port_id.strip_prefix("branch:") {
            return index
                .parse::<usize>()
                .ok()
                .map(|index| Self::Branch { index });
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DialogGraphPortView {
    pub edge_kind: DialogGraphEdgeKind,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DialogGraphNodePayload {
    Line {
        speaker_name: Option<String>,
        conditions: Vec<DialogCondition>,
        body: String,
        next_node_id: Option<String>,
    },
    Choice {
        speaker_name: Option<String>,
        conditions: Vec<DialogCondition>,
        body: String,
        choices: Vec<DialogChoice>,
    },
    Branch {
        speaker_name: Option<String>,
        conditions: Vec<DialogCondition>,
        branches: Vec<DialogBranch>,
        default_next_node_id: Option<String>,
    },
    End {
        speaker_name: Option<String>,
        conditions: Vec<DialogCondition>,
        body: String,
        outcome_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DialogGraphNodeView {
    pub node_id: String,
    pub kind_label: String,
    pub title: String,
    pub summary: String,
    pub position: [f32; 2],
    pub ports: Vec<DialogGraphPortView>,
    pub is_entry: bool,
    pub is_unreachable: bool,
    pub has_invalid: bool,
    pub is_conditional: bool,
    pub is_branching: bool,
    pub payload: DialogGraphNodePayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DialogGraphEdgeView {
    pub from_node_id: String,
    pub to_node_id: String,
    pub edge_kind: DialogGraphEdgeKind,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DialogGraphDocument {
    pub entry_node_id: String,
    pub nodes: Vec<DialogGraphNodeView>,
    pub edges: Vec<DialogGraphEdgeView>,
}

impl DialogGraphDocument {
    pub(crate) fn from_dialog(dialog: &DialogTree, layout: Option<&DialogGraphLayout>) -> Self {
        let validation = dialog.validate();
        let auto_layout = auto_layout_positions(dialog);
        let saved_positions = layout.map(|layout| &layout.node_positions);

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for node in &dialog.nodes {
            let unreachable = validation
                .warnings
                .iter()
                .any(|warning| warning.contains(&format!("node '{}' is unreachable", node.id)));
            let invalid = validation
                .errors
                .iter()
                .chain(validation.warnings.iter())
                .any(|issue| issue.contains(&format!("node '{}'", node.id)));

            let position = saved_positions
                .and_then(|positions| positions.get(&node.id).copied())
                .or_else(|| auto_layout.node_positions.get(&node.id).copied())
                .unwrap_or([AUTO_LAYOUT_START_X, AUTO_LAYOUT_START_Y]);

            let (kind_label, summary, ports, payload, node_edges) = node_projection(node);
            let is_conditional = !node.conditions.is_empty()
                || match &node.kind {
                    DialogNodeKind::Choice { choices, .. } => {
                        choices.iter().any(|choice| !choice.conditions.is_empty())
                    }
                    DialogNodeKind::Branch { branches, .. } => {
                        branches.iter().any(|branch| !branch.conditions.is_empty())
                    }
                    DialogNodeKind::Line { .. } | DialogNodeKind::End { .. } => false,
                };
            let is_branching = node.next_node_ids().len() > 1;

            nodes.push(DialogGraphNodeView {
                node_id: node.id.clone(),
                kind_label: kind_label.to_string(),
                title: node.id.clone(),
                summary,
                position,
                ports,
                is_entry: dialog.entry_node_id == node.id,
                is_unreachable: unreachable,
                has_invalid: invalid,
                is_conditional,
                is_branching,
                payload,
            });
            edges.extend(node_edges);
        }

        Self {
            entry_node_id: dialog.entry_node_id.clone(),
            nodes,
            edges,
        }
    }

    #[cfg(test)]
    pub(crate) fn to_dialog_tree(
        &self,
        id: toki_core::ids::DialogId,
        title: String,
        allow_cancel: bool,
        gate_gameplay: bool,
    ) -> DialogTree {
        DialogTree {
            id,
            title,
            entry_node_id: self.entry_node_id.clone(),
            allow_cancel,
            gate_gameplay,
            nodes: self
                .nodes
                .iter()
                .map(|node| {
                    let (speaker_name, conditions, kind) = match &node.payload {
                        DialogGraphNodePayload::Line {
                            speaker_name,
                            conditions,
                            body,
                            next_node_id,
                        } => (
                            speaker_name.clone(),
                            conditions.clone(),
                            DialogNodeKind::Line {
                                body: body.clone(),
                                next_node_id: next_node_id.clone(),
                            },
                        ),
                        DialogGraphNodePayload::Choice {
                            speaker_name,
                            conditions,
                            body,
                            choices,
                        } => (
                            speaker_name.clone(),
                            conditions.clone(),
                            DialogNodeKind::Choice {
                                body: body.clone(),
                                choices: choices.clone(),
                            },
                        ),
                        DialogGraphNodePayload::Branch {
                            speaker_name,
                            conditions,
                            branches,
                            default_next_node_id,
                        } => (
                            speaker_name.clone(),
                            conditions.clone(),
                            DialogNodeKind::Branch {
                                branches: branches.clone(),
                                default_next_node_id: default_next_node_id.clone(),
                            },
                        ),
                        DialogGraphNodePayload::End {
                            speaker_name,
                            conditions,
                            body,
                            outcome_id,
                        } => (
                            speaker_name.clone(),
                            conditions.clone(),
                            DialogNodeKind::End {
                                body: body.clone(),
                                outcome_id: outcome_id.clone(),
                            },
                        ),
                    };
                    DialogNode {
                        id: node.node_id.clone(),
                        speaker_name,
                        conditions,
                        kind,
                    }
                })
                .collect(),
        }
    }

    pub(crate) fn as_canvas_nodes(&self) -> Vec<GraphCanvasNode> {
        self.nodes
            .iter()
            .map(|node| GraphCanvasNode {
                id: node.node_id.clone(),
                kind_label: node.kind_label.clone(),
                title: node.title.clone(),
                summary: node.summary.clone(),
                position: node.position,
                badges: node_badges(node),
                style: node_style(&node.kind_label),
                output_ports: node
                    .ports
                    .iter()
                    .map(|port| GraphCanvasPort {
                        id: port.edge_kind.to_port_id(),
                        label: port.label.clone(),
                    })
                    .collect(),
            })
            .collect()
    }

    pub(crate) fn as_canvas_edges(&self) -> Vec<GraphCanvasEdge> {
        self.edges
            .iter()
            .map(|edge| GraphCanvasEdge {
                from_node_id: edge.from_node_id.clone(),
                from_port_id: edge.edge_kind.to_port_id(),
                to_node_id: edge.to_node_id.clone(),
                label: edge.label.clone(),
                highlight: false,
            })
            .collect()
    }
}

pub(crate) fn auto_layout_positions(dialog: &DialogTree) -> DialogGraphLayout {
    let node_map = dialog
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect::<HashMap<_, _>>();
    let mut positions = HashMap::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut depth_columns = HashMap::<usize, Vec<String>>::new();

    if node_map.contains_key(&dialog.entry_node_id) {
        queue.push_back((dialog.entry_node_id.clone(), 0usize));
    }

    while let Some((node_id, depth)) = queue.pop_front() {
        if !visited.insert(node_id.clone()) {
            continue;
        }
        depth_columns
            .entry(depth)
            .or_default()
            .push(node_id.clone());
        if let Some(node) = node_map.get(&node_id) {
            for next in node.next_node_ids() {
                if node_map.contains_key(next) {
                    queue.push_back((next.to_string(), depth + 1));
                }
            }
        }
    }

    let mut max_depth = 0usize;
    for (depth, node_ids) in &depth_columns {
        max_depth = max_depth.max(*depth);
        for (row, node_id) in node_ids.iter().enumerate() {
            positions.insert(
                node_id.clone(),
                [
                    AUTO_LAYOUT_START_X + *depth as f32 * AUTO_LAYOUT_STEP_X,
                    AUTO_LAYOUT_START_Y + row as f32 * AUTO_LAYOUT_STEP_Y,
                ],
            );
        }
    }

    let mut unreachable_row = 0usize;
    for node in &dialog.nodes {
        if positions.contains_key(&node.id) {
            continue;
        }
        positions.insert(
            node.id.clone(),
            [
                AUTO_LAYOUT_START_X + (max_depth as f32 + 2.0) * AUTO_LAYOUT_STEP_X,
                AUTO_LAYOUT_START_Y + unreachable_row as f32 * AUTO_LAYOUT_STEP_Y,
            ],
        );
        unreachable_row += 1;
    }

    DialogGraphLayout {
        node_positions: positions,
        ..DialogGraphLayout::default()
    }
}

pub(crate) fn normalize_dialog_graph_layout(
    dialog: &DialogTree,
    layout: Option<DialogGraphLayout>,
) -> DialogGraphLayout {
    let mut normalized = layout.unwrap_or_else(|| auto_layout_positions(dialog));
    let auto = auto_layout_positions(dialog);
    normalized
        .node_positions
        .retain(|node_id, _| dialog.nodes.iter().any(|node| node.id == *node_id));
    for node in &dialog.nodes {
        normalized
            .node_positions
            .entry(node.id.clone())
            .or_insert_with(|| {
                auto.node_positions
                    .get(&node.id)
                    .copied()
                    .unwrap_or([AUTO_LAYOUT_START_X, AUTO_LAYOUT_START_Y])
            });
    }
    normalized
}

pub(crate) fn rename_layout_node_key(
    layout: &mut DialogGraphLayout,
    old_node_id: &str,
    new_node_id: &str,
) {
    if old_node_id == new_node_id {
        return;
    }
    if let Some(position) = layout.node_positions.remove(old_node_id) {
        layout
            .node_positions
            .insert(new_node_id.to_string(), position);
    }
}

pub(crate) fn remove_layout_node_key(layout: &mut DialogGraphLayout, node_id: &str) {
    layout.node_positions.remove(node_id);
}

pub(crate) fn set_layout_node_position(
    layout: &mut DialogGraphLayout,
    node_id: &str,
    position: [f32; 2],
) {
    layout.node_positions.insert(node_id.to_string(), position);
}

pub(crate) fn unique_node_id(dialog: &DialogTree, prefix: &str) -> String {
    let mut index = 1usize;
    loop {
        let candidate = format!("{prefix}_{index}");
        if !dialog.nodes.iter().any(|node| node.id == candidate) {
            return candidate;
        }
        index += 1;
    }
}

pub(crate) fn create_line_node_at(
    dialog: &mut DialogTree,
    layout: &mut DialogGraphLayout,
    position: [f32; 2],
) -> String {
    let node_id = unique_node_id(dialog, "node");
    dialog.nodes.push(DialogNode {
        id: node_id.clone(),
        speaker_name: None,
        conditions: Vec::new(),
        kind: DialogNodeKind::Line {
            body: String::new(),
            next_node_id: None,
        },
    });
    set_layout_node_position(layout, &node_id, position);
    node_id
}

pub(crate) fn duplicate_node(
    dialog: &mut DialogTree,
    layout: &mut DialogGraphLayout,
    node_id: &str,
) -> Result<String, String> {
    let Some(node) = dialog.nodes.iter().find(|node| node.id == node_id).cloned() else {
        return Err(format!("Node '{node_id}' no longer exists."));
    };
    let prefix = if node_id.trim().is_empty() {
        "node"
    } else {
        node_id
    };
    let new_node_id = unique_node_id(dialog, prefix);
    let new_position = layout
        .node_positions
        .get(node_id)
        .copied()
        .map(|[x, y]| [x + 48.0, y + 32.0])
        .unwrap_or([AUTO_LAYOUT_START_X, AUTO_LAYOUT_START_Y]);
    let mut duplicate = node;
    duplicate.id = new_node_id.clone();
    dialog.nodes.push(duplicate);
    set_layout_node_position(layout, &new_node_id, new_position);
    Ok(new_node_id)
}

pub(crate) fn connect_edge(
    dialog: &mut DialogTree,
    from_node_id: &str,
    edge_kind: &DialogGraphEdgeKind,
    to_node_id: &str,
) -> Result<(), String> {
    if !dialog.nodes.iter().any(|node| node.id == to_node_id) {
        return Err(format!("Dialog node '{to_node_id}' does not exist."));
    }
    let Some(node) = dialog.nodes.iter_mut().find(|node| node.id == from_node_id) else {
        return Err(format!("Dialog node '{from_node_id}' does not exist."));
    };
    match (&mut node.kind, edge_kind) {
        (DialogNodeKind::Line { next_node_id, .. }, DialogGraphEdgeKind::LineNext) => {
            *next_node_id = Some(to_node_id.to_string());
        }
        (DialogNodeKind::Choice { choices, .. }, DialogGraphEdgeKind::Choice { index }) => {
            let Some(choice) = choices.get_mut(*index) else {
                return Err(format!(
                    "Choice port {} no longer exists on node '{}'.",
                    index + 1,
                    from_node_id
                ));
            };
            choice.next_node_id = to_node_id.to_string();
        }
        (DialogNodeKind::Branch { branches, .. }, DialogGraphEdgeKind::Branch { index }) => {
            let Some(branch) = branches.get_mut(*index) else {
                return Err(format!(
                    "Branch port {} no longer exists on node '{}'.",
                    index + 1,
                    from_node_id
                ));
            };
            branch.next_node_id = to_node_id.to_string();
        }
        (
            DialogNodeKind::Branch {
                default_next_node_id,
                ..
            },
            DialogGraphEdgeKind::DefaultBranch,
        ) => {
            *default_next_node_id = Some(to_node_id.to_string());
        }
        _ => {
            return Err(format!(
                "Port '{}' is not valid for dialog node '{}'.",
                edge_kind.to_port_id(),
                from_node_id
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn disconnect_edge(
    dialog: &mut DialogTree,
    from_node_id: &str,
    edge_kind: &DialogGraphEdgeKind,
) -> Result<(), String> {
    let Some(node) = dialog.nodes.iter_mut().find(|node| node.id == from_node_id) else {
        return Err(format!("Dialog node '{from_node_id}' does not exist."));
    };
    match (&mut node.kind, edge_kind) {
        (DialogNodeKind::Line { next_node_id, .. }, DialogGraphEdgeKind::LineNext) => {
            *next_node_id = None;
        }
        (DialogNodeKind::Choice { choices, .. }, DialogGraphEdgeKind::Choice { index }) => {
            let Some(choice) = choices.get_mut(*index) else {
                return Err(format!(
                    "Choice port {} no longer exists on node '{}'.",
                    index + 1,
                    from_node_id
                ));
            };
            choice.next_node_id.clear();
        }
        (DialogNodeKind::Branch { branches, .. }, DialogGraphEdgeKind::Branch { index }) => {
            let Some(branch) = branches.get_mut(*index) else {
                return Err(format!(
                    "Branch port {} no longer exists on node '{}'.",
                    index + 1,
                    from_node_id
                ));
            };
            branch.next_node_id.clear();
        }
        (
            DialogNodeKind::Branch {
                default_next_node_id,
                ..
            },
            DialogGraphEdgeKind::DefaultBranch,
        ) => {
            *default_next_node_id = None;
        }
        _ => {
            return Err(format!(
                "Port '{}' is not valid for dialog node '{}'.",
                edge_kind.to_port_id(),
                from_node_id
            ));
        }
    }
    Ok(())
}

pub(crate) fn disconnect_all_outgoing(
    dialog: &mut DialogTree,
    node_id: &str,
) -> Result<(), String> {
    let Some(node) = dialog.nodes.iter_mut().find(|node| node.id == node_id) else {
        return Err(format!("Dialog node '{node_id}' does not exist."));
    };
    match &mut node.kind {
        DialogNodeKind::Line { next_node_id, .. } => {
            *next_node_id = None;
        }
        DialogNodeKind::Choice { choices, .. } => {
            for choice in choices {
                choice.next_node_id.clear();
            }
        }
        DialogNodeKind::Branch {
            branches,
            default_next_node_id,
        } => {
            for branch in branches {
                branch.next_node_id.clear();
            }
            *default_next_node_id = None;
        }
        DialogNodeKind::End { .. } => {}
    }
    Ok(())
}

fn node_projection(
    node: &DialogNode,
) -> (
    &'static str,
    String,
    Vec<DialogGraphPortView>,
    DialogGraphNodePayload,
    Vec<DialogGraphEdgeView>,
) {
    match &node.kind {
        DialogNodeKind::Line { body, next_node_id } => (
            "Line",
            truncate_summary(body),
            vec![DialogGraphPortView {
                edge_kind: DialogGraphEdgeKind::LineNext,
                label: "Next".to_string(),
            }],
            DialogGraphNodePayload::Line {
                speaker_name: node.speaker_name.clone(),
                conditions: node.conditions.clone(),
                body: body.clone(),
                next_node_id: next_node_id.clone(),
            },
            next_node_id
                .iter()
                .map(|next_node_id| DialogGraphEdgeView {
                    from_node_id: node.id.clone(),
                    to_node_id: next_node_id.clone(),
                    edge_kind: DialogGraphEdgeKind::LineNext,
                    label: None,
                })
                .collect(),
        ),
        DialogNodeKind::Choice { body, choices } => (
            "Choice",
            format!("{} • {} choice(s)", truncate_summary(body), choices.len()),
            choices
                .iter()
                .enumerate()
                .map(|(index, choice)| DialogGraphPortView {
                    edge_kind: DialogGraphEdgeKind::Choice { index },
                    label: if choice.label.trim().is_empty() {
                        choice.id.clone()
                    } else {
                        choice.label.clone()
                    },
                })
                .collect(),
            DialogGraphNodePayload::Choice {
                speaker_name: node.speaker_name.clone(),
                conditions: node.conditions.clone(),
                body: body.clone(),
                choices: choices.clone(),
            },
            choices
                .iter()
                .enumerate()
                .map(|(index, choice)| DialogGraphEdgeView {
                    from_node_id: node.id.clone(),
                    to_node_id: choice.next_node_id.clone(),
                    edge_kind: DialogGraphEdgeKind::Choice { index },
                    label: Some(if choice.label.trim().is_empty() {
                        choice.id.clone()
                    } else {
                        choice.label.clone()
                    }),
                })
                .collect(),
        ),
        DialogNodeKind::Branch {
            branches,
            default_next_node_id,
        } => {
            let mut ports = branches
                .iter()
                .enumerate()
                .map(|(index, _branch)| DialogGraphPortView {
                    edge_kind: DialogGraphEdgeKind::Branch { index },
                    label: format!("Branch {}", index + 1),
                })
                .collect::<Vec<_>>();
            ports.push(DialogGraphPortView {
                edge_kind: DialogGraphEdgeKind::DefaultBranch,
                label: "Default".to_string(),
            });

            let mut edges = branches
                .iter()
                .enumerate()
                .map(|(index, branch)| DialogGraphEdgeView {
                    from_node_id: node.id.clone(),
                    to_node_id: branch.next_node_id.clone(),
                    edge_kind: DialogGraphEdgeKind::Branch { index },
                    label: Some(format!("Branch {}", index + 1)),
                })
                .collect::<Vec<_>>();
            if let Some(default_next_node_id) = default_next_node_id {
                edges.push(DialogGraphEdgeView {
                    from_node_id: node.id.clone(),
                    to_node_id: default_next_node_id.clone(),
                    edge_kind: DialogGraphEdgeKind::DefaultBranch,
                    label: Some("Default".to_string()),
                });
            }

            (
                "Branch",
                format!("{} branch(es)", branches.len()),
                ports,
                DialogGraphNodePayload::Branch {
                    speaker_name: node.speaker_name.clone(),
                    conditions: node.conditions.clone(),
                    branches: branches.clone(),
                    default_next_node_id: default_next_node_id.clone(),
                },
                edges,
            )
        }
        DialogNodeKind::End { body, outcome_id } => (
            "End",
            outcome_id
                .as_ref()
                .map(|outcome| format!("Outcome: {outcome}"))
                .unwrap_or_else(|| truncate_summary(body)),
            Vec::new(),
            DialogGraphNodePayload::End {
                speaker_name: node.speaker_name.clone(),
                conditions: node.conditions.clone(),
                body: body.clone(),
                outcome_id: outcome_id.clone(),
            },
            Vec::new(),
        ),
    }
}

fn node_badges(node: &DialogGraphNodeView) -> Vec<GraphCanvasBadge> {
    let mut badges = Vec::new();
    if node.is_entry {
        badges.push(GraphCanvasBadge {
            label: "Entry".to_string(),
            color: Color32::from_rgb(255, 211, 94),
        });
    }
    if node.has_invalid {
        badges.push(GraphCanvasBadge {
            label: "Invalid".to_string(),
            color: Color32::from_rgb(255, 118, 117),
        });
    }
    if node.is_unreachable {
        badges.push(GraphCanvasBadge {
            label: "Detached".to_string(),
            color: Color32::from_rgb(255, 184, 108),
        });
    }
    if node.is_conditional {
        badges.push(GraphCanvasBadge {
            label: "Conditional".to_string(),
            color: Color32::from_rgb(139, 214, 255),
        });
    }
    if node.is_branching {
        badges.push(GraphCanvasBadge {
            label: "Branching".to_string(),
            color: Color32::from_rgb(151, 211, 163),
        });
    }
    badges
}

fn node_style(kind_label: &str) -> GraphCanvasNodeStyle {
    match kind_label {
        "Line" => GraphCanvasNodeStyle {
            fill: Color32::from_rgb(45, 122, 199),
            stroke: Color32::from_rgb(140, 190, 245),
        },
        "Choice" => GraphCanvasNodeStyle {
            fill: Color32::from_rgb(92, 72, 166),
            stroke: Color32::from_rgb(178, 165, 240),
        },
        "Branch" => GraphCanvasNodeStyle {
            fill: Color32::from_rgb(139, 92, 46),
            stroke: Color32::from_rgb(214, 158, 106),
        },
        "End" => GraphCanvasNodeStyle {
            fill: Color32::from_rgb(58, 140, 82),
            stroke: Color32::from_rgb(133, 208, 154),
        },
        _ => GraphCanvasNodeStyle {
            fill: Color32::from_rgb(74, 84, 98),
            stroke: Color32::from_rgb(180, 186, 194),
        },
    }
}

fn truncate_summary(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "Empty".to_string();
    }
    let line = trimmed.lines().next().unwrap_or(trimmed);
    if line.chars().count() > 36 {
        format!("{}...", line.chars().take(33).collect::<String>())
    } else {
        line.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample_dialog() -> DialogTree {
        DialogTree {
            id: "intro".to_string().into(),
            title: "Intro".to_string(),
            entry_node_id: "start".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "start".to_string(),
                    speaker_name: Some("Guide".to_string()),
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Line {
                        body: "Hello there".to_string(),
                        next_node_id: Some("choice".to_string()),
                    },
                },
                DialogNode {
                    id: "choice".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Choice {
                        body: "Pick one".to_string(),
                        choices: vec![
                            DialogChoice {
                                id: "yes".to_string(),
                                label: "Yes".to_string(),
                                next_node_id: "end_yes".to_string(),
                                conditions: Vec::new(),
                            },
                            DialogChoice {
                                id: "no".to_string(),
                                label: "No".to_string(),
                                next_node_id: "end_no".to_string(),
                                conditions: Vec::new(),
                            },
                        ],
                    },
                },
                DialogNode {
                    id: "end_yes".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: String::new(),
                        outcome_id: Some("accepted".to_string()),
                    },
                },
                DialogNode {
                    id: "end_no".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: String::new(),
                        outcome_id: Some("declined".to_string()),
                    },
                },
            ],
        }
    }

    #[test]
    fn dialog_tree_round_trips_through_graph_document() {
        let dialog = sample_dialog();
        let document = DialogGraphDocument::from_dialog(&dialog, None);
        let roundtrip = document.to_dialog_tree(
            dialog.id.clone(),
            dialog.title.clone(),
            dialog.allow_cancel,
            dialog.gate_gameplay,
        );
        assert_eq!(roundtrip, dialog);
    }

    #[test]
    fn auto_layout_places_unreachable_nodes_in_detached_column() {
        let mut dialog = sample_dialog();
        dialog.nodes.push(DialogNode {
            id: "unused".to_string(),
            speaker_name: None,
            conditions: Vec::new(),
            kind: DialogNodeKind::End {
                body: String::new(),
                outcome_id: None,
            },
        });

        let layout = auto_layout_positions(&dialog);
        let reachable_x = layout
            .node_positions
            .get("start")
            .expect("start position")
            .first()
            .copied()
            .unwrap_or_default();
        let unreachable_x = layout
            .node_positions
            .get("unused")
            .expect("unused position")
            .first()
            .copied()
            .unwrap_or_default();
        assert!(unreachable_x > reachable_x + AUTO_LAYOUT_STEP_X);
    }

    #[test]
    fn dialog_edge_kind_port_ids_round_trip() {
        for edge_kind in [
            DialogGraphEdgeKind::LineNext,
            DialogGraphEdgeKind::Choice { index: 1 },
            DialogGraphEdgeKind::Branch { index: 2 },
            DialogGraphEdgeKind::DefaultBranch,
        ] {
            let port_id = edge_kind.to_port_id();
            assert_eq!(DialogGraphEdgeKind::from_port_id(&port_id), Some(edge_kind));
        }
    }

    #[test]
    fn duplicate_node_offsets_layout_and_preserves_payload() {
        let mut dialog = sample_dialog();
        let mut layout = auto_layout_positions(&dialog);
        let original_position = layout
            .node_positions
            .get("choice")
            .copied()
            .expect("choice");

        let duplicate_id = duplicate_node(&mut dialog, &mut layout, "choice").expect("duplicate");

        assert!(dialog.nodes.iter().any(|node| node.id == duplicate_id));
        assert_eq!(
            layout.node_positions.get(&duplicate_id).copied(),
            Some([original_position[0] + 48.0, original_position[1] + 32.0])
        );
    }

    #[test]
    fn connect_and_disconnect_edge_update_underlying_dialog_tree() {
        let mut dialog = sample_dialog();
        connect_edge(
            &mut dialog,
            "start",
            &DialogGraphEdgeKind::LineNext,
            "end_yes",
        )
        .expect("connect");
        let Some(DialogNode {
            kind: DialogNodeKind::Line { next_node_id, .. },
            ..
        }) = dialog.nodes.iter().find(|node| node.id == "start")
        else {
            panic!("start node");
        };
        assert_eq!(next_node_id.as_deref(), Some("end_yes"));

        disconnect_edge(&mut dialog, "start", &DialogGraphEdgeKind::LineNext).expect("disconnect");
        let Some(DialogNode {
            kind: DialogNodeKind::Line { next_node_id, .. },
            ..
        }) = dialog.nodes.iter().find(|node| node.id == "start")
        else {
            panic!("start node");
        };
        assert_eq!(next_node_id, &None);
    }

    #[test]
    fn branch_projection_always_exposes_default_port() {
        let dialog = DialogTree {
            id: "branching".to_string().into(),
            title: String::new(),
            entry_node_id: "branch".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![DialogNode {
                id: "branch".to_string(),
                speaker_name: None,
                conditions: Vec::new(),
                kind: DialogNodeKind::Branch {
                    branches: vec![DialogBranch {
                        conditions: Vec::new(),
                        next_node_id: "end".to_string(),
                    }],
                    default_next_node_id: None,
                },
            }],
        };

        let document = DialogGraphDocument::from_dialog(&dialog, None);
        let branch = document
            .nodes
            .iter()
            .find(|node| node.node_id == "branch")
            .expect("branch");
        assert!(branch
            .ports
            .iter()
            .any(|port| port.edge_kind == DialogGraphEdgeKind::DefaultBranch));
    }
}
