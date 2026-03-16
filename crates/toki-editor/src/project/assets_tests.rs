
use super::ProjectAssets;
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
