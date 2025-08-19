use glam::{IVec2, UVec2};
use crate::entity::EntityId;

#[derive(Debug, Clone)]
pub struct CollisionBox {
    pub offset: IVec2,    // Offset from entity position
    pub size: UVec2,      // Collision box size (can differ from entity.size)
    pub trigger: bool,    // Fire events but don't block movement
}

#[derive(Debug, Default)]
pub struct CollisionResult {
    pub blocked: bool,
    pub blocking_entities: Vec<EntityId>,
    pub trigger_entities: Vec<EntityId>,
}

impl CollisionBox {
    pub fn new(offset: IVec2, size: UVec2, trigger: bool) -> Self {
        Self { offset, size, trigger }
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
    pos1.x < pos2.x + size2.x as i32 &&
    pos1.x + size1.x as i32 > pos2.x &&
    pos1.y < pos2.y + size2.y as i32 &&
    pos1.y + size1.y as i32 > pos2.y
}
