use glam::{IVec2, UVec2, Vec2};

/// Converts a world position to a tile index.
///
/// Uses floor division to determine which tile contains the given world position.
/// Supports negative world positions, returning negative tile indices as appropriate.
///
/// # Arguments
/// * `world_pos` - Position in world coordinates
/// * `tile_size` - Size of each tile in pixels
///
/// # Returns
/// The tile index (can be negative for negative world positions)
pub fn world_to_tile_index(world_pos: Vec2, tile_size: UVec2) -> IVec2 {
    IVec2::new(
        (world_pos.x / tile_size.x as f32).floor() as i32,
        (world_pos.y / tile_size.y as f32).floor() as i32,
    )
}

/// Converts a tile index to the world position of the tile's top-left corner.
///
/// # Arguments
/// * `tile_index` - The tile index (can be negative)
/// * `tile_size` - Size of each tile in pixels
///
/// # Returns
/// World position of the tile's top-left corner
pub fn tile_index_to_world(tile_index: IVec2, tile_size: UVec2) -> Vec2 {
    Vec2::new(
        tile_index.x as f32 * tile_size.x as f32,
        tile_index.y as f32 * tile_size.y as f32,
    )
}

/// Snaps a world position to the nearest grid cell's top-left corner.
///
/// Equivalent to `tile_index_to_world(world_to_tile_index(pos, grid_size), grid_size)`.
///
/// # Arguments
/// * `world_pos` - Position in world coordinates
/// * `grid_size` - Size of each grid cell in pixels
///
/// # Returns
/// The snapped world position (top-left corner of the containing cell)
pub fn snap_to_grid(world_pos: Vec2, grid_size: UVec2) -> Vec2 {
    Vec2::new(
        (world_pos.x / grid_size.x as f32).floor() * grid_size.x as f32,
        (world_pos.y / grid_size.y as f32).floor() * grid_size.y as f32,
    )
}
