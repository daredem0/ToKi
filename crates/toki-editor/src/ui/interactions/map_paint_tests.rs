use super::MapPaintInteraction;
use glam::{UVec2, Vec2};
use std::path::PathBuf;
use toki_core::assets::tilemap::{TileLayer, TileMap};

fn sample_tilemap() -> TileMap {
    TileMap {
        size: UVec2::new(3, 2),
        tile_size: UVec2::new(8, 8),
        atlas: PathBuf::from("terrain.json"),
        layers: vec![TileLayer::new(
            "ground",
            vec![
                "grass".to_string(),
                "grass".to_string(),
                "grass".to_string(),
                "water".to_string(),
                "water".to_string(),
                "water".to_string(),
            ],
        )],
    }
}

fn multi_layer_tilemap() -> TileMap {
    TileMap {
        size: UVec2::new(2, 2),
        tile_size: UVec2::new(8, 8),
        atlas: PathBuf::from("terrain.json"),
        layers: vec![
            TileLayer::new(
                "ground",
                vec![
                    "grass".to_string(),
                    "grass".to_string(),
                    "grass".to_string(),
                    "grass".to_string(),
                ],
            ),
            TileLayer::new(
                "detail",
                vec![
                    "empty".to_string(),
                    "empty".to_string(),
                    "empty".to_string(),
                    "empty".to_string(),
                ],
            ),
        ],
    }
}

#[test]
fn tile_position_at_world_returns_none_for_negative_or_out_of_bounds() {
    let tilemap = sample_tilemap();

    assert_eq!(
        MapPaintInteraction::tile_position_at_world(&tilemap, Vec2::new(-1.0, 0.0)),
        None
    );
    assert_eq!(
        MapPaintInteraction::tile_position_at_world(&tilemap, Vec2::new(24.0, 0.0)),
        None
    );
    assert_eq!(
        MapPaintInteraction::tile_position_at_world(&tilemap, Vec2::new(0.0, 16.0)),
        None
    );
}

#[test]
fn tile_position_at_world_uses_tile_size_grid() {
    let tilemap = sample_tilemap();

    assert_eq!(
        MapPaintInteraction::tile_position_at_world(&tilemap, Vec2::new(15.9, 8.1)),
        Some(UVec2::new(1, 1))
    );
}

#[test]
fn paint_tile_updates_tile_and_reports_whether_it_changed() {
    let mut tilemap = sample_tilemap();

    assert!(MapPaintInteraction::paint_tile(
        &mut tilemap,
        0,
        UVec2::new(1, 0),
        "bush"
    ));
    assert_eq!(tilemap.tiles()[1], "bush");
    assert!(!MapPaintInteraction::paint_tile(
        &mut tilemap,
        0,
        UVec2::new(1, 0),
        "bush"
    ));
}

#[test]
fn paint_brush_paints_square_area_and_clips_to_map_bounds() {
    let mut tilemap = sample_tilemap();

    assert!(MapPaintInteraction::paint_brush(
        &mut tilemap,
        0,
        UVec2::new(1, 0),
        "bush",
        2
    ));

    assert_eq!(tilemap.tiles()[1], "bush");
    assert_eq!(tilemap.tiles()[2], "bush");
    assert_eq!(tilemap.tiles()[4], "bush");
    assert_eq!(tilemap.tiles()[5], "bush");
    assert_eq!(tilemap.tiles()[0], "grass");
    assert_eq!(tilemap.tiles()[3], "water");
}

#[test]
fn brush_footprint_bounds_clips_to_tilemap_edges() {
    let tilemap = sample_tilemap();

    assert_eq!(
        MapPaintInteraction::brush_footprint_bounds(&tilemap, UVec2::new(1, 0), 2),
        Some((UVec2::new(1, 0), UVec2::new(3, 2)))
    );
    assert_eq!(
        MapPaintInteraction::brush_footprint_bounds(&tilemap, UVec2::new(0, 0), 3),
        Some((UVec2::new(0, 0), UVec2::new(3, 2)))
    );
}

#[test]
fn fill_all_replaces_every_tile_and_reports_changes() {
    let mut tilemap = sample_tilemap();

    assert!(MapPaintInteraction::fill_all(&mut tilemap, 0, "bush"));
    assert!(tilemap.tiles().iter().all(|tile| tile == "bush"));
    assert!(!MapPaintInteraction::fill_all(&mut tilemap, 0, "bush"));
}

// --- Layer-specific tests ---

#[test]
fn paint_tile_on_layer_1_does_not_affect_layer_0() {
    let mut tilemap = multi_layer_tilemap();

    assert!(MapPaintInteraction::paint_tile(
        &mut tilemap,
        1,
        UVec2::new(0, 0),
        "flower"
    ));
    assert_eq!(tilemap.get_tile_name_on_layer(1, 0, 0).unwrap(), "flower");
    assert_eq!(
        tilemap.get_tile_name_on_layer(0, 0, 0).unwrap(),
        "grass",
        "layer 0 should be unaffected"
    );
}

#[test]
fn fill_on_specific_layer() {
    let mut tilemap = multi_layer_tilemap();

    assert!(MapPaintInteraction::fill_all(&mut tilemap, 1, "flower"));
    assert!(tilemap
        .layer_tiles_mut(1)
        .unwrap()
        .iter()
        .all(|t| t == "flower"));
    assert!(tilemap.tiles().iter().all(|t| t == "grass"));
}

#[test]
fn paint_brush_on_specific_layer() {
    let mut tilemap = multi_layer_tilemap();

    assert!(MapPaintInteraction::paint_brush(
        &mut tilemap,
        1,
        UVec2::new(0, 0),
        "flower",
        2
    ));
    assert_eq!(tilemap.get_tile_name_on_layer(1, 0, 0).unwrap(), "flower");
    assert_eq!(tilemap.get_tile_name_on_layer(1, 1, 0).unwrap(), "flower");
    assert_eq!(
        tilemap.get_tile_name_on_layer(0, 0, 0).unwrap(),
        "grass",
        "layer 0 should be unaffected"
    );
}

#[test]
fn paint_on_invalid_layer_returns_false() {
    let mut tilemap = sample_tilemap();

    assert!(!MapPaintInteraction::paint_tile(
        &mut tilemap,
        99,
        UVec2::new(0, 0),
        "bush"
    ));
    assert!(!MapPaintInteraction::fill_all(&mut tilemap, 99, "bush"));
}
