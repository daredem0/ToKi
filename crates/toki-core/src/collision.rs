use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::entity::{Entity, EntityId};
use glam::{IVec2, UVec2};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionBox {
    pub offset: IVec2, // Offset from entity position
    pub size: UVec2,   // Collision box size (can differ from entity.size)
    pub trigger: bool, // Fire events but don't block movement
}

#[derive(Debug, Default)]
pub struct CollisionResult {
    pub blocked: bool,
    pub blocking_entities: Vec<EntityId>,
    pub trigger_entities: Vec<EntityId>,
}

impl CollisionBox {
    pub fn new(offset: IVec2, size: UVec2, trigger: bool) -> Self {
        Self {
            offset,
            size,
            trigger,
        }
    }

    // Create collision box same size as entity
    pub fn solid_box(entity_size: UVec2) -> Self {
        Self::new(IVec2::ZERO, entity_size, false)
    }

    // Create trigger zone
    pub fn trigger_box(size: UVec2) -> Self {
        Self::new(IVec2::ZERO, size, true)
    }

    // Get world position of collision box
    pub fn world_bounds(&self, entity_position: IVec2) -> (IVec2, UVec2) {
        (entity_position + self.offset, self.size)
    }
}

pub fn aabb_overlap(pos1: IVec2, size1: UVec2, pos2: IVec2, size2: UVec2) -> bool {
    pos1.x < pos2.x + size2.x as i32
        && pos1.x + size1.x as i32 > pos2.x
        && pos1.y < pos2.y + size2.y as i32
        && pos1.y + size1.y as i32 > pos2.y
}

/// Check if an entity can move to a specific position without colliding with solid tiles.
/// This is a generalized function that can be used by both the runtime and editor.
///
/// # Arguments
/// * `entity` - The entity to check collision for
/// * `new_position` - The world position to check
/// * `tilemap` - The tilemap to check against
/// * `atlas` - The atlas containing tile collision data
///
/// # Returns
/// * `true` if the entity can move to the position (no collision)
/// * `false` if the movement would result in a collision with solid tiles
pub fn can_entity_move_to_position(
    entity: &Entity,
    new_position: IVec2,
    tilemap: &TileMap,
    atlas: &AtlasMeta,
) -> bool {
    // If entity has no collision box, allow movement
    let Some(collision_box) = &entity.collision_box else {
        return true;
    };

    // Skip collision for trigger boxes (they don't block movement)
    if collision_box.trigger {
        return true;
    }

    // Get the world bounds of the collision box at the new position
    let (box_pos, box_size) = collision_box.world_bounds(new_position);

    // Handle negative coordinates - treat as blocked
    if box_pos.x < 0 || box_pos.y < 0 {
        return false;
    }

    // Convert collision box bounds to tile coordinates
    let tile_size = tilemap.tile_size;
    let min_tile_x = (box_pos.x as u32) / tile_size.x;
    let min_tile_y = (box_pos.y as u32) / tile_size.y;
    let max_tile_x = ((box_pos.x + box_size.x as i32 - 1) as u32) / tile_size.x;
    let max_tile_y = ((box_pos.y + box_size.y as i32 - 1) as u32) / tile_size.y;

    // Check all tiles that the collision box would overlap
    for tile_y in min_tile_y..=max_tile_y {
        for tile_x in min_tile_x..=max_tile_x {
            match tilemap.is_tile_solid_at(atlas, tile_x, tile_y) {
                Ok(is_solid) => {
                    if is_solid {
                        return false;
                    }
                }
                Err(_) => {
                    // Out of bounds or other error - treat as blocking
                    return false;
                }
            }
        }
    }

    true
}

/// Check if a collision box at a specific position would collide with solid tiles.
/// This is a lighter version that doesn't require a full entity, useful for placement validation.
///
/// # Arguments
/// * `collision_box` - The collision box to check (can be None for no collision)
/// * `position` - The world position to check
/// * `tilemap` - The tilemap to check against
/// * `atlas` - The atlas containing tile collision data
///
/// # Returns
/// * `true` if the position is valid (no collision)
/// * `false` if the movement would result in a collision with solid tiles
pub fn can_place_collision_box_at_position(
    collision_box: Option<&CollisionBox>,
    position: IVec2,
    tilemap: &TileMap,
    atlas: &AtlasMeta,
) -> bool {
    // If no collision box, allow placement
    let Some(collision_box) = collision_box else {
        return true;
    };

    // Skip collision for trigger boxes (they don't block placement)
    if collision_box.trigger {
        return true;
    }

    // Get the world bounds of the collision box at the position
    let (box_pos, box_size) = collision_box.world_bounds(position);

    // Handle negative coordinates - treat as blocked
    if box_pos.x < 0 || box_pos.y < 0 {
        return false;
    }

    // Convert collision box bounds to tile coordinates
    let tile_size = tilemap.tile_size;
    let min_tile_x = (box_pos.x as u32) / tile_size.x;
    let min_tile_y = (box_pos.y as u32) / tile_size.y;
    let max_tile_x = ((box_pos.x + box_size.x as i32 - 1) as u32) / tile_size.x;
    let max_tile_y = ((box_pos.y + box_size.y as i32 - 1) as u32) / tile_size.y;

    // Check all tiles that the collision box would overlap
    for tile_y in min_tile_y..=max_tile_y {
        for tile_x in min_tile_x..=max_tile_x {
            match tilemap.is_tile_solid_at(atlas, tile_x, tile_y) {
                Ok(is_solid) => {
                    if is_solid {
                        return false;
                    }
                }
                Err(_) => {
                    // Out of bounds or other error - treat as blocking
                    return false;
                }
            }
        }
    }

    true
}
