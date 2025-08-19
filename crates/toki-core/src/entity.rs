use glam::Vec2;
use std::collections::{HashMap, HashSet};

pub type EntityId = u32;

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: EntityId,
    pub position: glam::Vec2,
    pub size: glam::Vec2,
    pub entity_type: EntityType,
    pub attributes: EntityAttributes,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum EntityType {
    Player,
    Npc,
    Item,
    Decoration,
    Trigger,
}

#[derive(Debug, Clone)]
pub struct EntityAttributes {
    // Core gameplay
    pub health: Option<u32>,
    pub speed: u32,  // We only move in full pixels
    pub solid: bool, // Can we collide with other entities

    // Rendering
    pub visible: bool, // Can we be seen by the player
    pub sprite_info: Option<SpriteInfo>,
    pub render_layer: i32, // lower layers are drawn first

    // Behavior flags
    pub active: bool,
    pub can_move: bool, // Can we be moved by the player
}

#[derive(Debug, Clone)]
pub struct SpriteInfo {
    pub atlas_name: String,
    pub animation_name: String,
    pub current_frame: usize,
    pub frame_timer: f32,
}

impl Default for EntityAttributes {
    fn default() -> Self {
        Self {
            health: None,
            speed: 2,
            solid: true,
            visible: true,
            sprite_info: None,
            render_layer: 0,
            active: true,
            can_move: true,
        }
    }
}

#[derive(Debug)]
pub struct EntityManager {
    entities: HashMap<EntityId, Entity>,
    next_id: EntityId,

    // Quick lookups
    player_id: Option<EntityId>,
    entities_by_type: HashMap<EntityType, HashSet<EntityId>>,

    // This is prepared for spatial queries (collission)
    active_entities: HashSet<EntityId>,
}

impl EntityManager {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_id: 1, // we start at 1 to use 0 for invalid entities
            player_id: None,
            entities_by_type: HashMap::new(),
            active_entities: HashSet::new(),
        }
    }

    pub fn spawn_entity(
        &mut self,
        entity_type: EntityType,
        position: Vec2,
        size: Vec2,
        attributes: EntityAttributes,
    ) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        let entity = Entity {
            id,
            position,
            size,
            entity_type: entity_type.clone(),
            attributes,
        };

        // Insert into main storage
        self.entities.insert(id, entity);

        // Update lookup tables
        if matches!(entity_type, EntityType::Player) {
            self.player_id = Some(id);
        }

        self.entities_by_type
            .entry(entity_type)
            .or_insert_with(HashSet::new)
            .insert(id);

        if self.entities.get(&id).unwrap().attributes.active {
            self.active_entities.insert(id);
        }

        id
    }

    pub fn despawn_entity(&mut self, id: EntityId) -> bool {
        let Some(entity) = self.entities.remove(&id) else {
            return false;
        };

        // Clean up lookup tables
        if self.player_id.is_some_and(|pid| pid == id) {
            self.player_id = None;
        }

        if let Some(type_set) = self.entities_by_type.get_mut(&entity.entity_type) {
            type_set.remove(&id);
        }

        // We don't care whether it was present; just ensure it's gone.
        self.active_entities.remove(&id);

        true
    }

    // Basic getters
    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    // Convenience methods
    pub fn get_player(&self) -> Option<&Entity> {
        self.player_id.and_then(|id| self.entities.get(&id))
    }

    pub fn get_player_mut(&mut self) -> Option<&mut Entity> {
        self.player_id.and_then(|id| self.entities.get_mut(&id))
    }

    pub fn get_player_id(&self) -> Option<EntityId> {
        self.player_id
    }

    // Queries
    pub fn entities_of_type(&self, entity_type: &EntityType) -> Vec<EntityId> {
        self.entities_by_type
            .get(entity_type)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn active_entities(&self) -> Vec<EntityId> {
        self.active_entities.iter().copied().collect()
    }

    pub fn visible_entities(&self) -> Vec<EntityId> {
        self.entities
            .iter()
            .filter(|(_, entity)| entity.attributes.visible)
            .map(|(id, _)| *id)
            .collect()
    }

    // Update entity active status
    pub fn set_entity_active(&mut self, id: EntityId, active: bool) {
        if let Some(entity) = self.entities.get_mut(&id) {
            let was_active = entity.attributes.active;
            entity.attributes.active = active;
            // Update active_entities set
            if active && !was_active {
                self.active_entities.insert(id);
            } else if !active && was_active {
                self.active_entities.remove(&id);
            }
        }
    }

    // Factory methods
    pub fn spawn_player(&mut self, position: Vec2) -> EntityId {
        let attributes = EntityAttributes {
            health: Some(100),
            speed: 2,
            sprite_info: Some(SpriteInfo {
                atlas_name: "creatures".to_string(),
                animation_name: "player_idle".to_string(),
                current_frame: 0,
                frame_timer: 0.0,
            }),
            ..Default::default()
        };
        self.spawn_entity(
            EntityType::Player,
            position,
            glam::Vec2::new(16.0, 16.0),
            attributes,
        )
    }

    pub fn spawn_npc(&mut self, position: glam::Vec2, animation_name: &str) -> EntityId {
        let attributes = EntityAttributes {
            health: Some(50),
            speed: 1,
            can_move: false,
            sprite_info: Some(SpriteInfo {
                atlas_name: "creatures".to_string(),
                animation_name: animation_name.to_string(),
                current_frame: 0,
                frame_timer: 0.0,
            }),
            ..Default::default()
        };
        self.spawn_entity(
            EntityType::Npc,
            position,
            glam::Vec2::new(16.0, 16.0),
            attributes,
        )
    }

    pub fn spawn_item(&mut self, position: Vec2, item_name: &str) -> EntityId {
        let attributes = EntityAttributes {
            health: None,    // Items don't have health
            solid: false,    // Items can be walked through
            can_move: false, // Items don't move
            sprite_info: Some(SpriteInfo {
                atlas_name: "objects".to_string(),
                animation_name: item_name.to_string(),
                current_frame: 0,
                frame_timer: 0.0,
            }),
            ..Default::default()
        };

        self.spawn_entity(
            EntityType::Item,
            position,
            Vec2::new(16.0, 16.0),
            attributes,
        )
    }

    pub fn spawn_decoration(&mut self, position: Vec2, decoration_name: &str) -> EntityId {
        let attributes = EntityAttributes {
            health: None,
            solid: false, // Decorations don't block movement
            can_move: false,
            sprite_info: Some(SpriteInfo {
                atlas_name: "terrain".to_string(),
                animation_name: decoration_name.to_string(),
                current_frame: 0,
                frame_timer: 0.0,
            }),
            render_layer: -1, // Decorations render behind other entities
            ..Default::default()
        };

        self.spawn_entity(
            EntityType::Decoration,
            position,
            Vec2::new(16.0, 16.0),
            attributes,
        )
    }
}

impl Default for EntityManager {
    fn default() -> Self {
        Self::new()
    }
}
