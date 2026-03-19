use super::{EditorCommand, EntityPosition, IndexedEntity, UndoRedoHistory};
use crate::project::Project;
use crate::project::SceneGraphLayout;
use crate::ui::inspector::build_delete_scene_command;
use crate::ui::rule_graph::RuleGraph;
use crate::ui::EditorUI;
use glam::{IVec2, UVec2};
use tempfile::tempdir;
use toki_core::entity::{Entity, EntityAttributes, EntityKind};
use toki_core::rules::{Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleTrigger};
use toki_template_lowering::ProjectFileChange;

fn sample_entity(id: u32, position: IVec2) -> Entity {
    Entity {
        id,
        position,
        size: UVec2::new(16, 16),
        entity_kind: EntityKind::Npc,
        category: "creature".to_string(),
        definition_name: Some("npc".to_string()),
        control_role: toki_core::entity::ControlRole::None,
        audio: toki_core::entity::EntityAudioSettings::default(),
        attributes: EntityAttributes::default(),
        collision_box: None,
        movement_accumulator: glam::Vec2::ZERO,
    }
}

fn sample_rule_set() -> RuleSet {
    RuleSet {
        rules: vec![Rule {
            id: "rule_1".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnStart,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_start".to_string(),
            }],
        }],
    }
}

fn scene_rules(ui_state: &EditorUI) -> RuleSet {
    ui_state
        .scenes
        .iter()
        .find(|scene| scene.name == "Main Scene")
        .expect("main scene should exist")
        .rules
        .clone()
}

fn scene_graph(ui_state: &EditorUI) -> RuleGraph {
    ui_state
        .rule_graph_for_scene("Main Scene")
        .cloned()
        .expect("scene graph should exist")
}

fn seed_scene_graph(ui_state: &mut EditorUI, rules: RuleSet) {
    let graph = RuleGraph::from_rule_set(&rules);
    let scene = ui_state
        .scenes
        .iter_mut()
        .find(|scene| scene.name == "Main Scene")
        .expect("main scene should exist");
    scene.rules = rules;
    ui_state.set_rule_graph_for_scene("Main Scene".to_string(), graph);
}

fn apply_graph_transition(
    history: &mut UndoRedoHistory,
    ui_state: &mut EditorUI,
    zoom: f32,
    pan: [f32; 2],
    mutate: impl FnOnce(&mut RuleGraph),
) {
    let before_rule_set = scene_rules(ui_state);
    let before_graph = ui_state.rule_graph_for_scene("Main Scene").cloned();
    let before_layout = ui_state.graph.layouts_by_scene.get("Main Scene").cloned();
    let mut after_graph = before_graph
        .clone()
        .unwrap_or_else(|| RuleGraph::from_rule_set(&before_rule_set));
    mutate(&mut after_graph);
    let after_rule_set = after_graph
        .to_rule_set()
        .expect("mutated graph should remain serializable");

    let mut after_layout = before_layout.clone().unwrap_or_default();
    after_layout.node_positions.clear();
    for node in &after_graph.nodes {
        let Some(node_key) = after_graph.stable_node_key(node.id) else {
            continue;
        };
        after_layout.node_positions.insert(node_key, node.position);
    }
    after_layout.zoom = zoom;
    after_layout.pan = pan;

    assert!(history.execute(
        EditorCommand::update_scene_rules_graph(
            "Main Scene",
            before_rule_set,
            after_rule_set,
            before_graph,
            Some(after_graph),
            before_layout,
            Some(after_layout),
        ),
        ui_state,
        None
    ));
}

fn main_scene_entities(ui_state: &EditorUI) -> Vec<Entity> {
    ui_state
        .scenes
        .iter()
        .find(|scene| scene.name == "Main Scene")
        .expect("main scene should exist")
        .entities
        .clone()
}

#[test]
fn execute_clears_redo_stack_when_new_command_is_applied() {
    let mut ui_state = EditorUI::new();
    let mut history = UndoRedoHistory::default();

    assert!(history.execute(
        EditorCommand::add_entity("Main Scene", sample_entity(1, IVec2::new(1, 1))),
        &mut ui_state,
        None
    ));
    assert!(history.can_undo());
    assert!(!history.can_redo());

    assert!(history.undo(&mut ui_state, None));
    assert!(history.can_redo());

    assert!(history.execute(
        EditorCommand::add_entity("Main Scene", sample_entity(2, IVec2::new(2, 2))),
        &mut ui_state,
        None
    ));
    assert!(history.can_undo());
    assert!(!history.can_redo());
    assert_eq!(history.undo_stack.len(), 1);
    assert_eq!(history.redo_stack.len(), 0);
}

#[test]
fn add_entity_command_supports_undo_and_redo() {
    let mut ui_state = EditorUI::new();
    let mut history = UndoRedoHistory::default();

    let command = EditorCommand::add_entity("Main Scene", sample_entity(7, IVec2::new(4, 8)));
    assert!(history.execute(command, &mut ui_state, None));
    assert_eq!(main_scene_entities(&ui_state).len(), 1);

    assert!(history.undo(&mut ui_state, None));
    assert!(main_scene_entities(&ui_state).is_empty());

    assert!(history.redo(&mut ui_state, None));
    assert_eq!(main_scene_entities(&ui_state).len(), 1);
}

#[test]
fn move_entities_command_round_trips_positions() {
    let mut ui_state = EditorUI::new();
    let mut history = UndoRedoHistory::default();

    let first = sample_entity(1, IVec2::new(10, 20));
    let second = sample_entity(2, IVec2::new(30, 40));
    ui_state
        .scenes
        .iter_mut()
        .find(|scene| scene.name == "Main Scene")
        .expect("main scene should exist")
        .entities
        .extend([first, second]);

    let command = EditorCommand::move_entities(
        "Main Scene",
        vec![
            EntityPosition::new(1, IVec2::new(10, 20)),
            EntityPosition::new(2, IVec2::new(30, 40)),
        ],
        vec![
            EntityPosition::new(1, IVec2::new(15, 17)),
            EntityPosition::new(2, IVec2::new(35, 37)),
        ],
    );
    assert!(history.execute(command, &mut ui_state, None));

    let entities = main_scene_entities(&ui_state);
    assert_eq!(
        entities
            .iter()
            .find(|entity| entity.id == 1)
            .expect("first entity should exist")
            .position,
        IVec2::new(15, 17)
    );
    assert_eq!(
        entities
            .iter()
            .find(|entity| entity.id == 2)
            .expect("second entity should exist")
            .position,
        IVec2::new(35, 37)
    );

    assert!(history.undo(&mut ui_state, None));
    let entities = main_scene_entities(&ui_state);
    assert_eq!(
        entities
            .iter()
            .find(|entity| entity.id == 1)
            .expect("first entity should exist")
            .position,
        IVec2::new(10, 20)
    );
    assert_eq!(
        entities
            .iter()
            .find(|entity| entity.id == 2)
            .expect("second entity should exist")
            .position,
        IVec2::new(30, 40)
    );
}

#[test]
fn update_entities_command_restores_previous_state_on_undo() {
    let mut ui_state = EditorUI::new();
    let mut history = UndoRedoHistory::default();

    let before = sample_entity(42, IVec2::new(5, 5));
    ui_state
        .scenes
        .iter_mut()
        .find(|scene| scene.name == "Main Scene")
        .expect("main scene should exist")
        .entities
        .push(before.clone());

    let mut after = before.clone();
    after.attributes.visible = false;
    after.attributes.render_layer = 9;

    let command =
        EditorCommand::update_entities("Main Scene", vec![before.clone()], vec![after.clone()]);
    assert!(history.execute(command, &mut ui_state, None));

    let entity = main_scene_entities(&ui_state)
        .into_iter()
        .find(|entity| entity.id == 42)
        .expect("entity should exist");
    assert!(!entity.attributes.visible);
    assert_eq!(entity.attributes.render_layer, 9);

    assert!(history.undo(&mut ui_state, None));
    let entity = main_scene_entities(&ui_state)
        .into_iter()
        .find(|entity| entity.id == 42)
        .expect("entity should exist");
    assert!(entity.attributes.visible);
    assert_eq!(entity.attributes.render_layer, 0);
}

#[test]
fn remove_entities_command_restores_original_order_on_undo() {
    let mut ui_state = EditorUI::new();
    let mut history = UndoRedoHistory::default();

    let entities = vec![
        sample_entity(1, IVec2::new(0, 0)),
        sample_entity(2, IVec2::new(16, 0)),
        sample_entity(3, IVec2::new(32, 0)),
    ];
    ui_state
        .scenes
        .iter_mut()
        .find(|scene| scene.name == "Main Scene")
        .expect("main scene should exist")
        .entities
        .extend(entities.clone());

    let command = EditorCommand::remove_entities(
        "Main Scene",
        vec![
            IndexedEntity {
                index: 0,
                entity: entities[0].clone(),
            },
            IndexedEntity {
                index: 2,
                entity: entities[2].clone(),
            },
        ],
    );
    assert!(history.execute(command, &mut ui_state, None));
    let ids = main_scene_entities(&ui_state)
        .into_iter()
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![2]);

    assert!(history.undo(&mut ui_state, None));
    let ids = main_scene_entities(&ui_state)
        .into_iter()
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![1, 2, 3]);
}

#[test]
fn apply_project_file_changes_command_round_trips_file_contents_and_selection() {
    let temp = tempdir().expect("temp dir should exist");
    let project_root = temp.path().to_path_buf();
    std::fs::create_dir_all(project_root.join("entities")).expect("entities dir should exist");
    let entity_path = project_root.join("entities/player.json");
    std::fs::write(&entity_path, "{\n  \"name\": \"before\"\n}\n")
        .expect("before file should write");

    let mut project = Project::new("TestProject".to_string(), project_root.clone());
    let mut ui_state = EditorUI::new();
    ui_state.set_selection(crate::ui::editor_ui::Selection::Scene(
        "Main Scene".to_string(),
    ));
    let mut history = UndoRedoHistory::default();

    let command = EditorCommand::apply_project_file_changes(
        "apply template entity update",
        vec![ProjectFileChange {
            relative_path: std::path::PathBuf::from("entities/player.json"),
            before_contents: Some("{\n  \"name\": \"before\"\n}\n".to_string()),
            after_contents: Some("{\n  \"name\": \"after\"\n}\n".to_string()),
        }],
        Some(crate::ui::editor_ui::Selection::Scene(
            "Main Scene".to_string(),
        )),
        Some(crate::ui::editor_ui::Selection::EntityDefinition(
            "player".to_string(),
        )),
        None,
        None,
    );

    assert!(history.execute(command, &mut ui_state, Some(&mut project)));
    assert_eq!(
        std::fs::read_to_string(&entity_path).expect("after file should read"),
        "{\n  \"name\": \"after\"\n}\n"
    );
    assert!(matches!(
        ui_state.selection,
        Some(crate::ui::editor_ui::Selection::EntityDefinition(ref id)) if id == "player"
    ));
    assert!(ui_state.project.rescan_assets_requested);

    ui_state.project.rescan_assets_requested = false;
    assert!(history.undo(&mut ui_state, Some(&mut project)));
    assert_eq!(
        std::fs::read_to_string(&entity_path).expect("before file should read"),
        "{\n  \"name\": \"before\"\n}\n"
    );
    assert!(matches!(
        ui_state.selection,
        Some(crate::ui::editor_ui::Selection::Scene(ref id)) if id == "Main Scene"
    ));
    assert!(ui_state.project.rescan_assets_requested);
}

#[test]
fn delete_scene_command_round_trips_scene_file_ui_state_and_project_metadata() {
    let temp = tempdir().expect("temp dir should exist");
    let project_root = temp.path().to_path_buf();
    std::fs::create_dir_all(project_root.join("scenes")).expect("scenes dir should exist");
    let main_scene_file = project_root.join("scenes/Main Scene.json");
    std::fs::write(&main_scene_file, "{\n  \"name\": \"Main Scene\"\n}\n")
        .expect("scene file should write");

    let mut project = Project::new("TestProject".to_string(), project_root.clone());
    project
        .metadata
        .scenes
        .insert("Main Scene".to_string(), "scenes/Main Scene.json".to_string());
    std::fs::write(
        project.project_file_path(),
        toml::to_string_pretty(&project.metadata).expect("project metadata should serialize"),
    )
    .expect("project metadata should write");

    let mut ui_state = EditorUI::new();
    ui_state.scenes = vec![
        toki_core::Scene::new("Main Scene".to_string()),
        toki_core::Scene::new("Backup Scene".to_string()),
    ];
    ui_state.active_scene = Some("Main Scene".to_string());
    ui_state.set_selection(crate::ui::editor_ui::Selection::Scene(
        "Main Scene".to_string(),
    ));

    let mut metadata_after = project.metadata.clone();
    metadata_after.scenes.remove("Main Scene");
    let command = EditorCommand::delete_scene(super::DeleteSceneCommandData {
        removed_scene: super::SceneSnapshot {
            index: 0,
            scene: toki_core::Scene::new("Main Scene".to_string()),
        },
        active_scene_before: Some("Main Scene".to_string()),
        active_scene_after: Some("Backup Scene".to_string()),
        selection_before: Some(crate::ui::editor_ui::Selection::Scene(
            "Main Scene".to_string(),
        )),
        selection_after: Some(crate::ui::editor_ui::Selection::Scene(
            "Backup Scene".to_string(),
        )),
        changes: vec![
            ProjectFileChange {
                relative_path: std::path::PathBuf::from("scenes/Main Scene.json"),
                before_contents: Some("{\n  \"name\": \"Main Scene\"\n}\n".to_string()),
                after_contents: None,
            },
            ProjectFileChange {
                relative_path: std::path::PathBuf::from("project.toml"),
                before_contents: Some(
                    std::fs::read_to_string(project.project_file_path())
                        .expect("project metadata should read"),
                ),
                after_contents: Some(
                    toml::to_string_pretty(&metadata_after)
                        .expect("updated project metadata should serialize"),
                ),
            },
        ],
        project_metadata_before: Some(project.metadata.clone()),
        project_metadata_after: Some(metadata_after.clone()),
    });

    let mut history = UndoRedoHistory::default();
    assert!(history.execute(command, &mut ui_state, Some(&mut project)));
    assert_eq!(
        ui_state
            .scenes
            .iter()
            .map(|scene| scene.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Backup Scene"]
    );
    assert_eq!(ui_state.active_scene.as_deref(), Some("Backup Scene"));
    assert!(!main_scene_file.exists());
    assert!(!project.metadata.scenes.contains_key("Main Scene"));

    assert!(history.undo(&mut ui_state, Some(&mut project)));
    assert_eq!(
        ui_state
            .scenes
            .iter()
            .map(|scene| scene.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Main Scene", "Backup Scene"]
    );
    assert_eq!(ui_state.active_scene.as_deref(), Some("Main Scene"));
    assert!(main_scene_file.exists());
    assert!(project.metadata.scenes.contains_key("Main Scene"));
}

#[test]
fn delete_scene_command_round_trips_ui_and_project_metadata_without_scene_file() {
    let temp = tempdir().expect("temp dir should exist");
    let project_root = temp.path().to_path_buf();
    let mut project = Project::new("TestProject".to_string(), project_root.clone());
    std::fs::write(
        project.project_file_path(),
        toml::to_string_pretty(&project.metadata).expect("project metadata should serialize"),
    )
    .expect("project metadata should write");

    let mut ui_state = EditorUI::new();
    ui_state.scenes = vec![
        toki_core::Scene::new("Scene 3".to_string()),
        toki_core::Scene::new("Backup Scene".to_string()),
    ];
    ui_state.active_scene = Some("Scene 3".to_string());
    ui_state.set_selection(crate::ui::editor_ui::Selection::Scene(
        "Scene 3".to_string(),
    ));

    let command =
        build_delete_scene_command(&ui_state, &project, "Scene 3").expect("command should build");

    let mut history = UndoRedoHistory::default();
    assert!(history.execute(command, &mut ui_state, Some(&mut project)));
    assert_eq!(
        ui_state
            .scenes
            .iter()
            .map(|scene| scene.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Backup Scene"]
    );
    assert_eq!(ui_state.active_scene.as_deref(), Some("Backup Scene"));
    assert!(!project.metadata.scenes.contains_key("Scene 3"));

    assert!(history.undo(&mut ui_state, Some(&mut project)));
    assert_eq!(
        ui_state
            .scenes
            .iter()
            .map(|scene| scene.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Scene 3", "Backup Scene"]
    );
    assert_eq!(ui_state.active_scene.as_deref(), Some("Scene 3"));
}

#[test]
fn execute_noop_command_does_not_affect_history() {
    let mut ui_state = EditorUI::new();
    let mut history = UndoRedoHistory::default();

    let command = EditorCommand::add_entity("Missing Scene", sample_entity(1, IVec2::new(0, 0)));
    assert!(!history.execute(command, &mut ui_state, None));
    assert_eq!(history.undo_stack.len(), 0);
    assert_eq!(history.redo_stack.len(), 0);
    assert!(!history.can_undo());
    assert!(!history.can_redo());
}

#[test]
fn update_scene_rules_graph_command_round_trips_rules_graph_and_layout() {
    let mut ui_state = EditorUI::new();
    let mut history = UndoRedoHistory::default();

    let before_rule_set = RuleSet::default();
    let after_rule_set = RuleSet {
        rules: vec![Rule {
            id: "rule_1".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnStart,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_start".to_string(),
            }],
        }],
    };

    let before_graph = None;
    let after_graph = Some(RuleGraph::from_rule_set(&after_rule_set));
    let before_layout = None;
    let mut after_layout = SceneGraphLayout::default();
    after_layout
        .node_positions
        .insert("rule_1::trigger".to_string(), [12.0, 34.0]);
    after_layout.zoom = 1.25;
    after_layout.pan = [32.0, 48.0];

    let command = EditorCommand::update_scene_rules_graph(
        "Main Scene",
        before_rule_set.clone(),
        after_rule_set.clone(),
        before_graph,
        after_graph.clone(),
        before_layout,
        Some(after_layout.clone()),
    );
    assert!(history.execute(command, &mut ui_state, None));

    let scene = ui_state
        .scenes
        .iter()
        .find(|scene| scene.name == "Main Scene")
        .expect("main scene should exist");
    assert_eq!(scene.rules, after_rule_set);
    assert_eq!(
        ui_state.rule_graph_for_scene("Main Scene"),
        after_graph.as_ref()
    );
    let layout = ui_state
        .graph
        .layouts_by_scene
        .get("Main Scene")
        .expect("graph layout should exist");
    assert_eq!(layout.node_positions, after_layout.node_positions);
    assert_eq!(layout.zoom, after_layout.zoom);
    assert_eq!(layout.pan, after_layout.pan);

    assert!(history.undo(&mut ui_state, None));
    let scene = ui_state
        .scenes
        .iter()
        .find(|scene| scene.name == "Main Scene")
        .expect("main scene should exist");
    assert_eq!(scene.rules, before_rule_set);
    assert!(ui_state.rule_graph_for_scene("Main Scene").is_none());
    assert!(!ui_state.graph.layouts_by_scene.contains_key("Main Scene"));
}

#[test]
fn graph_connect_and_disconnect_operations_are_undoable() {
    let mut ui_state = EditorUI::new();
    let mut history = UndoRedoHistory::default();
    seed_scene_graph(&mut ui_state, sample_rule_set());

    let graph_before = scene_graph(&ui_state);
    let trigger = graph_before.chains[0].trigger_node_id;
    let sequence = graph_before
        .chain_node_sequence(trigger)
        .expect("chain sequence should resolve");
    let condition = sequence[1];
    let action = sequence[2];

    apply_graph_transition(&mut history, &mut ui_state, 1.0, [16.0, 16.0], |graph| {
        assert!(graph.disconnect_nodes(condition, action));
    });
    assert!(!scene_graph(&ui_state)
        .edges
        .iter()
        .any(|edge| edge.from == condition && edge.to == action));

    apply_graph_transition(&mut history, &mut ui_state, 1.0, [16.0, 16.0], |graph| {
        graph
            .connect_nodes(condition, action)
            .expect("reconnect should succeed");
    });
    assert!(scene_graph(&ui_state)
        .edges
        .iter()
        .any(|edge| edge.from == condition && edge.to == action));

    assert!(history.undo(&mut ui_state, None));
    assert!(!scene_graph(&ui_state)
        .edges
        .iter()
        .any(|edge| edge.from == condition && edge.to == action));
    assert!(history.undo(&mut ui_state, None));
    assert!(scene_graph(&ui_state)
        .edges
        .iter()
        .any(|edge| edge.from == condition && edge.to == action));
}

#[test]
fn graph_node_rule_deletion_is_undoable() {
    let mut ui_state = EditorUI::new();
    let mut history = UndoRedoHistory::default();
    let mut rules = sample_rule_set();
    rules.rules.push(Rule {
        id: "rule_2".to_string(),
        enabled: true,
        priority: 0,
        once: false,
        trigger: RuleTrigger::OnUpdate,
        conditions: vec![RuleCondition::Always],
        actions: vec![RuleAction::PlaySound {
            channel: RuleSoundChannel::Movement,
            sound_id: "sfx_loop".to_string(),
        }],
    });
    seed_scene_graph(&mut ui_state, rules);
    let trigger_to_remove = scene_graph(&ui_state).chains[0].trigger_node_id;

    apply_graph_transition(&mut history, &mut ui_state, 1.0, [16.0, 16.0], |graph| {
        graph
            .remove_node(trigger_to_remove)
            .expect("trigger deletion should remove full rule chain");
    });
    assert_eq!(scene_rules(&ui_state).rules.len(), 1);

    assert!(history.undo(&mut ui_state, None));
    assert_eq!(scene_rules(&ui_state).rules.len(), 2);
    assert!(history.redo(&mut ui_state, None));
    assert_eq!(scene_rules(&ui_state).rules.len(), 1);
}

#[test]
fn layout_reset_like_updates_are_undoable() {
    let mut ui_state = EditorUI::new();
    let mut history = UndoRedoHistory::default();
    seed_scene_graph(&mut ui_state, sample_rule_set());

    let mut initial_layout = SceneGraphLayout {
        zoom: 1.2,
        pan: [8.0, 12.0],
        ..SceneGraphLayout::default()
    };
    {
        let graph = scene_graph(&ui_state);
        for (index, node) in graph.nodes.iter().enumerate() {
            if let Some(key) = graph.stable_node_key(node.id) {
                initial_layout
                    .node_positions
                    .insert(key, [24.0 + index as f32 * 32.0, 48.0]);
            }
        }
    }
    ui_state
        .graph
        .layouts_by_scene
        .insert("Main Scene".to_string(), initial_layout.clone());

    apply_graph_transition(&mut history, &mut ui_state, 0.8, [16.0, 16.0], |graph| {
        let node_ids = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        for (index, node_id) in node_ids.into_iter().enumerate() {
            graph
                .set_node_position(node_id, [100.0 + index as f32 * 40.0, 180.0])
                .expect("node position should update");
        }
    });
    let updated_layout = ui_state
        .graph
        .layouts_by_scene
        .get("Main Scene")
        .expect("updated layout should exist");
    assert_eq!(updated_layout.zoom, 0.8);
    assert_eq!(updated_layout.pan, [16.0, 16.0]);

    assert!(history.undo(&mut ui_state, None));
    let restored_layout = ui_state
        .graph
        .layouts_by_scene
        .get("Main Scene")
        .expect("restored layout should exist");
    assert_eq!(restored_layout.zoom, initial_layout.zoom);
    assert_eq!(restored_layout.pan, initial_layout.pan);
    assert_eq!(
        restored_layout.node_positions,
        initial_layout.node_positions
    );
}

#[test]
fn inspector_like_node_edit_updates_are_undoable() {
    let mut ui_state = EditorUI::new();
    let mut history = UndoRedoHistory::default();
    seed_scene_graph(&mut ui_state, sample_rule_set());

    let graph = scene_graph(&ui_state);
    let trigger = graph.chains[0].trigger_node_id;
    let sequence = graph
        .chain_node_sequence(trigger)
        .expect("chain sequence should resolve");
    let action_node = sequence[2];

    apply_graph_transition(&mut history, &mut ui_state, 1.0, [16.0, 16.0], |graph| {
        graph
            .set_action_for_node(
                action_node,
                RuleAction::PlayMusic {
                    track_id: "lavandia".to_string(),
                },
            )
            .expect("action update should succeed");
    });
    assert!(matches!(
        &scene_rules(&ui_state).rules[0].actions[0],
        RuleAction::PlayMusic { track_id } if track_id == "lavandia"
    ));

    assert!(history.undo(&mut ui_state, None));
    assert!(matches!(
        &scene_rules(&ui_state).rules[0].actions[0],
        RuleAction::PlaySound { .. }
    ));
}

#[test]
fn ui_rule_mutation_paths_route_through_command_history_layer() {
    let panels_src = include_str!("panels.rs");
    let inspector_src = include_str!("inspector.rs");
    assert!(
        !panels_src.contains("scenes[scene_index].rules ="),
        "panels should not write scene rules directly"
    );
    assert!(
        !inspector_src.contains("scenes[scene_index].rules ="),
        "inspector should not write scene rules directly"
    );
}
