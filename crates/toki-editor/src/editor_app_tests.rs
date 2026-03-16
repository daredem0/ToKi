
use super::EditorApp;
use crate::project::ProjectAssets;
use crate::ui::editor_ui::{EntityMoveDragState, MapEditorDraft};
use glam::{IVec2, UVec2, Vec2};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
use toki_core::assets::tilemap::TileMap;
use toki_core::collision::CollisionBox;
use toki_core::entity::{Entity, EntityAttributes, EntityKind};
use winit::keyboard::ModifiersState;

#[test]
fn resolve_scene_map_to_load_prefers_previously_loaded_map() {
    let scene = toki_core::Scene::with_maps(
        "Test Scene".to_string(),
        vec!["map_a".to_string(), "map_b".to_string()],
    );

    let chosen = EditorApp::resolve_scene_map_to_load(&scene, Some("map_b"));
    assert_eq!(chosen.as_deref(), Some("map_b"));
}

#[test]
fn resolve_scene_map_to_load_falls_back_to_first_map_when_preferred_missing() {
    let scene = toki_core::Scene::with_maps(
        "Test Scene".to_string(),
        vec!["map_a".to_string(), "map_b".to_string()],
    );

    let chosen = EditorApp::resolve_scene_map_to_load(&scene, Some("map_missing"));
    assert_eq!(chosen.as_deref(), Some("map_a"));
}

#[test]
fn resolve_scene_map_to_load_returns_none_when_scene_has_no_maps() {
    let scene = toki_core::Scene::new("Empty Scene".to_string());
    let chosen = EditorApp::resolve_scene_map_to_load(&scene, Some("any_map"));
    assert_eq!(chosen, None);
}

#[test]
fn parse_legacy_graph_layout_key_splits_project_scene_and_node() {
    let key = "/tmp/project::Main Scene::rule_1:action:0";
    let parsed = EditorApp::parse_legacy_graph_layout_key(key)
        .expect("legacy graph layout key should parse");
    assert_eq!(parsed.0, "/tmp/project");
    assert_eq!(parsed.1, "Main Scene");
    assert_eq!(parsed.2, "rule_1:action:0");
}

#[test]
fn editor_shortcut_action_maps_ctrl_z_to_undo() {
    let action = EditorApp::editor_shortcut_action(
        &winit::keyboard::Key::Character("z".into()),
        ModifiersState::CONTROL,
    );
    assert_eq!(action, Some(super::EditorShortcutAction::Undo));
}

#[test]
fn editor_shortcut_action_maps_ctrl_y_and_ctrl_shift_z_to_redo() {
    let redo_y = EditorApp::editor_shortcut_action(
        &winit::keyboard::Key::Character("y".into()),
        ModifiersState::CONTROL,
    );
    assert_eq!(redo_y, Some(super::EditorShortcutAction::Redo));

    let redo_shift_z = EditorApp::editor_shortcut_action(
        &winit::keyboard::Key::Character("z".into()),
        ModifiersState::CONTROL | ModifiersState::SHIFT,
    );
    assert_eq!(redo_shift_z, Some(super::EditorShortcutAction::Redo));
}

#[test]
fn editor_shortcut_action_ignores_non_ctrl_sequences() {
    let no_ctrl = EditorApp::editor_shortcut_action(
        &winit::keyboard::Key::Character("z".into()),
        ModifiersState::default(),
    );
    assert_eq!(no_ctrl, None);

    let other_key = EditorApp::editor_shortcut_action(
        &winit::keyboard::Key::Character("x".into()),
        ModifiersState::CONTROL,
    );
    assert_eq!(other_key, None);
}

#[test]
fn build_runtime_launch_args_includes_optional_map_and_splash_duration() {
    let args = EditorApp::build_runtime_launch_args(
        std::path::Path::new("/tmp/project"),
        "Main Scene",
        Some("main_map"),
        Some(2600),
    );

    assert_eq!(
        args,
        vec![
            "--project",
            "/tmp/project",
            "--scene",
            "Main Scene",
            "--map",
            "main_map",
            "--splash-duration-ms",
            "2600",
        ]
    );
}

#[test]
fn build_runtime_launch_args_omits_absent_optional_values() {
    let args = EditorApp::build_runtime_launch_args(
        std::path::Path::new("/tmp/project"),
        "Main Scene",
        None,
        None,
    );

    assert_eq!(
        args,
        vec!["--project", "/tmp/project", "--scene", "Main Scene",]
    );
}

#[test]
fn build_map_editor_draft_prefers_terrain_atlas_and_fills_tiles() {
    let temp_dir = tempfile::tempdir().expect("temp dir should exist");
    let project_path = temp_dir.path().to_path_buf();
    fs::create_dir_all(project_path.join("assets").join("sprites"))
        .expect("sprites dir should exist");

    fs::write(
        project_path
            .join("assets")
            .join("sprites")
            .join("terrain.json"),
        r#"{
                "image": "terrain.png",
                "tile_size": [8, 8],
                "tiles": {
                    "grass": { "position": [0, 0] },
                    "water": { "position": [1, 0] }
                }
            }"#,
    )
    .expect("terrain atlas should be written");
    fs::write(
        project_path
            .join("assets")
            .join("sprites")
            .join("other.json"),
        r#"{
                "image": "other.png",
                "tile_size": [16, 16],
                "tiles": {
                    "stone": { "position": [0, 0] }
                }
            }"#,
    )
    .expect("other atlas should be written");

    let mut project_assets = ProjectAssets::new(project_path);
    project_assets.scan_assets().expect("assets should scan");

    let draft = EditorApp::build_map_editor_draft(&project_assets, "new_map", 5, 4)
        .expect("draft should build");

    assert_eq!(draft.name, "new_map");
    assert_eq!(draft.tilemap.size, UVec2::new(5, 4));
    assert_eq!(draft.tilemap.tile_size, UVec2::new(8, 8));
    assert_eq!(draft.tilemap.atlas, PathBuf::from("terrain.json"));
    assert_eq!(draft.tilemap.tiles.len(), 20);
    assert!(draft.tilemap.tiles.iter().all(|tile| tile == "grass"));
}

#[test]
fn tilemap_to_save_for_map_editor_draft_prefers_live_viewport_tilemap() {
    let draft = MapEditorDraft {
        name: "draft_map".to_string(),
        tilemap: TileMap {
            size: UVec2::new(2, 2),
            tile_size: UVec2::new(8, 8),
            atlas: PathBuf::from("terrain.json"),
            tiles: vec!["grass".to_string(); 4],
            objects: vec![],
        },
    };
    let live_tilemap = TileMap {
        size: UVec2::new(2, 2),
        tile_size: UVec2::new(8, 8),
        atlas: PathBuf::from("terrain.json"),
        tiles: vec![
            "water".to_string(),
            "grass".to_string(),
            "grass".to_string(),
            "grass".to_string(),
        ],
        objects: vec![],
    };

    let saved = EditorApp::tilemap_to_save_for_map_editor_draft(&draft, Some(&live_tilemap));

    assert_eq!(saved.tiles[0], "water");
    assert_eq!(saved.tiles, live_tilemap.tiles);
}

#[test]
fn tilemap_to_save_for_map_editor_draft_falls_back_to_original_draft_when_viewport_missing() {
    let draft = MapEditorDraft {
        name: "draft_map".to_string(),
        tilemap: TileMap {
            size: UVec2::new(2, 2),
            tile_size: UVec2::new(8, 8),
            atlas: PathBuf::from("terrain.json"),
            tiles: vec!["grass".to_string(); 4],
            objects: vec![],
        },
    };

    let saved = EditorApp::tilemap_to_save_for_map_editor_draft(&draft, None);

    assert_eq!(saved, draft.tilemap);
}

fn collision_assets_with_center_solid_tile() -> (TileMap, AtlasMeta) {
    let mut tiles = HashMap::new();
    tiles.insert(
        "solid".to_string(),
        TileInfo {
            position: UVec2::new(0, 0),
            properties: TileProperties {
                solid: true,
                trigger: false,
            },
        },
    );
    tiles.insert(
        "floor".to_string(),
        TileInfo {
            position: UVec2::new(1, 0),
            properties: TileProperties {
                solid: false,
                trigger: false,
            },
        },
    );

    let atlas = AtlasMeta {
        image: PathBuf::from("test.png"),
        tile_size: UVec2::new(16, 16),
        tiles,
    };

    let tilemap = TileMap {
        size: UVec2::new(3, 3),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test_atlas.json"),
        tiles: vec![
            "floor".to_string(),
            "floor".to_string(),
            "floor".to_string(),
            "floor".to_string(),
            "solid".to_string(),
            "floor".to_string(),
            "floor".to_string(),
            "floor".to_string(),
            "floor".to_string(),
        ],
        objects: vec![],
    };

    (tilemap, atlas)
}

fn solid_entity(id: u32, position: IVec2) -> Entity {
    Entity {
        id,
        position,
        size: UVec2::new(16, 16),
        entity_kind: EntityKind::Npc,
        category: "creature".to_string(),
        definition_name: Some("test".to_string()),
        control_role: toki_core::entity::ControlRole::None,
        audio: toki_core::entity::EntityAudioSettings::default(),
        attributes: EntityAttributes::default(),
        collision_box: Some(CollisionBox::solid_box(UVec2::new(16, 16))),
    }
}

#[test]
fn build_drag_preview_sprites_computes_validity_per_entity() {
    let (tilemap, atlas) = collision_assets_with_center_solid_tile();
    let first = solid_entity(1, IVec2::new(0, 0));
    let second = solid_entity(2, IVec2::new(0, 16));
    let drag_state = EntityMoveDragState {
        scene_name: "Main Scene".to_string(),
        entity: first.clone(),
        dragged_entities: vec![first.clone(), second.clone()],
        grab_offset: Vec2::ZERO,
    };

    let previews = EditorApp::build_drag_preview_sprites(
        &drag_state,
        Vec2::new(16.0, 0.0),
        Some(&tilemap),
        Some(&atlas),
    );

    let first_preview = previews
        .iter()
        .find(|preview| preview.entity_id == first.id)
        .expect("first preview should exist");
    let second_preview = previews
        .iter()
        .find(|preview| preview.entity_id == second.id)
        .expect("second preview should exist");

    assert_eq!(first_preview.world_position, IVec2::new(16, 0));
    assert_eq!(second_preview.world_position, IVec2::new(16, 16));
    assert!(first_preview.is_valid);
    assert!(!second_preview.is_valid);
}
