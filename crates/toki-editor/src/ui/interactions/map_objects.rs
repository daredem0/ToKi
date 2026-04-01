use toki_core::assets::tilemap::TileMap;
use toki_core::entity::{Entity, EntityId};
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

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn object_entity_at_world<'a>(
        entities: impl IntoIterator<Item = &'a Entity>,
        world_pos: glam::Vec2,
    ) -> Option<EntityId> {
        if world_pos.x < 0.0 || world_pos.y < 0.0 {
            return None;
        }

        let world_point = glam::IVec2::new(world_pos.x.floor() as i32, world_pos.y.floor() as i32);
        let mut decorations = entities
            .into_iter()
            .filter(|entity| {
                entity.rendering.visible && entity.rendering.static_object_render.is_some()
            })
            .collect::<Vec<_>>();
        decorations.sort_by_key(|entity| {
            (
                entity.ground_contact_y(),
                entity.rendering.render_layer,
                entity.id,
            )
        });
        decorations
            .iter()
            .rev()
            .find(|entity| {
                toki_core::collision::aabb_overlap(
                    world_point,
                    glam::UVec2::new(1, 1),
                    entity.position,
                    entity.size,
                )
            })
            .map(|entity| entity.id)
    }
}

#[cfg(test)]
#[path = "map_objects_tests.rs"]
mod tests;
