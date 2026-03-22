//! Tests for domain inspectors.

use super::*;
use crate::project::Project;
use crate::ui::editor_ui::Selection;
use crate::ui::inspector_trait::InspectorContext;
use crate::ui::EditorUI;

#[test]
fn create_inspector_for_none_returns_no_selection_inspector() {
    let inspector = create_inspector_for_selection(None);
    assert_eq!(inspector.name(), "NoSelection");
}

#[test]
fn create_inspector_for_scene_returns_scene_inspector() {
    let selection = Selection::Scene("TestScene".to_string());
    let inspector = create_inspector_for_selection(Some(&selection));
    assert_eq!(inspector.name(), "Scene");
}

#[test]
fn create_inspector_for_scene_player_entry_returns_scene_player_entry_inspector() {
    let selection = Selection::ScenePlayerEntry("TestScene".to_string());
    let inspector = create_inspector_for_selection(Some(&selection));
    assert_eq!(inspector.name(), "ScenePlayerEntry");
}

#[test]
fn create_inspector_for_entity_returns_entity_inspector() {
    let selection = Selection::Entity(42);
    let inspector = create_inspector_for_selection(Some(&selection));
    assert_eq!(inspector.name(), "Entity");
}

#[test]
fn create_inspector_for_map_returns_map_inspector() {
    let selection = Selection::Map("Scene1".to_string(), "map1".to_string());
    let inspector = create_inspector_for_selection(Some(&selection));
    assert_eq!(inspector.name(), "Map");
}

#[test]
fn create_inspector_for_standalone_map_returns_standalone_map_inspector() {
    let selection = Selection::StandaloneMap("standalone".to_string());
    let inspector = create_inspector_for_selection(Some(&selection));
    assert_eq!(inspector.name(), "StandaloneMap");
}

#[test]
fn create_inspector_for_entity_definition_returns_entity_definition_inspector() {
    let selection = Selection::EntityDefinition("player".to_string());
    let inspector = create_inspector_for_selection(Some(&selection));
    assert_eq!(inspector.name(), "EntityDefinition");
}

#[test]
fn create_inspector_for_rule_graph_node_returns_rule_graph_node_inspector() {
    let selection = Selection::RuleGraphNode {
        scene_name: "Scene1".to_string(),
        node_key: "node1".to_string(),
    };
    let inspector = create_inspector_for_selection(Some(&selection));
    assert_eq!(inspector.name(), "RuleGraphNode");
}

#[test]
fn create_inspector_for_menu_selection_returns_menu_selection_inspector() {
    let selection = Selection::MenuScreen("main".to_string());
    let inspector = create_inspector_for_selection(Some(&selection));
    assert_eq!(inspector.name(), "MenuSelection");
}

#[test]
fn entity_inspector_has_correct_name() {
    let inspector = EntityInspector::new(1);
    assert_eq!(inspector.name(), "Entity");
}

#[test]
fn scene_inspector_has_correct_name() {
    let inspector = SceneInspector::new("TestScene".to_string());
    assert_eq!(inspector.name(), "Scene");
}

#[test]
fn scene_player_entry_inspector_has_correct_name() {
    let inspector = ScenePlayerEntryInspector::new("TestScene".to_string());
    assert_eq!(inspector.name(), "ScenePlayerEntry");
}

#[test]
fn map_inspector_has_correct_name() {
    let inspector = MapInspector::new("Scene1".to_string(), "map1".to_string());
    assert_eq!(inspector.name(), "Map");
}

#[test]
fn standalone_map_inspector_has_correct_name() {
    let inspector = StandaloneMapInspector::new("standalone".to_string());
    assert_eq!(inspector.name(), "StandaloneMap");
}

#[test]
fn entity_definition_inspector_has_correct_name() {
    let inspector = EntityDefinitionInspector::new("player".to_string());
    assert_eq!(inspector.name(), "EntityDefinition");
}

#[test]
fn rule_graph_node_inspector_has_correct_name() {
    let inspector = RuleGraphNodeInspector::new("Scene1".to_string(), "node1".to_string());
    assert_eq!(inspector.name(), "RuleGraphNode");
}

#[test]
fn menu_selection_inspector_has_correct_name() {
    let inspector = MenuSelectionInspector;
    assert_eq!(inspector.name(), "MenuSelection");
}

#[test]
fn build_delete_scene_command_resolves_existing_scene_file_when_metadata_path_is_stale() {
    let temp_dir = tempfile::tempdir().expect("temp dir should exist");
    let project_root = temp_dir.path().to_path_buf();
    std::fs::create_dir_all(project_root.join("scenes")).expect("scenes dir should exist");
    std::fs::write(
        project_root.join("scenes").join("Main Scene.json"),
        "{\n  \"name\": \"Main Scene\"\n}\n",
    )
    .expect("scene file should write");

    let mut project = Project::new("Demo".to_string(), project_root);
    project.metadata.scenes.insert(
        "Main Scene".to_string(),
        "scenes/mainscene.json".to_string(),
    );
    std::fs::write(
        project.project_file_path(),
        toml::to_string_pretty(&project.metadata).expect("project metadata should serialize"),
    )
    .expect("project metadata should write");

    let mut ui_state = EditorUI::new();
    ui_state.scenes = vec![toki_core::Scene::new("Main Scene".to_string())];
    ui_state.active_scene = Some("Main Scene".to_string());
    ui_state.set_selection(Selection::Scene("Main Scene".to_string()));

    let command = build_delete_scene_command(&ui_state, &project, "Main Scene");
    assert!(command.is_ok());
}

#[test]
fn build_delete_scene_command_allows_scene_without_backing_file() {
    let temp_dir = tempfile::tempdir().expect("temp dir should exist");
    let project_root = temp_dir.path().to_path_buf();

    let project = Project::new("Demo".to_string(), project_root);
    std::fs::write(
        project.project_file_path(),
        toml::to_string_pretty(&project.metadata).expect("project metadata should serialize"),
    )
    .expect("project metadata should write");

    let mut ui_state = EditorUI::new();
    ui_state.scenes = vec![toki_core::Scene::new("Scene 3".to_string())];
    ui_state.active_scene = Some("Scene 3".to_string());
    ui_state.set_selection(Selection::Scene("Scene 3".to_string()));

    let command = build_delete_scene_command(&ui_state, &project, "Scene 3");
    assert!(command.is_ok());
}

#[test]
fn load_scene_music_choices_discovers_tracks_from_project_music_folder() {
    let temp_dir = tempfile::tempdir().expect("temp dir should exist");
    let project_root = temp_dir.path().to_path_buf();
    let music_dir = project_root.join("assets").join("audio").join("music");
    std::fs::create_dir_all(&music_dir).expect("music dir should exist");
    std::fs::write(music_dir.join("forest.ogg"), b"").expect("forest track should write");
    std::fs::write(music_dir.join("town.wav"), b"").expect("town track should write");
    std::fs::write(music_dir.join("ignore.txt"), b"").expect("ignore file should write");

    let project = Project::new("Demo".to_string(), project_root);
    let mut ui_state = EditorUI::new();
    let inspector_ctx = InspectorContext {
        ui_state: &mut ui_state,
        game_state: None,
        project: Some(&mut project.clone()),
        config: None,
    };

    let choices = SceneInspector::load_scene_music_choices(&inspector_ctx, None);
    assert_eq!(choices, vec!["forest".to_string(), "town".to_string()]);
}

#[test]
fn load_scene_music_choices_keeps_current_track_even_if_not_discovered() {
    let mut ui_state = EditorUI::new();
    let temp_dir = tempfile::tempdir().expect("temp dir should exist");
    let project = Project::new("Demo".to_string(), temp_dir.path().to_path_buf());
    let mut project = project;
    let inspector_ctx = InspectorContext {
        ui_state: &mut ui_state,
        game_state: None,
        project: Some(&mut project),
        config: None,
    };

    let choices = SceneInspector::load_scene_music_choices(&inspector_ctx, Some("legacy_track"));
    assert_eq!(choices, vec!["legacy_track".to_string()]);
}
