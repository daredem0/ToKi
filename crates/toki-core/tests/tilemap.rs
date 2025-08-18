use toki_core::assets::{atlas::AtlasMeta, tilemap::TileMap};
use toki_core::CoreError;
use glam::UVec2;
use std::collections::HashMap;
use std::path::PathBuf;

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
    }
}

// Helper function to create a test atlas
fn create_test_atlas() -> AtlasMeta {
    let mut tiles = HashMap::new();
    tiles.insert("grass".to_string(), UVec2::new(0, 0));
    tiles.insert("stone".to_string(), UVec2::new(1, 0));
    tiles.insert("water".to_string(), UVec2::new(0, 1));
    tiles.insert("dirt".to_string(), UVec2::new(1, 1));
    
    AtlasMeta {
        image: PathBuf::from("test_texture.png"),
        tile_size: UVec2::new(16, 16),
        tiles,
    }
}

#[test]
fn tilemap_get_tile_name_returns_correct_tiles() {
    let tilemap = create_test_tilemap();
    
    assert_eq!(tilemap.get_tile_name(0, 0), Some("grass"));
    assert_eq!(tilemap.get_tile_name(1, 0), Some("stone"));
    assert_eq!(tilemap.get_tile_name(0, 1), Some("water"));
    assert_eq!(tilemap.get_tile_name(1, 1), Some("dirt"));
}

#[test]
fn tilemap_get_tile_name_out_of_bounds_returns_none() {
    let tilemap = create_test_tilemap();
    
    assert_eq!(tilemap.get_tile_name(2, 0), None);
    assert_eq!(tilemap.get_tile_name(0, 2), None);
    assert_eq!(tilemap.get_tile_name(2, 2), None);
    assert_eq!(tilemap.get_tile_name(100, 100), None);
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
    };
    
    assert!(tilemap.validate().is_ok());
}

#[test]
fn tilemap_tile_to_world_converts_correctly() {
    let tilemap = create_test_tilemap();
    
    assert_eq!(tilemap.tile_to_world(UVec2::new(0, 0)), Some(UVec2::new(0, 0)));
    assert_eq!(tilemap.tile_to_world(UVec2::new(1, 0)), Some(UVec2::new(16, 0)));
    assert_eq!(tilemap.tile_to_world(UVec2::new(0, 1)), Some(UVec2::new(0, 16)));
    assert_eq!(tilemap.tile_to_world(UVec2::new(1, 1)), Some(UVec2::new(16, 16)));
}

#[test]
fn tilemap_tile_to_world_with_different_tile_sizes() {
    let mut tilemap = create_test_tilemap();
    tilemap.tile_size = UVec2::new(32, 24);
    
    assert_eq!(tilemap.tile_to_world(UVec2::new(0, 0)), Some(UVec2::new(0, 0)));
    assert_eq!(tilemap.tile_to_world(UVec2::new(1, 0)), Some(UVec2::new(32, 0)));
    assert_eq!(tilemap.tile_to_world(UVec2::new(0, 1)), Some(UVec2::new(0, 24)));
    assert_eq!(tilemap.tile_to_world(UVec2::new(1, 1)), Some(UVec2::new(32, 24)));
}

#[test]
fn tilemap_tile_to_world_out_of_bounds_returns_none() {
    let tilemap = create_test_tilemap();
    
    assert_eq!(tilemap.tile_to_world(UVec2::new(2, 0)), None);
    assert_eq!(tilemap.tile_to_world(UVec2::new(0, 2)), None);
    assert_eq!(tilemap.tile_to_world(UVec2::new(2, 2)), None);
    assert_eq!(tilemap.tile_to_world(UVec2::new(100, 100)), None);
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
    assert_eq!(first_quad[0].position, [0.0, 0.0]);        // Top-left
    assert_eq!(first_quad[1].position, [16.0, 0.0]);       // Top-right  
    assert_eq!(first_quad[2].position, [0.0, 16.0]);       // Bottom-left
    
    // Triangle 2: top-right, bottom-right, bottom-left  
    assert_eq!(first_quad[3].position, [16.0, 0.0]);       // Top-right
    assert_eq!(first_quad[4].position, [16.0, 16.0]);      // Bottom-right
    assert_eq!(first_quad[5].position, [0.0, 16.0]);       // Bottom-left
    
    // UV coordinates for grass tile (atlas position 0,0)
    assert_eq!(first_quad[0].tex_coords, [0.0, 0.0]);      // Top-left UV
    assert_eq!(first_quad[1].tex_coords, [0.5, 0.0]);      // Top-right UV (16/32)
    assert_eq!(first_quad[2].tex_coords, [0.0, 0.5]);      // Bottom-left UV (16/32)
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
            "grass".to_string(), "stone".to_string(), "water".to_string(),
            "dirt".to_string(),  "grass".to_string(), "stone".to_string(),
        ],
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
            "a".to_string(), "b".to_string(), "c".to_string(),  // Row 0
            "d".to_string(), "e".to_string(), "f".to_string(),  // Row 1
        ],
    };
    
    // First row
    assert_eq!(tilemap.get_tile_name(0, 0), Some("a"));
    assert_eq!(tilemap.get_tile_name(1, 0), Some("b"));
    assert_eq!(tilemap.get_tile_name(2, 0), Some("c"));
    
    // Second row
    assert_eq!(tilemap.get_tile_name(0, 1), Some("d"));
    assert_eq!(tilemap.get_tile_name(1, 1), Some("e"));
    assert_eq!(tilemap.get_tile_name(2, 1), Some("f"));
}

#[test]
fn tilemap_single_tile_map() {
    let tilemap = TileMap {
        size: UVec2::new(1, 1),
        tile_size: UVec2::new(32, 32),
        atlas: PathBuf::from("test.json"),
        tiles: vec!["single".to_string()],
    };
    
    assert!(tilemap.validate().is_ok());
    assert_eq!(tilemap.get_tile_name(0, 0), Some("single"));
    assert_eq!(tilemap.get_tile_name(1, 0), None);
    assert_eq!(tilemap.tile_to_world(UVec2::new(0, 0)), Some(UVec2::new(0, 0)));
    assert_eq!(tilemap.tile_to_world(UVec2::new(1, 0)), None);
}