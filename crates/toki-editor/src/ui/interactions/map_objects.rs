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
#[path = "map_objects_tests.rs"]
mod tests;
