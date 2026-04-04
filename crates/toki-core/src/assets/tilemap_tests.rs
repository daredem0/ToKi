use super::tilemap::{TileLayer, TileMap};
use crate::assets::atlas::{AtlasMeta, ColorMode, TileInfo, TileProperties};
use glam::UVec2;
use std::collections::HashMap;
use std::path::PathBuf;

fn make_layer(name: &str, tiles: Vec<&str>) -> TileLayer {
    TileLayer {
        name: name.to_string(),
        tiles: tiles.into_iter().map(String::from).collect(),
        visible: true,
        collision_enabled: name == "ground",
        above_entities: false,
    }
}

fn make_tilemap(size: UVec2, layers: Vec<TileLayer>) -> TileMap {
    TileMap {
        size,
        tile_size: UVec2::new(8, 8),
        atlas: PathBuf::from("test.json"),
        layers,
    }
}

// --- Validation tests ---

#[test]
fn test_single_layer_validates() {
    let map = make_tilemap(
        UVec2::new(2, 2),
        vec![make_layer("ground", vec!["a", "b", "c", "d"])],
    );
    assert!(map.validate().is_ok());
}

#[test]
fn test_multi_layer_validates() {
    let tiles = vec!["a", "b", "c", "d"];
    let map = make_tilemap(
        UVec2::new(2, 2),
        vec![
            make_layer("ground", tiles.clone()),
            make_layer("detail", tiles.clone()),
            make_layer("above", tiles),
        ],
    );
    assert!(map.validate().is_ok());
}

#[test]
fn test_layer_tile_count_mismatch_fails() {
    let map = make_tilemap(
        UVec2::new(2, 2),
        vec![make_layer("ground", vec!["a", "b", "c"])], // 3 tiles, need 4
    );
    assert!(map.validate().is_err());
}

#[test]
fn test_empty_layers_fails() {
    let map = make_tilemap(UVec2::new(2, 2), vec![]);
    assert!(map.validate().is_err());
}

// --- Serde backward compat tests ---

#[test]
fn test_backward_compat_deserialize() {
    let json = r#"{
        "size": [2, 2],
        "tile_size": [8, 8],
        "atlas": "terrain.json",
        "tiles": ["grass", "stone", "grass", "dirt"]
    }"#;
    let map: TileMap = serde_json::from_str(json).expect("old format should deserialize");
    assert_eq!(map.layers.len(), 1);
    assert_eq!(map.layers[0].name, "ground");
    assert!(map.layers[0].visible);
    assert!(map.layers[0].collision_enabled);
    assert_eq!(map.layers[0].tiles, vec!["grass", "stone", "grass", "dirt"]);
    assert!(map.validate().is_ok());
}

#[test]
fn test_serialize_round_trip() {
    let tiles = vec!["a", "b", "c", "d"];
    let original = make_tilemap(
        UVec2::new(2, 2),
        vec![
            make_layer("ground", tiles.clone()),
            make_layer("detail", tiles),
        ],
    );
    let json = serde_json::to_string(&original).expect("serialize");
    let restored: TileMap = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, restored);
}

// --- Layer accessor tests ---

#[test]
fn test_get_tile_name_on_layer() {
    let map = make_tilemap(
        UVec2::new(2, 2),
        vec![
            make_layer("ground", vec!["g0", "g1", "g2", "g3"]),
            make_layer("detail", vec!["d0", "d1", "d2", "d3"]),
        ],
    );
    assert_eq!(map.get_tile_name_on_layer(0, 1, 0).unwrap(), "g1");
    assert_eq!(map.get_tile_name_on_layer(1, 1, 0).unwrap(), "d1");
    assert_eq!(map.get_tile_name_on_layer(0, 0, 1).unwrap(), "g2");
    assert_eq!(map.get_tile_name_on_layer(1, 0, 1).unwrap(), "d2");
}

#[test]
fn test_get_tile_name_defaults_to_layer_zero() {
    let map = make_tilemap(
        UVec2::new(2, 2),
        vec![
            make_layer("ground", vec!["g0", "g1", "g2", "g3"]),
            make_layer("detail", vec!["d0", "d1", "d2", "d3"]),
        ],
    );
    assert_eq!(map.get_tile_name(1, 0).unwrap(), "g1");
}

#[test]
fn test_get_tile_name_on_layer_out_of_bounds() {
    let map = make_tilemap(
        UVec2::new(2, 2),
        vec![make_layer("ground", vec!["a", "b", "c", "d"])],
    );
    assert!(map.get_tile_name_on_layer(0, 5, 0).is_err());
    assert!(map.get_tile_name_on_layer(2, 0, 0).is_err()); // layer index out of bounds
}

// --- TileLayer::new_empty tests ---

#[test]
fn test_new_empty_creates_correct_tile_count() {
    let layer = TileLayer::new_empty("detail", 9);
    assert_eq!(layer.name, "detail");
    assert_eq!(layer.tiles.len(), 9);
    assert!(layer.tiles.iter().all(|t| t.is_empty()));
    assert!(layer.visible);
    assert!(!layer.collision_enabled);
    assert!(!layer.above_entities);
}

// --- above_entities serde tests ---

#[test]
fn test_above_entities_defaults_false_on_deserialize() {
    let json = r#"{
        "size": [1, 1],
        "tile_size": [8, 8],
        "atlas": "test.json",
        "layers": [{"name": "ground", "tiles": ["a"]}]
    }"#;
    let map: TileMap = serde_json::from_str(json).expect("should parse without above_entities");
    assert!(!map.layers[0].above_entities);
}

#[test]
fn test_above_entities_round_trips() {
    let mut layer = make_layer("canopy", vec!["a", "b", "c", "d"]);
    layer.above_entities = true;
    let map = make_tilemap(
        UVec2::new(2, 2),
        vec![make_layer("ground", vec!["a", "b", "c", "d"]), layer],
    );
    let json = serde_json::to_string(&map).expect("serialize");
    let restored: TileMap = serde_json::from_str(&json).expect("deserialize");
    assert!(!restored.layers[0].above_entities);
    assert!(restored.layers[1].above_entities);
}

// --- SplitTilemapVertices tests ---

fn test_atlas() -> AtlasMeta {
    let mut tiles = HashMap::new();
    tiles.insert(
        "a".to_string(),
        TileInfo {
            position: UVec2::new(0, 0),
            properties: TileProperties::default(),
        },
    );
    tiles.insert(
        "b".to_string(),
        TileInfo {
            position: UVec2::new(1, 0),
            properties: TileProperties::default(),
        },
    );
    AtlasMeta {
        image: PathBuf::from("test.png"),
        tile_size: UVec2::new(8, 8),
        color_mode: ColorMode::TrueColor,
        palette: None,
        tiles,
    }
}

#[test]
fn test_split_vertices_no_above_layers() {
    let map = make_tilemap(UVec2::new(1, 1), vec![make_layer("ground", vec!["a"])]);
    let atlas = test_atlas();
    let tex = UVec2::new(16, 8);

    let split = map.generate_split_vertices(&atlas, tex);
    let flat = map.generate_vertices(&atlas, tex);

    assert_eq!(split.below.len(), flat.len());
    assert!(split.above.is_empty());
}

#[test]
fn test_split_vertices_partitions_by_above_entities() {
    let mut above_layer = make_layer("canopy", vec!["b"]);
    above_layer.above_entities = true;
    let map = make_tilemap(
        UVec2::new(1, 1),
        vec![make_layer("ground", vec!["a"]), above_layer],
    );
    let atlas = test_atlas();
    let tex = UVec2::new(16, 8);

    let split = map.generate_split_vertices(&atlas, tex);

    assert_eq!(split.below.len(), 6); // 1 tile = 2 triangles = 6 vertices
    assert_eq!(split.above.len(), 6);
}

#[test]
fn test_split_vertices_invisible_above_layer_produces_nothing() {
    let mut above_layer = make_layer("canopy", vec!["b"]);
    above_layer.above_entities = true;
    above_layer.visible = false;
    let map = make_tilemap(
        UVec2::new(1, 1),
        vec![make_layer("ground", vec!["a"]), above_layer],
    );
    let atlas = test_atlas();
    let tex = UVec2::new(16, 8);

    let split = map.generate_split_vertices(&atlas, tex);

    assert_eq!(split.below.len(), 6);
    assert!(split.above.is_empty());
}

#[test]
fn test_split_vertices_for_chunks_partitions_correctly() {
    let mut above_layer = make_layer("canopy", vec!["b"]);
    above_layer.above_entities = true;
    let map = make_tilemap(
        UVec2::new(1, 1),
        vec![make_layer("ground", vec!["a"]), above_layer],
    );
    let atlas = test_atlas();
    let tex = UVec2::new(16, 8);

    let split = map.generate_split_vertices_for_chunks(&atlas, tex, &[(0, 0)]);

    assert_eq!(split.below.len(), 6);
    assert_eq!(split.above.len(), 6);
}
