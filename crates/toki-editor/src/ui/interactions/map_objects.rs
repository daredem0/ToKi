use toki_core::assets::tilemap::{MapObjectInstance, TileMap};
use toki_core::math::coordinates::world_to_tile_index;

pub struct MapObjectInteraction;

impl MapObjectInteraction {
    pub fn object_anchor_at_world(tilemap: &TileMap, world_pos: glam::Vec2) -> Option<glam::UVec2> {
        let tile_index = world_to_tile_index(world_pos, tilemap.tile_size);
        if tile_index.x < 0 || tile_index.y < 0 {
            return None;
        }
        tilemap.tile_to_world(tile_index.as_uvec2())
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
