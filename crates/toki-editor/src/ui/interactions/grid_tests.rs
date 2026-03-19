use super::GridInteraction;
use crate::config::{EditorConfig, GridSettings};
use glam::{UVec2, Vec2};
use std::path::PathBuf;
use toki_core::assets::tilemap::TileMap;

fn sample_tilemap(tile_size: UVec2) -> TileMap {
    TileMap {
        size: UVec2::new(1, 1),
        tile_size,
        atlas: PathBuf::from("test_atlas.json"),
        tiles: vec!["floor".to_string()],
        objects: vec![],
    }
}

#[test]
fn maybe_snap_world_position_returns_input_when_snap_disabled() {
    let mut config = EditorConfig::default();
    config.editor_settings.grid = GridSettings {
        show_grid: true,
        grid_size: [8, 8],
        snap_to_grid: false,
    };

    let world =
        GridInteraction::maybe_snap_world_position(Vec2::new(13.7, 9.3), None, Some(&config));
    assert_eq!(world, Vec2::new(13.7, 9.3));
}

#[test]
fn maybe_snap_world_position_uses_editor_grid_size_without_tilemap() {
    let mut config = EditorConfig::default();
    config.editor_settings.grid = GridSettings {
        show_grid: true,
        grid_size: [8, 12],
        snap_to_grid: true,
    };

    let world =
        GridInteraction::maybe_snap_world_position(Vec2::new(13.7, 25.9), None, Some(&config));
    assert_eq!(world, Vec2::new(8.0, 24.0));
}

#[test]
fn maybe_snap_world_position_prefers_tilemap_tile_size() {
    let mut config = EditorConfig::default();
    config.editor_settings.grid = GridSettings {
        show_grid: true,
        grid_size: [8, 8],
        snap_to_grid: true,
    };
    let tilemap = sample_tilemap(UVec2::new(16, 16));

    let world = GridInteraction::maybe_snap_world_position(
        Vec2::new(13.7, 25.9),
        Some(&tilemap),
        Some(&config),
    );
    assert_eq!(world, Vec2::new(0.0, 16.0));
}

#[test]
fn maybe_snap_world_position_handles_negative_values_with_floor() {
    let mut config = EditorConfig::default();
    config.editor_settings.grid = GridSettings {
        show_grid: true,
        grid_size: [16, 16],
        snap_to_grid: true,
    };

    let world =
        GridInteraction::maybe_snap_world_position(Vec2::new(-1.0, -17.2), None, Some(&config));
    assert_eq!(world, Vec2::new(-16.0, -32.0));
}

#[test]
fn drag_target_world_position_applies_grab_offset_before_snapping() {
    let mut config = EditorConfig::default();
    config.editor_settings.grid = GridSettings {
        show_grid: true,
        grid_size: [16, 16],
        snap_to_grid: true,
    };

    // Regression scenario:
    // cursor slightly crosses a tile boundary, but with a large grab offset
    // the anchored top-left should remain in the previous tile.
    let snapped = GridInteraction::drag_target_world_position(
        Vec2::new(32.1, 32.1),
        Vec2::new(15.5, 15.5),
        None,
        Some(&config),
    );
    assert_eq!(snapped, Vec2::new(16.0, 16.0));
    assert_ne!(
        snapped,
        GridInteraction::maybe_snap_world_position(Vec2::new(32.1, 32.1), None, Some(&config))
    );
}

#[test]
fn drag_target_world_position_keeps_unsnapped_anchored_world_when_snap_disabled() {
    let mut config = EditorConfig::default();
    config.editor_settings.grid = GridSettings {
        show_grid: true,
        grid_size: [16, 16],
        snap_to_grid: false,
    };

    let target = GridInteraction::drag_target_world_position(
        Vec2::new(40.3, 22.6),
        Vec2::new(8.0, 6.0),
        None,
        Some(&config),
    );
    assert_eq!(target, Vec2::new(32.3, 16.6));
}

#[test]
fn scene_anchor_marker_world_position_uses_grid_cell_center_when_snap_enabled() {
    let mut config = EditorConfig::default();
    config.editor_settings.grid = GridSettings {
        show_grid: true,
        grid_size: [16, 16],
        snap_to_grid: true,
    };

    let marker =
        GridInteraction::placement_pose(Vec2::new(32.0, 48.0), None, Some(&config)).marker_world;

    assert_eq!(marker, Vec2::new(40.0, 56.0));
}

#[test]
fn placement_pose_uses_shared_origin_and_marker_for_snapped_placement() {
    let mut config = EditorConfig::default();
    config.editor_settings.grid = GridSettings {
        show_grid: true,
        grid_size: [16, 16],
        snap_to_grid: true,
    };

    let pose = GridInteraction::placement_pose(Vec2::new(39.5, 55.9), None, Some(&config));

    assert_eq!(pose.world_origin, Vec2::new(32.0, 48.0));
    assert_eq!(pose.marker_world, Vec2::new(40.0, 56.0));
    assert_eq!(pose.snapped_cell_size, Some(UVec2::new(16, 16)));
}

#[test]
fn scene_anchor_marker_world_position_prefers_tilemap_cell_center() {
    let mut config = EditorConfig::default();
    config.editor_settings.grid = GridSettings {
        show_grid: true,
        grid_size: [32, 32],
        snap_to_grid: true,
    };
    let tilemap = sample_tilemap(UVec2::new(16, 24));

    let marker =
        GridInteraction::placement_pose(Vec2::new(32.0, 48.0), Some(&tilemap), Some(&config))
            .marker_world;

    assert_eq!(marker, Vec2::new(40.0, 60.0));
}

#[test]
fn scene_anchor_marker_world_position_keeps_raw_position_when_snap_disabled() {
    let mut config = EditorConfig::default();
    config.editor_settings.grid = GridSettings {
        show_grid: true,
        grid_size: [16, 16],
        snap_to_grid: false,
    };

    let marker =
        GridInteraction::placement_pose(Vec2::new(32.0, 48.0), None, Some(&config)).marker_world;

    assert_eq!(marker, Vec2::new(32.0, 48.0));
}
