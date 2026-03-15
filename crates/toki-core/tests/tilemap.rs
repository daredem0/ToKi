use glam::UVec2;
use std::collections::HashMap;
use std::path::PathBuf;
use toki_core::assets::{
    atlas::{AtlasMeta, TileInfo, TileProperties},
    tilemap::{MapObjectInstance, TileMap},
};
use toki_core::CoreError;

// Helper function to create a simple test tilemap
fn create_test_tilemap() -> TileMap {
    TileMap {
        size: UVec2::new(2, 2),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test_atlas.json"),
        tiles: vec![
            "grass".to_string(),
            "stone".to_string(),
            "water".to_string(),
            "dirt".to_string(),
        ],
        objects: vec![],
    }
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

#[test]
fn tilemap_get_tile_name_returns_correct_tiles() {
    let tilemap = create_test_tilemap();

    assert_eq!(tilemap.get_tile_name(0, 0).unwrap(), "grass");
    assert_eq!(tilemap.get_tile_name(1, 0).unwrap(), "stone");
    assert_eq!(tilemap.get_tile_name(0, 1).unwrap(), "water");
    assert_eq!(tilemap.get_tile_name(1, 1).unwrap(), "dirt");
}

#[test]
fn tilemap_get_tile_name_out_of_bounds_returns_none() {
    let tilemap = create_test_tilemap();

    assert!(tilemap.get_tile_name(2, 0).is_err());
    assert!(tilemap.get_tile_name(0, 2).is_err());
    assert!(tilemap.get_tile_name(2, 2).is_err());
    assert!(tilemap.get_tile_name(100, 100).is_err());
}

#[test]
fn tilemap_validate_correct_size_passes() {
    let tilemap = create_test_tilemap();
    assert!(tilemap.validate().is_ok());
}

#[test]
fn tilemap_validate_incorrect_size_fails() {
    let mut tilemap = create_test_tilemap();
    tilemap.tiles.push("extra".to_string()); // Now we have 5 tiles but expect 4

    let result = tilemap.validate();
    assert!(result.is_err());

    if let Err(CoreError::InvalidMapSize { expected, actual }) = result {
        assert_eq!(expected, 4);
        assert_eq!(actual, 5);
    } else {
        panic!("Expected InvalidMapSize error");
    }
}

#[test]
fn tilemap_validate_too_few_tiles_fails() {
    let mut tilemap = create_test_tilemap();
    tilemap.tiles.pop(); // Now we have 3 tiles but expect 4

    let result = tilemap.validate();
    assert!(result.is_err());

    if let Err(CoreError::InvalidMapSize { expected, actual }) = result {
        assert_eq!(expected, 4);
        assert_eq!(actual, 3);
    } else {
        panic!("Expected InvalidMapSize error");
    }
}

#[test]
fn tilemap_validate_empty_map_passes() {
    let tilemap = TileMap {
        size: UVec2::new(0, 0),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test.json"),
        tiles: vec![],
        objects: vec![],
    };

    assert!(tilemap.validate().is_ok());
}

#[test]
fn tilemap_tile_to_world_converts_correctly() {
    let tilemap = create_test_tilemap();

    assert_eq!(
        tilemap.tile_to_world(UVec2::new(0, 0)),
        Some(UVec2::new(0, 0))
    );
    assert_eq!(
        tilemap.tile_to_world(UVec2::new(1, 0)),
        Some(UVec2::new(16, 0))
    );
    assert_eq!(
        tilemap.tile_to_world(UVec2::new(0, 1)),
        Some(UVec2::new(0, 16))
    );
    assert_eq!(
        tilemap.tile_to_world(UVec2::new(1, 1)),
        Some(UVec2::new(16, 16))
    );
}

#[test]
fn tilemap_tile_to_world_with_different_tile_sizes() {
    let mut tilemap = create_test_tilemap();
    tilemap.tile_size = UVec2::new(32, 24);

    assert_eq!(
        tilemap.tile_to_world(UVec2::new(0, 0)),
        Some(UVec2::new(0, 0))
    );
    assert_eq!(
        tilemap.tile_to_world(UVec2::new(1, 0)),
        Some(UVec2::new(32, 0))
    );
    assert_eq!(
        tilemap.tile_to_world(UVec2::new(0, 1)),
        Some(UVec2::new(0, 24))
    );
    assert_eq!(
        tilemap.tile_to_world(UVec2::new(1, 1)),
        Some(UVec2::new(32, 24))
    );
}

#[test]
fn tilemap_tile_to_world_out_of_bounds_returns_none() {
    let tilemap = create_test_tilemap();

    assert!(tilemap.tile_to_world(UVec2::new(2, 0)).is_none());
    assert!(tilemap.tile_to_world(UVec2::new(0, 2)).is_none());
    assert!(tilemap.tile_to_world(UVec2::new(2, 2)).is_none());
    assert!(tilemap.tile_to_world(UVec2::new(100, 100)).is_none());
}

#[test]
fn tilemap_generate_vertices_creates_correct_quads() {
    let tilemap = create_test_tilemap();
    let atlas = create_test_atlas();
    let texture_size = UVec2::new(32, 32); // 2x2 tiles of 16x16 each

    let vertices = tilemap.generate_vertices(&atlas, texture_size);

    // Should have 4 tiles * 6 vertices per quad = 24 vertices
    assert_eq!(vertices.len(), 24);

    // Check first quad (grass tile at 0,0) - vertices are in triangle order
    let first_quad = &vertices[0..6];

    // Triangle 1: top-left, top-right, bottom-left
    assert_eq!(first_quad[0].position, [0.0, 0.0]); // Top-left
    assert_eq!(first_quad[1].position, [16.0, 0.0]); // Top-right
    assert_eq!(first_quad[2].position, [0.0, 16.0]); // Bottom-left

    // Triangle 2: top-right, bottom-right, bottom-left
    assert_eq!(first_quad[3].position, [16.0, 0.0]); // Top-right
    assert_eq!(first_quad[4].position, [16.0, 16.0]); // Bottom-right
    assert_eq!(first_quad[5].position, [0.0, 16.0]); // Bottom-left

    // UV coordinates for grass tile (atlas position 0,0)
    assert_eq!(first_quad[0].tex_coords, [0.0, 0.0]); // Top-left UV
    assert_eq!(first_quad[1].tex_coords, [0.5, 0.0]); // Top-right UV (16/32)
    assert_eq!(first_quad[2].tex_coords, [0.0, 0.5]); // Bottom-left UV (16/32)
}

#[test]
fn tilemap_generate_vertices_handles_missing_tiles() {
    let tilemap = create_test_tilemap();
    let mut atlas = create_test_atlas();
    atlas.tiles.remove("stone"); // Remove the stone tile from atlas
    let texture_size = UVec2::new(32, 32);

    let vertices = tilemap.generate_vertices(&atlas, texture_size);

    // Should have 3 tiles * 6 vertices per quad = 18 vertices (stone tile skipped)
    assert_eq!(vertices.len(), 18);
}

#[test]
fn tilemap_generate_vertices_empty_map() {
    let tilemap = TileMap {
        size: UVec2::new(0, 0),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test.json"),
        tiles: vec![],
        objects: vec![],
    };
    let atlas = create_test_atlas();
    let texture_size = UVec2::new(32, 32);

    let vertices = tilemap.generate_vertices(&atlas, texture_size);
    assert_eq!(vertices.len(), 0);
}

#[test]
fn tilemap_generate_vertices_with_larger_map() {
    let tilemap = TileMap {
        size: UVec2::new(3, 2), // 3x2 map
        tile_size: UVec2::new(8, 8),
        atlas: PathBuf::from("test.json"),
        tiles: vec![
            "grass".to_string(),
            "stone".to_string(),
            "water".to_string(),
            "dirt".to_string(),
            "grass".to_string(),
            "stone".to_string(),
        ],
        objects: vec![],
    };
    let atlas = create_test_atlas();
    let texture_size = UVec2::new(32, 32);

    let vertices = tilemap.generate_vertices(&atlas, texture_size);

    // 6 tiles * 6 vertices per quad = 36 vertices
    assert_eq!(vertices.len(), 36);

    // Check positioning of second row first tile (dirt at tile position 1,0)
    let second_row_start = 18; // 3 tiles * 6 vertices
    let dirt_quad = &vertices[second_row_start..second_row_start + 6];

    // Should be at world position (0, 8) since tile_size is 8x8
    assert_eq!(dirt_quad[0].position, [0.0, 8.0]);
}

#[test]
fn tilemap_load_from_nonexistent_file_returns_error() {
    let result = TileMap::load_from_file("nonexistent_tilemap.json");
    assert!(result.is_err());

    // Should be an IO error
    if let Err(CoreError::Io(_)) = result {
        // Expected
    } else {
        panic!("Expected IO error for nonexistent file");
    }
}

#[test]
fn tilemap_row_major_indexing() {
    let tilemap = TileMap {
        size: UVec2::new(3, 2), // 3 wide, 2 tall
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test.json"),
        tiles: vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(), // Row 0
            "d".to_string(),
            "e".to_string(),
            "f".to_string(), // Row 1
        ],
        objects: vec![],
    };

    // First row
    assert_eq!(tilemap.get_tile_name(0, 0).unwrap(), "a");
    assert_eq!(tilemap.get_tile_name(1, 0).unwrap(), "b");
    assert_eq!(tilemap.get_tile_name(2, 0).unwrap(), "c");

    // Second row
    assert_eq!(tilemap.get_tile_name(0, 1).unwrap(), "d");
    assert_eq!(tilemap.get_tile_name(1, 1).unwrap(), "e");
    assert_eq!(tilemap.get_tile_name(2, 1).unwrap(), "f");
}

#[test]
fn tilemap_single_tile_map() {
    let tilemap = TileMap {
        size: UVec2::new(1, 1),
        tile_size: UVec2::new(32, 32),
        atlas: PathBuf::from("test.json"),
        tiles: vec!["single".to_string()],
        objects: vec![],
    };

    assert!(tilemap.validate().is_ok());
    assert_eq!(tilemap.get_tile_name(0, 0).unwrap(), "single");
    assert!(tilemap.get_tile_name(1, 0).is_err());
    assert_eq!(
        tilemap.tile_to_world(UVec2::new(0, 0)),
        Some(UVec2::new(0, 0))
    );
    assert!(tilemap.tile_to_world(UVec2::new(1, 0)).is_none());
}
#[test]
fn tilemap_chunk_calculations() {
    let tilemap = TileMap {
        size: UVec2::new(64, 64),    // Your actual map size
        tile_size: UVec2::new(8, 8), // 8x8 pixel tiles
        atlas: PathBuf::from("test.json"),
        tiles: vec![], // Empty for this test
        objects: vec![],
    };

    // Test chunk count calculation
    let chunks = tilemap.chunk_count();
    assert_eq!(chunks, UVec2::new(4, 4)); // 64/16 = 4 chunks each direction

    // Test chunk bounds for top-left chunk (chunk 0,0)
    let bounds = tilemap.chunk_bounds(0, 0).unwrap();
    assert_eq!(bounds.0, UVec2::new(0, 0)); // Start at world (0,0)
    assert_eq!(bounds.1, UVec2::new(128, 128)); // End at (16*8, 16*8) pixels

    // Test chunk bounds for bottom-right chunk (chunk 3,3)
    let bounds = tilemap.chunk_bounds(3, 3).unwrap();
    assert_eq!(bounds.0, UVec2::new(384, 384)); // Start at (48*8, 48*8)
    assert_eq!(bounds.1, UVec2::new(512, 512)); // End at (64*8, 64*8)
}

#[test]
fn tilemap_deserialization_defaults_objects_for_legacy_maps() {
    let tilemap: TileMap = serde_json::from_str(
        r#"{
            "size": [1, 1],
            "tile_size": [16, 16],
            "atlas": "terrain.json",
            "tiles": ["grass"]
        }"#,
    )
    .expect("legacy tilemap json should parse");

    assert!(tilemap.objects.is_empty());
}

#[test]
fn tilemap_deserialization_defaults_object_visibility_solidity_and_size() {
    let tilemap: TileMap = serde_json::from_str(
        r#"{
            "size": [1, 1],
            "tile_size": [16, 16],
            "atlas": "terrain.json",
            "tiles": ["grass"],
            "objects": [
                {
                    "sheet": "fauna.json",
                    "object_name": "bush",
                    "position": [16, 32]
                }
            ]
        }"#,
    )
    .expect("legacy object instance should parse");

    assert_eq!(tilemap.objects.len(), 1);
    assert_eq!(tilemap.objects[0].size_px, UVec2::new(16, 16));
    assert!(tilemap.objects[0].visible);
    assert!(tilemap.objects[0].solid);
}

#[test]
fn tilemap_serialization_round_trips_object_instances() {
    let tilemap = TileMap {
        size: UVec2::new(1, 1),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("terrain.json"),
        tiles: vec!["grass".to_string()],
        objects: vec![MapObjectInstance {
            sheet: PathBuf::from("fauna.json"),
            object_name: "fauna_a".to_string(),
            position: UVec2::new(16, 32),
            size_px: UVec2::new(16, 16),
            visible: false,
            solid: true,
        }],
    };

    let json = serde_json::to_string(&tilemap).expect("tilemap should serialize");
    let round_trip: TileMap = serde_json::from_str(&json).expect("tilemap should deserialize");

    assert_eq!(round_trip, tilemap);
}
