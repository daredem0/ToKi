use super::{ProjectAssets, ProjectAudioAssetKind};
use std::fs;
use tempfile::tempdir;

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
