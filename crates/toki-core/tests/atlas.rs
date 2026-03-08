use glam::UVec2;
use std::collections::HashMap;
use std::path::PathBuf;
use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
use toki_core::CoreError;

#[test]
fn test_load_atlas() {
    let atlas = AtlasMeta::load_from_file("../../assets/terrain.json").unwrap();
    let rect = atlas.get_tile_rect("grass").unwrap();
    assert_eq!(rect, [0, 0, 8, 8]);
}

#[test]
fn atlas_load_nonexistent_file_returns_error() {
    let result = AtlasMeta::load_from_file("definitely_nonexistent_atlas.json");
    assert!(result.is_err());

    match result.unwrap_err() {
        CoreError::Io(_) => {} // Expected
        other => panic!("Expected IO error, got: {:?}", other),
    }
}

#[test]
fn atlas_get_tile_rect_missing_tile_returns_none() {
    let atlas = create_test_atlas();
    assert_eq!(atlas.get_tile_rect("nonexistent_tile"), None);
    assert_eq!(atlas.get_tile_rect(""), None);
}

#[test]
fn atlas_image_size_calculation() {
    let atlas = create_test_atlas();
    let size = atlas.image_size().unwrap();

    // With tiles at positions (0,0), (1,0), (0,1), (1,1) and tile_size 16x16
    // Max tile is at (1,1), so image size should be (2,2) * (16,16) = (32,32)
    assert_eq!(size, UVec2::new(32, 32));
}

#[test]
fn atlas_image_size_with_scattered_tiles() {
    let mut tiles = HashMap::new();
    tiles.insert(
        "tile1".to_string(),
        TileInfo {
            position: UVec2::new(0, 0),
            properties: TileProperties::default(),
        },
    );
    tiles.insert(
        "tile2".to_string(),
        TileInfo {
            position: UVec2::new(5, 3),
            properties: TileProperties::default(),
        },
    );

    let atlas = AtlasMeta {
        image: PathBuf::from("test.png"),
        tile_size: UVec2::new(8, 8),
        tiles,
    };

    let size = atlas.image_size().unwrap();
    // Max tile at (5,3), so size should be (6,4) * (8,8) = (48,32)
    assert_eq!(size, UVec2::new(48, 32));
}

#[test]
fn atlas_image_size_empty_atlas() {
    let atlas = AtlasMeta {
        image: PathBuf::from("test.png"),
        tile_size: UVec2::new(16, 16),
        tiles: HashMap::new(),
    };

    // Empty atlas should return None
    assert_eq!(atlas.image_size(), None);
}

#[test]
fn atlas_get_tile_rect_with_different_tile_sizes() {
    let mut tiles = HashMap::new();
    tiles.insert(
        "big_tile".to_string(),
        TileInfo {
            position: UVec2::new(2, 1),
            properties: TileProperties::default(),
        },
    );

    let atlas = AtlasMeta {
        image: PathBuf::from("test.png"),
        tile_size: UVec2::new(32, 24),
        tiles,
    };

    let rect = atlas.get_tile_rect("big_tile").unwrap();
    // Position (2,1) * tile_size (32,24) = offset (64, 24), size (32, 24)
    assert_eq!(rect, [64, 24, 32, 24]);
}

#[test]
fn atlas_case_sensitive_tile_names() {
    let atlas = create_test_atlas();

    // Should be case sensitive
    assert_eq!(atlas.get_tile_rect("grass"), Some([0, 0, 16, 16]));
    assert_eq!(atlas.get_tile_rect("Grass"), None);
    assert_eq!(atlas.get_tile_rect("GRASS"), None);
}

#[test]
fn atlas_with_special_characters_in_tile_names() {
    let mut tiles = HashMap::new();
    tiles.insert(
        "tile-with-dashes".to_string(),
        TileInfo {
            position: UVec2::new(0, 0),
            properties: TileProperties::default(),
        },
    );
    tiles.insert(
        "tile_with_underscores".to_string(),
        TileInfo {
            position: UVec2::new(1, 0),
            properties: TileProperties::default(),
        },
    );
    tiles.insert(
        "tile with spaces".to_string(),
        TileInfo {
            position: UVec2::new(0, 1),
            properties: TileProperties::default(),
        },
    );
    tiles.insert(
        "tile123".to_string(),
        TileInfo {
            position: UVec2::new(1, 1),
            properties: TileProperties::default(),
        },
    );

    let atlas = AtlasMeta {
        image: PathBuf::from("test.png"),
        tile_size: UVec2::new(16, 16),
        tiles,
    };

    assert_eq!(
        atlas.get_tile_rect("tile-with-dashes"),
        Some([0, 0, 16, 16])
    );
    assert_eq!(
        atlas.get_tile_rect("tile_with_underscores"),
        Some([16, 0, 16, 16])
    );
    assert_eq!(
        atlas.get_tile_rect("tile with spaces"),
        Some([0, 16, 16, 16])
    );
    assert_eq!(atlas.get_tile_rect("tile123"), Some([16, 16, 16, 16]));
}

#[test]
fn atlas_tile_collision_properties() {
    let atlas = create_test_atlas();

    // Test solid properties
    assert!(!atlas.is_tile_solid("grass"));
    assert!(atlas.is_tile_solid("stone"));
    assert!(!atlas.is_tile_solid("water"));
    assert!(!atlas.is_tile_solid("dirt"));

    // Test trigger properties
    assert!(!atlas.is_tile_trigger("grass"));
    assert!(!atlas.is_tile_trigger("stone"));
    assert!(atlas.is_tile_trigger("water"));
    assert!(!atlas.is_tile_trigger("dirt"));

    // Test nonexistent tiles default to false
    assert!(!atlas.is_tile_solid("nonexistent"));
    assert!(!atlas.is_tile_trigger("nonexistent"));
}

#[test]
fn atlas_get_tile_properties() {
    let atlas = create_test_atlas();

    // Test getting properties directly
    let stone_props = atlas.get_tile_properties("stone").unwrap();
    assert!(stone_props.solid);
    assert!(!stone_props.trigger);

    let water_props = atlas.get_tile_properties("water").unwrap();
    assert!(!water_props.solid);
    assert!(water_props.trigger);

    // Test nonexistent tile returns None
    assert_eq!(atlas.get_tile_properties("nonexistent"), None);
}

// Helper function to create a test atlas
fn create_test_atlas() -> AtlasMeta {
    let mut tiles = HashMap::new();
    tiles.insert(
        "grass".to_string(),
        TileInfo {
            position: UVec2::new(0, 0),
            properties: TileProperties::default(),
        },
    );
    tiles.insert(
        "stone".to_string(),
        TileInfo {
            position: UVec2::new(1, 0),
            properties: TileProperties {
                solid: true,
                trigger: false,
            },
        },
    );
    tiles.insert(
        "water".to_string(),
        TileInfo {
            position: UVec2::new(0, 1),
            properties: TileProperties {
                solid: false,
                trigger: true,
            },
        },
    );
    tiles.insert(
        "dirt".to_string(),
        TileInfo {
            position: UVec2::new(1, 1),
            properties: TileProperties::default(),
        },
    );

    AtlasMeta {
        image: PathBuf::from("test_texture.png"),
        tile_size: UVec2::new(16, 16),
        tiles,
    }
}
