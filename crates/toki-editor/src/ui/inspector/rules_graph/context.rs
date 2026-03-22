//! Context and helper structs for rule graph node editing.

use std::collections::HashMap;

use crate::config::EditorConfig;
use crate::project::SceneGraphLayout;
use crate::ui::rule_graph::RuleGraph;
use crate::ui::EditorUI;

use super::super::{RuleAudioChoices, RuleValidationIssue};
use super::InspectorSystem;

/// Result of node action button interactions.
#[derive(Default)]
pub(super) struct NodeActionResult {
    pub mutated: bool,
    pub error: Option<String>,
    pub pending_connect_from: Option<u64>,
    pub pending_connect_to: Option<u64>,
}

/// Parameters for node editing operations.
pub(super) struct NodeEditParams<'a> {
    pub scene_name: &'a str,
    pub node_key: &'a str,
    pub node_id: u64,
}

/// Context for node editor operations, captures state before modifications.
pub(super) struct NodeEditorContext {
    pub scene_index: usize,
    pub before_rules: toki_core::rules::RuleSet,
    pub before_graph: Option<RuleGraph>,
    pub before_layout: Option<SceneGraphLayout>,
    pub node_badges: HashMap<u64, String>,
    pub audio_choices: RuleAudioChoices,
    pub validation_issues: Vec<RuleValidationIssue>,
}

impl NodeEditorContext {
    /// Creates context and returns both the graph (for mutation) and the context (for read-only data).
    pub fn new(
        ui_state: &mut EditorUI,
        scene_name: &str,
        _node_key: &str,
        config: Option<&EditorConfig>,
    ) -> Option<(RuleGraph, Self)> {
        let scene_index = ui_state.scenes.iter().position(|s| s.name == scene_name)?;
        let scene_rules = ui_state.scenes[scene_index].rules.clone();
        let before_rules = scene_rules.clone();
        let before_graph = ui_state.rule_graph_for_scene(scene_name).cloned();
        let before_layout = ui_state.graph.layouts_by_scene.get(scene_name).cloned();

        ui_state.sync_rule_graph_with_rule_set(scene_name, &scene_rules);

        let graph = ui_state
            .rule_graph_for_scene(scene_name)
            .cloned()
            .unwrap_or_else(|| RuleGraph::from_rule_set(&scene_rules));
        let node_badges = InspectorSystem::rule_graph_node_badges(&graph);
        let audio_choices = InspectorSystem::load_rule_audio_choices(config);
        let validation_issues = InspectorSystem::validate_rule_set(&scene_rules);

        let ctx = Self {
            scene_index,
            before_rules,
            before_graph,
            before_layout,
            node_badges,
            audio_choices,
            validation_issues,
        };

        Some((graph, ctx))
    }
}
