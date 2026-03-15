use toki_core::assets::tilemap::{MapObjectInstance, TileMap};

pub struct MapObjectInteraction;

impl MapObjectInteraction {
    pub fn object_anchor_at_world(tilemap: &TileMap, world_pos: glam::Vec2) -> Option<glam::UVec2> {
        if world_pos.x < 0.0 || world_pos.y < 0.0 {
            return None;
        }

        let tile_x = (world_pos.x / tilemap.tile_size.x as f32).floor() as u32;
        let tile_y = (world_pos.y / tilemap.tile_size.y as f32).floor() as u32;
        let tile_pos = glam::UVec2::new(tile_x, tile_y);
        tilemap.tile_to_world(tile_pos)
    }

    pub fn place_object(
        tilemap: &mut TileMap,
        world_anchor: glam::UVec2,
        sheet: &str,
        object_name: &str,
    ) -> bool {
        let instance = MapObjectInstance {
            sheet: std::path::PathBuf::from(sheet),
            object_name: object_name.to_string(),
            position: world_anchor,
        };
        tilemap.objects.push(instance);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::MapObjectInteraction;
    use glam::UVec2;
    use std::path::PathBuf;
    use toki_core::assets::tilemap::{MapObjectInstance, TileMap};

    fn sample_tilemap() -> TileMap {
        TileMap {
            size: UVec2::new(4, 4),
            tile_size: UVec2::new(16, 16),
            atlas: PathBuf::from("terrain.json"),
            tiles: vec!["grass".to_string(); 16],
            objects: vec![],
        }
    }

    #[test]
    fn object_anchor_at_world_snaps_to_tile_grid() {
        let tilemap = sample_tilemap();

        assert_eq!(
            MapObjectInteraction::object_anchor_at_world(&tilemap, glam::Vec2::new(23.9, 31.9)),
            Some(UVec2::new(16, 16))
        );
    }

    #[test]
    fn place_object_appends_map_object_instance() {
        let mut tilemap = sample_tilemap();

        let changed =
            MapObjectInteraction::place_object(&mut tilemap, UVec2::new(16, 32), "fauna.json", "fauna_a");

        assert!(changed);
        assert_eq!(
            tilemap.objects,
            vec![MapObjectInstance {
                sheet: PathBuf::from("fauna.json"),
                object_name: "fauna_a".to_string(),
                position: UVec2::new(16, 32),
            }]
        );
    }
}
