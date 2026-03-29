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

pub(crate) fn load_graph_layouts_from_project(
    ui_state: &mut EditorUI,
    graph_layouts: &HashMap<String, SceneGraphLayout>,
) {
    crate::ui::editor_context::graph_state_mut(ui_state).layouts_by_scene = graph_layouts.clone();
    crate::ui::editor_context::graph_state_mut(ui_state).layout_dirty = false;
}

pub(crate) fn load_rule_graph_drafts_from_project(
    ui_state: &mut EditorUI,
    drafts: &HashMap<String, RuleGraph>,
) {
    crate::ui::editor_context::graph_state_mut(ui_state).rule_graphs_by_scene = drafts.clone();
}

pub(crate) fn export_graph_layouts_for_project(
    ui_state: &EditorUI,
) -> HashMap<String, SceneGraphLayout> {
    crate::ui::editor_context::graph_state(ui_state)
        .layouts_by_scene
        .clone()
}

pub(crate) fn export_rule_graph_drafts_for_project(ui_state: &EditorUI) -> HashMap<String, RuleGraph> {
    crate::ui::editor_context::graph_state(ui_state)
        .rule_graphs_by_scene
        .clone()
}

pub(crate) fn is_graph_layout_dirty(ui_state: &EditorUI) -> bool {
    crate::ui::editor_context::graph_state(ui_state).layout_dirty
}

pub(crate) fn clear_graph_layout_dirty(ui_state: &mut EditorUI) {
    crate::ui::editor_context::graph_state_mut(ui_state).layout_dirty = false;
}

pub(crate) fn graph_layout_position(
    ui_state: &EditorUI,
    scene_name: &str,
    node_key: &str,
) -> Option<[f32; 2]> {
    crate::ui::editor_context::graph_state(ui_state)
        .layouts_by_scene
        .get(scene_name)
        .and_then(|layout| layout.node_positions.get(node_key).copied())
}

pub(crate) fn graph_view_for_scene(ui_state: &EditorUI, scene_name: &str) -> (f32, [f32; 2]) {
    if let Some(layout) = crate::ui::editor_context::graph_state(ui_state)
        .layouts_by_scene
        .get(scene_name)
    {
        (layout.zoom, layout.pan)
    } else {
        (1.0, [16.0, 16.0])
    }
}

pub(crate) fn set_graph_view_for_scene(
    ui_state: &mut EditorUI,
    scene_name: &str,
    zoom: f32,
    pan: [f32; 2],
) {
    let layout = crate::ui::editor_context::graph_state_mut(ui_state)
        .layouts_by_scene
        .entry(scene_name.to_string())
        .or_default();
    if (layout.zoom - zoom).abs() > f32::EPSILON || layout.pan != pan {
        layout.zoom = zoom;
        layout.pan = pan;
        crate::ui::editor_context::graph_state_mut(ui_state).layout_dirty = true;
    }
}

pub(crate) fn build_scene_graph_layout_snapshot(
    ui_state: &EditorUI,
    scene_name: &str,
    graph: &RuleGraph,
    zoom: f32,
    pan: [f32; 2],
    base_layout: Option<SceneGraphLayout>,
) -> SceneGraphLayout {
    let mut layout = base_layout.unwrap_or_else(|| {
        crate::ui::editor_context::graph_state(ui_state)
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

pub(crate) fn execute_scene_rules_graph_command(
    ui_state: &mut EditorUI,
    scene_name: &str,
    data: SceneRulesGraphCommandData,
) -> bool {
    let after_layout = build_scene_graph_layout_snapshot(
        ui_state,
        scene_name,
        &data.after_graph,
        data.zoom,
        data.pan,
        data.before_layout.clone(),
    );
    editor_commands::execute(
        ui_state,
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

pub(crate) fn sync_rule_graph_with_rule_set(
    ui_state: &mut EditorUI,
    scene_name: &str,
    rule_set: &RuleSet,
) {
    let needs_rebuild = match crate::ui::editor_context::graph_state_mut(ui_state)
        .rule_graphs_by_scene
        .get(scene_name)
    {
        None => true,
        Some(graph) => match graph.to_rule_set() {
            Ok(graph_rules) => graph_rules != *rule_set,
            Err(_) => false,
        },
    };
    if needs_rebuild {
        crate::ui::editor_context::graph_state_mut(ui_state)
            .rule_graphs_by_scene
            .insert(scene_name.to_string(), RuleGraph::from_rule_set(rule_set));
    }
}

pub(crate) fn rule_graph_for_scene<'a>(
    ui_state: &'a EditorUI,
    scene_name: &str,
) -> Option<&'a RuleGraph> {
    crate::ui::editor_context::graph_state(ui_state)
        .rule_graphs_by_scene
        .get(scene_name)
}

pub(crate) fn set_rule_graph_for_scene(ui_state: &mut EditorUI, scene_name: String, graph: RuleGraph) {
    crate::ui::editor_context::graph_state_mut(ui_state)
        .rule_graphs_by_scene
        .insert(scene_name, graph);
}
