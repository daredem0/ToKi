use glam::{IVec2, UVec2, Vec2};
use toki_core::math::coordinates::{snap_to_grid, tile_index_to_world, world_to_tile_index};

// ============================================================================
// world_to_tile_index tests
// ============================================================================

#[test]
fn world_to_tile_index_at_origin_returns_zero() {
    let result = world_to_tile_index(Vec2::ZERO, UVec2::new(16, 16));
    assert_eq!(result, IVec2::new(0, 0));
}

#[test]
fn world_to_tile_index_at_tile_boundary_returns_next_tile() {
    // Position exactly at (16, 16) should be in tile (1, 1)
    let result = world_to_tile_index(Vec2::new(16.0, 16.0), UVec2::new(16, 16));
    assert_eq!(result, IVec2::new(1, 1));
}

#[test]
fn world_to_tile_index_mid_tile_returns_current_tile() {
    // Position (8, 12) is inside tile (0, 0) for 16x16 tiles
    let result = world_to_tile_index(Vec2::new(8.0, 12.0), UVec2::new(16, 16));
    assert_eq!(result, IVec2::new(0, 0));
}

#[test]
fn world_to_tile_index_negative_position_returns_negative_index() {
    // Position (-1, -1) should be in tile (-1, -1)
    let result = world_to_tile_index(Vec2::new(-1.0, -1.0), UVec2::new(16, 16));
    assert_eq!(result, IVec2::new(-1, -1));
}

#[test]
fn world_to_tile_index_negative_far_position_returns_correct_index() {
    // Position (-17, -33) should be in tile (-2, -3)
    let result = world_to_tile_index(Vec2::new(-17.0, -33.0), UVec2::new(16, 16));
    assert_eq!(result, IVec2::new(-2, -3));
}

#[test]
fn world_to_tile_index_with_non_square_tiles() {
    // Position (12, 12) with 16x8 tiles -> tile (0, 1)
    let result = world_to_tile_index(Vec2::new(12.0, 12.0), UVec2::new(16, 8));
    assert_eq!(result, IVec2::new(0, 1));
}

#[test]
fn world_to_tile_index_just_before_boundary() {
    // Position (15.99, 15.99) should still be in tile (0, 0)
    let result = world_to_tile_index(Vec2::new(15.99, 15.99), UVec2::new(16, 16));
    assert_eq!(result, IVec2::new(0, 0));
}

// ============================================================================
// tile_index_to_world tests
// ============================================================================

#[test]
fn tile_index_to_world_at_origin_returns_zero() {
    let result = tile_index_to_world(IVec2::new(0, 0), UVec2::new(16, 16));
    assert_eq!(result, Vec2::new(0.0, 0.0));
}

#[test]
fn tile_index_to_world_positive_tile_returns_correct_position() {
    // Tile (1, 1) with 16x16 tiles -> world (16, 16)
    let result = tile_index_to_world(IVec2::new(1, 1), UVec2::new(16, 16));
    assert_eq!(result, Vec2::new(16.0, 16.0));
}

#[test]
fn tile_index_to_world_negative_tile_returns_negative_position() {
    // Tile (-1, -1) with 16x16 tiles -> world (-16, -16)
    let result = tile_index_to_world(IVec2::new(-1, -1), UVec2::new(16, 16));
    assert_eq!(result, Vec2::new(-16.0, -16.0));
}

#[test]
fn tile_index_to_world_with_non_square_tiles() {
    // Tile (2, 3) with 16x8 tiles -> world (32, 24)
    let result = tile_index_to_world(IVec2::new(2, 3), UVec2::new(16, 8));
    assert_eq!(result, Vec2::new(32.0, 24.0));
}

// ============================================================================
// snap_to_grid tests
// ============================================================================

#[test]
fn snap_to_grid_already_snapped_returns_same() {
    let result = snap_to_grid(Vec2::new(16.0, 32.0), UVec2::new(16, 16));
    assert_eq!(result, Vec2::new(16.0, 32.0));
}

#[test]
fn snap_to_grid_mid_cell_snaps_down() {
    // Position (17.5, 25.3) snaps to (16, 16) for 16x16 grid
    let result = snap_to_grid(Vec2::new(17.5, 25.3), UVec2::new(16, 16));
    assert_eq!(result, Vec2::new(16.0, 16.0));
}

#[test]
fn snap_to_grid_at_origin_returns_origin() {
    let result = snap_to_grid(Vec2::ZERO, UVec2::new(16, 16));
    assert_eq!(result, Vec2::ZERO);
}

#[test]
fn snap_to_grid_negative_position_snaps_down() {
    // Position (-5, -12) snaps to (-16, -16) for 16x16 grid
    let result = snap_to_grid(Vec2::new(-5.0, -12.0), UVec2::new(16, 16));
    assert_eq!(result, Vec2::new(-16.0, -16.0));
}

#[test]
fn snap_to_grid_negative_far_position() {
    // Position (-20, -40) snaps to (-32, -48) for 16x16 grid
    let result = snap_to_grid(Vec2::new(-20.0, -40.0), UVec2::new(16, 16));
    assert_eq!(result, Vec2::new(-32.0, -48.0));
}

#[test]
fn snap_to_grid_with_non_square_grid() {
    // Position (25, 12) with 16x8 grid snaps to (16, 8)
    let result = snap_to_grid(Vec2::new(25.0, 12.0), UVec2::new(16, 8));
    assert_eq!(result, Vec2::new(16.0, 8.0));
}

// ============================================================================
// Roundtrip tests
// ============================================================================

#[test]
fn roundtrip_tile_index_to_world_to_tile_index() {
    let tile = IVec2::new(5, 3);
    let tile_size = UVec2::new(16, 16);
    let world = tile_index_to_world(tile, tile_size);
    let back = world_to_tile_index(world, tile_size);
    assert_eq!(back, tile);
}

#[test]
fn roundtrip_negative_tile_index() {
    let tile = IVec2::new(-3, -7);
    let tile_size = UVec2::new(8, 8);
    let world = tile_index_to_world(tile, tile_size);
    let back = world_to_tile_index(world, tile_size);
    assert_eq!(back, tile);
}

#[test]
fn snap_equals_tile_index_to_world_of_world_to_tile_index() {
    // snap_to_grid(pos) should equal tile_index_to_world(world_to_tile_index(pos))
    let pos = Vec2::new(25.7, 13.2);
    let tile_size = UVec2::new(16, 16);

    let snapped = snap_to_grid(pos, tile_size);
    let via_tile = tile_index_to_world(world_to_tile_index(pos, tile_size), tile_size);

    assert_eq!(snapped, via_tile);
}
