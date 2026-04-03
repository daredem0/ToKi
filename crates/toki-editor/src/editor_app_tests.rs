use super::EditorApp;
use crate::project::assets::TilemapAsset;
use crate::project::ProjectAssets;
use crate::ui::editor_ui::EditorConfirmation;
use crate::ui::editor_ui::{CenterPanelTab, EntityMoveDragState, MapEditorDraft, SelectionMask};
use glam::{IVec2, UVec2, Vec2};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
use toki_core::assets::tilemap::TileMap;
use toki_core::collision::CollisionBox;
use toki_core::entity::{
    AnimationClipDef, AnimationsDef, AudioDef, CollisionDef, CombatComponent, ComponentsDef,
    Entity, EntityDefinition, EntityFootprint, EntityGrounding, EntityKind, EntityRendering,
    EntityStats, Inventory, MovementSoundTrigger, PickupDef, RenderingDef, StaticObjectRenderDef,
};
use toki_core::game::SceneSystem;
use toki_core::scene::{SceneAnchor, SceneAnchorKind, ScenePlayerEntry};
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
fn escape_exits_placement_mode_before_requesting_editor_close() {
    let mut app = EditorApp::new(None);
    crate::ui::editor_context::scene_viewport_context_mut(&mut app.tabs.ui)
        .placement
        .enter_placement_mode("player".to_string());

    app.handle_escape_key();

    assert!(
        !crate::ui::editor_context::scene_viewport_context(&app.tabs.ui)
            .placement
            .is_in_placement_mode()
    );
    assert!(app.tabs.ui.project.pending_confirmation.is_none());
}

#[test]
fn escape_requests_exit_confirmation_when_not_in_placement_mode() {
    let mut app = EditorApp::new(None);

    app.handle_escape_key();

    assert_eq!(
        app.tabs.ui.project.pending_confirmation,
        Some(EditorConfirmation::ExitEditor)
    );
}

#[test]
fn escape_does_not_request_exit_when_sprite_editor_has_selection() {
    let mut app = EditorApp::new(None);
    app.tabs.ui.set_active_tab(CenterPanelTab::SpriteEditor);
    crate::ui::editor_context::sprite_state_mut(&mut app.tabs.ui).new_canvas(8, 8);
    let mut selection = SelectionMask::new(8, 8);
    selection.select_pixel(1, 1);
    crate::ui::editor_context::sprite_state_mut(&mut app.tabs.ui)
        .active_mut()
        .selection = Some(selection);

    app.handle_escape_key();

    assert!(app.tabs.ui.project.pending_confirmation.is_none());
}

#[test]
fn escape_does_not_request_exit_when_sprite_editor_has_floating_selection() {
    let mut app = EditorApp::new(None);
    app.tabs.ui.set_active_tab(CenterPanelTab::SpriteEditor);
    crate::ui::editor_context::sprite_state_mut(&mut app.tabs.ui).new_canvas(8, 8);
    if let Some(canvas) = &mut crate::ui::editor_context::sprite_state_mut(&mut app.tabs.ui)
        .active_mut()
        .canvas
    {
        canvas.set_pixel(1, 1, crate::ui::editor_ui::PixelColor::rgb(255, 0, 0));
    }
    let mut selection = SelectionMask::new(8, 8);
    selection.select_pixel(1, 1);
    crate::ui::editor_context::sprite_state_mut(&mut app.tabs.ui)
        .active_mut()
        .selection = Some(selection);
    assert!(crate::ui::editor_context::sprite_state_mut(&mut app.tabs.ui).lift_selection());

    app.handle_escape_key();

    assert!(app.tabs.ui.project.pending_confirmation.is_none());
}

#[test]
fn reload_project_assets_refreshes_available_palettes_and_scene_list() {
    let temp_dir = tempfile::tempdir().expect("temp dir should exist");
    let project_path = temp_dir.path().to_path_buf();
    fs::create_dir_all(project_path.join("palettes")).expect("palettes dir should exist");
    fs::create_dir_all(project_path.join("scenes")).expect("scenes dir should exist");

    let scene = Scene::new("ReloadedScene".to_string());
    fs::write(
        project_path.join("scenes").join("ReloadedScene.json"),
        serde_json::to_string_pretty(&scene).expect("scene should serialize"),
    )
    .expect("scene should be written");
    fs::write(
        project_path.join("palettes").join("swamp.json"),
        r#"{"colors":[[8,24,8,255],[32,72,24,255],[96,144,56,255],[184,216,104,255]]}"#,
    )
    .expect("palette should be written");

    let mut app = EditorApp::new(None);
    app.core.project_manager.current_project = Some(crate::project::Project::new(
        "Reload Test".to_string(),
        project_path.clone(),
    ));
    app.core.project_manager.project_assets = Some(ProjectAssets::new(project_path));

    app.handle_reload_project_assets_request();

    assert!(app.tabs.ui.project.available_palettes.contains_key("swamp"));
    assert!(app
        .tabs
        .ui
        .scenes
        .iter()
        .any(|scene| scene.name == "ReloadedScene"));
}

#[test]
fn toggled_fullscreen_state_enters_borderless_fullscreen_when_windowed() {
    let state = EditorApp::toggled_fullscreen_state(false);
    assert!(matches!(
        state,
        Some(winit::window::Fullscreen::Borderless(None))
    ));
}

#[test]
fn toggled_fullscreen_state_exits_fullscreen_when_already_fullscreen() {
    let state = EditorApp::toggled_fullscreen_state(true);
    assert_eq!(state, None);
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
fn discover_runtime_binary_prefers_persisted_path_then_bundled_then_workspace_release() {
    let temp = tempfile::tempdir().expect("temp dir");
    let persisted = temp.path().join("configured-runtime");
    let bundled_dir = temp.path().join("bundle");
    let bundled_exe = bundled_dir.join(EditorApp::runtime_binary_name());
    let workspace_root = temp.path().join("workspace");
    let workspace_release = workspace_root
        .join("target")
        .join("release")
        .join(EditorApp::runtime_binary_name());
    std::fs::create_dir_all(&bundled_dir).expect("bundled dir");
    std::fs::create_dir_all(workspace_release.parent().expect("release dir"))
        .expect("workspace release dir");

    std::fs::write(&workspace_release, "workspace").expect("workspace runtime");
    let discovered = EditorApp::discover_runtime_binary_from_sources(
        Some(persisted.as_path()),
        Some(bundled_exe.as_path()),
        &workspace_root,
    )
    .expect("workspace runtime should be found");
    assert_eq!(discovered, workspace_release);

    std::fs::write(&bundled_exe, "bundled").expect("bundled runtime");
    let discovered = EditorApp::discover_runtime_binary_from_sources(
        Some(persisted.as_path()),
        Some(bundled_exe.as_path()),
        &workspace_root,
    )
    .expect("bundled runtime should be found");
    assert_eq!(discovered, bundled_exe);

    std::fs::write(&persisted, "configured").expect("configured runtime");
    let discovered = EditorApp::discover_runtime_binary_from_sources(
        Some(persisted.as_path()),
        Some(bundled_exe.as_path()),
        &workspace_root,
    )
    .expect("configured runtime should be found");
    assert_eq!(discovered, persisted);
}

#[test]
fn runtime_discovery_error_message_lists_all_candidates() {
    let candidates = vec![
        PathBuf::from("/tmp/runtime-configured"),
        PathBuf::from("/opt/toki/toki-runtime"),
        PathBuf::from("/workspace/target/release/toki-runtime"),
    ];

    let message = EditorApp::runtime_discovery_error_message(&candidates);

    assert!(message.contains("Could not find a ToKi runtime binary for export."));
    assert!(message.contains("/tmp/runtime-configured"));
    assert!(message.contains("/opt/toki/toki-runtime"));
    assert!(message.contains("/workspace/target/release/toki-runtime"));
}

#[test]
fn collect_available_map_names_sorts_tilemap_names() {
    let mut project_assets = ProjectAssets::new(PathBuf::from("/tmp/demo"));
    project_assets.tilemaps.insert(
        "z_map".to_string(),
        TilemapAsset {
            path: PathBuf::from("maps/z_map.json"),
        },
    );
    project_assets.tilemaps.insert(
        "a_map".to_string(),
        TilemapAsset {
            path: PathBuf::from("maps/a_map.json"),
        },
    );

    let names = EditorApp::collect_available_map_names(Some(&project_assets));

    assert_eq!(names, Some(vec!["a_map".to_string(), "z_map".to_string()]));
}

#[test]
fn project_indexed_palette_override_reads_runtime_display_setting() {
    let mut project =
        crate::project::Project::new("Demo".to_string(), PathBuf::from("/tmp/DemoProject"));
    project.metadata.runtime.display.indexed_palette_override = Some("pocket".to_string());

    let override_name = EditorApp::project_indexed_palette_override(Some(&project));

    assert_eq!(override_name.as_deref(), Some("pocket"));
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
        },
    };

    let saved = EditorApp::tilemap_to_save_for_map_editor_draft(&draft, None);

    assert_eq!(saved, draft.tilemap);
}

#[test]
fn init_viewport_with_returns_none_when_renderer_is_missing() {
    let mut create_called = false;
    let viewport = EditorApp::init_viewport_with(
        Option::<&()>::None,
        || {
            create_called = true;
            Ok(7_u8)
        },
        |_renderer: &(), _viewport: &mut u8| Ok(()),
        "scene viewport",
    );

    assert!(viewport.is_none());
    assert!(
        !create_called,
        "viewport creation should be skipped without a renderer"
    );
}

#[test]
fn init_viewport_with_returns_none_when_viewport_creation_fails() {
    let mut initialize_called = false;
    let viewport = EditorApp::init_viewport_with(
        Some(&()),
        || Err(anyhow::anyhow!("creation failed")),
        |_renderer: &(), _viewport: &mut u8| {
            initialize_called = true;
            Ok(())
        },
        "scene viewport",
    );

    assert!(viewport.is_none());
    assert!(
        !initialize_called,
        "viewport initialization should not run after a creation error"
    );
}

#[test]
fn init_viewport_with_returns_none_when_viewport_initialization_fails() {
    let viewport = EditorApp::init_viewport_with(
        Some(&()),
        || Ok(11_u8),
        |_renderer: &(), _viewport: &mut u8| Err(anyhow::anyhow!("wgpu init failed")),
        "scene viewport",
    );

    assert!(viewport.is_none());
}

#[test]
fn init_viewport_with_returns_viewport_when_creation_and_initialization_succeed() {
    let viewport = EditorApp::init_viewport_with(
        Some(&()),
        || Ok(13_u8),
        |_renderer: &(), viewport: &mut u8| {
            *viewport = 21;
            Ok(())
        },
        "scene viewport",
    );

    assert_eq!(viewport, Some(21));
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
        color_mode: toki_core::assets::atlas::ColorMode::TrueColor,
        palette: None,
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
        definition_name: Some("test".to_string().into()),
        persistent_across_saves: false,
        control_role: toki_core::entity::ControlRole::None,
        audio: toki_core::entity::EntityAudioSettings::default(),
        rendering: EntityRendering::default(),
        collision_box: Some(CollisionBox::solid_box(UVec2::new(16, 16))),
        solid: true,
        active: true,
        movement_accumulator: glam::Vec2::ZERO,
        tags: Vec::new(),
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
        name: "coin_pickup".to_string().into(),
        display_name: "Coin Pickup".to_string(),
        description: "Collectible coin".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            has_drop_shadow: false,
            palette_override: None,
            static_object: Some(StaticObjectRenderDef {
                sheet: "items".to_string(),
                object_name: "coin".to_string(),
            }),
            grounding: Default::default(),
        },
        solid: false,
        active: true,
        components: ComponentsDef {
            pickup: Some(PickupDef {
                item_id: "coin".to_string(),
                count: 1,
            }),
            ..Default::default()
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
fn load_preview_sprite_frame_static_prefers_animation_for_decorations_with_both_paths() {
    let temp_dir = tempfile::tempdir().expect("temp dir should exist");
    let project_path = temp_dir.path().to_path_buf();
    fs::create_dir_all(project_path.join("entities")).expect("entities dir should exist");
    fs::create_dir_all(project_path.join("assets/sprites")).expect("sprites dir should exist");
    fs::write(
        project_path.join("assets/sprites/objects.json"),
        r#"{
            "sheet_type": "objects",
            "image": "objects.png",
            "tile_size": [16, 16],
            "objects": {
                "torch_static": {
                    "position": [0, 0],
                    "size_tiles": [1, 2]
                }
            }
        }"#,
    )
    .expect("object sheet should be written");
    fs::write(
        project_path.join("assets/sprites/decor.json"),
        r#"{
            "image": "decor.png",
            "tile_size": [16, 16],
            "tiles": {
                "torch/idle_a": {
                    "position": [0, 0],
                    "properties": { "solid": false, "trigger": false }
                }
            }
        }"#,
    )
    .expect("atlas should be written");
    let entity_def = EntityDefinition {
        name: "torch".to_string().into(),
        display_name: "Torch".to_string(),
        description: "Animated torch".to_string(),
        rendering: RenderingDef {
            size: [16, 32],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            has_drop_shadow: false,
            palette_override: None,
            static_object: Some(StaticObjectRenderDef {
                sheet: "objects".to_string(),
                object_name: "torch_static".to_string(),
            }),
            grounding: Default::default(),
        },
        solid: false,
        active: true,
        components: ComponentsDef::default(),
        collision: CollisionDef {
            enabled: false,
            offset: [0, 0],
            size: [16, 32],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 0.0,
            hearing_radius: 0,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: String::new(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "decor".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["torch/idle_a".to_string()],
                frame_positions: None,
                frame_duration_ms: 120.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle".to_string(),
        },
        category: "decoration".to_string(),
        tags: vec![],
    };
    fs::write(
        project_path.join("entities/torch.json"),
        serde_json::to_string_pretty(&entity_def).expect("entity json should serialize"),
    )
    .expect("entity definition should be written");

    let mut project_assets = ProjectAssets::new(project_path.clone());
    project_assets
        .scan_assets()
        .expect("project assets should scan");

    let preview =
        EditorApp::load_preview_sprite_frame_static("torch", &project_path, &project_assets)
            .expect("animated decoration should produce a preview visual");

    assert_eq!(preview.size, UVec2::new(16, 32));
    assert!(preview
        .texture_path
        .as_ref()
        .is_some_and(|path| path.ends_with("decor.png")));
}

#[test]
fn load_preview_sprite_frame_static_supports_object_sheet_backed_decoration_animation() {
    let temp_dir = tempfile::tempdir().expect("temp dir should exist");
    let project_path = temp_dir.path().to_path_buf();
    fs::create_dir_all(project_path.join("entities")).expect("entities dir should exist");
    fs::create_dir_all(project_path.join("assets/sprites")).expect("sprites dir should exist");
    fs::write(
        project_path.join("assets/sprites/HouseM.json"),
        r#"{
            "sheet_type": "objects",
            "image": "HouseM.png",
            "tile_size": [64, 64],
            "objects": {
                "object_0": {"position": [0, 0], "size_tiles": [1, 1]},
                "object_1": {"position": [1, 0], "size_tiles": [1, 1]}
            }
        }"#,
    )
    .expect("object sheet should be written");
    let entity_def = EntityDefinition {
        name: "house_anim".to_string().into(),
        display_name: "House Anim".to_string(),
        description: "Animated house".to_string(),
        rendering: RenderingDef {
            size: [64, 64],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            has_drop_shadow: false,
            palette_override: None,
            static_object: Some(StaticObjectRenderDef {
                sheet: "HouseM".to_string(),
                object_name: "object_0".to_string(),
            }),
            grounding: Default::default(),
        },
        solid: true,
        active: true,
        components: ComponentsDef::default(),
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [64, 64],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 0.0,
            hearing_radius: 0,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: String::new(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "HouseM".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["object_1".to_string()],
                frame_positions: None,
                frame_duration_ms: 120.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle".to_string(),
        },
        category: "decoration".to_string(),
        tags: vec![],
    };
    fs::write(
        project_path.join("entities/house_anim.json"),
        serde_json::to_string_pretty(&entity_def).expect("entity json should serialize"),
    )
    .expect("entity definition should be written");

    let mut project_assets = ProjectAssets::new(project_path.clone());
    project_assets
        .scan_assets()
        .expect("project assets should scan");

    let preview =
        EditorApp::load_preview_sprite_frame_static("house_anim", &project_path, &project_assets)
            .expect("object-sheet-backed animated decoration should produce a preview visual");

    assert_eq!(preview.size, UVec2::new(64, 64));
    assert!(preview
        .texture_path
        .as_ref()
        .is_some_and(|path| path.ends_with("HouseM.png")));
}

#[test]
fn build_scene_player_overlay_sprites_uses_scene_player_entry_spawn_point() {
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
                "hero_idle": {
                    "position": [0, 0],
                    "size_tiles": [1, 1]
                }
            }
        }"#,
    )
    .expect("object sheet should be written");

    let entity_def = EntityDefinition {
        name: "player".to_string().into(),
        display_name: "Player".to_string(),
        description: "Scene player preview".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            has_drop_shadow: false,
            palette_override: None,
            static_object: Some(StaticObjectRenderDef {
                sheet: "items".to_string(),
                object_name: "hero_idle".to_string(),
            }),
            grounding: Default::default(),
        },
        solid: false,
        active: true,
        components: ComponentsDef::default(),
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
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
        category: "character".to_string(),
        tags: vec![],
    };
    fs::write(
        project_path.join("entities/player.json"),
        serde_json::to_string_pretty(&entity_def).expect("entity json should serialize"),
    )
    .expect("entity definition should be written");

    let mut project_assets = ProjectAssets::new(project_path.clone());
    project_assets
        .scan_assets()
        .expect("project assets should scan");

    let mut ui_state = crate::ui::EditorUI::new();
    let mut scene = Scene::new("Main Scene".to_string());
    scene.anchors.push(SceneAnchor {
        id: "spawn_a".to_string(),
        kind: SceneAnchorKind::SpawnPoint,
        position: IVec2::new(64, 80),
        facing: None,
    });
    scene.player_entry = Some(ScenePlayerEntry {
        entity_definition_name: "player".to_string().into(),
        spawn_point_id: "spawn_a".to_string(),
    });
    ui_state.scenes = vec![scene];
    ui_state.active_scene = Some("Main Scene".to_string());

    let sprites = EditorApp::build_scene_player_overlay_sprites(
        &ui_state,
        &project_path,
        &project_assets,
        &mut HashMap::new(),
    );

    assert_eq!(sprites.len(), 1);
    assert_eq!(sprites[0].world_position, IVec2::new(64, 80));
    assert_eq!(sprites[0].visual.size, UVec2::new(16, 16));
}

#[test]
fn build_scene_preview_game_state_keeps_scene_entities_when_scene_has_player_entry() {
    let temp_dir = tempfile::tempdir().expect("temp dir should exist");
    let project_path = temp_dir.path().to_path_buf();
    fs::create_dir_all(project_path.join("entities")).expect("entities dir should exist");

    fs::write(
        project_path.join("entities/player.json"),
        serde_json::to_string_pretty(&EntityDefinition {
            name: "player".to_string().into(),
            display_name: "Player".to_string(),
            description: "Scene player preview".to_string(),
            rendering: RenderingDef {
                size: [16, 16],
                render_layer: 0,
                visible: true,
                has_shadow: true,
                has_drop_shadow: false,
                palette_override: None,
                static_object: None,
                grounding: Default::default(),
            },
            solid: true,
            active: true,
            components: ComponentsDef {
                combat: Some(CombatComponent {
                    health: Some(100),
                    stats: EntityStats::from_legacy_health(Some(100)),
                }),
                inventory: Some(Inventory::default()),
                ..Default::default()
            },
            collision: CollisionDef {
                enabled: true,
                offset: [0, 0],
                size: [16, 16],
                trigger: false,
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
            category: "character".to_string(),
            tags: vec!["player".to_string()],
        })
        .expect("entity json should serialize"),
    )
    .expect("entity definition should be written");
    fs::write(
        project_path.join("entities/test.json"),
        serde_json::to_string_pretty(&EntityDefinition {
            name: "test".to_string().into(),
            display_name: "Test Entity".to_string(),
            description: "Scene entity".to_string(),
            rendering: RenderingDef {
                size: [16, 16],
                render_layer: 0,
                visible: true,
                has_shadow: true,
                has_drop_shadow: false,
                palette_override: None,
                static_object: None,
                grounding: Default::default(),
            },
            solid: true,
            active: true,
            components: ComponentsDef {
                combat: Some(CombatComponent {
                    health: Some(100),
                    stats: EntityStats::from_legacy_health(Some(100)),
                }),
                ..Default::default()
            },
            collision: CollisionDef {
                enabled: true,
                offset: [0, 0],
                size: [16, 16],
                trigger: false,
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
            category: "character".to_string(),
            tags: vec!["test".to_string()],
        })
        .expect("entity json should serialize"),
    )
    .expect("entity definition should be written");

    let mut project_assets = ProjectAssets::new(project_path);
    project_assets
        .scan_assets()
        .expect("project assets should scan");

    let mut scene = Scene::new("Main Scene".to_string());
    scene.anchors.push(SceneAnchor {
        id: "spawn_a".to_string(),
        kind: SceneAnchorKind::SpawnPoint,
        position: IVec2::new(64, 80),
        facing: None,
    });
    scene.player_entry = Some(ScenePlayerEntry {
        entity_definition_name: "player".to_string().into(),
        spawn_point_id: "spawn_a".to_string(),
    });
    scene.add_entity(solid_entity(77, IVec2::new(16, 32)));

    let game_state = EditorApp::build_scene_preview_game_state(&scene, Some(&mut project_assets))
        .expect("scene preview game state should build");

    assert_eq!(
        SceneSystem::active_scene(&game_state).map(|scene| scene.name.as_str()),
        Some("Main Scene")
    );
    assert!(game_state
        .world()
        .player_id()
        .and_then(|id| game_state.world().entity_manager().get_entity(id))
        .is_some());
    assert!(
        game_state
            .world()
            .entity_manager()
            .active_entities()
            .iter()
            .filter_map(|&id| game_state.world().entity_manager().get_entity(id))
            .any(|entity| entity.id == 77),
        "authored scene entity should still be present in preview GameState"
    );
    assert_eq!(
        game_state.world().entity_manager().active_entities().len(),
        2
    );
}

#[test]
fn build_scene_preview_game_state_loads_scene_entity_definitions_for_legacy_grounding_hydration() {
    let temp_dir = tempfile::tempdir().expect("temp dir should exist");
    let project_path = temp_dir.path().to_path_buf();
    fs::create_dir_all(project_path.join("entities")).expect("entities dir should exist");

    let soldier_definition = EntityDefinition {
        name: "soldier".to_string().into(),
        display_name: "Soldier".to_string(),
        description: "Soldier".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            has_drop_shadow: false,
            palette_override: None,
            static_object: None,
            grounding: EntityGrounding {
                origin: None,
                footprint: Some(EntityFootprint::new([4, 12], [8, 4])),
            },
        },
        solid: true,
        active: true,
        components: ComponentsDef {
            combat: Some(CombatComponent {
                health: Some(100),
                stats: EntityStats::from_legacy_health(Some(100)),
            }),
            ..Default::default()
        },
        collision: CollisionDef {
            enabled: true,
            offset: [4, 12],
            size: [8, 4],
            trigger: false,
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
        category: "character".to_string(),
        tags: vec!["soldier".to_string()],
    };

    fs::write(
        project_path.join("entities/soldier.json"),
        serde_json::to_string_pretty(&soldier_definition).expect("entity json should serialize"),
    )
    .expect("entity definition should be written");

    let mut project_assets = ProjectAssets::new(project_path);
    project_assets
        .scan_assets()
        .expect("project assets should scan");

    let mut scene = Scene::new("Main Scene".to_string());
    let mut stale_scene_entity = soldier_definition
        .create_entity(IVec2::new(24, 48), 15)
        .expect("scene entity should instantiate");
    stale_scene_entity.rendering.grounding = EntityGrounding::default();
    stale_scene_entity.collision_box = Some(CollisionBox::solid_box(stale_scene_entity.size));
    scene.add_entity(stale_scene_entity);

    let game_state = EditorApp::build_scene_preview_game_state(&scene, Some(&mut project_assets))
        .expect("scene preview game state should build");

    let soldier = game_state
        .world()
        .entity_manager()
        .active_entities()
        .iter()
        .filter_map(|&id| game_state.world().entity_manager().get_entity(id))
        .cloned()
        .into_iter()
        .find(|entity| entity.definition_name.as_deref() == Some("soldier"))
        .expect("soldier should exist in preview");
    assert_eq!(
        soldier.rendering.grounding.footprint,
        Some(EntityFootprint::new([4, 12], [8, 4]))
    );
    let collision_box = soldier
        .collision_box
        .as_ref()
        .expect("collision should exist");
    assert_eq!(collision_box.offset, IVec2::new(4, 12));
    assert_eq!(collision_box.size, UVec2::new(8, 4));
}

#[test]
fn build_scene_preview_game_state_errors_when_player_entry_definition_is_unavailable() {
    let mut scene = Scene::new("Main Scene".to_string());
    scene.anchors.push(SceneAnchor {
        id: "spawn_a".to_string(),
        kind: SceneAnchorKind::SpawnPoint,
        position: IVec2::new(64, 80),
        facing: None,
    });
    scene.player_entry = Some(ScenePlayerEntry {
        entity_definition_name: "player".to_string().into(),
        spawn_point_id: "spawn_a".to_string(),
    });

    let error = EditorApp::build_scene_preview_game_state(&scene, None)
        .expect_err("scene preview build should fail without required project assets");

    assert!(error.contains("no project assets are available"));
}

#[test]
fn build_scene_player_overlay_sprites_skips_when_scene_already_contains_authored_player_entity() {
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
                "hero_idle": {
                    "position": [0, 0],
                    "size_tiles": [1, 1]
                }
            }
        }"#,
    )
    .expect("object sheet should be written");
    fs::write(
        project_path.join("entities/player.json"),
        serde_json::to_string_pretty(&EntityDefinition {
            name: "player".to_string().into(),
            display_name: "Player".to_string(),
            description: String::new(),
            rendering: RenderingDef {
                size: [16, 16],
                render_layer: 0,
                visible: true,
                has_shadow: true,
                has_drop_shadow: false,
                palette_override: None,
                static_object: Some(StaticObjectRenderDef {
                    sheet: "items".to_string(),
                    object_name: "hero_idle".to_string(),
                }),
                grounding: Default::default(),
            },
            solid: false,
            active: true,
            components: ComponentsDef::default(),
            collision: CollisionDef {
                enabled: true,
                offset: [0, 0],
                size: [16, 16],
                trigger: false,
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
            category: "character".to_string(),
            tags: vec![],
        })
        .expect("entity json should serialize"),
    )
    .expect("entity definition should be written");

    let mut project_assets = ProjectAssets::new(project_path.clone());
    project_assets
        .scan_assets()
        .expect("project assets should scan");

    let mut ui_state = crate::ui::EditorUI::new();
    let mut scene = Scene::new("Main Scene".to_string());
    scene.anchors.push(SceneAnchor {
        id: "spawn_a".to_string(),
        kind: SceneAnchorKind::SpawnPoint,
        position: IVec2::new(64, 80),
        facing: None,
    });
    scene.player_entry = Some(ScenePlayerEntry {
        entity_definition_name: "player".to_string().into(),
        spawn_point_id: "spawn_a".to_string(),
    });
    let mut placed_player = solid_entity(1, IVec2::new(64, 80));
    placed_player.control_role = toki_core::entity::ControlRole::PlayerCharacter;
    scene.add_entity(placed_player);
    ui_state.scenes = vec![scene];
    ui_state.active_scene = Some("Main Scene".to_string());

    let sprites = EditorApp::build_scene_player_overlay_sprites(
        &ui_state,
        &project_path,
        &project_assets,
        &mut HashMap::new(),
    );

    assert!(sprites.is_empty());
}

#[test]
fn cached_preview_sprite_frame_reuses_loaded_visual_without_reloading_from_disk() {
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
                "hero_idle": {
                    "position": [0, 0],
                    "size_tiles": [1, 1]
                }
            }
        }"#,
    )
    .expect("object sheet should be written");
    fs::write(
        project_path.join("entities/player.json"),
        serde_json::to_string_pretty(&EntityDefinition {
            name: "player".to_string().into(),
            display_name: "Player".to_string(),
            description: String::new(),
            rendering: RenderingDef {
                size: [16, 16],
                render_layer: 0,
                visible: true,
                has_shadow: true,
                has_drop_shadow: false,
                palette_override: None,
                static_object: Some(StaticObjectRenderDef {
                    sheet: "items".to_string(),
                    object_name: "hero_idle".to_string(),
                }),
                grounding: Default::default(),
            },
            solid: false,
            active: true,
            components: ComponentsDef::default(),
            collision: CollisionDef {
                enabled: true,
                offset: [0, 0],
                size: [16, 16],
                trigger: false,
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
            category: "character".to_string(),
            tags: vec![],
        })
        .expect("entity json should serialize"),
    )
    .expect("entity definition should be written");

    let mut project_assets = ProjectAssets::new(project_path.clone());
    project_assets
        .scan_assets()
        .expect("project assets should scan");

    let mut app = super::EditorApp::new(None);
    let first = super::EditorApp::cached_preview_sprite_frame(
        &mut app.panel_coordinator.preview_sprite_frames,
        "player",
        &project_path,
        &project_assets,
        None,
    )
    .expect("first cached load should succeed");

    fs::remove_file(project_path.join("entities/player.json"))
        .expect("entity definition should be removable after caching");

    let second = super::EditorApp::cached_preview_sprite_frame(
        &mut app.panel_coordinator.preview_sprite_frames,
        "player",
        &project_path,
        &project_assets,
        None,
    )
    .expect("cached preview should still be returned");

    assert_eq!(first.size, second.size);
    assert_eq!(first.texture_path, second.texture_path);
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

    let lines =
        EditorApp::build_scene_anchor_overlay_lines(&ui_state, Some(&tilemap), Some(&config));

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
    crate::ui::editor_context::scene_viewport_context_mut(&mut ui_state)
        .placement
        .begin_scene_anchor_move_drag(crate::ui::editor_ui::SceneAnchorMoveDragState {
            scene_name: "Main Scene".to_string(),
            anchor: SceneAnchor {
                id: "spawn_point_1".to_string(),
                kind: SceneAnchorKind::SpawnPoint,
                position: IVec2::new(16, 16),
                facing: None,
            },
            grab_offset: glam::Vec2::ZERO,
        });
    crate::ui::editor_context::scene_viewport_context_mut(&mut ui_state)
        .placement
        .preview_position = Some(glam::Vec2::new(48.0, 64.0));
    let config = crate::config::EditorConfig::default();

    let lines = EditorApp::build_scene_anchor_overlay_lines(&ui_state, None, Some(&config));

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].start, glam::Vec2::new(48.0, 64.0));
    assert_eq!(lines[0].end, glam::Vec2::new(64.0, 80.0));
    assert_eq!(lines[1].start, glam::Vec2::new(64.0, 64.0));
    assert_eq!(lines[1].end, glam::Vec2::new(48.0, 80.0));
}

// =============================================================================
// ProjectSessionManager tests
// =============================================================================

#[test]
fn editor_session_state_defaults_to_no_loaded_scene() {
    let session = super::ProjectSessionManager::default();
    assert!(session.last_loaded_active_scene.is_none());
}

#[test]
fn editor_session_state_defaults_to_empty_loaded_maps() {
    let session = super::ProjectSessionManager::default();
    assert!(session.loaded_scene_maps.is_empty());
}

#[test]
fn editor_session_state_defaults_to_startup_auto_open_not_done() {
    let session = super::ProjectSessionManager::default();
    assert!(!session.startup_project_auto_open_done);
}

#[test]
fn editor_session_state_tracks_loaded_scene_maps() {
    let mut session = super::ProjectSessionManager::default();
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
    let session = super::ProjectSessionManager {
        last_loaded_active_scene: Some("Main Scene".to_string()),
        ..Default::default()
    };

    assert_eq!(
        session.last_loaded_active_scene,
        Some("Main Scene".to_string())
    );
}

// =============================================================================
// EditorPanelCoordinator tests
// =============================================================================

#[test]
fn editor_resource_cache_defaults_to_no_texture() {
    let cache = super::EditorPanelCoordinator::default();
    assert!(cache.busy_logo_texture.is_none());
}

#[test]
fn editor_resource_cache_defaults_to_no_font_project_path() {
    let cache = super::EditorPanelCoordinator::default();
    assert!(cache.menu_font_project_path.is_none());
}

#[test]
fn editor_resource_cache_defaults_to_empty_preview_sprite_cache() {
    let cache = super::EditorPanelCoordinator::default();
    assert!(cache.preview_sprite_frames.is_empty());
}

#[test]
fn editor_resource_cache_tracks_font_project_path() {
    let cache = super::EditorPanelCoordinator {
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
// EditorViewportManager tests
// =============================================================================

#[test]
fn editor_viewports_defaults_to_no_viewports() {
    let viewports = super::EditorViewportManager::default();
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
fn editor_core_has_empty_project_manager() {
    let core = super::EditorCore::default();
    // Project manager should have no current project by default
    assert!(core.project_manager.current_project.is_none());
}

#[test]
fn editor_tab_manager_has_default_ui() {
    let tabs = super::EditorTabManager::default();
    assert_eq!(tabs.ui.active_scene, Some("Main Scene".to_string()));
}
