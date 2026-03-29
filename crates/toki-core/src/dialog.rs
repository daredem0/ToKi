use serde::{Deserialize, Serialize};
use std::fmt;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::entity::{EntityId, EntityKind};
use crate::flags::FlagValue;
use crate::ids::DialogId;

fn default_allow_cancel() -> bool {
    true
}

fn default_gate_gameplay() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogTree {
    pub id: DialogId,
    #[serde(default)]
    pub title: String,
    pub entry_node_id: String,
    #[serde(default = "default_allow_cancel")]
    pub allow_cancel: bool,
    #[serde(default = "default_gate_gameplay")]
    pub gate_gameplay: bool,
    #[serde(default)]
    pub nodes: Vec<DialogNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogNode {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker_name: Option<String>,
    #[serde(default)]
    pub conditions: Vec<DialogCondition>,
    #[serde(flatten)]
    pub kind: DialogNodeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DialogNodeKind {
    Line {
        body: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        next_node_id: Option<String>,
    },
    Choice {
        body: String,
        #[serde(default)]
        choices: Vec<DialogChoice>,
    },
    Branch {
        #[serde(default)]
        branches: Vec<DialogBranch>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default_next_node_id: Option<String>,
    },
    End {
        #[serde(default)]
        body: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outcome_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogChoice {
    pub id: String,
    pub label: String,
    pub next_node_id: String,
    #[serde(default)]
    pub conditions: Vec<DialogCondition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogBranch {
    #[serde(default)]
    pub conditions: Vec<DialogCondition>,
    pub next_node_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogConditionTarget {
    Player,
    Interactor,
    Speaker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DialogCondition {
    HealthBelow {
        target: DialogConditionTarget,
        threshold: i32,
    },
    HealthAbove {
        target: DialogConditionTarget,
        threshold: i32,
    },
    HasInventoryItem {
        target: DialogConditionTarget,
        item_id: String,
        min_count: u32,
    },
    EntityHasTag {
        target: DialogConditionTarget,
        tag: String,
    },
    EntityIsKind {
        target: DialogConditionTarget,
        entity_kind: EntityKind,
    },
    FlagEquals {
        flag: String,
        value: FlagValue,
    },
    FlagSet {
        flag: String,
    },
    FlagGreaterThan {
        flag: String,
        value: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DialogRuntimeContext {
    pub interactor: Option<EntityId>,
    pub speaker: Option<EntityId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DialogValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl DialogValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

impl fmt::Display for DialogValidationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.errors.is_empty() && self.warnings.is_empty() {
            return write!(f, "dialog is valid");
        }

        if !self.errors.is_empty() {
            write!(f, "errors: {}", self.errors.join("; "))?;
        }

        if !self.warnings.is_empty() {
            if !self.errors.is_empty() {
                write!(f, " | ")?;
            }
            write!(f, "warnings: {}", self.warnings.join("; "))?;
        }

        Ok(())
    }
}

impl DialogTree {
    pub fn validate(&self) -> DialogValidationReport {
        let mut report = DialogValidationReport::default();
        if self.id.trim().is_empty() {
            report
                .errors
                .push("Dialog id must not be empty".to_string());
        }
        if self.entry_node_id.trim().is_empty() {
            report.errors.push(format!(
                "Dialog '{}' entry node id must not be empty",
                self.id
            ));
        }
        if self.nodes.is_empty() {
            report.errors.push(format!(
                "Dialog '{}' must contain at least one node",
                self.id
            ));
            return report;
        }

        let mut node_map = HashMap::new();
        for node in &self.nodes {
            if node.id.trim().is_empty() {
                report.errors.push(format!(
                    "Dialog '{}' contains a node with an empty id",
                    self.id
                ));
                continue;
            }
            if node_map.insert(node.id.clone(), node).is_some() {
                report.errors.push(format!(
                    "Dialog '{}' contains duplicate node id '{}'",
                    self.id, node.id
                ));
            }
        }

        if !node_map.contains_key(&self.entry_node_id) {
            report.errors.push(format!(
                "Dialog '{}' entry node '{}' does not exist",
                self.id, self.entry_node_id
            ));
        }

        for node in &self.nodes {
            match &node.kind {
                DialogNodeKind::Line { next_node_id, .. } => {
                    if !node.conditions.is_empty() && next_node_id.is_none() {
                        report.warnings.push(format!(
                            "Dialog '{}' conditioned line node '{}' has no next node fallback",
                            self.id, node.id
                        ));
                    }
                    if let Some(next_node_id) = next_node_id {
                        validate_node_ref(self, &node_map, next_node_id, &node.id, &mut report);
                    }
                }
                DialogNodeKind::Choice { choices, .. } => {
                    if choices.is_empty() {
                        report.warnings.push(format!(
                            "Dialog '{}' choice node '{}' has no choices",
                            self.id, node.id
                        ));
                    }
                    let mut choice_ids = HashSet::new();
                    for choice in choices {
                        if choice.id.trim().is_empty() {
                            report.errors.push(format!(
                                "Dialog '{}' choice node '{}' contains an empty choice id",
                                self.id, node.id
                            ));
                        } else if !choice_ids.insert(choice.id.clone()) {
                            report.errors.push(format!(
                                "Dialog '{}' choice node '{}' contains duplicate choice id '{}'",
                                self.id, node.id, choice.id
                            ));
                        }
                        if choice.label.trim().is_empty() {
                            report.warnings.push(format!(
                                "Dialog '{}' choice '{}' in node '{}' has an empty label",
                                self.id, choice.id, node.id
                            ));
                        }
                        validate_node_ref(
                            self,
                            &node_map,
                            &choice.next_node_id,
                            &node.id,
                            &mut report,
                        );
                    }
                }
                DialogNodeKind::Branch {
                    branches,
                    default_next_node_id,
                } => {
                    if branches.is_empty() && default_next_node_id.is_none() {
                        report.errors.push(format!(
                            "Dialog '{}' branch node '{}' has no branch targets",
                            self.id, node.id
                        ));
                    }
                    for branch in branches {
                        validate_node_ref(
                            self,
                            &node_map,
                            &branch.next_node_id,
                            &node.id,
                            &mut report,
                        );
                    }
                    if let Some(default_next_node_id) = default_next_node_id {
                        validate_node_ref(
                            self,
                            &node_map,
                            default_next_node_id,
                            &node.id,
                            &mut report,
                        );
                    }
                }
                DialogNodeKind::End { .. } => {}
            }
        }

        if report.errors.is_empty() && node_map.contains_key(&self.entry_node_id) {
            let mut reachable = HashSet::new();
            let mut queue = VecDeque::from([self.entry_node_id.clone()]);
            while let Some(node_id) = queue.pop_front() {
                if !reachable.insert(node_id.clone()) {
                    continue;
                }
                let Some(node) = node_map.get(&node_id) else {
                    continue;
                };
                for next_id in node.next_node_ids() {
                    if !reachable.contains(next_id) {
                        queue.push_back(next_id.to_string());
                    }
                }
            }
            for node in &self.nodes {
                if !reachable.contains(&node.id) {
                    report.warnings.push(format!(
                        "Dialog '{}' node '{}' is unreachable from the entry node",
                        self.id, node.id
                    ));
                }
            }
        }

        report
    }

    pub fn node(&self, node_id: &str) -> Option<&DialogNode> {
        self.nodes.iter().find(|node| node.id == node_id)
    }
}

impl DialogNode {
    pub fn next_node_ids(&self) -> Vec<&str> {
        match &self.kind {
            DialogNodeKind::Line { next_node_id, .. } => {
                next_node_id.iter().map(String::as_str).collect()
            }
            DialogNodeKind::Choice { choices, .. } => choices
                .iter()
                .map(|choice| choice.next_node_id.as_str())
                .collect(),
            DialogNodeKind::Branch {
                branches,
                default_next_node_id,
            } => branches
                .iter()
                .map(|branch| branch.next_node_id.as_str())
                .chain(default_next_node_id.iter().map(String::as_str))
                .collect(),
            DialogNodeKind::End { .. } => Vec::new(),
        }
    }
}

fn validate_node_ref(
    dialog: &DialogTree,
    node_map: &HashMap<String, &DialogNode>,
    next_node_id: &str,
    source_node_id: &str,
    report: &mut DialogValidationReport,
) {
    if next_node_id.trim().is_empty() {
        report.errors.push(format!(
            "Dialog '{}' node '{}' contains an empty next-node reference",
            dialog.id, source_node_id
        ));
    } else if !node_map.contains_key(next_node_id) {
        report.errors.push(format!(
            "Dialog '{}' node '{}' references missing node '{}'",
            dialog.id, source_node_id, next_node_id
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_validation_accepts_reachable_linear_dialog() {
        let dialog = DialogTree {
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
                        body: "Hello".to_string(),
                        next_node_id: Some("done".to_string()),
                    },
                },
                DialogNode {
                    id: "done".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: "Bye".to_string(),
                        outcome_id: Some("completed".to_string()),
                    },
                },
            ],
        };

        let report = dialog.validate();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }

    #[test]
    fn dialog_validation_reports_missing_node_and_unreachable_node() {
        let dialog = DialogTree {
            id: "intro".to_string().into(),
            title: String::new(),
            entry_node_id: "start".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "start".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Line {
                        body: "Hello".to_string(),
                        next_node_id: Some("missing".to_string()),
                    },
                },
                DialogNode {
                    id: "unused".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: String::new(),
                        outcome_id: None,
                    },
                },
            ],
        };

        let report = dialog.validate();
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("references missing node 'missing'")));
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn dialog_validation_warns_for_unreachable_nodes_when_graph_is_otherwise_valid() {
        let dialog = DialogTree {
            id: "intro".to_string().into(),
            title: String::new(),
            entry_node_id: "start".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "start".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: "Hello".to_string(),
                        outcome_id: None,
                    },
                },
                DialogNode {
                    id: "unused".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: String::new(),
                        outcome_id: None,
                    },
                },
            ],
        };

        let report = dialog.validate();
        assert!(report.errors.is_empty());
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("node 'unused' is unreachable")));
    }
}
