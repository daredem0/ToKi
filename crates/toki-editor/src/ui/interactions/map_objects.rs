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
        size_px: glam::UVec2,
    ) -> bool {
        let instance = MapObjectInstance {
            sheet: std::path::PathBuf::from(sheet),
            object_name: object_name.to_string(),
            position: world_anchor,
            size_px,
            visible: true,
            solid: true,
        };
        tilemap.objects.push(instance);
        true
    }

    pub fn object_index_at_world(tilemap: &TileMap, world_pos: glam::Vec2) -> Option<usize> {
        if world_pos.x < 0.0 || world_pos.y < 0.0 {
            return None;
        }

        let world_point = glam::IVec2::new(world_pos.x.floor() as i32, world_pos.y.floor() as i32);
        tilemap
            .objects
            .iter()
            .enumerate()
            .rev()
            .find(|(_, object)| {
                if !object.visible {
                    return false;
                }
                let object_pos = object.position.as_ivec2();
                toki_core::collision::aabb_overlap(
                    world_point,
                    glam::UVec2::new(1, 1),
                    object_pos,
                    object.size_px,
                )
            })
            .map(|(index, _)| index)
    }

    pub fn move_object(
        tilemap: &mut TileMap,
        object_index: usize,
        world_anchor: glam::UVec2,
    ) -> bool {
        let Some(object) = tilemap.objects.get_mut(object_index) else {
            return false;
        };
        if object.position == world_anchor {
            return false;
        }
        object.position = world_anchor;
        true
    }

    pub fn delete_object(tilemap: &mut TileMap, object_index: usize) -> bool {
        if object_index >= tilemap.objects.len() {
            return false;
        }
        tilemap.objects.remove(object_index);
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

        let changed = MapObjectInteraction::place_object(
            &mut tilemap,
            UVec2::new(16, 32),
            "fauna.json",
            "fauna_a",
            UVec2::new(16, 16),
        );

        assert!(changed);
        assert_eq!(
            tilemap.objects,
            vec![MapObjectInstance {
                sheet: PathBuf::from("fauna.json"),
                object_name: "fauna_a".to_string(),
                position: UVec2::new(16, 32),
                size_px: UVec2::new(16, 16),
                visible: true,
                solid: true,
            }]
        );
    }

    #[test]
    fn object_index_at_world_prefers_last_placed_object() {
        let mut tilemap = sample_tilemap();
        tilemap.objects = vec![
            MapObjectInstance {
                sheet: PathBuf::from("fauna.json"),
                object_name: "first".to_string(),
                position: UVec2::new(16, 16),
                size_px: UVec2::new(16, 16),
                visible: true,
                solid: true,
            },
            MapObjectInstance {
                sheet: PathBuf::from("fauna.json"),
                object_name: "second".to_string(),
                position: UVec2::new(16, 16),
                size_px: UVec2::new(16, 16),
                visible: true,
                solid: true,
            },
        ];

        assert_eq!(
            MapObjectInteraction::object_index_at_world(&tilemap, glam::Vec2::new(20.0, 20.0)),
            Some(1)
        );
    }

    #[test]
    fn move_object_updates_selected_map_object_position() {
        let mut tilemap = sample_tilemap();
        MapObjectInteraction::place_object(
            &mut tilemap,
            UVec2::new(16, 16),
            "fauna.json",
            "fauna_a",
            UVec2::new(16, 16),
        );

        assert!(MapObjectInteraction::move_object(
            &mut tilemap,
            0,
            UVec2::new(32, 16)
        ));
        assert_eq!(tilemap.objects[0].position, UVec2::new(32, 16));
    }

    #[test]
    fn object_index_at_world_ignores_invisible_objects() {
        let mut tilemap = sample_tilemap();
        tilemap.objects = vec![MapObjectInstance {
            sheet: PathBuf::from("fauna.json"),
            object_name: "hidden".to_string(),
            position: UVec2::new(16, 16),
            size_px: UVec2::new(16, 16),
            visible: false,
            solid: true,
        }];

        assert_eq!(
            MapObjectInteraction::object_index_at_world(&tilemap, glam::Vec2::new(20.0, 20.0)),
            None
        );
    }

    #[test]
    fn delete_object_removes_object_at_index() {
        let mut tilemap = sample_tilemap();
        MapObjectInteraction::place_object(
            &mut tilemap,
            UVec2::new(16, 16),
            "fauna.json",
            "fauna_a",
            UVec2::new(16, 16),
        );

        assert!(MapObjectInteraction::delete_object(&mut tilemap, 0));
        assert!(tilemap.objects.is_empty());
    }
}
