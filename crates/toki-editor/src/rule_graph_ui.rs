use std::collections::HashMap;

use crate::ui::editor_domain::{rule_key_label, rule_sound_channel_label, rule_target_label};
use crate::ui::rule_graph::{RuleGraph, RuleGraphEdge, RuleGraphNodeKind};
use toki_core::rules::{RuleAction, RuleCondition, RuleTrigger};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleGraphSummaryStyle {
    Compact,
    Detailed,
}

#[derive(Default)]
pub struct NodeActionResult {
    pub mutated: bool,
    pub error: Option<String>,
    pub pending_connect_from: Option<u64>,
    pub pending_connect_to: Option<u64>,
}

pub fn rule_graph_node_badges(graph: &RuleGraph) -> HashMap<u64, String> {
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

pub fn rule_graph_node_label(
    graph: &RuleGraph,
    node_badges: &HashMap<u64, String>,
    node_id: u64,
    style: RuleGraphSummaryStyle,
) -> Option<String> {
    let node = graph.nodes.iter().find(|node| node.id == node_id)?;
    let badge = node_badges
        .get(&node_id)
        .cloned()
        .unwrap_or_else(|| "?".to_string());
    match style {
        RuleGraphSummaryStyle::Compact => Some(format!(
            "{}: {}",
            badge,
            match &node.kind {
                RuleGraphNodeKind::Trigger(trigger) => rule_graph_trigger_summary(*trigger, style),
                RuleGraphNodeKind::Condition(condition) => {
                    rule_graph_condition_summary(condition, style)
                }
                RuleGraphNodeKind::Action(action) => rule_graph_action_summary(action, style),
            }
        )),
        RuleGraphSummaryStyle::Detailed => {
            let details = match &node.kind {
                RuleGraphNodeKind::Trigger(trigger) => {
                    format!("Trigger {}", rule_graph_trigger_summary(*trigger, style))
                }
                RuleGraphNodeKind::Condition(condition) => {
                    format!("Condition {}", rule_graph_condition_summary(condition, style))
                }
                RuleGraphNodeKind::Action(action) => {
                    format!("Action {}", rule_graph_action_summary(action, style))
                }
            };
            Some(format!("{badge}: {details}"))
        }
    }
}

pub fn rule_graph_target_summary(target: toki_core::rules::RuleTarget) -> String {
    rule_target_label(target)
}

pub fn rule_graph_trigger_summary(trigger: RuleTrigger, style: RuleGraphSummaryStyle) -> String {
    match trigger {
        RuleTrigger::OnStart => "OnStart".to_string(),
        RuleTrigger::OnUpdate => "OnUpdate".to_string(),
        RuleTrigger::OnPlayerMove => "OnPlayerMove".to_string(),
        RuleTrigger::OnKey { key } => format!("OnKey({})", rule_key_label(key)),
        RuleTrigger::OnCollision { entity: None } => "OnCollision".to_string(),
        RuleTrigger::OnCollision {
            entity: Some(target),
        } => format!("OnCollision({})", rule_graph_target_summary(target)),
        RuleTrigger::OnDamaged { entity: None } => "OnDamaged".to_string(),
        RuleTrigger::OnDamaged {
            entity: Some(target),
        } => format!("OnDamaged({})", rule_graph_target_summary(target)),
        RuleTrigger::OnDeath { entity: None } => "OnDeath".to_string(),
        RuleTrigger::OnDeath {
            entity: Some(target),
        } => format!("OnDeath({})", rule_graph_target_summary(target)),
        RuleTrigger::OnTrigger => "OnTrigger".to_string(),
        RuleTrigger::OnInteract { entity: None, .. } => "OnInteract".to_string(),
        RuleTrigger::OnInteract {
            entity: Some(target),
            ..
        } => match style {
            RuleGraphSummaryStyle::Compact => {
                format!("OnInteract({})", rule_graph_target_summary(target))
            }
            RuleGraphSummaryStyle::Detailed => "OnInteract".to_string(),
        },
        RuleTrigger::OnTileEnter { x, y } => format!("OnTileEnter({}, {})", x, y),
        RuleTrigger::OnTileExit { x, y } => format!("OnTileExit({}, {})", x, y),
    }
}

pub fn rule_graph_condition_summary(
    condition: &RuleCondition,
    _style: RuleGraphSummaryStyle,
) -> String {
    match condition {
        RuleCondition::Always => "Always".to_string(),
        RuleCondition::TargetExists { target } => {
            format!("TargetExists({})", rule_graph_target_summary(*target))
        }
        RuleCondition::KeyHeld { key } => format!("KeyHeld({})", rule_key_label(*key)),
        RuleCondition::EntityActive { target, is_active } => match _style {
            RuleGraphSummaryStyle::Compact => format!(
                "EntityActive({}, active={})",
                rule_graph_target_summary(*target),
                is_active
            ),
            RuleGraphSummaryStyle::Detailed => format!(
                "EntityActive({}, {})",
                rule_graph_target_summary(*target),
                if *is_active { "true" } else { "false" }
            ),
        },
        RuleCondition::HealthBelow { target, threshold } => {
            format!("HealthBelow({}, {})", rule_graph_target_summary(*target), threshold)
        }
        RuleCondition::HealthAbove { target, threshold } => {
            format!("HealthAbove({}, {})", rule_graph_target_summary(*target), threshold)
        }
        RuleCondition::TriggerOtherIsPlayer => "TriggerOtherIsPlayer".to_string(),
        RuleCondition::EntityIsKind { target, kind } => {
            format!("EntityIsKind({}, {:?})", rule_graph_target_summary(*target), kind)
        }
        RuleCondition::TriggerOtherIsKind { kind } => format!("TriggerOtherIsKind({:?})", kind),
        RuleCondition::EntityHasTag { target, tag } => {
            format!("EntityHasTag({}, {})", rule_graph_target_summary(*target), tag)
        }
        RuleCondition::TriggerOtherHasTag { tag } => format!("TriggerOtherHasTag({})", tag),
        RuleCondition::HasInventoryItem {
            target,
            item_id,
            min_count,
        } => format!(
            "HasInventoryItem({}, {}, {})",
            rule_graph_target_summary(*target),
            item_id,
            min_count
        ),
    }
}

pub fn rule_graph_action_summary(action: &RuleAction, style: RuleGraphSummaryStyle) -> String {
    match action {
        RuleAction::PlaySound { channel, sound_id } => match style {
            RuleGraphSummaryStyle::Compact => format!(
                "PlaySound({}, {})",
                rule_sound_channel_label(*channel),
                sound_id
            ),
            RuleGraphSummaryStyle::Detailed => format!(
                "PlaySound({}, {})",
                rule_sound_channel_label(*channel),
                if sound_id.is_empty() { "<empty>" } else { sound_id }
            ),
        },
        RuleAction::PlayMusic { track_id } => {
            let track = if matches!(style, RuleGraphSummaryStyle::Detailed) && track_id.is_empty() {
                "<empty>"
            } else {
                track_id
            };
            format!("PlayMusic({track})")
        }
        RuleAction::PlayAnimation { target, state } => {
            format!("PlayAnimation({}, {:?})", rule_graph_target_summary(*target), state)
        }
        RuleAction::SetVelocity { target, velocity } => match style {
            RuleGraphSummaryStyle::Compact => format!(
                "SetVelocity({}, {}, {})",
                rule_graph_target_summary(*target),
                velocity[0],
                velocity[1]
            ),
            RuleGraphSummaryStyle::Detailed => format!(
                "SetVelocity({}, [{}, {}])",
                rule_graph_target_summary(*target),
                velocity[0],
                velocity[1]
            ),
        },
        RuleAction::Spawn {
            entity_type,
            position,
        } => match style {
            RuleGraphSummaryStyle::Compact => {
                format!("Spawn({:?}, {}, {})", entity_type, position[0], position[1])
            }
            RuleGraphSummaryStyle::Detailed => {
                format!("Spawn({:?}, [{}, {}])", entity_type, position[0], position[1])
            }
        },
        RuleAction::DestroySelf { target } => {
            format!("DestroySelf({})", rule_graph_target_summary(*target))
        }
        RuleAction::SwitchScene {
            scene_name,
            spawn_point_id,
        } => {
            let scene = if matches!(style, RuleGraphSummaryStyle::Detailed) && scene_name.is_empty()
            {
                "<empty>"
            } else {
                scene_name
            };
            let spawn =
                if matches!(style, RuleGraphSummaryStyle::Detailed) && spawn_point_id.is_empty() {
                    "<empty>"
                } else {
                    spawn_point_id
                };
            format!("SwitchScene({scene} -> {spawn})")
        }
        RuleAction::DamageEntity { target, amount } => {
            format!("DamageEntity({}, {})", rule_graph_target_summary(*target), amount)
        }
        RuleAction::HealEntity { target, amount } => {
            format!("HealEntity({}, {})", rule_graph_target_summary(*target), amount)
        }
        RuleAction::AddInventoryItem {
            target,
            item_id,
            count,
        } => {
            let item = if matches!(style, RuleGraphSummaryStyle::Detailed) && item_id.is_empty() {
                "<empty>"
            } else {
                item_id
            };
            format!(
                "AddItem({}, {}, {})",
                rule_graph_target_summary(*target),
                item,
                count
            )
        }
        RuleAction::RemoveInventoryItem {
            target,
            item_id,
            count,
        } => {
            let item = if matches!(style, RuleGraphSummaryStyle::Detailed) && item_id.is_empty() {
                "<empty>"
            } else {
                item_id
            };
            format!(
                "RemoveItem({}, {}, {})",
                rule_graph_target_summary(*target),
                item,
                count
            )
        }
        RuleAction::SetEntityActive { target, active } => {
            format!("SetActive({}, {})", rule_graph_target_summary(*target), active)
        }
        RuleAction::TeleportEntity {
            target,
            tile_x,
            tile_y,
        } => format!(
            "Teleport({}, tile[{}, {}])",
            rule_graph_target_summary(*target),
            tile_x,
            tile_y
        ),
    }
}

pub fn collect_connection_ids(graph: &RuleGraph, node_id: u64) -> (Vec<u64>, Vec<u64>) {
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

pub fn build_connectable_nodes(
    graph: &RuleGraph,
    node_badges: &HashMap<u64, String>,
    node_id: u64,
    connected_ids: &[u64],
    connect_from: bool,
    style: RuleGraphSummaryStyle,
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
            rule_graph_node_label(graph, node_badges, node.id, style).map(|label| (node.id, label))
        })
        .collect()
}

pub fn render_node_action_buttons(
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
                if ui.button("Disconnect Node").clicked() {
                    if let Err(error) = graph.disconnect_node(node_id) {
                        result.error = Some(format!("Failed to disconnect node: {:?}", error));
                    } else {
                        result.mutated = true;
                    }
                }
                if ui
                    .add(
                        egui::Button::new("Delete Node")
                            .fill(egui::Color32::from_rgb(120, 30, 30)),
                    )
                    .clicked()
                {
                    if let Err(error) = graph.remove_node(node_id) {
                        result.error = Some(format!("Failed to delete node: {:?}", error));
                    } else {
                        result.mutated = true;
                    }
                }
                ui.end_row();
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
                ui.end_row();
            });
    });

    result
}

pub fn render_connections_list(
    ui: &mut egui::Ui,
    graph: &RuleGraph,
    node_badges: &HashMap<u64, String>,
    node_id: u64,
    style: RuleGraphSummaryStyle,
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
    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
        render_edge_list(
            ui,
            graph,
            node_badges,
            &outgoing,
            true,
            &mut pending_disconnect,
            style,
        );
        render_edge_list(
            ui,
            graph,
            node_badges,
            &incoming,
            false,
            &mut pending_disconnect,
            style,
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
    style: RuleGraphSummaryStyle,
) {
    if edges.is_empty() {
        return;
    }
    ui.label(if is_outgoing { "Outgoing" } else { "Incoming" });
    for edge in edges {
        let target_id = if is_outgoing { edge.to } else { edge.from };
        let label = rule_graph_node_label(graph, node_badges, target_id, style)
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

pub fn process_pending_operations(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::rule_graph::RuleGraph;
    use toki_core::rules::{Rule, RuleAction, RuleCondition, RuleSet, RuleTrigger};

    fn sample_graph() -> RuleGraph {
        RuleGraph::from_rule_set(&RuleSet {
            rules: vec![Rule {
                id: "rule_1".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnStart,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::PlayMusic {
                    track_id: "bgm".to_string(),
                }],
            }],
        })
    }

    #[test]
    fn node_badges_assign_expected_prefixes() {
        let graph = sample_graph();
        let badges = rule_graph_node_badges(&graph);
        assert!(badges.values().any(|badge| badge.starts_with('T')));
        assert!(badges.values().any(|badge| badge.starts_with('C')));
        assert!(badges.values().any(|badge| badge.starts_with('A')));
    }

    #[test]
    fn build_connectable_nodes_excludes_existing_edges_and_self() {
        let mut graph = sample_graph();
        let condition_id = graph
            .nodes
            .iter()
            .find(|node| matches!(node.kind, RuleGraphNodeKind::Condition(_)))
            .expect("condition node")
            .id;
        let action_id = graph
            .nodes
            .iter()
            .find(|node| matches!(node.kind, RuleGraphNodeKind::Action(_)))
            .expect("action node")
            .id;
        let extra_action_id = graph
            .add_action_node(RuleAction::PlayMusic {
                track_id: "extra".to_string(),
            })
            .expect("new action");
        let badges = rule_graph_node_badges(&graph);
        let (outgoing, _) = collect_connection_ids(&graph, condition_id);

        let connectable = build_connectable_nodes(
            &graph,
            &badges,
            condition_id,
            &outgoing,
            false,
            RuleGraphSummaryStyle::Compact,
        );

        assert!(!connectable.iter().any(|(id, _)| *id == condition_id));
        assert!(!connectable.iter().any(|(id, _)| *id == action_id));
        assert!(connectable.iter().any(|(id, _)| *id == extra_action_id));
    }
}
