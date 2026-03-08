use glam::UVec2;
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use toki_core::resources::{ResourceError, ResourceManager};

fn create_test_atlas(dir: &Path, name: &str) -> std::io::Result<()> {
    let atlas_content = r#"{
  "image": "terrain.png",
  "tile_size": [16, 16],
  "tiles": {
    "grass": {
      "position": [0, 0]
    },
    "stone": {
      "position": [1, 0],
      "properties": {
        "solid": true
      }
    }
  }
}"#;
    fs::write(dir.join(format!("{}.json", name)), atlas_content)
}

fn create_test_tilemap(dir: &Path, name: &str) -> std::io::Result<()> {
    let tilemap_content = r#"{
  "size": [4, 4],
  "tile_size": [16, 16],
  "atlas": "terrain.json",
  "tiles": [
    "grass", "grass", "stone", "grass",
    "grass", "stone", "stone", "grass", 
    "stone", "grass", "grass", "stone",
    "grass", "grass", "grass", "grass"
  ]
}"#;
    fs::write(dir.join(name), tilemap_content)
}

#[test]
fn test_resource_manager_load_with_paths() {
    let temp_dir = TempDir::new().unwrap();
    let assets_dir = temp_dir.path();

    // Create test assets
    create_test_atlas(assets_dir, "terrain").unwrap();
    create_test_atlas(assets_dir, "creatures").unwrap();
    create_test_tilemap(assets_dir, "test_map.json").unwrap();

    let terrain_path = assets_dir.join("terrain.json");
    let creature_path = assets_dir.join("creatures.json");
    let tilemap_path = assets_dir.join("test_map.json");

    let result = ResourceManager::load_with_paths(&terrain_path, &creature_path, &tilemap_path);
    assert!(result.is_ok());

    let resources = result.unwrap();

    // Test atlas access
    assert_eq!(resources.terrain_tile_size(), UVec2::new(16, 16));
    assert_eq!(resources.creature_tile_size(), UVec2::new(16, 16));

    // Test tilemap access
    assert_eq!(resources.tilemap_size(), UVec2::new(4, 4));
    assert_eq!(resources.tilemap_tile_size(), UVec2::new(16, 16));

    // Test atlas references
    let terrain_atlas = resources.get_terrain_atlas();
    assert!(terrain_atlas.tiles.contains_key("grass"));
    assert!(terrain_atlas.tiles.contains_key("stone"));

    let creature_atlas = resources.get_creature_atlas();
    assert!(creature_atlas.tiles.contains_key("grass"));

    let tilemap = resources.get_tilemap();
    assert_eq!(tilemap.tiles.len(), 16); // 4x4 tiles
    assert_eq!(tilemap.tiles[0], "grass");
}

#[test]
fn test_resource_manager_load_from_project_dir() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();
    let assets_dir = project_dir.join("assets");
    let maps_dir = assets_dir.join("maps");

    // Create directory structure
    fs::create_dir_all(&maps_dir).unwrap();

    // Create test assets
    create_test_atlas(&assets_dir, "terrain").unwrap();
    create_test_atlas(&assets_dir, "creatures").unwrap();
    create_test_tilemap(&maps_dir, "new_town_map_64x64_crossings.json").unwrap();

    let result = ResourceManager::load_from_project_dir(project_dir);
    assert!(result.is_ok());

    let resources = result.unwrap();

    // Test convenience methods
    assert_eq!(resources.terrain_tile_size(), UVec2::new(16, 16));
    assert_eq!(resources.creature_tile_size(), UVec2::new(16, 16));
    assert_eq!(resources.tilemap_size(), UVec2::new(4, 4));

    // Test image size calculation
    let terrain_image_size = resources.terrain_image_size();
    assert!(terrain_image_size.is_some());
    assert_eq!(terrain_image_size.unwrap(), UVec2::new(32, 16)); // 2 tiles x 1 tile * 16px each

    let creature_image_size = resources.creature_image_size();
    assert!(creature_image_size.is_some());
    assert_eq!(creature_image_size.unwrap(), UVec2::new(32, 16));
}

#[test]
fn test_resource_manager_missing_terrain_atlas() {
    let temp_dir = TempDir::new().unwrap();
    let assets_dir = temp_dir.path();

    // Only create creatures atlas and tilemap
    create_test_atlas(assets_dir, "creatures").unwrap();
    create_test_tilemap(assets_dir, "test_map.json").unwrap();

    let terrain_path = assets_dir.join("terrain.json");
    let creature_path = assets_dir.join("creatures.json");
    let tilemap_path = assets_dir.join("test_map.json");

    let result = ResourceManager::load_with_paths(&terrain_path, &creature_path, &tilemap_path);
    assert!(result.is_err());

    match result.unwrap_err() {
        ResourceError::Validation(msg) => {
            assert!(msg.contains("Failed to load terrain atlas"));
        }
        _ => panic!("Expected validation error"),
    }
}

#[test]
fn test_resource_manager_missing_creature_atlas() {
    let temp_dir = TempDir::new().unwrap();
    let assets_dir = temp_dir.path();

    // Only create terrain atlas and tilemap
    create_test_atlas(assets_dir, "terrain").unwrap();
    create_test_tilemap(assets_dir, "test_map.json").unwrap();

    let terrain_path = assets_dir.join("terrain.json");
    let creature_path = assets_dir.join("creatures.json");
    let tilemap_path = assets_dir.join("test_map.json");

    let result = ResourceManager::load_with_paths(&terrain_path, &creature_path, &tilemap_path);
    assert!(result.is_err());

    match result.unwrap_err() {
        ResourceError::Validation(msg) => {
            assert!(msg.contains("Failed to load creature atlas"));
        }
        _ => panic!("Expected validation error"),
    }
}

#[test]
fn test_resource_manager_missing_tilemap() {
    let temp_dir = TempDir::new().unwrap();
    let assets_dir = temp_dir.path();

    // Only create atlas files
    create_test_atlas(assets_dir, "terrain").unwrap();
    create_test_atlas(assets_dir, "creatures").unwrap();

    let terrain_path = assets_dir.join("terrain.json");
    let creature_path = assets_dir.join("creatures.json");
    let tilemap_path = assets_dir.join("test_map.json");

    let result = ResourceManager::load_with_paths(&terrain_path, &creature_path, &tilemap_path);
    assert!(result.is_err());

    match result.unwrap_err() {
        ResourceError::Validation(msg) => {
            assert!(msg.contains("Failed to load tilemap"));
        }
        _ => panic!("Expected validation error"),
    }
}

#[test]
fn test_resource_manager_invalid_atlas_json() {
    let temp_dir = TempDir::new().unwrap();
    let assets_dir = temp_dir.path();

    // Create invalid atlas
    fs::write(assets_dir.join("terrain.json"), "invalid json").unwrap();
    create_test_atlas(assets_dir, "creatures").unwrap();
    create_test_tilemap(assets_dir, "test_map.json").unwrap();

    let terrain_path = assets_dir.join("terrain.json");
    let creature_path = assets_dir.join("creatures.json");
    let tilemap_path = assets_dir.join("test_map.json");

    let result = ResourceManager::load_with_paths(&terrain_path, &creature_path, &tilemap_path);
    assert!(result.is_err());

    match result.unwrap_err() {
        ResourceError::Validation(msg) => {
            assert!(msg.contains("Failed to load terrain atlas"));
        }
        _ => panic!("Expected validation error"),
    }
}

fn create_invalid_tilemap(dir: &Path, name: &str) -> std::io::Result<()> {
    let tilemap_content = r#"{
  "size": [2, 2],
  "tile_size": [16, 16],
  "atlas": "terrain.json",
  "tiles": [
    "grass", "stone"
  ]
}"#;
    fs::write(dir.join(name), tilemap_content)
}

#[test]
fn test_resource_manager_invalid_tilemap_validation() {
    let temp_dir = TempDir::new().unwrap();
    let assets_dir = temp_dir.path();

    create_test_atlas(assets_dir, "terrain").unwrap();
    create_test_atlas(assets_dir, "creatures").unwrap();
    create_invalid_tilemap(assets_dir, "test_map.json").unwrap(); // 2x2 size but only 2 tiles

    let terrain_path = assets_dir.join("terrain.json");
    let creature_path = assets_dir.join("creatures.json");
    let tilemap_path = assets_dir.join("test_map.json");

    let result = ResourceManager::load_with_paths(&terrain_path, &creature_path, &tilemap_path);
    assert!(result.is_err());

    match result.unwrap_err() {
        ResourceError::Validation(msg) => {
            assert!(msg.contains("Tilemap validation failed"));
        }
        _ => panic!("Expected validation error"),
    }
}

#[test]
fn test_resource_error_display() {
    let io_error = ResourceError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "File not found",
    ));
    assert!(format!("{}", io_error).contains("IO error"));

    let json_error: Result<serde_json::Value, _> = serde_json::from_str("invalid json");
    let json_error = ResourceError::Json(json_error.unwrap_err());
    assert!(format!("{}", json_error).contains("JSON parsing error"));

    let validation_error = ResourceError::Validation("Test validation error".to_string());
    assert_eq!(
        format!("{}", validation_error),
        "Asset validation error: Test validation error"
    );
}
