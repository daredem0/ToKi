use super::{
    classify_sprite_metadata_file, find_first_json_file, first_existing_path,
    resolve_project_resource_paths, resolve_tilemap_atlas_path, ResourceManager,
    SpriteMetadataFileKind,
};
use std::fs;
use std::path::PathBuf;
use toki_core::assets::tilemap::TileMap;

fn make_unique_temp_dir() -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("toki_runtime_resources_tests_{nanos}"));
    std::fs::create_dir_all(&dir).expect("temp dir should be created");
    dir
}

fn write_minimal_atlas(path: &std::path::Path, image_name: &str) {
    let content = format!(
        r#"{{
  "image": "{image_name}",
  "tile_size": [16, 16],
  "tiles": {{
    "floor": {{
      "position": [0, 0],
      "properties": {{
        "solid": false
      }}
    }}
  }}
}}"#
    );
    fs::write(path, content).expect("atlas write");
}

fn write_minimal_map(path: &std::path::Path, atlas_ref: &str) {
    let content = format!(
        r#"{{
  "size": [1, 1],
  "tile_size": [16, 16],
  "atlas": "{atlas_ref}",
  "tiles": ["floor"]
}}"#
    );
    fs::write(path, content).expect("map write");
}

#[test]
fn first_existing_path_picks_first_existing_candidate() {
    let dir = make_unique_temp_dir();
    let missing = dir.join("missing.json");
    let first = dir.join("a.json");
    let second = dir.join("b.json");
    fs::write(&first, "{}").expect("first write");
    fs::write(&second, "{}").expect("second write");

    let resolved = first_existing_path(&[missing, first.clone(), second]);
    assert_eq!(resolved, Some(first));
}

#[test]
fn find_first_json_file_returns_sorted_first_json_entry() {
    let dir = make_unique_temp_dir();
    fs::create_dir_all(&dir).expect("dir");
    fs::write(dir.join("z_map.json"), "{}").expect("z map");
    fs::write(dir.join("a_map.json"), "{}").expect("a map");
    fs::write(dir.join("note.txt"), "ignore").expect("txt");

    let first = find_first_json_file(&dir)
        .expect("json file lookup should succeed")
        .expect("json file should be found");
    assert_eq!(
        first.file_name().and_then(|name| name.to_str()),
        Some("a_map.json")
    );
}

#[test]
fn resolve_tilemap_atlas_path_prefers_map_directory_relative_atlas() {
    let project_dir = make_unique_temp_dir();
    let tilemaps_dir = project_dir.join("assets").join("tilemaps");
    fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");
    let tilemap_path = tilemaps_dir.join("main_map.json");
    let atlas_path = tilemaps_dir.join("terrain.json");
    fs::write(&tilemap_path, "{}").expect("tilemap file");
    fs::write(&atlas_path, "{}").expect("atlas file");

    let tilemap = TileMap {
        size: glam::UVec2::new(1, 1),
        tile_size: glam::UVec2::new(16, 16),
        atlas: PathBuf::from("terrain.json"),
        tiles: vec!["floor".to_string()],
        objects: vec![],
    };

    let resolved = resolve_tilemap_atlas_path(&project_dir, &tilemap_path, &tilemap)
        .expect("atlas should resolve");
    assert_eq!(resolved, atlas_path);
}

#[test]
fn resolve_tilemap_atlas_path_falls_back_to_project_sprites_dir() {
    let project_dir = make_unique_temp_dir();
    let tilemaps_dir = project_dir.join("assets").join("tilemaps");
    let sprites_dir = project_dir.join("assets").join("sprites");
    fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");
    fs::create_dir_all(&sprites_dir).expect("sprites dir");
    let tilemap_path = tilemaps_dir.join("main_map.json");
    let sprites_atlas = sprites_dir.join("terrain.json");
    fs::write(&tilemap_path, "{}").expect("tilemap file");
    fs::write(&sprites_atlas, "{}").expect("sprites atlas");

    let tilemap = TileMap {
        size: glam::UVec2::new(1, 1),
        tile_size: glam::UVec2::new(16, 16),
        atlas: PathBuf::from("terrain.json"),
        tiles: vec!["floor".to_string()],
        objects: vec![],
    };

    let resolved = resolve_tilemap_atlas_path(&project_dir, &tilemap_path, &tilemap)
        .expect("atlas should resolve from sprites dir");
    assert_eq!(resolved, sprites_atlas);
}

#[test]
fn load_for_project_with_named_map_loads_resources() {
    let project_dir = make_unique_temp_dir();
    let sprites_dir = project_dir.join("assets").join("sprites");
    let tilemaps_dir = project_dir.join("assets").join("tilemaps");
    fs::create_dir_all(&sprites_dir).expect("sprites dir");
    fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");

    write_minimal_atlas(&sprites_dir.join("creatures.json"), "creatures.png");
    write_minimal_atlas(&tilemaps_dir.join("terrain.json"), "terrain.png");
    write_minimal_map(&tilemaps_dir.join("demo_map.json"), "terrain.json");

    let manager = ResourceManager::load_for_project(&project_dir, Some("demo_map"))
        .expect("project resources should load");
    assert_eq!(manager.tilemap_size(), glam::UVec2::new(1, 1));
    assert_eq!(manager.tilemap_tile_size(), glam::UVec2::new(16, 16));
    assert_eq!(manager.terrain_tile_size(), glam::UVec2::new(16, 16));
    assert_eq!(manager.creature_tile_size(), glam::UVec2::new(16, 16));
}

#[test]
fn load_for_project_without_map_name_discovers_first_tilemap() {
    let project_dir = make_unique_temp_dir();
    let sprites_dir = project_dir.join("assets").join("sprites");
    let tilemaps_dir = project_dir.join("assets").join("tilemaps");
    fs::create_dir_all(&sprites_dir).expect("sprites dir");
    fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");

    write_minimal_atlas(&sprites_dir.join("creatures.json"), "creatures.png");
    write_minimal_atlas(&tilemaps_dir.join("terrain.json"), "terrain.png");
    write_minimal_map(&tilemaps_dir.join("b_map.json"), "terrain.json");
    let a_map = r#"{
  "size": [2, 1],
  "tile_size": [16, 16],
  "atlas": "terrain.json",
  "tiles": ["floor", "floor"]
}"#;
    fs::write(tilemaps_dir.join("a_map.json"), a_map).expect("a_map write");

    let manager =
        ResourceManager::load_for_project(&project_dir, None).expect("resources should load");
    assert_eq!(
        manager.tilemap_size(),
        glam::UVec2::new(2, 1),
        "alphabetically first discovered map should be selected"
    );
}

#[test]
fn load_for_project_errors_when_no_sprite_atlas_exists() {
    let project_dir = make_unique_temp_dir();
    let tilemaps_dir = project_dir.join("assets").join("tilemaps");
    fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");
    write_minimal_atlas(&tilemaps_dir.join("terrain.json"), "terrain.png");
    write_minimal_map(&tilemaps_dir.join("demo_map.json"), "terrain.json");

    let error = ResourceManager::load_for_project(&project_dir, Some("demo_map"))
        .expect_err("missing sprite atlas should fail");
    assert!(
        error
            .to_string()
            .contains("Could not find any sprite atlas"),
        "unexpected error: {error}"
    );
}

#[test]
fn load_for_project_registers_sprite_atlas_by_filename_and_stem() {
    let project_dir = make_unique_temp_dir();
    let sprites_dir = project_dir.join("assets").join("sprites");
    let tilemaps_dir = project_dir.join("assets").join("tilemaps");
    fs::create_dir_all(&sprites_dir).expect("sprites dir");
    fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");

    write_minimal_atlas(&sprites_dir.join("players.json"), "player.png");
    write_minimal_atlas(&tilemaps_dir.join("terrain.json"), "terrain.png");
    write_minimal_map(&tilemaps_dir.join("demo_map.json"), "terrain.json");
    fs::write(sprites_dir.join("player.png"), "png").expect("player image");

    let manager = ResourceManager::load_for_project(&project_dir, Some("demo_map"))
        .expect("project resources should load");

    assert!(manager.get_sprite_atlas("players.json").is_some());
    assert!(manager.get_sprite_atlas("players").is_some());
    assert_eq!(
        manager.get_sprite_texture_path("players.json"),
        Some(&sprites_dir.join("player.png"))
    );
}

#[test]
fn resolve_project_resource_paths_returns_expected_texture_paths() {
    let project_dir = make_unique_temp_dir();
    let sprites_dir = project_dir.join("assets").join("sprites");
    let tilemaps_dir = project_dir.join("assets").join("tilemaps");
    fs::create_dir_all(&sprites_dir).expect("sprites dir");
    fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");

    write_minimal_atlas(&sprites_dir.join("creatures.json"), "creatures.png");
    write_minimal_atlas(&tilemaps_dir.join("terrain.json"), "terrain.png");
    write_minimal_map(&tilemaps_dir.join("demo_map.json"), "terrain.json");
    fs::write(sprites_dir.join("creatures.png"), "png").expect("creatures image");
    fs::write(tilemaps_dir.join("terrain.png"), "png").expect("terrain image");

    let resolved = resolve_project_resource_paths(&project_dir, Some("demo_map"))
        .expect("project resource paths should resolve");
    assert_eq!(
        resolved.tilemap_texture_path,
        Some(tilemaps_dir.join("terrain.png"))
    );
    assert_eq!(
        resolved.sprite_texture_path,
        Some(sprites_dir.join("creatures.png"))
    );
}

#[test]
fn classify_sprite_metadata_file_distinguishes_object_sheets_from_atlases() {
    let project_dir = make_unique_temp_dir();
    let sprites_dir = project_dir.join("assets").join("sprites");
    fs::create_dir_all(&sprites_dir).expect("sprites dir");

    let atlas_path = sprites_dir.join("creatures.json");
    write_minimal_atlas(&atlas_path, "creatures.png");
    let object_sheet_path = sprites_dir.join("fauna.json");
    fs::write(
        &object_sheet_path,
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
    .expect("object sheet should be written");

    assert_eq!(
        classify_sprite_metadata_file(&atlas_path).expect("atlas should classify"),
        SpriteMetadataFileKind::Atlas
    );
    assert_eq!(
        classify_sprite_metadata_file(&object_sheet_path).expect("object sheet should classify"),
        SpriteMetadataFileKind::ObjectSheet
    );
}

#[test]
fn resolve_project_resource_paths_ignores_object_sheets_in_sprite_registry() {
    let project_dir = make_unique_temp_dir();
    let sprites_dir = project_dir.join("assets").join("sprites");
    let tilemaps_dir = project_dir.join("assets").join("tilemaps");
    fs::create_dir_all(&sprites_dir).expect("sprites dir");
    fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");

    write_minimal_atlas(&sprites_dir.join("creatures.json"), "creatures.png");
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
    .expect("object sheet should be written");
    write_minimal_atlas(&tilemaps_dir.join("terrain.json"), "terrain.png");
    write_minimal_map(&tilemaps_dir.join("demo_map.json"), "terrain.json");
    fs::write(sprites_dir.join("creatures.png"), "png").expect("creatures image");
    fs::write(tilemaps_dir.join("terrain.png"), "png").expect("terrain image");

    let resolved = resolve_project_resource_paths(&project_dir, Some("demo_map"))
        .expect("project resource paths should resolve");

    assert_eq!(resolved.sprite_atlas_paths.len(), 1);
    assert_eq!(
        resolved.sprite_atlas_paths[0]
            .file_name()
            .and_then(|name| name.to_str()),
        Some("creatures.json")
    );
}

#[test]
fn load_for_project_registers_object_sheet_by_filename_and_stem() {
    let project_dir = make_unique_temp_dir();
    let sprites_dir = project_dir.join("assets").join("sprites");
    let tilemaps_dir = project_dir.join("assets").join("tilemaps");
    fs::create_dir_all(&sprites_dir).expect("sprites dir");
    fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");

    write_minimal_atlas(&sprites_dir.join("players.json"), "player.png");
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
    .expect("object sheet should be written");
    write_minimal_atlas(&tilemaps_dir.join("terrain.json"), "terrain.png");
    write_minimal_map(&tilemaps_dir.join("demo_map.json"), "terrain.json");
    fs::write(sprites_dir.join("player.png"), "png").expect("player image");
    fs::write(sprites_dir.join("fauna.png"), "png").expect("fauna image");

    let manager = ResourceManager::load_for_project(&project_dir, Some("demo_map"))
        .expect("project resources should load");

    assert!(manager.get_object_sheet("fauna.json").is_some());
    assert!(manager.get_object_sheet("fauna").is_some());
    assert_eq!(
        manager.get_object_texture_path("fauna.json"),
        Some(&sprites_dir.join("fauna.png"))
    );
}
