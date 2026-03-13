use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use toki_core::rules::{Rule, RuleAction, RuleCondition, RuleSet, RuleTrigger};

pub type RuleGraphNodeId = u64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleGraphNodeKind {
    Trigger(RuleTrigger),
    Condition(RuleCondition),
    Action(RuleAction),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleGraphNode {
    pub id: RuleGraphNodeId,
    pub kind: RuleGraphNodeKind,
    pub position: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleGraphEdge {
    pub from: RuleGraphNodeId,
    pub to: RuleGraphNodeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleGraphChain {
    pub rule_id: String,
    pub enabled: bool,
    pub priority: i32,
    pub once: bool,
    pub trigger_node_id: RuleGraphNodeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    // Auto-layout spacing between node edges (not centers).
    const H_SPACING: f32 = 30.0;
    const V_SPACING: f32 = 40.0;
    // Keep in sync with graph canvas max node size at 100% zoom.
    const AUTO_LAYOUT_NODE_WIDTH: f32 = 320.0;
    const AUTO_LAYOUT_NODE_HEIGHT: f32 = 36.0;
    const AUTO_LAYOUT_START_X: f32 = 60.0;
    const AUTO_LAYOUT_START_Y: f32 = 50.0;

    pub fn from_rule_set(rule_set: &RuleSet) -> Self {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut chains = Vec::new();
        let mut next_node_id: RuleGraphNodeId = 1;
        let mut edge_set = HashSet::<(RuleGraphNodeId, RuleGraphNodeId)>::new();
        let mut suffix_cache = Vec::<(
            (RuleGraphNodeKind, Option<RuleGraphNodeId>),
            RuleGraphNodeId,
        )>::new();

        for rule in &rule_set.rules {
            let trigger_id = next_node_id;
            next_node_id += 1;
            nodes.push(RuleGraphNode {
                id: trigger_id,
                kind: RuleGraphNodeKind::Trigger(rule.trigger),
                position: [0.0, 0.0],
            });
            let mut next_in_chain = None::<RuleGraphNodeId>;

            for action in rule.actions.iter().rev() {
                let node_id = Self::get_or_create_suffix_node(
                    &mut nodes,
                    &mut next_node_id,
                    &mut suffix_cache,
                    RuleGraphNodeKind::Action(action.clone()),
                    next_in_chain,
                );
                if let Some(next_node) = next_in_chain {
                    Self::insert_edge_unique(&mut edges, &mut edge_set, node_id, next_node);
                }
                next_in_chain = Some(node_id);
            }

            for condition in rule.conditions.iter().rev() {
                let node_id = Self::get_or_create_suffix_node(
                    &mut nodes,
                    &mut next_node_id,
                    &mut suffix_cache,
                    RuleGraphNodeKind::Condition(*condition),
                    next_in_chain,
                );
                if let Some(next_node) = next_in_chain {
                    Self::insert_edge_unique(&mut edges, &mut edge_set, node_id, next_node);
                }
                next_in_chain = Some(node_id);
            }

            if let Some(first_node) = next_in_chain {
                Self::insert_edge_unique(&mut edges, &mut edge_set, trigger_id, first_node);
            }

            chains.push(RuleGraphChain {
                rule_id: rule.id.clone(),
                enabled: rule.enabled,
                priority: rule.priority,
                once: rule.once,
                trigger_node_id: trigger_id,
            });
        }

        let mut graph = Self {
            nodes,
            edges,
            chains,
        };
        graph.apply_auto_layout_positions();
        graph
    }

    fn get_or_create_suffix_node(
        nodes: &mut Vec<RuleGraphNode>,
        next_node_id: &mut RuleGraphNodeId,
        suffix_cache: &mut Vec<(
            (RuleGraphNodeKind, Option<RuleGraphNodeId>),
            RuleGraphNodeId,
        )>,
        kind: RuleGraphNodeKind,
        next_node: Option<RuleGraphNodeId>,
    ) -> RuleGraphNodeId {
        if let Some((_, node_id)) =
            suffix_cache
                .iter()
                .find(|((existing_kind, existing_next), _)| {
                    *existing_kind == kind && *existing_next == next_node
                })
        {
            return *node_id;
        }

        let node_id = *next_node_id;
        *next_node_id += 1;
        nodes.push(RuleGraphNode {
            id: node_id,
            kind: kind.clone(),
            position: [0.0, 0.0],
        });
        suffix_cache.push(((kind, next_node), node_id));
        node_id
    }

    fn insert_edge_unique(
        edges: &mut Vec<RuleGraphEdge>,
        edge_set: &mut HashSet<(RuleGraphNodeId, RuleGraphNodeId)>,
        from: RuleGraphNodeId,
        to: RuleGraphNodeId,
    ) {
        if edge_set.insert((from, to)) {
            edges.push(RuleGraphEdge { from, to });
        }
    }

    fn apply_auto_layout_positions(&mut self) {
        let mut positioned = HashSet::<RuleGraphNodeId>::new();
        for (chain_index, chain) in self.chains.iter().enumerate() {
            let Ok(sequence) = self.chain_node_sequence(chain.trigger_node_id) else {
                continue;
            };

            let y = Self::AUTO_LAYOUT_START_Y
                + (chain_index as f32 * Self::auto_layout_vertical_step());
            let mut x = Self::AUTO_LAYOUT_START_X;
            for node_id in sequence {
                if positioned.insert(node_id) {
                    if let Some(node) = self.nodes.iter_mut().find(|node| node.id == node_id) {
                        node.position = [x, y];
                    }
                }
                x += Self::auto_layout_horizontal_step();
            }
        }

        for node in &mut self.nodes {
            if positioned.contains(&node.id) {
                continue;
            }
            node.position = [Self::AUTO_LAYOUT_START_X, Self::AUTO_LAYOUT_START_Y];
        }
    }

    pub(crate) fn auto_layout_node_height() -> f32 {
        Self::AUTO_LAYOUT_NODE_HEIGHT
    }

    pub(crate) fn auto_layout_vertical_edge_spacing() -> f32 {
        Self::V_SPACING
    }

    fn auto_layout_vertical_step() -> f32 {
        Self::auto_layout_node_height() + Self::auto_layout_vertical_edge_spacing()
    }

    pub(crate) fn auto_layout_node_width() -> f32 {
        Self::AUTO_LAYOUT_NODE_WIDTH
    }

    pub(crate) fn auto_layout_horizontal_edge_spacing() -> f32 {
        Self::H_SPACING
    }

    fn auto_layout_horizontal_step() -> f32 {
        Self::auto_layout_node_width() + Self::auto_layout_horizontal_edge_spacing()
    }

    pub(crate) fn auto_layout_start_x() -> f32 {
        Self::AUTO_LAYOUT_START_X
    }

    pub(crate) fn auto_layout_start_y() -> f32 {
        Self::AUTO_LAYOUT_START_Y
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
                    RuleGraphNodeKind::Trigger(_) => {}
                    RuleGraphNodeKind::Condition(condition) => {
                        conditions.push(*condition);
                    }
                    RuleGraphNodeKind::Action(action) => {
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

    pub fn set_node_position(
        &mut self,
        node_id: RuleGraphNodeId,
        position: [f32; 2],
    ) -> Result<(), RuleGraphEditError> {
        let Some(node) = self.nodes.iter_mut().find(|node| node.id == node_id) else {
            return Err(RuleGraphEditError::MissingNode { node_id });
        };
        node.position = position;
        Ok(())
    }

    pub fn stable_node_key(&self, node_id: RuleGraphNodeId) -> Option<String> {
        self.nodes
            .iter()
            .any(|node| node.id == node_id)
            .then_some(format!("node:{node_id}"))
    }

    fn legacy_stable_node_key(&self, node_id: RuleGraphNodeId) -> Option<String> {
        let node_by_id = self
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<HashMap<_, _>>();

        for chain in &self.chains {
            let Ok(sequence) = self.chain_node_sequence(chain.trigger_node_id) else {
                continue;
            };
            if !sequence.contains(&node_id) {
                continue;
            }

            let mut condition_index = 0_usize;
            let mut action_index = 0_usize;
            for current_node_id in sequence {
                let node = node_by_id.get(&current_node_id)?;
                match node.kind {
                    RuleGraphNodeKind::Trigger(_) => {
                        if current_node_id == node_id {
                            return Some(format!("{}:trigger", chain.rule_id));
                        }
                    }
                    RuleGraphNodeKind::Condition(_) => {
                        if current_node_id == node_id {
                            return Some(format!(
                                "{}:condition:{}",
                                chain.rule_id, condition_index
                            ));
                        }
                        condition_index += 1;
                    }
                    RuleGraphNodeKind::Action(_) => {
                        if current_node_id == node_id {
                            return Some(format!("{}:action:{}", chain.rule_id, action_index));
                        }
                        action_index += 1;
                    }
                }
            }
        }

        node_by_id
            .contains_key(&node_id)
            .then_some(format!("detached:{node_id}"))
    }

    pub fn node_id_for_stable_key(&self, stable_key: &str) -> Option<RuleGraphNodeId> {
        if let Some(node_id) = stable_key.strip_prefix("node:") {
            let parsed_id = node_id.parse::<RuleGraphNodeId>().ok()?;
            if self.nodes.iter().any(|node| node.id == parsed_id) {
                return Some(parsed_id);
            }
        }
        if let Some(detached_id) = stable_key.strip_prefix("detached:") {
            let parsed_id = detached_id.parse::<RuleGraphNodeId>().ok()?;
            if self.nodes.iter().any(|node| node.id == parsed_id) {
                return Some(parsed_id);
            }
        }
        self.nodes.iter().find_map(|node| {
            self.legacy_stable_node_key(node.id)
                .filter(|candidate| candidate == stable_key)
                .map(|_| node.id)
        })
    }

    pub fn add_trigger_chain(&mut self) -> Result<String, RuleGraphEditError> {
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
            conditions: Vec::new(),
            actions: Vec::new(),
        });
        *self = Self::from_rule_set(&rules);
        Ok(rule_id)
    }

    #[cfg(test)]
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

    pub fn add_condition_node(
        &mut self,
        condition: RuleCondition,
    ) -> Result<RuleGraphNodeId, RuleGraphEditError> {
        let node_id = self.next_node_id();
        self.nodes.push(RuleGraphNode {
            id: node_id,
            kind: RuleGraphNodeKind::Condition(condition),
            position: [
                Self::AUTO_LAYOUT_START_X + (self.nodes.len() as f32 * 12.0),
                Self::AUTO_LAYOUT_START_Y + (self.nodes.len() as f32 * 8.0),
            ],
        });
        Ok(node_id)
    }

    pub fn add_action_node(
        &mut self,
        action: RuleAction,
    ) -> Result<RuleGraphNodeId, RuleGraphEditError> {
        let node_id = self.next_node_id();
        self.nodes.push(RuleGraphNode {
            id: node_id,
            kind: RuleGraphNodeKind::Action(action),
            position: [
                Self::AUTO_LAYOUT_START_X + (self.nodes.len() as f32 * 12.0),
                Self::AUTO_LAYOUT_START_Y + (self.nodes.len() as f32 * 8.0),
            ],
        });
        Ok(node_id)
    }

    pub fn set_trigger_for_chain(
        &mut self,
        trigger_node_id: RuleGraphNodeId,
        trigger: RuleTrigger,
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
        rule.trigger = trigger;
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
            self.nodes.retain(|candidate| candidate.id != node_id);
            self.edges
                .retain(|edge| edge.from != node_id && edge.to != node_id);
            return Ok(());
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
            let Some(node) = self
                .nodes
                .iter_mut()
                .find(|candidate| candidate.id == node_id)
            else {
                return Err(RuleGraphEditError::MissingNode { node_id });
            };
            if !matches!(node.kind, RuleGraphNodeKind::Condition(_)) {
                return Err(RuleGraphEditError::MissingNode { node_id });
            }
            node.kind = RuleGraphNodeKind::Condition(condition);
            return Ok(());
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
            let Some(node) = self
                .nodes
                .iter_mut()
                .find(|candidate| candidate.id == node_id)
            else {
                return Err(RuleGraphEditError::MissingNode { node_id });
            };
            if !matches!(node.kind, RuleGraphNodeKind::Action(_)) {
                return Err(RuleGraphEditError::MissingNode { node_id });
            }
            node.kind = RuleGraphNodeKind::Action(action);
            return Ok(());
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
        if !node_by_id.contains_key(&from) {
            return Err(RuleGraphEditError::MissingNode { node_id: from });
        }
        if !node_by_id.contains_key(&to) {
            return Err(RuleGraphEditError::MissingNode { node_id: to });
        }

        if self
            .edges
            .iter()
            .any(|edge| edge.from == from && edge.to == to)
        {
            return Ok(());
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

    pub fn disconnect_node(&mut self, node_id: RuleGraphNodeId) -> Result<(), RuleGraphEditError> {
        if !self.nodes.iter().any(|node| node.id == node_id) {
            return Err(RuleGraphEditError::MissingNode { node_id });
        }
        self.edges
            .retain(|edge| edge.from != node_id && edge.to != node_id);
        Ok(())
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

    #[cfg(test)]
    pub fn trigger_node_for_node(&self, node_id: RuleGraphNodeId) -> Option<RuleGraphNodeId> {
        for chain in &self.chains {
            let Ok(sequence) = self.chain_node_sequence(chain.trigger_node_id) else {
                continue;
            };
            if sequence.contains(&node_id) {
                return Some(chain.trigger_node_id);
            }
        }
        None
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

    fn next_node_id(&self) -> RuleGraphNodeId {
        self.nodes.iter().map(|node| node.id).max().unwrap_or(0) + 1
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
    fn add_trigger_chain_appends_trigger_only_rule() {
        let mut graph = RuleGraph::from_rule_set(&sample_rules());
        let rule_id = graph
            .add_trigger_chain()
            .expect("adding trigger chain should work");
        let roundtrip = graph.to_rule_set().expect("graph must remain valid");
        let added_rule = roundtrip
            .rules
            .iter()
            .find(|rule| rule.id == rule_id)
            .expect("new rule should exist");
        assert!(added_rule.conditions.is_empty());
        assert!(added_rule.actions.is_empty());
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
    fn connect_nodes_allows_action_to_trigger_when_linear() {
        let mut graph = RuleGraph::from_rule_set(&sample_rules());
        let first_trigger = graph.chains[0].trigger_node_id;
        let second_trigger = graph.chains[1].trigger_node_id;
        let nodes = graph
            .chain_node_sequence(first_trigger)
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
        let previous_targets = graph
            .edges
            .iter()
            .filter(|edge| edge.from == action_node)
            .map(|edge| edge.to)
            .collect::<Vec<_>>();
        for previous_target in previous_targets {
            graph.disconnect_nodes(action_node, previous_target);
        }
        graph
            .connect_nodes(action_node, second_trigger)
            .expect("action -> trigger should be accepted in free-connection mode");
        graph
            .to_rule_set()
            .expect("rewired linear chain should still serialize");
    }

    #[test]
    fn connect_nodes_allows_joining_into_existing_trigger_chain() {
        let mut graph = RuleGraph::from_rule_set(&sample_rules());
        let first_trigger = graph.chains[0].trigger_node_id;
        let second_trigger = graph.chains[1].trigger_node_id;

        let first_sequence = graph
            .chain_node_sequence(first_trigger)
            .expect("first chain should be valid");
        let second_sequence = graph
            .chain_node_sequence(second_trigger)
            .expect("second chain should be valid");

        let first_action = *first_sequence
            .iter()
            .find(|node_id| {
                let node = graph
                    .nodes
                    .iter()
                    .find(|candidate| candidate.id == **node_id)
                    .expect("node must exist");
                matches!(node.kind, super::RuleGraphNodeKind::Action(_))
            })
            .expect("first chain action should exist");
        let second_condition = *second_sequence
            .iter()
            .find(|node_id| {
                let node = graph
                    .nodes
                    .iter()
                    .find(|candidate| candidate.id == **node_id)
                    .expect("node must exist");
                matches!(node.kind, super::RuleGraphNodeKind::Condition(_))
            })
            .expect("second chain condition should exist");
        let previous_targets = graph
            .edges
            .iter()
            .filter(|edge| edge.from == first_action)
            .map(|edge| edge.to)
            .collect::<Vec<_>>();
        for previous_target in previous_targets {
            graph.disconnect_nodes(first_action, previous_target);
        }

        graph
            .connect_nodes(first_action, second_condition)
            .expect("joining into existing condition node should be accepted");

        let roundtrip = graph
            .to_rule_set()
            .expect("joined chain should still serialize");
        assert_eq!(roundtrip.rules.len(), 2);
    }

    #[test]
    fn connect_nodes_preserves_existing_outgoing_edges() {
        let mut graph = RuleGraph::from_rule_set(&sample_rules());
        let from = graph
            .add_action_node(RuleAction::PlayMusic {
                track_id: "music_a".to_string(),
            })
            .expect("detached source node should be created");
        let to_a = graph
            .add_condition_node(RuleCondition::Always)
            .expect("first detached target should be created");
        let to_b = graph
            .add_condition_node(RuleCondition::KeyHeld { key: RuleKey::Left })
            .expect("second detached target should be created");

        graph
            .connect_nodes(from, to_a)
            .expect("first connect should succeed");
        graph
            .connect_nodes(from, to_b)
            .expect("second connect should keep the first one");

        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.from == from && edge.to == to_a));
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.from == from && edge.to == to_b));
    }

    #[test]
    fn stable_node_keys_remain_unique_after_cross_chain_connect() {
        let mut graph = RuleGraph::from_rule_set(&sample_rules());
        let first_trigger = graph.chains[0].trigger_node_id;
        let second_trigger = graph.chains[1].trigger_node_id;
        let first_sequence = graph
            .chain_node_sequence(first_trigger)
            .expect("first chain should be valid");
        let first_action = *first_sequence
            .iter()
            .find(|node_id| {
                graph
                    .nodes
                    .iter()
                    .find(|node| node.id == **node_id)
                    .is_some_and(|node| matches!(node.kind, super::RuleGraphNodeKind::Action(_)))
            })
            .expect("expected action in first chain");

        graph
            .connect_nodes(first_action, second_trigger)
            .expect("cross-chain connect should succeed");

        let mut keys = graph
            .nodes
            .iter()
            .filter_map(|node| graph.stable_node_key(node.id))
            .collect::<Vec<_>>();
        let total = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), total, "stable keys must remain unique");
    }

    #[test]
    fn from_rule_set_merges_shared_action_nodes_for_multiple_triggers() {
        let shared_action = RuleAction::PlayMusic {
            track_id: "bgm_shared".to_string(),
        };
        let rules = RuleSet {
            rules: vec![
                toki_core::rules::Rule {
                    id: "rule_a".to_string(),
                    enabled: true,
                    priority: 0,
                    once: false,
                    trigger: RuleTrigger::OnStart,
                    conditions: Vec::new(),
                    actions: vec![shared_action.clone()],
                },
                toki_core::rules::Rule {
                    id: "rule_b".to_string(),
                    enabled: true,
                    priority: 0,
                    once: false,
                    trigger: RuleTrigger::OnUpdate,
                    conditions: Vec::new(),
                    actions: vec![shared_action.clone()],
                },
            ],
        };

        let graph = RuleGraph::from_rule_set(&rules);
        let shared_action_nodes = graph
            .nodes
            .iter()
            .filter(|node| matches!(&node.kind, super::RuleGraphNodeKind::Action(action) if *action == shared_action))
            .map(|node| node.id)
            .collect::<Vec<_>>();
        assert_eq!(
            shared_action_nodes.len(),
            1,
            "identical action suffix should be represented by one shared node"
        );

        let shared_action_id = shared_action_nodes[0];
        let trigger_ids = graph
            .chains
            .iter()
            .map(|chain| chain.trigger_node_id)
            .collect::<Vec<_>>();
        assert!(
            graph
                .edges
                .iter()
                .filter(|edge| trigger_ids.contains(&edge.from) && edge.to == shared_action_id)
                .count()
                >= 2
        );
    }

    #[test]
    fn from_rule_set_merges_shared_condition_and_action_suffixes() {
        let shared_action = RuleAction::DestroySelf {
            target: RuleTarget::Player,
        };
        let rules = RuleSet {
            rules: vec![
                toki_core::rules::Rule {
                    id: "rule_a".to_string(),
                    enabled: true,
                    priority: 0,
                    once: false,
                    trigger: RuleTrigger::OnStart,
                    conditions: vec![RuleCondition::Always],
                    actions: vec![shared_action.clone()],
                },
                toki_core::rules::Rule {
                    id: "rule_b".to_string(),
                    enabled: true,
                    priority: 0,
                    once: false,
                    trigger: RuleTrigger::OnUpdate,
                    conditions: vec![RuleCondition::Always],
                    actions: vec![shared_action.clone()],
                },
            ],
        };

        let graph = RuleGraph::from_rule_set(&rules);
        let condition_nodes = graph
            .nodes
            .iter()
            .filter(|node| {
                matches!(
                    node.kind,
                    super::RuleGraphNodeKind::Condition(RuleCondition::Always)
                )
            })
            .map(|node| node.id)
            .collect::<Vec<_>>();
        let action_nodes = graph
            .nodes
            .iter()
            .filter(|node| matches!(&node.kind, super::RuleGraphNodeKind::Action(action) if *action == shared_action))
            .map(|node| node.id)
            .collect::<Vec<_>>();

        assert_eq!(condition_nodes.len(), 1);
        assert_eq!(action_nodes.len(), 1);
        assert!(graph
            .edges
            .iter()
            .any(|edge| { edge.from == condition_nodes[0] && edge.to == action_nodes[0] }));
    }

    #[test]
    fn add_condition_node_creates_new_independent_chain() {
        let mut graph = RuleGraph::from_rule_set(&sample_rules());
        let existing_trigger = graph.chains[0].trigger_node_id;
        let initial_chains = graph.chains.len();

        let new_condition = graph
            .add_condition_node(RuleCondition::Always)
            .expect("adding standalone condition node should succeed");
        assert_eq!(graph.chains.len(), initial_chains);

        assert!(
            !graph
                .edges
                .iter()
                .any(|edge| edge.from == existing_trigger && edge.to == new_condition),
            "new condition should not auto-connect to the existing chain"
        );
        assert!(
            graph.stable_node_key(new_condition).is_some(),
            "standalone condition should receive a stable key in the editor graph"
        );
    }

    #[test]
    fn disconnect_node_removes_all_incident_edges() {
        let mut graph = RuleGraph::from_rule_set(&sample_rules());
        let first_trigger = graph.chains[0].trigger_node_id;
        let sequence = graph
            .chain_node_sequence(first_trigger)
            .expect("chain should be valid");
        let disconnect_target = *sequence
            .iter()
            .find(|node_id| {
                graph
                    .nodes
                    .iter()
                    .find(|node| node.id == **node_id)
                    .is_some_and(|node| matches!(node.kind, super::RuleGraphNodeKind::Action(_)))
            })
            .expect("chain should contain an action node");

        graph
            .disconnect_node(disconnect_target)
            .expect("disconnecting node should succeed");
        assert!(!graph
            .edges
            .iter()
            .any(|edge| edge.from == disconnect_target || edge.to == disconnect_target));
        graph
            .to_rule_set()
            .expect("graph should remain serializable after disconnect");
    }

    #[test]
    fn trigger_node_for_node_resolves_owner_chain_trigger() {
        let graph = RuleGraph::from_rule_set(&sample_rules());
        let trigger = graph.chains[0].trigger_node_id;
        let sequence = graph
            .chain_node_sequence(trigger)
            .expect("chain should be valid");
        let action_node = *sequence
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
        assert_eq!(graph.trigger_node_for_node(action_node), Some(trigger));
    }

    #[test]
    fn stable_node_keys_are_generated_for_chain_nodes() {
        let graph = RuleGraph::from_rule_set(&sample_rules());
        let trigger = graph.chains[0].trigger_node_id;
        let sequence = graph
            .chain_node_sequence(trigger)
            .expect("chain should be valid");

        let keys = sequence
            .iter()
            .filter_map(|node_id| graph.stable_node_key(*node_id))
            .collect::<Vec<_>>();

        assert_eq!(keys.len(), sequence.len());
        assert!(keys.iter().all(|key| key.starts_with("node:")));
        let mut unique = keys.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), keys.len());
    }

    #[test]
    fn node_id_for_stable_key_resolves_existing_node() {
        let graph = RuleGraph::from_rule_set(&sample_rules());
        let trigger_key = graph
            .stable_node_key(graph.chains[0].trigger_node_id)
            .expect("expected trigger to have a stable key");
        let Some(trigger_id) = graph.node_id_for_stable_key(&trigger_key) else {
            panic!("expected stable key to resolve to a node id");
        };
        let trigger_node = graph
            .nodes
            .iter()
            .find(|node| node.id == trigger_id)
            .expect("resolved node id should exist");
        assert!(matches!(
            trigger_node.kind,
            super::RuleGraphNodeKind::Trigger(_)
        ));
    }

    #[test]
    fn node_id_for_stable_key_supports_legacy_chain_keys() {
        let graph = RuleGraph::from_rule_set(&sample_rules());
        let Some(trigger_id) = graph.node_id_for_stable_key("rule_spawn:trigger") else {
            panic!("expected legacy trigger key to resolve");
        };
        let trigger_node = graph
            .nodes
            .iter()
            .find(|node| node.id == trigger_id)
            .expect("resolved node id should exist");
        assert!(matches!(
            trigger_node.kind,
            super::RuleGraphNodeKind::Trigger(_)
        ));
    }

    #[test]
    fn auto_layout_uses_minimum_spacing_between_nodes_and_chains() {
        let graph = RuleGraph::from_rule_set(&sample_rules());

        let first_trigger = graph.chains[0].trigger_node_id;
        let second_trigger = graph.chains[1].trigger_node_id;

        let first_trigger_node = graph
            .nodes
            .iter()
            .find(|node| node.id == first_trigger)
            .expect("first trigger node should exist");
        let second_trigger_node = graph
            .nodes
            .iter()
            .find(|node| node.id == second_trigger)
            .expect("second trigger node should exist");

        let center_spacing = second_trigger_node.position[1] - first_trigger_node.position[1];
        let edge_spacing = center_spacing - super::RuleGraph::auto_layout_node_height();
        assert!(
            edge_spacing >= super::RuleGraph::auto_layout_vertical_edge_spacing(),
            "vertical edge spacing should keep chains visually separated"
        );

        let first_sequence = graph
            .chain_node_sequence(first_trigger)
            .expect("first chain should be valid");
        for pair in first_sequence.windows(2) {
            let left = graph
                .nodes
                .iter()
                .find(|node| node.id == pair[0])
                .expect("left node should exist");
            let right = graph
                .nodes
                .iter()
                .find(|node| node.id == pair[1])
                .expect("right node should exist");
            let center_spacing = right.position[0] - left.position[0];
            let edge_spacing = center_spacing - super::RuleGraph::auto_layout_node_width();
            assert!(
                edge_spacing >= super::RuleGraph::auto_layout_horizontal_edge_spacing(),
                "horizontal edge spacing should keep consecutive nodes visually separated"
            );
        }
    }
}
