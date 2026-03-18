use crate::project_assets::{
    classify_sprite_metadata_file, discover_audio_files, discover_sprite_metadata,
    resolve_project_resource_paths, scene_file_path, tilemap_file_path, ProjectAudioFormat,
    SpriteMetadataFileKind,
};
use std::fs;

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
        project.join("assets/tilemaps/demo_map.json"),
        r#"{
            "size": [1, 1],
            "tile_size": [16, 16],
            "atlas": "terrain.json",
            "tiles": ["grass"]
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
