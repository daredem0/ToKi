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
    // Auto-layout spacing between node edges.
    const H_SPACING: f32 = 50.0;
    const V_SPACING: f32 = 20.0;
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
                    RuleGraphNodeKind::Condition(condition.clone()),
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
                        conditions.push(condition.clone());
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

    /// Derive the pre-migration node key shape used by older persisted graph layouts.
    ///
    /// This remains intentionally supported so existing project metadata and migrated
    /// editor config data can still resolve onto the current node ids.
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

    /// Resolve a persisted node key to a live node id.
    ///
    /// Canonical `node:<id>` keys are preferred. Legacy chain-shaped keys remain supported
    /// as a compatibility seam for older saved graph layout metadata.
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
        self.validate_connection(from, to)?;
        if self
            .edges
            .iter()
            .any(|edge| edge.from == from && edge.to == to)
        {
            return Ok(());
        }
        self.edges.push(RuleGraphEdge { from, to });
        Ok(())
    }

    pub fn can_connect_nodes(
        &self,
        from: RuleGraphNodeId,
        to: RuleGraphNodeId,
    ) -> Result<(), RuleGraphEditError> {
        self.validate_connection(from, to)
    }

    fn validate_connection(
        &self,
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

        if self.is_reachable(to, from) {
            return Err(RuleGraphEditError::InvalidConnection {
                reason: "Connection would create a cycle".to_string(),
            });
        }
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
#[path = "rule_graph_tests.rs"]
mod tests;
