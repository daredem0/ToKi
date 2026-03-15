use glam::{UVec2, Vec2};
use toki_core::assets::tilemap::TileMap;

pub struct MapPaintInteraction;

impl MapPaintInteraction {
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
}
