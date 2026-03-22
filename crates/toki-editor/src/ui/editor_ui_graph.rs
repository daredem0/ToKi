use super::EditorUI;
use crate::editor_services::commands as editor_commands;
use crate::project::SceneGraphLayout;
use crate::ui::rule_graph::RuleGraph;
use crate::ui::undo_redo::EditorCommand;
use std::collections::HashMap;
use toki_core::rules::RuleSet;

#[derive(Debug, Clone)]
pub struct SceneRulesGraphCommandData {
    pub before_rule_set: RuleSet,
    pub after_rule_set: RuleSet,
    pub before_graph: Option<RuleGraph>,
    pub after_graph: RuleGraph,
    pub before_layout: Option<SceneGraphLayout>,
    pub zoom: f32,
    pub pan: [f32; 2],
}

impl EditorUI {
    pub fn load_graph_layouts_from_project(
        &mut self,
        graph_layouts: &HashMap<String, SceneGraphLayout>,
    ) {
        self.graph.layouts_by_scene = graph_layouts.clone();
        self.graph.layout_dirty = false;
    }

    pub fn load_rule_graph_drafts_from_project(&mut self, drafts: &HashMap<String, RuleGraph>) {
        self.graph.rule_graphs_by_scene = drafts.clone();
    }

    pub fn export_graph_layouts_for_project(&self) -> HashMap<String, SceneGraphLayout> {
        self.graph.layouts_by_scene.clone()
    }

    pub fn export_rule_graph_drafts_for_project(&self) -> HashMap<String, RuleGraph> {
        self.graph.rule_graphs_by_scene.clone()
    }

    pub fn is_graph_layout_dirty(&self) -> bool {
        self.graph.layout_dirty
    }

    pub fn clear_graph_layout_dirty(&mut self) {
        self.graph.layout_dirty = false;
    }

    pub fn graph_layout_position(&self, scene_name: &str, node_key: &str) -> Option<[f32; 2]> {
        self.graph
            .layouts_by_scene
            .get(scene_name)
            .and_then(|layout| layout.node_positions.get(node_key).copied())
    }

    pub fn graph_view_for_scene(&self, scene_name: &str) -> (f32, [f32; 2]) {
        if let Some(layout) = self.graph.layouts_by_scene.get(scene_name) {
            (layout.zoom, layout.pan)
        } else {
            (1.0, [16.0, 16.0])
        }
    }

    pub fn set_graph_view_for_scene(&mut self, scene_name: &str, zoom: f32, pan: [f32; 2]) {
        let layout = self
            .graph
            .layouts_by_scene
            .entry(scene_name.to_string())
            .or_default();
        if (layout.zoom - zoom).abs() > f32::EPSILON || layout.pan != pan {
            layout.zoom = zoom;
            layout.pan = pan;
            self.graph.layout_dirty = true;
        }
    }

    pub fn build_scene_graph_layout_snapshot(
        &self,
        scene_name: &str,
        graph: &RuleGraph,
        zoom: f32,
        pan: [f32; 2],
        base_layout: Option<SceneGraphLayout>,
    ) -> SceneGraphLayout {
        let mut layout = base_layout.unwrap_or_else(|| {
            self.graph
                .layouts_by_scene
                .get(scene_name)
                .cloned()
                .unwrap_or_default()
        });
        layout.node_positions.clear();
        for node in &graph.nodes {
            let Some(node_key) = graph.stable_node_key(node.id) else {
                continue;
            };
            layout.node_positions.insert(node_key, node.position);
        }
        layout.zoom = zoom;
        layout.pan = pan;
        layout
    }

    pub fn execute_scene_rules_graph_command(
        &mut self,
        scene_name: &str,
        data: SceneRulesGraphCommandData,
    ) -> bool {
        let after_layout = self.build_scene_graph_layout_snapshot(
            scene_name,
            &data.after_graph,
            data.zoom,
            data.pan,
            data.before_layout.clone(),
        );
        editor_commands::execute(
            self,
            EditorCommand::update_scene_rules_graph(
                scene_name.to_string(),
                data.before_rule_set,
                data.after_rule_set,
                data.before_graph,
                Some(data.after_graph),
                data.before_layout,
                Some(after_layout),
            ),
        )
    }

    pub fn sync_rule_graph_with_rule_set(&mut self, scene_name: &str, rule_set: &RuleSet) {
        let needs_rebuild = match self.graph.rule_graphs_by_scene.get(scene_name) {
            None => true,
            Some(graph) => match graph.to_rule_set() {
                Ok(graph_rules) => graph_rules != *rule_set,
                Err(_) => false,
            },
        };
        if needs_rebuild {
            self.graph
                .rule_graphs_by_scene
                .insert(scene_name.to_string(), RuleGraph::from_rule_set(rule_set));
        }
    }

    pub fn rule_graph_for_scene(&self, scene_name: &str) -> Option<&RuleGraph> {
        self.graph.rule_graphs_by_scene.get(scene_name)
    }

    pub fn set_rule_graph_for_scene(&mut self, scene_name: String, graph: RuleGraph) {
        self.graph.rule_graphs_by_scene.insert(scene_name, graph);
    }
}
