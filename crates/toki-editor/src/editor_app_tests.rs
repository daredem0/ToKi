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
use toki_core::entity::{
    AiBehavior, AnimationsDef, AttributesDef, AudioDef, CollisionDef, Entity, EntityAttributes,
    EntityDefinition, EntityKind, MovementProfile, MovementSoundTrigger, PickupDef, RenderingDef,
    StaticObjectRenderDef,
};
use toki_core::scene::{SceneAnchor, SceneAnchorKind};
use toki_core::Scene;
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
fn suggested_new_project_parent_path_uses_current_project_parent_directory() {
    let current_project_path = std::path::Path::new("/tmp/projects/MyGame");

    let suggested = EditorApp::suggested_new_project_parent_path(current_project_path);

    assert_eq!(suggested, PathBuf::from("/tmp/projects"));
}

#[test]
fn split_new_project_destination_accepts_directory_like_destination() {
    let destination = std::path::Path::new("/tmp/projects/NewProject");

    let split = EditorApp::split_new_project_destination(destination)
        .expect("directory-like project destination should split");

    assert_eq!(split.0, PathBuf::from("/tmp/projects"));
    assert_eq!(split.1, "NewProject");
}

#[test]
fn split_new_project_destination_accepts_project_toml_destination() {
    let destination = std::path::Path::new("/tmp/projects/NewProject/project.toml");

    let split = EditorApp::split_new_project_destination(destination)
        .expect("project.toml destination should split");

    assert_eq!(split.0, PathBuf::from("/tmp/projects"));
    assert_eq!(split.1, "NewProject");
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

    let draft = EditorApp::build_map_editor_draft(&project_assets, "new_map", 5, 4, 16, 16)
        .expect("draft should build");

    assert_eq!(draft.name, "new_map");
    assert_eq!(draft.tilemap.size, UVec2::new(5, 4));
    assert_eq!(draft.tilemap.tile_size, UVec2::new(16, 16));
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
        movement_accumulator: glam::Vec2::ZERO,
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

#[test]
fn load_preview_sprite_frame_static_supports_object_sheet_backed_entities() {
    let temp_dir = tempfile::tempdir().expect("temp dir should exist");
    let project_path = temp_dir.path().to_path_buf();
    fs::create_dir_all(project_path.join("entities")).expect("entities dir should exist");
    fs::create_dir_all(project_path.join("assets/sprites")).expect("sprites dir should exist");
    fs::write(
        project_path.join("assets/sprites/items.json"),
        r#"{
            "sheet_type": "objects",
            "image": "items.png",
            "tile_size": [16, 16],
            "objects": {
                "coin": {
                    "position": [0, 0],
                    "size_tiles": [1, 1]
                }
            }
        }"#,
    )
    .expect("object sheet should be written");
    let entity_def = EntityDefinition {
        name: "coin_pickup".to_string(),
        display_name: "Coin Pickup".to_string(),
        description: "Collectible coin".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            static_object: Some(StaticObjectRenderDef {
                sheet: "items".to_string(),
                object_name: "coin".to_string(),
            }),
        },
        attributes: AttributesDef {
            health: None,
            stats: HashMap::new(),
            speed: 0.0,
            solid: false,
            active: true,
            can_move: false,
            ai_behavior: AiBehavior::None,
            movement_profile: MovementProfile::None,
            primary_projectile: None,
            pickup: Some(PickupDef {
                item_id: "coin".to_string(),
                count: 1,
            }),
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: true,
        },
        audio: AudioDef {
            footstep_trigger_distance: 16.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "step".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "".to_string(),
            clips: vec![],
            default_state: "".to_string(),
        },
        category: "item".to_string(),
        tags: vec!["pickup".to_string()],
    };
    fs::write(
        project_path.join("entities/coin_pickup.json"),
        serde_json::to_string_pretty(&entity_def).expect("entity json should serialize"),
    )
    .expect("entity definition should be written");

    let mut project_assets = ProjectAssets::new(project_path.clone());
    project_assets
        .scan_assets()
        .expect("project assets should scan");

    let preview =
        EditorApp::load_preview_sprite_frame_static("coin_pickup", &project_path, &project_assets)
            .expect("static object-backed pickup should produce a preview visual");

    assert_eq!(preview.size, UVec2::new(16, 16));
    assert!(preview.texture_path.is_some());
}

#[test]
fn build_scene_anchor_overlay_lines_use_grid_sized_crossmark() {
    let mut config = crate::config::EditorConfig::default();
    config.editor_settings.grid.grid_size = [24, 32];
    config.editor_settings.grid.snap_to_grid = true;

    let mut ui_state = crate::ui::EditorUI::new();
    let mut scene = Scene::new("Main Scene".to_string());
    scene.anchors.push(SceneAnchor {
        id: "spawn_point_1".to_string(),
        kind: SceneAnchorKind::SpawnPoint,
        position: IVec2::new(48, 64),
        facing: None,
    });
    ui_state.scenes = vec![scene];
    ui_state.active_scene = Some("Main Scene".to_string());

    let lines = EditorApp::build_scene_anchor_overlay_lines(&ui_state, None, Some(&config));

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].start, glam::Vec2::new(48.0, 64.0));
    assert_eq!(lines[0].end, glam::Vec2::new(72.0, 96.0));
    assert_eq!(lines[1].start, glam::Vec2::new(72.0, 64.0));
    assert_eq!(lines[1].end, glam::Vec2::new(48.0, 96.0));
    assert_eq!(lines[0].thickness, 1.0);
    assert_eq!(lines[0].color, [0.1882, 0.5176, 1.0, 1.0]);
}

#[test]
fn build_scene_anchor_overlay_lines_prefer_tilemap_tile_size() {
    let mut config = crate::config::EditorConfig::default();
    config.editor_settings.grid.grid_size = [24, 32];
    config.editor_settings.grid.snap_to_grid = true;

    let tilemap = toki_core::assets::tilemap::TileMap {
        size: UVec2::new(8, 8),
        tile_size: UVec2::new(40, 48),
        atlas: std::path::PathBuf::from("dummy.json"),
        tiles: vec![],
        objects: vec![],
    };

    let mut ui_state = crate::ui::EditorUI::new();
    let mut scene = Scene::new("Main Scene".to_string());
    scene.anchors.push(SceneAnchor {
        id: "spawn_point_1".to_string(),
        kind: SceneAnchorKind::SpawnPoint,
        position: IVec2::new(80, 96),
        facing: None,
    });
    ui_state.scenes = vec![scene];
    ui_state.active_scene = Some("Main Scene".to_string());

    let lines = EditorApp::build_scene_anchor_overlay_lines(&ui_state, Some(&tilemap), Some(&config));

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].end - lines[0].start, glam::Vec2::new(40.0, 48.0));
    assert_eq!(lines[1].start, glam::Vec2::new(120.0, 96.0));
}

#[test]
fn build_scene_anchor_overlay_lines_use_drag_preview_instead_of_original_anchor() {
    let mut ui_state = crate::ui::EditorUI::new();
    let mut scene = Scene::new("Main Scene".to_string());
    scene.anchors.push(SceneAnchor {
        id: "spawn_point_1".to_string(),
        kind: SceneAnchorKind::SpawnPoint,
        position: IVec2::new(16, 16),
        facing: None,
    });
    ui_state.scenes = vec![scene];
    ui_state.active_scene = Some("Main Scene".to_string());
    ui_state.begin_scene_anchor_move_drag(crate::ui::editor_ui::SceneAnchorMoveDragState {
        scene_name: "Main Scene".to_string(),
        anchor: SceneAnchor {
            id: "spawn_point_1".to_string(),
            kind: SceneAnchorKind::SpawnPoint,
            position: IVec2::new(16, 16),
            facing: None,
        },
        grab_offset: glam::Vec2::ZERO,
    });
    ui_state.placement.preview_position = Some(glam::Vec2::new(48.0, 64.0));
    let config = crate::config::EditorConfig::default();

    let lines = EditorApp::build_scene_anchor_overlay_lines(&ui_state, None, Some(&config));

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].start, glam::Vec2::new(48.0, 64.0));
    assert_eq!(lines[0].end, glam::Vec2::new(64.0, 80.0));
    assert_eq!(lines[1].start, glam::Vec2::new(64.0, 64.0));
    assert_eq!(lines[1].end, glam::Vec2::new(48.0, 80.0));
}

// =============================================================================
// EditorSessionState tests
// =============================================================================

#[test]
fn editor_session_state_defaults_to_no_loaded_scene() {
    let session = super::EditorSessionState::default();
    assert!(session.last_loaded_active_scene.is_none());
}

#[test]
fn editor_session_state_defaults_to_empty_loaded_maps() {
    let session = super::EditorSessionState::default();
    assert!(session.loaded_scene_maps.is_empty());
}

#[test]
fn editor_session_state_defaults_to_startup_auto_open_not_done() {
    let session = super::EditorSessionState::default();
    assert!(!session.startup_project_auto_open_done);
}

#[test]
fn editor_session_state_tracks_loaded_scene_maps() {
    let mut session = super::EditorSessionState::default();
    session
        .loaded_scene_maps
        .insert("Main Scene".to_string(), "main_map".to_string());

    assert_eq!(
        session.loaded_scene_maps.get("Main Scene"),
        Some(&"main_map".to_string())
    );
}

#[test]
fn editor_session_state_tracks_last_loaded_scene() {
    let session = super::EditorSessionState {
        last_loaded_active_scene: Some("Main Scene".to_string()),
        ..Default::default()
    };

    assert_eq!(
        session.last_loaded_active_scene,
        Some("Main Scene".to_string())
    );
}

// =============================================================================
// EditorResourceCache tests
// =============================================================================

#[test]
fn editor_resource_cache_defaults_to_no_texture() {
    let cache = super::EditorResourceCache::default();
    assert!(cache.busy_logo_texture.is_none());
}

#[test]
fn editor_resource_cache_defaults_to_no_font_project_path() {
    let cache = super::EditorResourceCache::default();
    assert!(cache.menu_font_project_path.is_none());
}

#[test]
fn editor_resource_cache_tracks_font_project_path() {
    let cache = super::EditorResourceCache {
        menu_font_project_path: Some(PathBuf::from("/tmp/project")),
        ..Default::default()
    };

    assert_eq!(
        cache.menu_font_project_path,
        Some(PathBuf::from("/tmp/project"))
    );
}

// =============================================================================
// EditorPlatform tests
// =============================================================================

#[test]
fn editor_platform_defaults_to_uninitialized() {
    let platform = super::EditorPlatform::default();
    assert!(platform.window.is_none());
    assert!(platform.renderer.is_none());
    assert!(platform.egui_winit.is_none());
}

// =============================================================================
// EditorViewports tests
// =============================================================================

#[test]
fn editor_viewports_defaults_to_no_viewports() {
    let viewports = super::EditorViewports::default();
    assert!(viewports.scene.is_none());
    assert!(viewports.map_editor.is_none());
}

// =============================================================================
// EditorCore tests
// =============================================================================

#[test]
fn editor_core_has_default_config() {
    let core = super::EditorCore::default();
    // Config should have default editor settings
    assert_eq!(core.config.editor_settings.window_size, [1200, 800]);
}

#[test]
fn editor_core_has_default_ui() {
    let core = super::EditorCore::default();
    // UI starts with default scene active
    assert_eq!(core.ui.active_scene, Some("Main Scene".to_string()));
}

#[test]
fn editor_core_has_empty_project_manager() {
    let core = super::EditorCore::default();
    // Project manager should have no current project by default
    assert!(core.project_manager.current_project.is_none());
}
