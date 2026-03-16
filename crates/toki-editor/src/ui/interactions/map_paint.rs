use glam::{UVec2, Vec2};
use toki_core::assets::tilemap::TileMap;

pub struct MapPaintInteraction;

impl MapPaintInteraction {
    pub fn brush_footprint_bounds(
        tilemap: &TileMap,
        center_tile_pos: UVec2,
        brush_size_tiles: u32,
    ) -> Option<(UVec2, UVec2)> {
        if center_tile_pos.x >= tilemap.size.x || center_tile_pos.y >= tilemap.size.y {
            return None;
        }

        let brush_size = brush_size_tiles.max(1);
        let radius = (brush_size - 1) / 2;
        let start_x = center_tile_pos.x.saturating_sub(radius);
        let start_y = center_tile_pos.y.saturating_sub(radius);
        let end_x = (start_x + brush_size).min(tilemap.size.x);
        let end_y = (start_y + brush_size).min(tilemap.size.y);
        Some((UVec2::new(start_x, start_y), UVec2::new(end_x, end_y)))
    }

    pub fn tile_position_at_world(tilemap: &TileMap, world_pos: Vec2) -> Option<UVec2> {
        if world_pos.x < 0.0 || world_pos.y < 0.0 {
            return None;
        }

        let tile_x = (world_pos.x / tilemap.tile_size.x as f32).floor() as u32;
        let tile_y = (world_pos.y / tilemap.tile_size.y as f32).floor() as u32;

        if tile_x >= tilemap.size.x || tile_y >= tilemap.size.y {
            return None;
        }

        Some(UVec2::new(tile_x, tile_y))
    }

    pub fn paint_tile(tilemap: &mut TileMap, tile_pos: UVec2, tile_name: &str) -> bool {
        if tile_pos.x >= tilemap.size.x || tile_pos.y >= tilemap.size.y {
            return false;
        }

        let index = (tile_pos.y * tilemap.size.x + tile_pos.x) as usize;
        if tilemap
            .tiles
            .get(index)
            .is_some_and(|current| current == tile_name)
        {
            return false;
        }

        if let Some(slot) = tilemap.tiles.get_mut(index) {
            *slot = tile_name.to_string();
            return true;
        }

        false
    }

    pub fn paint_brush(
        tilemap: &mut TileMap,
        center_tile_pos: UVec2,
        tile_name: &str,
        brush_size_tiles: u32,
    ) -> bool {
        let Some((start, end)) =
            Self::brush_footprint_bounds(tilemap, center_tile_pos, brush_size_tiles)
        else {
            return false;
        };

        let mut changed = false;
        for y in start.y..end.y {
            for x in start.x..end.x {
                changed |= Self::paint_tile(tilemap, UVec2::new(x, y), tile_name);
            }
        }

        changed
    }

    pub fn fill_all(tilemap: &mut TileMap, tile_name: &str) -> bool {
        let mut changed = false;
        for slot in &mut tilemap.tiles {
            if slot != tile_name {
                *slot = tile_name.to_string();
                changed = true;
            }
        }
        changed
    }
}

#[cfg(test)]
#[path = "map_paint_tests.rs"]
mod tests;
