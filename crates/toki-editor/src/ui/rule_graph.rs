use std::collections::{HashMap, HashSet};

use toki_core::rules::{Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleTrigger};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleGraphEditError {
    GraphInvalid(RuleGraphError),
    MissingChain { trigger_node_id: RuleGraphNodeId },
    MissingNode { node_id: RuleGraphNodeId },
    InvalidConnection { reason: String },
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

    pub fn add_default_chain(&mut self) -> Result<String, RuleGraphEditError> {
        let mut rules = self
            .to_rule_set()
            .map_err(RuleGraphEditError::GraphInvalid)?;
        let rule_id = Self::next_rule_id(&rules);
        rules.rules.push(Rule {
            id: rule_id.clone(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_placeholder".to_string(),
            }],
        });
        *self = Self::from_rule_set(&rules);
        Ok(rule_id)
    }

    pub fn append_condition_to_chain(
        &mut self,
        trigger_node_id: RuleGraphNodeId,
        condition: RuleCondition,
    ) -> Result<(), RuleGraphEditError> {
        let mut rules = self
            .to_rule_set()
            .map_err(RuleGraphEditError::GraphInvalid)?;
        let rule_id = self
            .chains
            .iter()
            .find(|chain| chain.trigger_node_id == trigger_node_id)
            .map(|chain| chain.rule_id.clone())
            .ok_or(RuleGraphEditError::MissingChain { trigger_node_id })?;
        let rule = rules
            .rules
            .iter_mut()
            .find(|rule| rule.id == rule_id)
            .ok_or(RuleGraphEditError::MissingChain { trigger_node_id })?;
        rule.conditions.push(condition);
        *self = Self::from_rule_set(&rules);
        Ok(())
    }

    pub fn append_action_to_chain(
        &mut self,
        trigger_node_id: RuleGraphNodeId,
        action: RuleAction,
    ) -> Result<(), RuleGraphEditError> {
        let mut rules = self
            .to_rule_set()
            .map_err(RuleGraphEditError::GraphInvalid)?;
        let rule_id = self
            .chains
            .iter()
            .find(|chain| chain.trigger_node_id == trigger_node_id)
            .map(|chain| chain.rule_id.clone())
            .ok_or(RuleGraphEditError::MissingChain { trigger_node_id })?;
        let rule = rules
            .rules
            .iter_mut()
            .find(|rule| rule.id == rule_id)
            .ok_or(RuleGraphEditError::MissingChain { trigger_node_id })?;
        rule.actions.push(action);
        *self = Self::from_rule_set(&rules);
        Ok(())
    }

    pub fn remove_node(&mut self, node_id: RuleGraphNodeId) -> Result<(), RuleGraphEditError> {
        let node_by_id = self
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<HashMap<_, _>>();
        let node = node_by_id
            .get(&node_id)
            .copied()
            .ok_or(RuleGraphEditError::MissingNode { node_id })?;

        let mut rules = self
            .to_rule_set()
            .map_err(RuleGraphEditError::GraphInvalid)?;

        let mut chain_hit = None::<(usize, Vec<RuleGraphNodeId>)>;
        for (chain_index, chain) in self.chains.iter().enumerate() {
            let sequence = self
                .chain_node_sequence(chain.trigger_node_id)
                .map_err(RuleGraphEditError::GraphInvalid)?;
            if sequence.contains(&node_id) {
                chain_hit = Some((chain_index, sequence));
                break;
            }
        }
        let Some((chain_index, sequence)) = chain_hit else {
            return Err(RuleGraphEditError::MissingNode { node_id });
        };
        let chain = &self.chains[chain_index];
        let Some(rule) = rules.rules.iter_mut().find(|rule| rule.id == chain.rule_id) else {
            return Err(RuleGraphEditError::MissingChain {
                trigger_node_id: chain.trigger_node_id,
            });
        };

        let Some(node_position) = sequence.iter().position(|id| *id == node_id) else {
            return Err(RuleGraphEditError::MissingNode { node_id });
        };
        match node.kind {
            RuleGraphNodeKind::Trigger(_) => {
                rules
                    .rules
                    .retain(|candidate| candidate.id != chain.rule_id);
            }
            RuleGraphNodeKind::Condition(_) => {
                if node_position == 0 || node_position > rule.conditions.len() {
                    return Err(RuleGraphEditError::MissingNode { node_id });
                }
                rule.conditions.remove(node_position - 1);
            }
            RuleGraphNodeKind::Action(_) => {
                let action_start = 1 + rule.conditions.len();
                if node_position < action_start {
                    return Err(RuleGraphEditError::MissingNode { node_id });
                }
                let action_index = node_position - action_start;
                if action_index >= rule.actions.len() {
                    return Err(RuleGraphEditError::MissingNode { node_id });
                }
                rule.actions.remove(action_index);
            }
        }

        *self = Self::from_rule_set(&rules);
        Ok(())
    }

    pub fn set_condition_for_node(
        &mut self,
        node_id: RuleGraphNodeId,
        condition: RuleCondition,
    ) -> Result<(), RuleGraphEditError> {
        let mut rules = self
            .to_rule_set()
            .map_err(RuleGraphEditError::GraphInvalid)?;

        let mut chain_hit = None::<(usize, Vec<RuleGraphNodeId>)>;
        for (chain_index, chain) in self.chains.iter().enumerate() {
            let sequence = self
                .chain_node_sequence(chain.trigger_node_id)
                .map_err(RuleGraphEditError::GraphInvalid)?;
            if sequence.contains(&node_id) {
                chain_hit = Some((chain_index, sequence));
                break;
            }
        }
        let Some((chain_index, sequence)) = chain_hit else {
            return Err(RuleGraphEditError::MissingNode { node_id });
        };
        let chain = &self.chains[chain_index];
        let Some(rule) = rules.rules.iter_mut().find(|rule| rule.id == chain.rule_id) else {
            return Err(RuleGraphEditError::MissingChain {
                trigger_node_id: chain.trigger_node_id,
            });
        };
        let Some(node_position) = sequence.iter().position(|id| *id == node_id) else {
            return Err(RuleGraphEditError::MissingNode { node_id });
        };
        if node_position == 0 || node_position > rule.conditions.len() {
            return Err(RuleGraphEditError::MissingNode { node_id });
        }
        rule.conditions[node_position - 1] = condition;
        *self = Self::from_rule_set(&rules);
        Ok(())
    }

    pub fn set_action_for_node(
        &mut self,
        node_id: RuleGraphNodeId,
        action: RuleAction,
    ) -> Result<(), RuleGraphEditError> {
        let mut rules = self
            .to_rule_set()
            .map_err(RuleGraphEditError::GraphInvalid)?;

        let mut chain_hit = None::<(usize, Vec<RuleGraphNodeId>)>;
        for (chain_index, chain) in self.chains.iter().enumerate() {
            let sequence = self
                .chain_node_sequence(chain.trigger_node_id)
                .map_err(RuleGraphEditError::GraphInvalid)?;
            if sequence.contains(&node_id) {
                chain_hit = Some((chain_index, sequence));
                break;
            }
        }
        let Some((chain_index, sequence)) = chain_hit else {
            return Err(RuleGraphEditError::MissingNode { node_id });
        };
        let chain = &self.chains[chain_index];
        let Some(rule) = rules.rules.iter_mut().find(|rule| rule.id == chain.rule_id) else {
            return Err(RuleGraphEditError::MissingChain {
                trigger_node_id: chain.trigger_node_id,
            });
        };
        let Some(node_position) = sequence.iter().position(|id| *id == node_id) else {
            return Err(RuleGraphEditError::MissingNode { node_id });
        };
        let action_start = 1 + rule.conditions.len();
        if node_position < action_start {
            return Err(RuleGraphEditError::MissingNode { node_id });
        }
        let action_index = node_position - action_start;
        if action_index >= rule.actions.len() {
            return Err(RuleGraphEditError::MissingNode { node_id });
        }
        rule.actions[action_index] = action;
        *self = Self::from_rule_set(&rules);
        Ok(())
    }

    pub fn connect_nodes(
        &mut self,
        from: RuleGraphNodeId,
        to: RuleGraphNodeId,
    ) -> Result<(), RuleGraphEditError> {
        if from == to {
            return Err(RuleGraphEditError::InvalidConnection {
                reason: "Cannot connect a node to itself".to_string(),
            });
        }
        let node_by_id = self
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<HashMap<_, _>>();
        let Some(from_node) = node_by_id.get(&from).copied() else {
            return Err(RuleGraphEditError::MissingNode { node_id: from });
        };
        let Some(to_node) = node_by_id.get(&to).copied() else {
            return Err(RuleGraphEditError::MissingNode { node_id: to });
        };

        if matches!(to_node.kind, RuleGraphNodeKind::Trigger(_)) {
            return Err(RuleGraphEditError::InvalidConnection {
                reason: "Trigger nodes cannot be connection targets".to_string(),
            });
        }

        let from_rank = Self::node_stage_rank(&from_node.kind);
        let to_rank = Self::node_stage_rank(&to_node.kind);
        if to_rank < from_rank {
            return Err(RuleGraphEditError::InvalidConnection {
                reason: "Connections must follow Trigger -> Condition -> Action ordering"
                    .to_string(),
            });
        }

        if self
            .edges
            .iter()
            .any(|edge| edge.from == from && edge.to == to)
        {
            return Ok(());
        }

        if self.edges.iter().any(|edge| edge.from == from) {
            return Err(RuleGraphEditError::InvalidConnection {
                reason: "Source node already has an outgoing edge".to_string(),
            });
        }
        if self.edges.iter().any(|edge| edge.to == to) {
            return Err(RuleGraphEditError::InvalidConnection {
                reason: "Target node already has an incoming edge".to_string(),
            });
        }

        let from_chain = self.chain_index_for_node(from);
        let to_chain = self.chain_index_for_node(to);
        if from_chain.is_none() || to_chain.is_none() || from_chain != to_chain {
            return Err(RuleGraphEditError::InvalidConnection {
                reason: "Nodes must belong to the same rule chain".to_string(),
            });
        }

        if self.is_reachable(to, from) {
            return Err(RuleGraphEditError::InvalidConnection {
                reason: "Connection would create a cycle".to_string(),
            });
        }

        self.edges.push(RuleGraphEdge { from, to });
        Ok(())
    }

    pub fn disconnect_nodes(&mut self, from: RuleGraphNodeId, to: RuleGraphNodeId) -> bool {
        let original_len = self.edges.len();
        self.edges
            .retain(|edge| !(edge.from == from && edge.to == to));
        self.edges.len() != original_len
    }

    pub fn chain_node_sequence(
        &self,
        trigger_node_id: RuleGraphNodeId,
    ) -> Result<Vec<RuleGraphNodeId>, RuleGraphError> {
        let chain = self
            .chains
            .iter()
            .find(|chain| chain.trigger_node_id == trigger_node_id)
            .ok_or(RuleGraphError::MissingTriggerNode {
                rule_id: "<unknown>".to_string(),
                node_id: trigger_node_id,
            })?;
        let node_by_id = self
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<HashMap<_, _>>();
        let mut outgoing = HashMap::<RuleGraphNodeId, Vec<RuleGraphNodeId>>::new();
        for edge in &self.edges {
            outgoing.entry(edge.from).or_default().push(edge.to);
        }

        let Some(trigger_node) = node_by_id.get(&trigger_node_id).copied() else {
            return Err(RuleGraphError::MissingTriggerNode {
                rule_id: chain.rule_id.clone(),
                node_id: trigger_node_id,
            });
        };
        if !matches!(trigger_node.kind, RuleGraphNodeKind::Trigger(_)) {
            return Err(RuleGraphError::TriggerNodeKindMismatch {
                rule_id: chain.rule_id.clone(),
                node_id: trigger_node_id,
            });
        }

        let mut sequence = vec![trigger_node_id];
        let mut visited = HashSet::new();
        visited.insert(trigger_node_id);
        let mut current_id = trigger_node_id;
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
            if !node_by_id.contains_key(&next_id) {
                return Err(RuleGraphError::MissingNode {
                    rule_id: chain.rule_id.clone(),
                    node_id: next_id,
                });
            }
            sequence.push(next_id);
            current_id = next_id;
        }
        Ok(sequence)
    }

    fn next_rule_id(rule_set: &RuleSet) -> String {
        let mut index = 1_u32;
        loop {
            let candidate = format!("rule_{}", index);
            if !rule_set.rules.iter().any(|rule| rule.id == candidate) {
                return candidate;
            }
            index += 1;
        }
    }

    fn node_stage_rank(kind: &RuleGraphNodeKind) -> u8 {
        match kind {
            RuleGraphNodeKind::Trigger(_) => 0,
            RuleGraphNodeKind::Condition(_) => 1,
            RuleGraphNodeKind::Action(_) => 2,
        }
    }

    fn chain_index_for_node(&self, node_id: RuleGraphNodeId) -> Option<usize> {
        self.chains.iter().enumerate().find_map(|(index, chain)| {
            self.is_reachable(chain.trigger_node_id, node_id)
                .then_some(index)
        })
    }

    fn is_reachable(&self, start: RuleGraphNodeId, goal: RuleGraphNodeId) -> bool {
        if start == goal {
            return true;
        }
        let mut stack = vec![start];
        let mut visited = HashSet::new();
        while let Some(node_id) = stack.pop() {
            if !visited.insert(node_id) {
                continue;
            }
            for edge in self.edges.iter().filter(|edge| edge.from == node_id) {
                if edge.to == goal {
                    return true;
                }
                stack.push(edge.to);
            }
        }
        false
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

    #[test]
    fn add_default_chain_appends_new_rule() {
        let mut graph = RuleGraph::from_rule_set(&sample_rules());
        let rule_id = graph
            .add_default_chain()
            .expect("adding default chain should work");
        let roundtrip = graph.to_rule_set().expect("graph must remain valid");

        assert!(roundtrip.rules.iter().any(|rule| rule.id == rule_id));
    }

    #[test]
    fn append_condition_and_remove_node_roundtrip() {
        let mut graph = RuleGraph::from_rule_set(&sample_rules());
        let trigger = graph.chains[0].trigger_node_id;
        graph
            .append_condition_to_chain(trigger, RuleCondition::Always)
            .expect("append condition should work");

        let chain_nodes = graph
            .chain_node_sequence(trigger)
            .expect("chain should remain linear");
        let condition_node = chain_nodes[1];
        graph
            .remove_node(condition_node)
            .expect("removing condition node should work");

        graph.to_rule_set().expect("graph should remain valid");
    }

    #[test]
    fn connect_nodes_rejects_invalid_direction() {
        let mut graph = RuleGraph::from_rule_set(&sample_rules());
        let trigger = graph.chains[0].trigger_node_id;
        let nodes = graph
            .chain_node_sequence(trigger)
            .expect("chain should be valid");
        let action_node = *nodes
            .iter()
            .find(|node_id| {
                let node = graph
                    .nodes
                    .iter()
                    .find(|candidate| candidate.id == **node_id)
                    .expect("node must exist");
                matches!(node.kind, super::RuleGraphNodeKind::Action(_))
            })
            .expect("action node expected");
        let condition_node = *nodes
            .iter()
            .find(|node_id| {
                let node = graph
                    .nodes
                    .iter()
                    .find(|candidate| candidate.id == **node_id)
                    .expect("node must exist");
                matches!(node.kind, super::RuleGraphNodeKind::Condition(_))
            })
            .expect("condition node expected");

        graph.disconnect_nodes(condition_node, action_node);
        let error = graph
            .connect_nodes(action_node, condition_node)
            .expect_err("action -> condition should be rejected");
        assert!(matches!(
            error,
            super::RuleGraphEditError::InvalidConnection { .. }
        ));
    }
}
