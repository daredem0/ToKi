use glam::{IVec2, UVec2};
use tempfile::tempdir;
use toki_core::entity::{EntityAttributes, EntityKind};
use toki_core::menu::{MenuItemDefinition, MenuScreenDefinition, UiAction};
use toki_core::rules::{Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleTrigger};

use super::{EditorUI, MapEditorDraft, Selection};
use crate::project::Project;
use crate::ui::rule_graph::RuleGraph;
use crate::ui::undo_redo::EditorCommand;

fn sample_entity(id: u32, position: IVec2) -> toki_core::entity::Entity {
    toki_core::entity::Entity {
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

#[test]
fn sync_rule_graph_with_rule_set_preserves_unserializable_existing_draft() {
    let mut ui = EditorUI::new();
    let rule_set = RuleSet {
        rules: vec![Rule {
            id: "rule_1".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx".to_string(),
            }],
        }],
    };
    let mut graph = RuleGraph::from_rule_set(&rule_set);
    let trigger_id = graph.chains[0].trigger_node_id;
    let detached_target = graph
        .add_condition_node(RuleCondition::KeyHeld {
            key: toki_core::rules::RuleKey::Left,
        })
        .expect("detached target should be created");
    graph
        .connect_nodes(trigger_id, detached_target)
        .expect("branching connect should succeed");
    assert!(
        graph.to_rule_set().is_err(),
        "graph should be intentionally non-serializable due to branching"
    );

    ui.set_rule_graph_for_scene("Main Scene".to_string(), graph.clone());
    ui.sync_rule_graph_with_rule_set("Main Scene", &rule_set);

    let persisted_graph = ui
        .rule_graph_for_scene("Main Scene")
        .expect("graph draft should still exist");
    assert!(
        persisted_graph
            .edges
            .iter()
            .any(|edge| edge.from == trigger_id && edge.to == detached_target),
        "branching edge should be preserved instead of rebuilding from RuleSet"
    );
}

#[test]
fn add_entity_to_selection_preserves_existing_and_avoids_duplicates() {
    let mut ui = EditorUI::new();

    ui.add_entity_to_selection(1);
    ui.add_entity_to_selection(2);
    ui.add_entity_to_selection(1);

    assert_eq!(ui.selected_entity_ids(), &[1, 2]);
    assert_eq!(ui.selected_entity_id(), Some(1));
}

#[test]
fn marquee_selection_lifecycle_tracks_start_update_and_finish() {
    let mut ui = EditorUI::new();
    assert!(!ui.is_marquee_selection_active());

    ui.start_marquee_selection(egui::pos2(10.0, 20.0));
    ui.update_marquee_selection(egui::pos2(30.0, 40.0));

    let marquee = ui
        .finish_marquee_selection()
        .expect("marquee should be active");
    assert_eq!(marquee.start_screen, egui::pos2(10.0, 20.0));
    assert_eq!(marquee.current_screen, egui::pos2(30.0, 40.0));
    assert!(!ui.is_marquee_selection_active());
}

#[test]
fn execute_command_undo_and_redo_round_trip_entity_creation() {
    let mut ui = EditorUI::new();
    let command = EditorCommand::add_entity("Main Scene", sample_entity(11, IVec2::new(8, 9)));

    assert!(ui.execute_command(command));
    assert!(ui.can_undo());
    assert_eq!(
        ui.scenes
            .iter()
            .find(|scene| scene.name == "Main Scene")
            .expect("main scene should exist")
            .entities
            .len(),
        1
    );

    assert!(ui.undo());
    assert!(ui.can_redo());
    assert!(ui
        .scenes
        .iter()
        .find(|scene| scene.name == "Main Scene")
        .expect("main scene should exist")
        .entities
        .is_empty());

    assert!(ui.redo());
    assert_eq!(
        ui.scenes
            .iter()
            .find(|scene| scene.name == "Main Scene")
            .expect("main scene should exist")
            .entities
            .len(),
        1
    );
}

#[test]
fn load_scenes_from_project_clears_undo_redo_history() {
    let mut ui = EditorUI::new();
    assert!(ui.execute_command(EditorCommand::add_entity(
        "Main Scene",
        sample_entity(1, IVec2::new(0, 0))
    )));
    assert!(ui.can_undo());

    ui.load_scenes_from_project(vec![toki_core::Scene::new("Imported".to_string())]);

    assert!(!ui.can_undo());
    assert!(!ui.can_redo());
}

#[test]
fn load_scenes_from_project_replaces_missing_active_scene_with_first_loaded_scene() {
    let mut ui = EditorUI::new();
    ui.active_scene = Some("Missing".to_string());

    ui.load_scenes_from_project(vec![toki_core::Scene::new("main".to_string())]);

    assert_eq!(ui.active_scene.as_deref(), Some("main"));
}

#[test]
fn sync_map_editor_selection_picks_sorted_first_map_and_requests_load() {
    let mut ui = EditorUI::new();
    let maps = vec![
        "zeta".to_string(),
        "alpha".to_string(),
        "middle".to_string(),
    ];

    ui.sync_map_editor_selection(&maps);

    assert_eq!(ui.map.active_map.as_deref(), Some("alpha"));
    assert_eq!(ui.map.map_load_requested.as_deref(), Some("alpha"));
}

#[test]
fn sync_map_editor_selection_preserves_existing_valid_choice() {
    let mut ui = EditorUI::new();
    ui.map.active_map = Some("middle".to_string());
    let maps = vec![
        "zeta".to_string(),
        "alpha".to_string(),
        "middle".to_string(),
    ];

    ui.sync_map_editor_selection(&maps);

    assert_eq!(ui.map.active_map.as_deref(), Some("middle"));
    assert!(ui.map.map_load_requested.is_none());
}

#[test]
fn sync_map_editor_selection_preserves_unsaved_draft() {
    let mut ui = EditorUI::new();
    ui.set_map_editor_draft(MapEditorDraft {
        name: "draft_map".to_string(),
        tilemap: toki_core::assets::tilemap::TileMap {
            size: glam::UVec2::new(2, 2),
            tile_size: glam::UVec2::new(8, 8),
            atlas: std::path::PathBuf::from("terrain.json"),
            tiles: vec!["grass".to_string(); 4],
            objects: vec![],
        },
    });

    ui.sync_map_editor_selection(&["alpha".to_string(), "zeta".to_string()]);

    assert_eq!(ui.map.active_map.as_deref(), Some("draft_map"));
    assert!(ui.map.map_load_requested.is_none());
    assert!(ui.has_unsaved_map_editor_draft());
}

#[test]
fn finalize_saved_map_editor_draft_requests_reload_from_disk() {
    let mut ui = EditorUI::new();
    ui.set_map_editor_draft(MapEditorDraft {
        name: "draft_map".to_string(),
        tilemap: toki_core::assets::tilemap::TileMap {
            size: glam::UVec2::new(2, 2),
            tile_size: glam::UVec2::new(8, 8),
            atlas: std::path::PathBuf::from("terrain.json"),
            tiles: vec!["grass".to_string(); 4],
            objects: vec![],
        },
    });

    ui.finalize_saved_map_editor_draft("draft_map".to_string());

    assert!(!ui.has_unsaved_map_editor_draft());
    assert!(!ui.has_unsaved_map_editor_changes());
    assert_eq!(ui.map.active_map.as_deref(), Some("draft_map"));
    assert_eq!(
        ui.map.map_load_requested.as_deref(),
        Some("draft_map")
    );
}

#[test]
fn sync_map_editor_selection_preserves_dirty_loaded_map() {
    let mut ui = EditorUI::new();
    ui.map.active_map = Some("middle".to_string());
    ui.mark_map_editor_dirty();

    ui.sync_map_editor_selection(&["alpha".to_string(), "middle".to_string()]);

    assert_eq!(ui.map.active_map.as_deref(), Some("middle"));
    assert!(ui.map.map_load_requested.is_none());
}

fn sample_project_with_menu_screens(screen_ids: &[&str]) -> Project {
    let temp_dir = tempdir().expect("temp dir should exist");
    let mut project = Project::new("Menu Demo".to_string(), temp_dir.path().join("MenuDemo"));
    project.metadata.runtime.menu.screens = screen_ids
        .iter()
        .map(|screen_id| MenuScreenDefinition {
            id: (*screen_id).to_string(),
            title: format!("{screen_id} title"),
            title_border_style_override: None,
            items: vec![MenuItemDefinition::Button {
                text: "Resume".to_string(),
                border_style_override: None,
                action: UiAction::CloseUi,
            }],
        })
        .collect();
    project
}

fn sample_project_with_menu_dialogs(dialog_ids: &[&str]) -> Project {
    let temp_dir = tempdir().expect("temp dir should exist");
    let mut project = Project::new("Menu Demo".to_string(), temp_dir.path().join("MenuDemo"));
    project.metadata.runtime.menu.screens.clear();
    project.metadata.runtime.menu.dialogs = dialog_ids
        .iter()
        .map(|dialog_id| toki_core::menu::MenuDialogDefinition {
            id: (*dialog_id).to_string(),
            title: format!("{dialog_id} title"),
            body: "Are you sure?".to_string(),
            confirm_text: "Confirm".to_string(),
            cancel_text: "Cancel".to_string(),
            confirm_action: UiAction::CloseSurface,
            cancel_action: UiAction::CloseSurface,
        })
        .collect();
    project
}

#[test]
fn sync_menu_editor_selection_picks_first_screen_when_none_selected() {
    let mut ui = EditorUI::new();
    let project = sample_project_with_menu_screens(&["pause_menu", "inventory_menu"]);

    ui.sync_menu_editor_selection(Some(&project));

    assert_eq!(
        ui.selection,
        Some(Selection::MenuScreen("pause_menu".to_string()))
    );
}

#[test]
fn sync_menu_editor_selection_picks_first_dialog_when_only_dialogs_exist() {
    let mut ui = EditorUI::new();
    let project = sample_project_with_menu_dialogs(&["exit_confirm", "discard_confirm"]);

    ui.sync_menu_editor_selection(Some(&project));

    assert_eq!(
        ui.selection,
        Some(Selection::MenuDialog("exit_confirm".to_string()))
    );
}

#[test]
fn sync_menu_editor_selection_preserves_valid_entry_selection() {
    let mut ui = EditorUI::new();
    let project = sample_project_with_menu_screens(&["pause_menu"]);
    ui.select_menu_entry("pause_menu", 0);

    ui.sync_menu_editor_selection(Some(&project));

    assert_eq!(
        ui.selection,
        Some(Selection::MenuEntry {
            screen_id: "pause_menu".to_string(),
            item_index: 0,
        })
    );
}

#[test]
fn sync_menu_editor_selection_downgrades_missing_entry_to_screen_selection() {
    let mut ui = EditorUI::new();
    let project = sample_project_with_menu_screens(&["pause_menu"]);
    ui.select_menu_entry("pause_menu", 3);

    ui.sync_menu_editor_selection(Some(&project));

    assert_eq!(
        ui.selection,
        Some(Selection::MenuScreen("pause_menu".to_string()))
    );
}

#[test]
fn sync_menu_editor_selection_replaces_missing_screen_selection() {
    let mut ui = EditorUI::new();
    let project = sample_project_with_menu_screens(&["pause_menu", "inventory_menu"]);
    ui.select_menu_screen("missing_menu");

    ui.sync_menu_editor_selection(Some(&project));

    assert_eq!(
        ui.selection,
        Some(Selection::MenuScreen("pause_menu".to_string()))
    );
}

#[test]
fn sync_map_editor_brush_selection_picks_first_sorted_tile() {
    let mut ui = EditorUI::new();

    ui.sync_map_editor_brush_selection(&[
        "water".to_string(),
        "grass".to_string(),
        "bush".to_string(),
    ]);

    assert_eq!(ui.map.selected_tile.as_deref(), Some("bush"));
}

#[test]
fn map_editor_defaults_to_drag_tool() {
    let ui = EditorUI::new();
    assert_eq!(ui.map.tool, super::MapEditorTool::Drag);
    assert_eq!(ui.map.brush_size_tiles, 1);
    assert!(ui.map.selected_tile_info.is_none());
}

#[test]
fn sync_map_editor_object_sheet_selection_picks_first_sorted_sheet() {
    let mut ui = EditorUI::new();

    ui.sync_map_editor_object_sheet_selection(&[
        "trees".to_string(),
        "fauna".to_string(),
        "props".to_string(),
    ]);

    assert_eq!(
        ui.map.selected_object_sheet.as_deref(),
        Some("fauna")
    );
}

#[test]
fn sync_map_editor_object_selection_picks_first_sorted_object() {
    let mut ui = EditorUI::new();

    ui.sync_map_editor_object_selection(&[
        "tree_large".to_string(),
        "bush".to_string(),
        "flower".to_string(),
    ]);

    assert_eq!(ui.map.selected_object_name.as_deref(), Some("bush"));
}

#[test]
fn pick_map_editor_tile_sets_selected_tile_and_switches_back_to_brush() {
    let mut ui = EditorUI::new();
    ui.map.tool = super::MapEditorTool::PickTile;

    ui.pick_map_editor_tile("water".to_string());

    assert_eq!(ui.map.selected_tile.as_deref(), Some("water"));
    assert_eq!(ui.map.tool, super::MapEditorTool::Brush);
}

#[test]
fn select_map_editor_object_clears_tile_selection_and_syncs_changes() {
    let mut ui = EditorUI::new();
    ui.map.selected_tile_info = Some(super::MapEditorTileInfo {
        tile_x: 1,
        tile_y: 2,
        tile_name: "grass".to_string(),
        solid: false,
        trigger: false,
    });
    let object = toki_core::assets::tilemap::MapObjectInstance {
        sheet: std::path::PathBuf::from("fauna.json"),
        object_name: "bush".to_string(),
        position: glam::UVec2::new(16, 32),
        size_px: glam::UVec2::new(16, 16),
        visible: true,
        solid: false,
    };

    ui.select_map_editor_object(0, &object);
    assert!(ui.map.selected_tile_info.is_none());
    assert_eq!(
        ui.map.selected_object_info
            .as_ref()
            .map(|selected| selected.object_name.as_str()),
        Some("bush")
    );

    let tilemap = toki_core::assets::tilemap::TileMap {
        size: glam::UVec2::new(2, 2),
        tile_size: glam::UVec2::new(16, 16),
        atlas: std::path::PathBuf::from("terrain.json"),
        tiles: vec!["grass".to_string(); 4],
        objects: vec![toki_core::assets::tilemap::MapObjectInstance {
            solid: true,
            position: glam::UVec2::new(32, 32),
            ..object.clone()
        }],
    };

    ui.sync_selected_map_editor_object_from_tilemap(&tilemap);
    let selected = ui
        .map.selected_object_info
        .as_ref()
        .expect("selected object should remain");
    assert_eq!(selected.position, glam::UVec2::new(32, 32));
    assert!(selected.solid);
}

#[test]
fn queue_map_editor_object_property_edit_updates_selected_object_info() {
    let mut ui = EditorUI::new();
    let object = toki_core::assets::tilemap::MapObjectInstance {
        sheet: std::path::PathBuf::from("fauna.json"),
        object_name: "bush".to_string(),
        position: glam::UVec2::new(16, 16),
        size_px: glam::UVec2::new(16, 16),
        visible: true,
        solid: false,
    };
    ui.select_map_editor_object(2, &object);

    ui.queue_map_editor_object_property_edit(2, false, true);

    let selected = ui
        .map.selected_object_info
        .as_ref()
        .expect("selected object should exist");
    assert!(!selected.visible);
    assert!(selected.solid);
    let request = ui
        .take_map_editor_object_property_edit_request()
        .expect("edit request should exist");
    assert_eq!(request.object_index, 2);
    assert!(!request.visible);
    assert!(request.solid);
}

#[test]
fn map_editor_undo_and_redo_round_trip_a_draft_edit() {
    let mut ui = EditorUI::new();
    ui.center_panel_tab = super::CenterPanelTab::MapEditor;
    ui.set_map_editor_draft(MapEditorDraft {
        name: "draft_map".to_string(),
        tilemap: toki_core::assets::tilemap::TileMap {
            size: glam::UVec2::new(2, 2),
            tile_size: glam::UVec2::new(8, 8),
            atlas: std::path::PathBuf::from("terrain.json"),
            tiles: vec!["grass".to_string(); 4],
            objects: vec![],
        },
    });

    let before = ui
        .map.draft
        .as_ref()
        .expect("draft should exist")
        .tilemap
        .clone();
    let mut after = before.clone();
    after.tiles[0] = "water".to_string();

    ui.begin_map_editor_edit(&before);
    assert!(ui.finish_map_editor_edit(&after));
    assert!(ui.can_undo());

    assert!(ui.undo());
    let undone = ui
        .take_pending_map_editor_tilemap_sync()
        .expect("undo should queue a tilemap sync");
    assert_eq!(undone.tiles[0], "grass");

    assert!(ui.redo());
    let redone = ui
        .take_pending_map_editor_tilemap_sync()
        .expect("redo should queue a tilemap sync");
    assert_eq!(redone.tiles[0], "water");
}

#[test]
fn map_editor_can_undo_prefers_map_history_when_map_editor_tab_is_active() {
    let mut ui = EditorUI::new();
    assert!(ui.execute_command(EditorCommand::add_entity(
        "Main Scene",
        sample_entity(1, IVec2::new(0, 0))
    )));
    ui.center_panel_tab = super::CenterPanelTab::MapEditor;
    assert!(!ui.can_undo());

    ui.set_map_editor_draft(MapEditorDraft {
        name: "draft_map".to_string(),
        tilemap: toki_core::assets::tilemap::TileMap {
            size: glam::UVec2::new(1, 1),
            tile_size: glam::UVec2::new(8, 8),
            atlas: std::path::PathBuf::from("terrain.json"),
            tiles: vec!["grass".to_string()],
            objects: vec![],
        },
    });
    let before = ui.map.draft.as_ref().unwrap().tilemap.clone();
    let mut after = before.clone();
    after.tiles[0] = "water".to_string();
    ui.begin_map_editor_edit(&before);
    assert!(ui.finish_map_editor_edit(&after));

    assert!(ui.can_undo());
    assert!(ui.undo());
    assert!(ui.take_pending_map_editor_tilemap_sync().is_some());
}

// =============================================================================
// UIVisibilityState regression tests
// =============================================================================

#[test]
fn editor_ui_default_visibility_flags() {
    let ui = EditorUI::new();

    // Default visibility settings
    assert!(ui.visibility.show_hierarchy, "hierarchy panel should be visible by default");
    assert!(ui.visibility.show_inspector, "inspector panel should be visible by default");
    assert!(ui.visibility.show_maps, "maps panel should be visible by default");
    assert!(ui.visibility.show_console, "console panel should be visible by default");

    // Non-default visibility settings
    assert!(!ui.visibility.show_runtime_entities, "runtime entities should be hidden by default");
    assert!(!ui.visibility.should_exit, "should_exit should be false by default");
    assert!(!ui.visibility.create_test_entities, "create_test_entities should be false by default");
}

#[test]
fn apply_config_sets_visibility_flags() {
    use crate::config::EditorConfig;

    let mut ui = EditorUI::new();

    // Create config with inverted visibility
    let mut config = EditorConfig::default();
    config.editor_settings.panels.hierarchy_visible = false;
    config.editor_settings.panels.inspector_visible = false;
    config.editor_settings.panels.console_visible = false;

    ui.apply_config(&config);

    assert!(!ui.visibility.show_hierarchy, "hierarchy visibility should match config");
    assert!(!ui.visibility.show_inspector, "inspector visibility should match config");
    assert!(!ui.visibility.show_console, "console visibility should match config");
}

#[test]
fn apply_config_preserves_non_config_visibility_flags() {
    use crate::config::EditorConfig;

    let mut ui = EditorUI::new();
    ui.visibility.show_runtime_entities = true;
    ui.visibility.should_exit = true;
    ui.visibility.create_test_entities = true;

    let config = EditorConfig::default();
    ui.apply_config(&config);

    // These flags are not controlled by config and should be preserved
    assert!(ui.visibility.show_runtime_entities, "show_runtime_entities should be preserved");
    assert!(ui.visibility.should_exit, "should_exit should be preserved");
    assert!(ui.visibility.create_test_entities, "create_test_entities should be preserved");
}

// =============================================================================
// PlacementState regression tests
// =============================================================================

#[test]
fn placement_mode_default_is_inactive() {
    let ui = EditorUI::new();

    assert!(!ui.placement.mode);
    assert!(!ui.is_in_placement_mode());
    assert!(ui.placement.entity_definition.is_none());
    assert!(ui.placement.preview_position.is_none());
    assert!(ui.placement.preview_cached_frame.is_none());
    assert!(ui.placement.preview_valid.is_none());
    assert!(ui.placement.entity_move_drag.is_none());
    assert!(ui.placement.marquee_selection.is_none());
}

#[test]
fn enter_placement_mode_sets_mode_and_definition() {
    let mut ui = EditorUI::new();

    ui.enter_placement_mode("player".to_string());

    assert!(ui.placement.mode);
    assert!(ui.is_in_placement_mode());
    assert_eq!(ui.placement.entity_definition.as_deref(), Some("player"));
}

#[test]
fn exit_placement_mode_clears_all_placement_state() {
    let mut ui = EditorUI::new();

    // Setup placement mode with various state
    ui.enter_placement_mode("player".to_string());
    ui.placement.preview_position = Some(glam::Vec2::new(10.0, 20.0));
    ui.placement.preview_valid = Some(true);

    ui.exit_placement_mode();

    assert!(!ui.placement.mode);
    assert!(!ui.is_in_placement_mode());
    assert!(ui.placement.entity_definition.is_none());
    assert!(ui.placement.preview_position.is_none());
    assert!(ui.placement.preview_cached_frame.is_none());
    assert!(ui.placement.preview_valid.is_none());
    assert!(ui.placement.entity_move_drag.is_none());
    assert!(ui.placement.marquee_selection.is_none());
}

#[test]
fn entity_move_drag_lifecycle() {
    use super::EntityMoveDragState;

    let mut ui = EditorUI::new();
    assert!(!ui.is_entity_move_drag_active());

    let drag_state = EntityMoveDragState {
        scene_name: "Main Scene".to_string(),
        entity: sample_entity(1, IVec2::new(10, 20)),
        dragged_entities: vec![],
        grab_offset: glam::Vec2::new(5.0, 5.0),
    };
    ui.begin_entity_move_drag(drag_state);

    assert!(ui.is_entity_move_drag_active());
    assert!(ui.placement.entity_move_drag.is_some());

    // exit_placement_mode also clears drag
    ui.exit_placement_mode();
    assert!(!ui.is_entity_move_drag_active());
}

// =============================================================================
// ProjectEditorState regression tests
// =============================================================================

#[test]
fn project_editor_state_defaults() {
    let ui = EditorUI::new();

    // All project request flags should be false by default
    assert!(!ui.project.new_project_requested);
    assert!(!ui.project.new_top_down_project_requested);
    assert!(!ui.project.show_new_project_dialog);
    assert!(!ui.project.open_project_requested);
    assert!(!ui.project.browse_for_project_requested);
    assert!(!ui.project.save_project_requested);
    assert!(!ui.project.export_project_requested);
    assert!(!ui.project.play_scene_requested);
    assert!(!ui.project.init_config_requested);
    assert!(!ui.project.background_task_running);
    assert!(!ui.project.cancel_background_task_requested);
    assert!(!ui.project.validate_assets_requested);

    // Other project state defaults
    assert_eq!(ui.project.new_project_name, "NewProject");
    assert!(ui.project.new_project_parent_directory.is_none());
    assert!(ui.project.new_project_submit_requested.is_none());
    assert!(ui.project.background_task_status.is_none());
    assert!(ui.project.window_title.is_some());
}

#[test]
fn begin_new_project_dialog_sets_up_dialog_state() {
    use crate::project::ProjectTemplateKind;
    use std::path::PathBuf;

    let mut ui = EditorUI::new();

    ui.begin_new_project_dialog(
        ProjectTemplateKind::TopDownStarter,
        Some(PathBuf::from("/home/user/projects")),
        "MyGame".to_string(),
    );

    assert!(ui.project.show_new_project_dialog);
    assert_eq!(ui.project.new_project_template, ProjectTemplateKind::TopDownStarter);
    assert_eq!(
        ui.project.new_project_parent_directory.as_deref(),
        Some(std::path::Path::new("/home/user/projects"))
    );
    assert_eq!(ui.project.new_project_name, "MyGame");
}

#[test]
fn submit_new_project_request_creates_request_and_closes_dialog() {
    use crate::project::ProjectTemplateKind;
    use std::path::PathBuf;

    let mut ui = EditorUI::new();
    ui.project.show_new_project_dialog = true;
    ui.project.new_project_template = ProjectTemplateKind::Empty;
    ui.project.new_project_parent_directory = Some(PathBuf::from("/home/user"));
    ui.project.new_project_name = "TestProject".to_string();

    ui.submit_new_project_request();

    assert!(!ui.project.show_new_project_dialog, "dialog should close after submit");
    let request = ui.project.new_project_submit_requested.as_ref().expect("request should exist");
    assert_eq!(request.name, "TestProject");
    assert_eq!(request.parent_path, PathBuf::from("/home/user"));
    assert_eq!(request.template, ProjectTemplateKind::Empty);
}

#[test]
fn submit_new_project_request_requires_parent_directory() {
    let mut ui = EditorUI::new();
    ui.project.new_project_name = "TestProject".to_string();
    ui.project.new_project_parent_directory = None;

    ui.submit_new_project_request();

    assert!(ui.project.new_project_submit_requested.is_none(), "should not create request without parent");
}

#[test]
fn submit_new_project_request_requires_non_empty_name() {
    use std::path::PathBuf;

    let mut ui = EditorUI::new();
    ui.project.new_project_parent_directory = Some(PathBuf::from("/home/user"));
    ui.project.new_project_name = "   ".to_string(); // whitespace only

    ui.submit_new_project_request();

    assert!(ui.project.new_project_submit_requested.is_none(), "should not create request with empty name");
}

#[test]
fn map_load_request_stores_scene_and_map_name() {
    use super::MapLoadRequest;

    let request = MapLoadRequest {
        scene_name: "overworld".to_string(),
        map_name: "forest".to_string(),
    };

    assert_eq!(request.scene_name, "overworld");
    assert_eq!(request.map_name, "forest");
}

#[test]
fn map_editor_load_requested_uses_struct_instead_of_tuple() {
    use super::MapLoadRequest;

    let mut ui = EditorUI::new();
    assert!(ui.map.load_requested.is_none());

    ui.map.load_requested = Some(MapLoadRequest {
        scene_name: "main_scene".to_string(),
        map_name: "town_map".to_string(),
    });

    let request = ui.map.load_requested.as_ref().unwrap();
    assert_eq!(request.scene_name, "main_scene");
    assert_eq!(request.map_name, "town_map");
}
