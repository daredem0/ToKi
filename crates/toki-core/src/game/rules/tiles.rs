//! Tile-related rule utilities.
//!
//! Contains functions for checking entity overlap with trigger tiles.

use crate::assets::tilemap::TileMap;
use crate::assets::tileset::TileSetResolver;
use crate::entity::Entity;

#[cfg(test)]
use super::GameState;
use super::RuleEvaluationService;

impl RuleEvaluationService<'_> {
    pub(in crate::game) fn any_entity_overlaps_trigger_tile(
        &self,
        tilemap: &TileMap,
        tileset: &TileSetResolver<'_>,
    ) -> bool {
        for entity_id in self.world.entity_manager.active_entities_iter() {
            let Some(entity) = self.world.entity_manager.get_entity(entity_id) else {
                continue;
            };
            if Self::entity_overlaps_trigger_tile(entity, tilemap, tileset) {
                return true;
            }
        }
        false
    }

    fn entity_overlaps_trigger_tile(
        entity: &Entity,
        tilemap: &TileMap,
        tileset: &TileSetResolver<'_>,
    ) -> bool {
        if tilemap.tile_size.x == 0 || tilemap.tile_size.y == 0 {
            return false;
        }
        if tilemap.size.x == 0 || tilemap.size.y == 0 {
            return false;
        }

        let (box_pos, box_size) = if let Some(collision_box) = &entity.collision_box {
            collision_box.world_bounds(entity.position)
        } else {
            (entity.position, entity.size)
        };
        if box_size.x == 0 || box_size.y == 0 {
            return false;
        }

        let tile_w = tilemap.tile_size.x as i32;
        let tile_h = tilemap.tile_size.y as i32;

        let tile_range = calculate_tile_range(box_pos, box_size, tile_w, tile_h, tilemap);

        for y in tile_range.min_y..=tile_range.max_y {
            for x in tile_range.min_x..=tile_range.max_x {
                let Ok(_tile_name) = tilemap.get_tile_name(x, y) else {
                    continue;
                };
                if tileset.is_tile_trigger_at(tilemap, x, y).unwrap_or(false) {
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl GameState {
    pub(in crate::game) fn any_entity_overlaps_trigger_tile(
        &mut self,
        tilemap: &TileMap,
        tileset: &TileSetResolver<'_>,
    ) -> bool {
        self.rule_evaluation_service()
            .any_entity_overlaps_trigger_tile(tilemap, tileset)
    }
}

struct TileRange {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

fn calculate_tile_range(
    box_pos: glam::IVec2,
    box_size: glam::UVec2,
    tile_w: i32,
    tile_h: i32,
    tilemap: &TileMap,
) -> TileRange {
    let tile_min_x = (box_pos.x / tile_w).max(0) as u32;
    let tile_min_y = (box_pos.y / tile_h).max(0) as u32;
    let tile_max_x = ((box_pos.x + box_size.x as i32 - 1) / tile_w).max(0) as u32;
    let tile_max_y = ((box_pos.y + box_size.y as i32 - 1) / tile_h).max(0) as u32;

    let map_max_x = tilemap.size.x.saturating_sub(1);
    let map_max_y = tilemap.size.y.saturating_sub(1);

    TileRange {
        min_x: tile_min_x.min(map_max_x),
        min_y: tile_min_y.min(map_max_y),
        max_x: tile_max_x.min(map_max_x),
        max_y: tile_max_y.min(map_max_y),
    }
}
