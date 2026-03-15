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
mod tests {
    use super::MapPaintInteraction;
    use glam::{UVec2, Vec2};
    use std::path::PathBuf;
    use toki_core::assets::tilemap::TileMap;

    fn sample_tilemap() -> TileMap {
        TileMap {
            size: UVec2::new(3, 2),
            tile_size: UVec2::new(8, 8),
            atlas: PathBuf::from("terrain.json"),
            tiles: vec![
                "grass".to_string(),
                "grass".to_string(),
                "grass".to_string(),
                "water".to_string(),
                "water".to_string(),
                "water".to_string(),
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
            UVec2::new(1, 0),
            "bush"
        ));
        assert_eq!(tilemap.tiles[1], "bush");
        assert!(!MapPaintInteraction::paint_tile(
            &mut tilemap,
            UVec2::new(1, 0),
            "bush"
        ));
    }

    #[test]
    fn paint_brush_paints_square_area_and_clips_to_map_bounds() {
        let mut tilemap = sample_tilemap();

        assert!(MapPaintInteraction::paint_brush(
            &mut tilemap,
            UVec2::new(1, 0),
            "bush",
            2
        ));

        assert_eq!(tilemap.tiles[1], "bush");
        assert_eq!(tilemap.tiles[2], "bush");
        assert_eq!(tilemap.tiles[4], "bush");
        assert_eq!(tilemap.tiles[5], "bush");
        assert_eq!(tilemap.tiles[0], "grass");
        assert_eq!(tilemap.tiles[3], "water");
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

        assert!(MapPaintInteraction::fill_all(&mut tilemap, "bush"));
        assert!(tilemap.tiles.iter().all(|tile| tile == "bush"));
        assert!(!MapPaintInteraction::fill_all(&mut tilemap, "bush"));
    }
}
