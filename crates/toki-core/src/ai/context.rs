//! AI context for bundling commonly-used references.

use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::collision::can_entity_move_to_position;
use crate::entity::{Entity, EntityId, EntityManager};
use glam::{IVec2, UVec2};

/// Context for AI movement operations, grouping related parameters.
///
/// This reduces parameter counts for AI methods by bundling commonly-used
/// references together: entity manager, world bounds, tilemap, and atlas.
#[derive(Clone, Copy)]
pub struct AiContext<'a> {
    pub entity_manager: &'a EntityManager,
    pub world_bounds: UVec2,
    pub tilemap: &'a TileMap,
    pub atlas: &'a AtlasMeta,
}

impl<'a> AiContext<'a> {
    /// Create a new AI context with all required references.
    pub fn new(
        entity_manager: &'a EntityManager,
        world_bounds: UVec2,
        tilemap: &'a TileMap,
        atlas: &'a AtlasMeta,
    ) -> Self {
        Self {
            entity_manager,
            world_bounds,
            tilemap,
            atlas,
        }
    }

    /// Compute maximum position for an entity of the given size.
    /// Returns (max_x, max_y) clamped to at least 0.
    pub fn max_position(&self, entity_size: UVec2) -> (i32, i32) {
        let max_x = (self.world_bounds.x as i32 - entity_size.x as i32).max(0);
        let max_y = (self.world_bounds.y as i32 - entity_size.y as i32).max(0);
        (max_x, max_y)
    }

    /// Check if movement to new position is valid (no collisions with tiles or entities).
    pub fn is_movement_valid(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        new_position: IVec2,
    ) -> bool {
        can_entity_move_to_position(entity, new_position, self.tilemap, self.atlas)
            && !self
                .entity_manager
                .would_collide_with_solid_entity(entity_id, new_position)
    }
}
