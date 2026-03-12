use std::collections::{HashMap, HashSet};

use toki_core::rules::{Rule, RuleAction, RuleCondition, RuleSet, RuleTrigger};

pub type RuleGraphNodeId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleGraphNodeKind {
    Trigger(RuleTrigger),
    Condition(RuleCondition),
    Action(RuleAction),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuleGraphNode {
    pub id: RuleGraphNodeId,
    pub kind: RuleGraphNodeKind,
    pub position: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleGraphEdge {
    pub from: RuleGraphNodeId,
    pub to: RuleGraphNodeId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleGraphChain {
    pub rule_id: String,
    pub enabled: bool,
    pub priority: i32,
    pub once: bool,
    pub trigger_node_id: RuleGraphNodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuleGraph {
    pub nodes: Vec<RuleGraphNode>,
    pub edges: Vec<RuleGraphEdge>,
    pub chains: Vec<RuleGraphChain>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleGraphError {
    MissingTriggerNode {
        rule_id: String,
        node_id: RuleGraphNodeId,
    },
    TriggerNodeKindMismatch {
        rule_id: String,
        node_id: RuleGraphNodeId,
    },
    MissingNode {
        rule_id: String,
        node_id: RuleGraphNodeId,
    },
    NonLinearChain {
        rule_id: String,
        node_id: RuleGraphNodeId,
    },
    CycleDetected {
        rule_id: String,
        node_id: RuleGraphNodeId,
    },
}

impl RuleGraph {
    pub fn from_rule_set(rule_set: &RuleSet) -> Self {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut chains = Vec::new();
        let mut next_node_id: RuleGraphNodeId = 1;

        for (rule_index, rule) in rule_set.rules.iter().enumerate() {
            let y = 40.0 + (rule_index as f32 * 140.0);
            let mut next_x = 40.0;

            let trigger_id = next_node_id;
            next_node_id += 1;
            nodes.push(RuleGraphNode {
                id: trigger_id,
                kind: RuleGraphNodeKind::Trigger(rule.trigger),
                position: [next_x, y],
            });
            next_x += 220.0;

            let mut previous_id = trigger_id;
            for condition in &rule.conditions {
                let node_id = next_node_id;
                next_node_id += 1;
                nodes.push(RuleGraphNode {
                    id: node_id,
                    kind: RuleGraphNodeKind::Condition(*condition),
                    position: [next_x, y],
                });
                edges.push(RuleGraphEdge {
                    from: previous_id,
                    to: node_id,
                });
                previous_id = node_id;
                next_x += 220.0;
            }

            for action in &rule.actions {
                let node_id = next_node_id;
                next_node_id += 1;
                nodes.push(RuleGraphNode {
                    id: node_id,
                    kind: RuleGraphNodeKind::Action(action.clone()),
                    position: [next_x, y],
                });
                edges.push(RuleGraphEdge {
                    from: previous_id,
                    to: node_id,
                });
                previous_id = node_id;
                next_x += 220.0;
            }

            chains.push(RuleGraphChain {
                rule_id: rule.id.clone(),
                enabled: rule.enabled,
                priority: rule.priority,
                once: rule.once,
                trigger_node_id: trigger_id,
            });
        }

        Self {
            nodes,
            edges,
            chains,
        }
    }

    pub fn to_rule_set(&self) -> Result<RuleSet, RuleGraphError> {
        let node_by_id = self
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<HashMap<_, _>>();
        let mut outgoing = HashMap::<RuleGraphNodeId, Vec<RuleGraphNodeId>>::new();
        for edge in &self.edges {
            outgoing.entry(edge.from).or_default().push(edge.to);
        }

        let mut rules = Vec::with_capacity(self.chains.len());
        for chain in &self.chains {
            let Some(trigger_node) = node_by_id.get(&chain.trigger_node_id).copied() else {
                return Err(RuleGraphError::MissingTriggerNode {
                    rule_id: chain.rule_id.clone(),
                    node_id: chain.trigger_node_id,
                });
            };
            let RuleGraphNodeKind::Trigger(trigger) = trigger_node.kind else {
                return Err(RuleGraphError::TriggerNodeKindMismatch {
                    rule_id: chain.rule_id.clone(),
                    node_id: chain.trigger_node_id,
                });
            };

            let mut conditions = Vec::new();
            let mut actions = Vec::new();
            let mut visited = HashSet::new();
            let mut current_id = chain.trigger_node_id;
            let mut has_seen_action = false;
            visited.insert(current_id);

            loop {
                let next_nodes = outgoing.get(&current_id).cloned().unwrap_or_default();
                if next_nodes.len() > 1 {
                    return Err(RuleGraphError::NonLinearChain {
                        rule_id: chain.rule_id.clone(),
                        node_id: current_id,
                    });
                }
                let Some(next_id) = next_nodes.first().copied() else {
                    break;
                };
                if !visited.insert(next_id) {
                    return Err(RuleGraphError::CycleDetected {
                        rule_id: chain.rule_id.clone(),
                        node_id: next_id,
                    });
                }

                let Some(next_node) = node_by_id.get(&next_id).copied() else {
                    return Err(RuleGraphError::MissingNode {
                        rule_id: chain.rule_id.clone(),
                        node_id: next_id,
                    });
                };

                match &next_node.kind {
                    RuleGraphNodeKind::Trigger(_) => {
                        return Err(RuleGraphError::NonLinearChain {
                            rule_id: chain.rule_id.clone(),
                            node_id: next_id,
                        });
                    }
                    RuleGraphNodeKind::Condition(condition) => {
                        if has_seen_action {
                            return Err(RuleGraphError::NonLinearChain {
                                rule_id: chain.rule_id.clone(),
                                node_id: next_id,
                            });
                        }
                        conditions.push(*condition);
                    }
                    RuleGraphNodeKind::Action(action) => {
                        has_seen_action = true;
                        actions.push(action.clone());
                    }
                }
                current_id = next_id;
            }

            rules.push(Rule {
                id: chain.rule_id.clone(),
                enabled: chain.enabled,
                priority: chain.priority,
                once: chain.once,
                trigger,
                conditions,
                actions,
            });
        }

        Ok(RuleSet { rules })
    }
}

#[cfg(test)]
mod tests {
    use toki_core::animation::AnimationState;
    use toki_core::rules::{
        RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel, RuleTarget, RuleTrigger,
    };

    use super::{RuleGraph, RuleGraphEdge, RuleGraphError};

    fn sample_rules() -> RuleSet {
        RuleSet {
            rules: vec![
                toki_core::rules::Rule {
                    id: "rule_spawn".to_string(),
                    enabled: true,
                    priority: 20,
                    once: false,
                    trigger: RuleTrigger::OnPlayerMove,
                    conditions: vec![
                        RuleCondition::KeyHeld {
                            key: RuleKey::Right,
                        },
                        RuleCondition::TargetExists {
                            target: RuleTarget::Player,
                        },
                    ],
                    actions: vec![
                        RuleAction::PlaySound {
                            channel: RuleSoundChannel::Movement,
                            sound_id: "sfx_step".to_string(),
                        },
                        RuleAction::Spawn {
                            entity_type: toki_core::rules::RuleSpawnEntityType::Item,
                            position: [10, 20],
                        },
                    ],
                },
                toki_core::rules::Rule {
                    id: "rule_music".to_string(),
                    enabled: false,
                    priority: -2,
                    once: true,
                    trigger: RuleTrigger::OnStart,
                    conditions: vec![RuleCondition::Always],
                    actions: vec![
                        RuleAction::PlayMusic {
                            track_id: "bgm_1".to_string(),
                        },
                        RuleAction::PlayAnimation {
                            target: RuleTarget::Player,
                            state: AnimationState::Idle,
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn rule_set_roundtrip_through_graph_is_lossless_and_deterministic() {
        let original = sample_rules();
        let graph = RuleGraph::from_rule_set(&original);
        let roundtrip = graph
            .to_rule_set()
            .expect("graph generated from rules should always be valid");

        assert_eq!(roundtrip, original);
    }

    #[test]
    fn graph_to_rule_set_rejects_branching_chains() {
        let base = sample_rules();
        let mut graph = RuleGraph::from_rule_set(&base);
        let Some(first_chain) = graph.chains.first() else {
            panic!("expected at least one chain");
        };
        let trigger_id = first_chain.trigger_node_id;
        let Some(trigger_edge_target) = graph
            .edges
            .iter()
            .find(|edge| edge.from == trigger_id)
            .map(|edge| edge.to)
        else {
            panic!("expected trigger edge");
        };
        graph.edges.push(RuleGraphEdge {
            from: trigger_id,
            to: trigger_edge_target,
        });

        let error = graph
            .to_rule_set()
            .expect_err("branching trigger node must be rejected");
        assert!(matches!(error, RuleGraphError::NonLinearChain { .. }));
    }
}
