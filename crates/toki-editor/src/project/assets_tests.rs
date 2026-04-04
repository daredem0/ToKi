use super::{ProjectAssets, ProjectAudioAssetKind};
use std::fs;
use tempfile::tempdir;
use toki_core::palette::{save_palette_asset_to_path, Palette, PaletteSize};
use toki_core::ui_layout::{UiLayoutAsset, UiWidgetNode};

#[test]
fn scan_assets_discovers_atlases_and_object_sheets_separately() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let sprites_dir = temp_dir.path().join("assets/sprites");
    fs::create_dir_all(&sprites_dir).expect("sprites dir should be created");

    fs::write(
        sprites_dir.join("terrain.json"),
        r#"{
                "image": "terrain.png",
                "tile_size": [8, 8],
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
    .expect("atlas json should be written");

    fs::write(
        sprites_dir.join("fauna.json"),
        r#"{
                "sheet_type": "objects",
                "image": "fauna.png",
                "tile_size": [16, 16],
                "objects": {
                    "fauna_a": {
                        "position": [0, 0],
                        "size_tiles": [1, 1]
                    }
                }
            }"#,
    )
    .expect("object sheet json should be written");

    let mut assets = ProjectAssets::new(temp_dir.path().to_path_buf());
    assets.scan_assets().expect("asset scan should succeed");

    assert!(assets.sprite_atlases.contains_key("terrain"));
    assert!(!assets.sprite_atlases.contains_key("fauna"));
    assert!(assets.object_sheets.contains_key("fauna"));
    assert_eq!(assets.get_sprite_atlas_names(), vec!["terrain".to_string()]);
    assert_eq!(assets.get_object_sheet_names(), vec!["fauna".to_string()]);
}

#[test]
fn scan_assets_skips_unknown_sprite_metadata_files() {
    let temp_dir = tempdir().expect("tempdir should be created");
    let sprites_dir = temp_dir.path().join("assets/sprites");
    fs::create_dir_all(&sprites_dir).expect("sprites dir should be created");

    fs::write(
        sprites_dir.join("mystery.json"),
        r#"{
                "hello": "world"
            }"#,
    )
    .expect("mystery json should be written");

    let mut assets = ProjectAssets::new(temp_dir.path().to_path_buf());
    assets.scan_assets().expect("asset scan should succeed");

    assert!(assets.sprite_atlases.is_empty());
    assert!(assets.object_sheets.is_empty());
}

#[test]
fn discover_project_audio_names_reads_supported_audio_files() {
    let temp_dir = tempdir().expect("temp dir should be created");
    let music_dir = temp_dir.path().join("assets/audio/music");
    fs::create_dir_all(&music_dir).expect("music dir should be created");

    fs::write(music_dir.join("battle_theme.ogg"), "x").expect("ogg file write");
    fs::write(music_dir.join("ambience.mp3"), "x").expect("mp3 file write");
    fs::write(music_dir.join("impact.wav"), "x").expect("wav file write");
    fs::write(music_dir.join("ignore.txt"), "x").expect("txt file write");
    fs::create_dir(music_dir.join("sub")).expect("subdir create");
    fs::write(music_dir.join("sub").join("nested.ogg"), "x").expect("nested write");

    let names =
        ProjectAssets::discover_project_audio_names(temp_dir.path(), ProjectAudioAssetKind::Music);
    assert_eq!(names, vec!["ambience", "battle_theme", "impact"]);
}

#[test]
fn discover_project_entity_definition_names_reads_json_files() {
    let temp_dir = tempdir().expect("temp dir should be created");
    let entities_dir = temp_dir.path().join("entities");
    fs::create_dir_all(&entities_dir).expect("entities dir should be created");

    fs::write(entities_dir.join("player.json"), "{}").expect("player write");
    fs::write(entities_dir.join("slime.json"), "{}").expect("slime write");
    fs::write(entities_dir.join("notes.txt"), "x").expect("txt write");

    let names = ProjectAssets::discover_project_entity_definition_names(temp_dir.path());
    assert_eq!(names, vec!["player", "slime"]);
}

#[test]
fn load_project_palettes_reads_palette_json_files() {
    let temp_dir = tempdir().expect("temp dir should be created");
    let palettes_dir = temp_dir.path().join("palettes");
    fs::create_dir_all(&palettes_dir).expect("palettes dir should be created");

    save_palette_asset_to_path(
        &palettes_dir.join("forest.json"),
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
    .expect("palette write");
    save_palette_asset_to_path(
        &palettes_dir.join("night.json"),
        &Palette::new(
            PaletteSize::Pal4,
            vec![
                [11, 12, 13, 255],
                [14, 15, 16, 255],
                [17, 18, 19, 255],
                [20, 21, 22, 255],
            ],
        )
        .unwrap(),
    )
    .expect("palette write");

    let mut assets = ProjectAssets::new(temp_dir.path().to_path_buf());
    assets.scan_assets().expect("asset scan should succeed");

    let palettes = assets
        .load_project_palettes()
        .expect("palette load should succeed");
    assert_eq!(
        palettes.keys().cloned().collect::<Vec<_>>(),
        vec!["forest", "night"]
    );
}

#[test]
fn scan_assets_discovers_palette_files() {
    let temp_dir = tempdir().expect("temp dir should be created");
    let palettes_dir = temp_dir.path().join("palettes");
    fs::create_dir_all(&palettes_dir).expect("palettes dir should be created");
    save_palette_asset_to_path(
        &palettes_dir.join("swamp.json"),
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
    .expect("palette write");

    let mut assets = ProjectAssets::new(temp_dir.path().to_path_buf());
    assets.scan_assets().expect("asset scan should succeed");

    assert!(assets.palettes.contains_key("swamp"));
}

#[test]
fn ui_layout_assets_scan_save_and_load() {
    let temp_dir = tempdir().expect("temp dir should be created");
    let ui_dir = temp_dir.path().join("ui");
    fs::create_dir_all(&ui_dir).expect("ui dir should be created");

    let layout = UiLayoutAsset {
        id: "hud".into(),
        title: "HUD".to_string(),
        startup_visible: true,
        z_order: 10,
        root: UiWidgetNode::default(),
    };

    let mut assets = ProjectAssets::new(temp_dir.path().to_path_buf());
    assets
        .save_ui_layout(&layout)
        .expect("ui layout save should succeed");
    assets.scan_assets().expect("asset scan should succeed");

    assert!(assets.ui_layouts.contains_key("hud"));
    assert_eq!(assets.get_ui_layout_names(), vec!["hud".to_string()]);

    let loaded = assets
        .load_ui_layout("hud")
        .expect("ui layout load should succeed")
        .expect("ui layout should exist");
    assert_eq!(loaded.id, layout.id);
    assert_eq!(loaded.title, "HUD");
}
