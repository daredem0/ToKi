use crate::assets::atlas::ColorMode;
use crate::graphics::image::{load_image_rgba8, save_image_rgba8};
use crate::palette::{save_palette_asset_to_path, validate_indexed_rgba8, Palette, PaletteSize};
use crate::project_assets::{
    classify_sprite_metadata_file, discover_audio_files, discover_palette_assets,
    discover_project_scene_paths, discover_sprite_metadata, load_entity_definition_from_path,
    load_project_palettes, load_scene_from_path, normalize_asset_name,
    resolve_project_resource_paths, resolve_tilemap_tileset_path, scene_file_path,
    tilemap_file_path, ProjectAudioFormat, SpriteMetadataFileKind,
};
use std::fs;

// ============================================================================
// normalize_asset_name tests
// ============================================================================

#[test]
fn normalize_asset_name_strips_json_suffix() {
    assert_eq!(normalize_asset_name("terrain.json"), "terrain");
}

#[test]
fn normalize_asset_name_preserves_name_without_suffix() {
    assert_eq!(normalize_asset_name("terrain"), "terrain");
}

#[test]
fn normalize_asset_name_only_strips_final_json_suffix() {
    assert_eq!(normalize_asset_name("data.json.backup"), "data.json.backup");
}

#[test]
fn normalize_asset_name_handles_empty_string() {
    assert_eq!(normalize_asset_name(""), "");
}

#[test]
fn normalize_asset_name_handles_just_json() {
    assert_eq!(normalize_asset_name(".json"), "");
}

#[test]
fn normalize_asset_name_case_sensitive() {
    // .JSON is not stripped (only lowercase .json is)
    assert_eq!(normalize_asset_name("terrain.JSON"), "terrain.JSON");
}

// ============================================================================
// Existing tests
// ============================================================================

#[test]
fn classify_sprite_metadata_file_distinguishes_atlases_and_object_sheets() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let atlas_path = tmp.path().join("players.json");
    let object_sheet_path = tmp.path().join("items.json");

    fs::write(
        &atlas_path,
        r#"{
            "image": "players.png",
            "tile_size": [16, 16],
            "tiles": {"idle": {"position": [0, 0]}}
        }"#,
    )
    .expect("write atlas");
    fs::write(
        &object_sheet_path,
        r#"{
            "sheet_type": "objects",
            "image": "items.png",
            "tile_size": [16, 16],
            "objects": {"coin": {"position": [0, 0], "size_tiles": [1, 1]}}
        }"#,
    )
    .expect("write object sheet");

    assert_eq!(
        classify_sprite_metadata_file(&atlas_path).expect("classify atlas"),
        SpriteMetadataFileKind::Atlas
    );
    assert_eq!(
        classify_sprite_metadata_file(&object_sheet_path).expect("classify object sheet"),
        SpriteMetadataFileKind::ObjectSheet
    );
}

#[test]
fn classify_sprite_metadata_file_accepts_palette_indexed_atlas_metadata() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let atlas_path = tmp.path().join("players.json");

    fs::write(
        &atlas_path,
        r#"{
            "image": "players.png",
            "tile_size": [16, 16],
            "color_mode": "palette_indexed",
            "palette": "gb_default",
            "tiles": {
                "idle": {
                    "position": [0, 0],
                    "properties": { "solid": false, "trigger": false }
                }
            }
        }"#,
    )
    .expect("write atlas");

    assert_eq!(
        classify_sprite_metadata_file(&atlas_path).expect("classify atlas"),
        SpriteMetadataFileKind::Atlas
    );

    let parsed = crate::assets::atlas::AtlasMeta::load_from_file(&atlas_path).expect("load atlas");
    assert_eq!(parsed.color_mode, ColorMode::PaletteIndexed);
    assert_eq!(parsed.palette.as_deref(), Some("gb_default"));
}

#[test]
fn discover_audio_files_returns_supported_formats_sorted() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(tmp.path().join("b.wav"), "").expect("wav");
    fs::write(tmp.path().join("a.ogg"), "").expect("ogg");
    fs::write(tmp.path().join("c.mp3"), "").expect("mp3");
    fs::write(tmp.path().join("notes.txt"), "").expect("txt");

    let assets = discover_audio_files(tmp.path()).expect("discover");
    let names = assets
        .iter()
        .map(|asset| asset.name.clone())
        .collect::<Vec<_>>();
    let formats = assets.iter().map(|asset| asset.format).collect::<Vec<_>>();

    assert_eq!(
        names,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
    assert_eq!(
        formats,
        vec![
            ProjectAudioFormat::Ogg,
            ProjectAudioFormat::Wav,
            ProjectAudioFormat::Mp3
        ]
    );
}

#[test]
fn discover_palette_assets_loads_palette_json_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    save_palette_asset_to_path(
        &tmp.path().join("forest.json"),
        &Palette::new(
            PaletteSize::Pal4,
            vec![
                [1, 2, 3, 255],
                [4, 5, 6, 255],
                [7, 8, 9, 255],
                [10, 11, 12, 255],
            ],
        )
        .unwrap(),
    )
    .expect("save palette");

    let palettes = discover_palette_assets(tmp.path()).expect("discover palettes");

    assert_eq!(palettes.len(), 1);
    assert_eq!(palettes[0].name, "forest");
    assert_eq!(palettes[0].palette.color(0), [1, 2, 3, 255]);
}

#[test]
fn load_project_palettes_reads_palette_folder() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let palette_dir = tmp.path().join("palettes");
    fs::create_dir_all(&palette_dir).expect("palette dir");
    save_palette_asset_to_path(
        &palette_dir.join("swamp.json"),
        &Palette::new(
            PaletteSize::Pal4,
            vec![
                [8, 24, 8, 255],
                [32, 72, 24, 255],
                [96, 144, 56, 255],
                [184, 216, 104, 255],
            ],
        )
        .unwrap(),
    )
    .expect("save palette");

    let palettes = load_project_palettes(tmp.path()).expect("load project palettes");

    assert_eq!(palettes.len(), 1);
    assert!(palettes.contains_key("swamp"));
}

#[test]
fn discover_sprite_metadata_splits_atlases_and_object_sheets() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(
        tmp.path().join("players.json"),
        r#"{
            "image": "players.png",
            "tile_size": [16, 16],
            "tiles": {"idle": {"position": [0, 0]}}
        }"#,
    )
    .expect("atlas");
    fs::write(
        tmp.path().join("items.json"),
        r#"{
            "sheet_type": "objects",
            "image": "items.png",
            "tile_size": [16, 16],
            "objects": {"coin": {"position": [0, 0], "size_tiles": [1, 1]}}
        }"#,
    )
    .expect("object sheet");

    let discovered = discover_sprite_metadata(tmp.path()).expect("discover");

    assert_eq!(discovered.sprite_atlas_paths.len(), 1);
    assert_eq!(discovered.object_sheet_paths.len(), 1);
}

#[test]
fn resolve_project_resource_paths_discovers_project_assets() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path();
    fs::create_dir_all(project.join("assets/sprites")).expect("sprites");
    fs::create_dir_all(project.join("assets/tilemaps")).expect("tilemaps");
    fs::create_dir_all(project.join("assets/tilesets")).expect("tilesets");

    fs::write(
        project.join("assets/sprites/players.json"),
        r#"{
            "image": "players.png",
            "tile_size": [16, 16],
            "tiles": {"idle": {"position": [0, 0]}}
        }"#,
    )
    .expect("players atlas");
    fs::write(project.join("assets/sprites/players.png"), "png").expect("players png");
    fs::write(
        project.join("assets/sprites/items.json"),
        r#"{
            "sheet_type": "objects",
            "image": "items.png",
            "tile_size": [16, 16],
            "objects": {"coin": {"position": [0, 0], "size_tiles": [1, 1]}}
        }"#,
    )
    .expect("items sheet");
    fs::write(project.join("assets/sprites/items.png"), "png").expect("items png");
    fs::write(
        project.join("assets/tilemaps/terrain.json"),
        r#"{
            "image": "terrain.png",
            "tile_size": [16, 16],
            "tiles": {"grass": {"position": [0, 0]}}
        }"#,
    )
    .expect("terrain atlas");
    fs::write(project.join("assets/tilemaps/terrain.png"), "png").expect("terrain png");
    fs::write(
        project.join("assets/tilesets/demo_map.json"),
        r#"{
            "tile_size": [16, 16],
            "entries": {
                "terrain/tile/grass": {
                    "atlas_name": "terrain.json",
                    "kind": "tile",
                    "source_name": "grass"
                }
            }
        }"#,
    )
    .expect("terrain tileset");
    fs::write(
        project.join("assets/tilemaps/demo_map.json"),
        r#"{
            "size": [1, 1],
            "tile_size": [16, 16],
            "tileset": "demo_map.json",
            "tiles": ["terrain/tile/grass"]
        }"#,
    )
    .expect("tilemap");

    let resolved =
        resolve_project_resource_paths(project, Some("demo_map")).expect("resolve project");

    assert_eq!(
        resolved
            .tilemap_path
            .file_name()
            .and_then(|name| name.to_str()),
        Some("demo_map.json")
    );
    assert_eq!(resolved.sprite_atlas_paths.len(), 1);
    assert_eq!(resolved.object_sheet_paths.len(), 1);
}

#[test]
fn scene_file_path_returns_canonical_path() {
    let project = std::path::Path::new("/projects/my_game");
    let path = scene_file_path(project, "Main Scene");
    assert_eq!(path, project.join("scenes").join("Main Scene.json"));
}

#[test]
fn tilemap_file_path_returns_canonical_path() {
    let project = std::path::Path::new("/projects/my_game");
    let path = tilemap_file_path(project, "Level 1");
    assert_eq!(
        path,
        project.join("assets").join("tilemaps").join("Level 1.json")
    );
}

#[test]
fn resolve_tilemap_tileset_path_prefers_assets_tilesets_over_tilemap_dir_name_collision() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join("assets").join("tilemaps")).expect("tilemaps dir");
    std::fs::create_dir_all(project.join("assets").join("tilesets")).expect("tilesets dir");

    let tilemap_path = project
        .join("assets")
        .join("tilemaps")
        .join("MapEditor_Test_B.json");
    std::fs::write(&tilemap_path, "{}").expect("tilemap placeholder");

    let tileset_path = project
        .join("assets")
        .join("tilesets")
        .join("MapEditor_Test_B.json");
    std::fs::write(&tileset_path, "{}").expect("tileset placeholder");

    let tilemap = crate::assets::tilemap::TileMap {
        size: glam::UVec2::new(1, 1),
        tile_size: glam::UVec2::new(8, 8),
        tileset: std::path::PathBuf::from("MapEditor_Test_B.json"),
        layers: vec![crate::assets::tilemap::TileLayer::new(
            "ground",
            vec!["terrain/tile/brick".to_string()],
        )],
    };

    let resolved = resolve_tilemap_tileset_path(&project, &tilemap_path, &tilemap)
        .expect("tileset path should resolve");
    assert_eq!(resolved, tileset_path);
}

#[test]
fn load_scene_from_path_reads_scene_json() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let scene_path = tmp.path().join("Main Scene.json");
    fs::write(
        &scene_path,
        r#"{
  "name": "Main Scene",
  "description": null,
  "maps": [],
  "entities": [],
  "anchors": [],
  "player_entry": null,
  "rules": {
    "chains": []
  },
  "camera_position": null,
  "camera_scale": null,
  "background_music_track_id": null
}"#,
    )
    .expect("write scene");

    let scene = load_scene_from_path(&scene_path).expect("load scene");
    assert_eq!(scene.name, "Main Scene");
}

#[test]
fn load_scene_from_path_migrates_legacy_tilemap_objects_into_decorations() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project_root = tmp.path().join("project");
    let scenes_dir = project_root.join("scenes");
    let tilemaps_dir = project_root.join("assets").join("tilemaps");
    fs::create_dir_all(&scenes_dir).expect("scenes dir");
    fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");

    let scene_path = scenes_dir.join("Main Scene.json");
    fs::write(
        &scene_path,
        r#"{
  "name": "Main Scene",
  "description": null,
  "maps": ["level_1"],
  "entities": [],
  "anchors": [],
  "player_entry": null,
  "rules": { "chains": [] },
  "camera_position": null,
  "camera_scale": null,
  "background_music_track_id": null
}"#,
    )
    .expect("write scene");
    fs::write(
        tilemaps_dir.join("level_1.json"),
        r#"{
  "size": [4, 4],
  "tile_size": [16, 16],
  "atlas": "terrain.json",
  "tiles": [
    "grass", "grass", "grass", "grass",
    "grass", "grass", "grass", "grass",
    "grass", "grass", "grass", "grass",
    "grass", "grass", "grass", "grass"
  ],
  "objects": [
    {
      "sheet": "fauna.json",
      "object_name": "bush",
      "position": [32, 48],
      "size_px": [16, 24],
      "grounding": {
        "origin": [8, 23],
        "footprint": { "offset": [4, 16], "size": [8, 8] }
      },
      "visible": true,
      "solid": true
    }
  ]
}"#,
    )
    .expect("write tilemap");

    let scene = load_scene_from_path(&scene_path).expect("scene should load");
    assert_eq!(scene.entities().len(), 1);

    let entity = &scene.entities()[0];
    assert_eq!(entity.entity_kind, crate::entity::EntityKind::Decoration);
    assert_eq!(entity.position, glam::IVec2::new(32, 48));
    assert_eq!(entity.size, glam::UVec2::new(16, 24));
    assert_eq!(entity.rendering.grounding.origin, Some([8, 23]));
    assert_eq!(
        entity.rendering.grounding.footprint,
        Some(crate::entity::EntityFootprint::new([4, 16], [8, 8]))
    );
    assert_eq!(
        entity.rendering.static_object_render,
        Some(crate::entity::StaticObjectRenderDef {
            sheet: "fauna".to_string(),
            object_name: "bush".to_string(),
        })
    );
    let collision_box = entity
        .collision_box
        .as_ref()
        .expect("migrated solid decoration should have a collision box");
    assert_eq!(collision_box.offset, glam::IVec2::new(4, 16));
    assert_eq!(collision_box.size, glam::UVec2::new(8, 8));
    assert!(!collision_box.trigger);
}

#[test]
fn load_entity_definition_from_path_reads_definition_json() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let entity_path = tmp.path().join("slime.json");
    let definition = crate::entity::EntityDefinition {
        name: "slime".to_string().into(),
        display_name: "Slime".to_string(),
        description: String::new(),
        rendering: crate::entity::RenderingDef {
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
        components: crate::entity::ComponentsDef {
            movement: Some(crate::entity::MovementComponent {
                speed: 1.0,
                movement_profile: crate::entity::MovementProfile::LegacyDefault,
                can_move: true,
            }),
            combat: Some(crate::entity::CombatComponent {
                health: Some(1),
                stats: crate::entity::EntityStats::from_legacy_health(Some(1)),
            }),
            primary_projectile: None,
            pickup: None,
            inventory: None,
            ..Default::default()
        },
        collision: crate::entity::CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: crate::entity::AudioDef {
            footstep_trigger_distance: 0.0,
            hearing_radius: 192,
            movement_sound_trigger: crate::entity::MovementSoundTrigger::Distance,
            movement_sound: String::new(),
            collision_sound: None,
        },
        animations: crate::entity::AnimationsDef {
            atlas_name: "slimes".to_string(),
            clips: vec![crate::entity::AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["slime/idle_0".to_string()],
                frame_positions: None,
                frame_duration_ms: 100.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle".to_string(),
        },
        category: "enemy".to_string(),
        tags: Vec::new(),
    };
    fs::write(
        &entity_path,
        serde_json::to_string_pretty(&definition).expect("serialize entity"),
    )
    .expect("write entity");

    let definition = load_entity_definition_from_path(&entity_path).expect("load entity");
    assert_eq!(definition.name, "slime");
}

#[test]
fn discover_project_scene_paths_falls_back_when_manifest_paths_are_stale() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path();
    fs::create_dir_all(project.join("scenes")).expect("scenes dir");
    fs::write(
        project.join("project.toml"),
        "[scenes]\n\"Main Scene\" = \"scenes/mainscene.json\"\n",
    )
    .expect("project");
    fs::write(project.join("scenes").join("Main Scene.json"), "{}").expect("scene");

    let discovered = discover_project_scene_paths(project).expect("discover scenes");

    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].0, "Main Scene");
    assert_eq!(
        discovered[0].1,
        project.join("scenes").join("Main Scene.json")
    );
}

#[test]
fn discover_project_scene_paths_includes_unlisted_scene_files_alongside_manifest_entries() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path();
    fs::create_dir_all(project.join("scenes")).expect("scenes dir");
    fs::write(
        project.join("project.toml"),
        "[scenes]\nMain = \"scenes/Main.json\"\n",
    )
    .expect("project");
    fs::write(project.join("scenes").join("Main.json"), "{}").expect("main scene");
    fs::write(project.join("scenes").join("Extra.json"), "{}").expect("extra scene");

    let discovered = discover_project_scene_paths(project).expect("discover scenes");

    assert_eq!(discovered.len(), 2);
    assert_eq!(
        discovered[0],
        (
            "Extra".to_string(),
            project.join("scenes").join("Extra.json")
        )
    );
    assert_eq!(
        discovered[1],
        ("Main".to_string(), project.join("scenes").join("Main.json"))
    );
}

#[test]
fn palette_demo_project_assets_parse_and_indexed_source_is_valid() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let project_root = temp_dir.path();
    fs::create_dir_all(project_root.join("assets").join("sprites")).expect("sprites dir");
    fs::create_dir_all(project_root.join("assets").join("tilemaps")).expect("tilemaps dir");
    fs::create_dir_all(project_root.join("assets").join("tilesets")).expect("tilesets dir");
    fs::create_dir_all(project_root.join("assets").join("audio")).expect("audio dir");
    fs::create_dir_all(project_root.join("entities")).expect("entities dir");
    fs::create_dir_all(project_root.join("palettes")).expect("palettes dir");
    fs::create_dir_all(project_root.join("scenes")).expect("scenes dir");

    fs::write(
        project_root.join("project.toml"),
        r#"[project]
name = "TestPalette"
version = "1.0.0"
created = "2026-03-22T20:30:00Z"
modified = "2026-03-22T21:29:07.851765198Z"
toki_editor_version = "0.1.0"
description = "Minimal palette-indexed rendering example."

[scenes]
"Main Scene" = "scenes/Main Scene.json"

[assets]
sprites = "assets/sprites/"
tilemaps = "assets/tilemaps/"
audio = "assets/audio/"

[runtime.display]
indexed_palette_override = "gb_default"
"#,
    )
    .expect("project metadata write");

    save_palette_asset_to_path(
        &project_root.join("palettes").join("sunset.json"),
        &Palette::new(
            PaletteSize::Pal4,
            vec![
                [36, 18, 18, 255],
                [120, 52, 36, 255],
                [224, 120, 56, 255],
                [255, 220, 140, 255],
            ],
        )
        .unwrap(),
    )
    .expect("palette write");

    fs::write(
        project_root
            .join("assets")
            .join("sprites")
            .join("indexed_demo.json"),
        r#"{
  "image": "indexed_demo.png",
  "tile_size": [16, 16],
  "color_mode": "palette_indexed",
  "palette": "gb_default",
  "tiles": {
    "hero/idle": {
      "position": [0, 0],
      "properties": {
        "solid": false,
        "trigger": false
      }
    },
    "guide/idle": {
      "position": [0, 0],
      "properties": {
        "solid": false,
        "trigger": false
      }
    }
  }
}"#,
    )
    .expect("indexed atlas");
    fs::write(
        project_root
            .join("assets")
            .join("sprites")
            .join("truecolor_demo.json"),
        r#"{
  "image": "truecolor_demo.png",
  "tile_size": [16, 16],
  "color_mode": "truecolor",
  "tiles": {
    "flower/idle": {
      "position": [0, 0],
      "properties": {
        "solid": false,
        "trigger": false
      }
    }
  }
}"#,
    )
    .expect("truecolor atlas");
    fs::write(
        project_root
            .join("assets")
            .join("tilemaps")
            .join("terrain.json"),
        r#"{
  "image": "terrain.png",
  "tile_size": [16, 16],
  "tiles": {
    "grass": {
      "position": [0, 0],
      "properties": {
        "solid": false,
        "trigger": false
      }
    }
  }
}"#,
    )
    .expect("terrain atlas");
    fs::write(
        project_root
            .join("assets")
            .join("tilesets")
            .join("palette_demo_map.json"),
        r#"{
  "tile_size": [16, 16],
  "entries": {
    "terrain/tile/grass": {
      "atlas_name": "terrain.json",
      "kind": "tile",
      "source_name": "grass"
    }
  }
}"#,
    )
    .expect("terrain tileset");
    fs::write(
        project_root
            .join("assets")
            .join("tilemaps")
            .join("palette_demo_map.json"),
        r#"{
  "size": [10, 9],
  "tile_size": [16, 16],
  "tileset": "palette_demo_map.json",
  "tiles": [
    "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass",
    "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass",
    "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass",
    "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass",
    "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass",
    "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass",
    "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass",
    "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass",
    "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass", "terrain/tile/grass"
  ]
}"#,
    )
    .expect("tilemap");
    fs::write(
        project_root.join("entities").join("palette_player.json"),
        r#"{
  "name": "palette_player",
  "display_name": "Palette Player",
  "description": "Indexed player sprite using the atlas default palette.",
  "rendering": {
    "size": [16, 16],
    "render_layer": 1,
    "visible": true,
    "has_shadow": true
  },
  "solid": false,
  "active": true,
  "components": {
    "movement": {
      "speed": 0.0,
      "movement_profile": "none",
      "can_move": false
    },
    "combat": {
      "health": 10,
      "stats": { "base": { "health": 10 }, "current": { "health": 10 } }
    }
  },
  "collision": {
    "enabled": false,
    "offset": [0, 0],
    "size": [16, 16],
    "trigger": false
  },
  "audio": {
    "footstep_trigger_distance": 16.0,
    "hearing_radius": 192,
    "movement_sound_trigger": "distance",
    "movement_sound": "",
    "collision_sound": null
  },
  "animations": {
    "atlas_name": "indexed_demo.json",
    "clips": [
      {
        "state": "idle",
        "frame_tiles": ["hero/idle"],
        "frame_duration_ms": 300.0,
        "loop_mode": "loop"
      }
    ],
    "default_state": "idle"
  },
  "category": "human",
  "tags": ["palette_demo", "player"]
}"#,
    )
    .expect("palette player definition");
    fs::write(
        project_root.join("entities").join("palette_guide.json"),
        r#"{
  "name": "palette_guide",
  "display_name": "Palette Guide",
  "description": "Indexed NPC using a per-definition palette override.",
  "rendering": {
    "size": [16, 16],
    "render_layer": 1,
    "visible": true,
    "has_shadow": true,
    "palette_override": "night"
  },
  "solid": false,
  "active": true,
  "components": {
    "movement": {
      "speed": 0.0,
      "movement_profile": "none",
      "can_move": false
    },
    "combat": {
      "health": 10,
      "stats": { "base": { "health": 10 }, "current": { "health": 10 } }
    }
  },
  "collision": {
    "enabled": false,
    "offset": [0, 0],
    "size": [16, 16],
    "trigger": false
  },
  "audio": {
    "footstep_trigger_distance": 16.0,
    "hearing_radius": 192,
    "movement_sound_trigger": "distance",
    "movement_sound": "",
    "collision_sound": null
  },
  "animations": {
    "atlas_name": "indexed_demo.json",
    "clips": [
      {
        "state": "idle",
        "frame_tiles": ["guide/idle"],
        "frame_duration_ms": 300.0,
        "loop_mode": "loop"
      }
    ],
    "default_state": "idle"
  },
  "category": "human",
  "tags": ["palette_demo", "guide"]
}"#,
    )
    .expect("palette guide definition");
    fs::write(
        project_root.join("entities").join("truecolor_flower.json"),
        r#"{
  "name": "truecolor_flower",
  "display_name": "Flower",
  "description": "True-color control sprite unaffected by indexed palette overrides.",
  "rendering": {
    "size": [16, 16],
    "render_layer": 1,
    "visible": true,
    "has_shadow": true
  },
  "solid": false,
  "active": true,
  "components": {
    "movement": {
      "speed": 0.0,
      "movement_profile": "none",
      "can_move": false
    }
  },
  "collision": {
    "enabled": false,
    "offset": [0, 0],
    "size": [16, 16],
    "trigger": false
  },
  "audio": {
    "footstep_trigger_distance": 16.0,
    "hearing_radius": 192,
    "movement_sound_trigger": "distance",
    "movement_sound": "",
    "collision_sound": null
  },
  "animations": {
    "atlas_name": "truecolor_demo.json",
    "clips": [
      {
        "state": "idle",
        "frame_tiles": ["flower/idle"],
        "frame_duration_ms": 300.0,
        "loop_mode": "loop"
      }
    ],
    "default_state": "idle"
  },
  "category": "flora",
  "tags": ["palette_demo", "truecolor_control"]
}"#,
    )
    .expect("truecolor definition");
    fs::write(
        project_root.join("scenes").join("Main Scene.json"),
        r#"{
  "name": "Main Scene",
  "description": "Palette rendering demo scene.",
  "maps": ["palette_demo_map"],
  "entities": [
    {
      "id": 2,
      "position": [72, 64],
      "size": [16, 16],
      "entity_kind": "Npc",
      "category": "palette_demo",
      "definition_name": "palette_guide",
      "control_role": "none",
      "audio": {
        "footstep_trigger_distance": 16.0,
        "hearing_radius": 192,
        "movement_sound_trigger": "distance",
        "movement_sound": null,
        "collision_sound": null
      },
      "rendering": {
        "visible": true,
        "has_shadow": true,
        "palette_override": "night",
        "animation_controller": {
          "clips": {
            "Idle": {
              "state": "Idle",
              "atlas_name": "indexed_demo.json",
              "frame_tile_names": ["guide/idle"],
              "frame_duration_ms": 300.0,
              "loop_mode": "Loop"
            }
          },
          "current_clip_state": "Idle",
          "current_frame_index": 0,
          "frame_timer": 0.0,
          "is_finished": false
        },
        "render_layer": 1,
        "grounding": {}
      },
      "collision_box": null,
      "solid": false,
      "active": true,
      "tags": ["palette_demo", "indexed_override"],
      "components": {
        "movement": {
          "speed": 0.0,
          "movement_profile": "none",
          "can_move": false
        },
        "combat": {
          "health": 10,
          "stats": { "base": { "health": 10 }, "current": { "health": 10 } }
        }
      }
    },
    {
      "id": 3,
      "position": [112, 64],
      "size": [16, 16],
      "entity_kind": "Decoration",
      "category": "palette_demo",
      "definition_name": "truecolor_flower",
      "control_role": "none",
      "audio": {
        "footstep_trigger_distance": 16.0,
        "hearing_radius": 192,
        "movement_sound_trigger": "distance",
        "movement_sound": null,
        "collision_sound": null
      },
      "rendering": {
        "visible": true,
        "has_shadow": true,
        "animation_controller": {
          "clips": {
            "Idle": {
              "state": "Idle",
              "atlas_name": "truecolor_demo.json",
              "frame_tile_names": ["flower/idle"],
              "frame_duration_ms": 300.0,
              "loop_mode": "Loop"
            }
          },
          "current_clip_state": "Idle",
          "current_frame_index": 0,
          "frame_timer": 0.0,
          "is_finished": false
        },
        "render_layer": 1,
        "grounding": {}
      },
      "collision_box": null,
      "solid": false,
      "active": true,
      "tags": ["palette_demo", "truecolor_control"],
      "components": {
        "movement": {
          "speed": 0.0,
          "movement_profile": "none",
          "can_move": false
        }
      }
    }
  ],
  "rules": { "rules": [] },
  "camera_position": null,
  "camera_scale": null,
  "background_music_track_id": null,
  "anchors": [
    {
      "id": "spawn_a",
      "kind": "SpawnPoint",
      "position": [32, 64],
      "facing": "Down"
    }
  ],
  "player_entry": {
    "entity_definition_name": "palette_player",
    "spawn_point_id": "spawn_a"
  }
}"#,
    )
    .expect("scene");

    save_image_rgba8(
        project_root
            .join("assets")
            .join("sprites")
            .join("indexed_demo.png"),
        16,
        16,
        &[
            0x00, 0x00, 0x00, 0xFF, 0x55, 0x55, 0x55, 0xFF, 0xAA, 0xAA, 0xAA, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x55, 0x55, 0x55, 0xFF, 0xAA, 0xAA, 0xAA, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x55, 0x55, 0x55, 0xFF, 0xAA, 0xAA,
            0xAA, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0x55, 0x55, 0x55, 0xFF,
            0xAA, 0xAA, 0xAA, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ]
        .repeat(16),
    )
    .expect("indexed png");
    save_image_rgba8(
        project_root
            .join("assets")
            .join("sprites")
            .join("truecolor_demo.png"),
        16,
        16,
        &[
            [255, 64, 96, 255],
            [96, 200, 96, 255],
            [255, 220, 64, 255],
            [255, 255, 255, 255],
        ]
        .concat()
        .repeat(16 * 16 / 4),
    )
    .expect("truecolor png");
    save_image_rgba8(
        project_root
            .join("assets")
            .join("tilemaps")
            .join("terrain.png"),
        16,
        16,
        &[
            [32, 160, 64, 255],
            [48, 184, 80, 255],
            [32, 160, 64, 255],
            [48, 184, 80, 255],
        ]
        .concat()
        .repeat(16 * 16 / 4),
    )
    .expect("terrain png");

    let atlas = crate::assets::atlas::AtlasMeta::load_from_file(
        project_root
            .join("assets")
            .join("sprites")
            .join("indexed_demo.json"),
    )
    .expect("indexed atlas should parse");
    assert_eq!(atlas.color_mode, ColorMode::PaletteIndexed);
    assert_eq!(atlas.palette.as_deref(), Some("gb_default"));

    let image = load_image_rgba8(
        project_root
            .join("assets")
            .join("sprites")
            .join("indexed_demo.png"),
    )
    .expect("indexed image should load");
    let validation = validate_indexed_rgba8(&image.data, PaletteSize::Pal4);
    assert!(
        validation.is_valid(),
        "indexed example should only use canonical shades, got invalid colors: {:?}",
        validation.invalid_colors
    );

    let scene = load_scene_from_path(&project_root.join("scenes").join("Main Scene.json"))
        .expect("scene should parse");
    assert_eq!(scene.name, "Main Scene");
    assert_eq!(scene.maps, vec!["palette_demo_map".to_string()]);
    assert_eq!(scene.entities().len(), 2);
    assert_eq!(
        scene
            .player_entry
            .as_ref()
            .map(|entry| entry.entity_definition_name.as_str()),
        Some("palette_player")
    );

    let definition = load_entity_definition_from_path(
        &project_root.join("entities").join("palette_player.json"),
    )
    .expect("palette player definition should parse");
    assert_eq!(definition.animations.atlas_name, "indexed_demo.json");

    let project_palettes = load_project_palettes(project_root).expect("project palettes");
    assert!(project_palettes.contains_key("sunset"));

    let guide_definition =
        load_entity_definition_from_path(&project_root.join("entities").join("palette_guide.json"))
            .expect("palette guide definition should parse");
    assert_eq!(
        guide_definition.rendering.palette_override.as_deref(),
        Some("night")
    );

    let truecolor_definition = load_entity_definition_from_path(
        &project_root.join("entities").join("truecolor_flower.json"),
    )
    .expect("truecolor flower definition should parse");
    assert_eq!(
        truecolor_definition.animations.atlas_name,
        "truecolor_demo.json"
    );

    let resolved = resolve_project_resource_paths(project_root, Some("palette_demo_map"))
        .expect("example project should resolve runtime resource paths");
    assert_eq!(
        resolved
            .tilemap_path
            .file_name()
            .and_then(|name| name.to_str()),
        Some("palette_demo_map.json")
    );
    assert_eq!(
        resolved
            .tileset_path
            .file_name()
            .and_then(|name| name.to_str()),
        Some("palette_demo_map.json")
    );
    assert!(resolved
        .tileset_atlas_paths
        .iter()
        .any(|path| path.ends_with("terrain.json")));
    assert!(resolved.sprite_texture_path.is_some());
}
