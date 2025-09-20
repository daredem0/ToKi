use crate::animation::{AnimationClip, AnimationController, AnimationState, LoopMode};
use crate::collision::CollisionBox;
use glam::{IVec2, UVec2};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub type EntityId = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub position: glam::IVec2,
    pub size: glam::UVec2,
    pub entity_type: EntityType,
    pub attributes: EntityAttributes,
    pub collision_box: Option<CollisionBox>,

    /// Audio state tracking
    pub footstep_distance_accumulator: f32,
    pub footstep_trigger_distance: f32,
    pub last_collision_state: bool,
    
    /// Audio configuration from entity definition
    #[serde(default)]
    pub movement_sound: Option<String>, // Sound to play when moving
}

#[derive(Debug, Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum EntityType {
    Player,
    Npc,
    Item,
    Decoration,
    Trigger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAttributes {
    // Core gameplay
    pub health: Option<u32>,
    pub speed: u32,  // We only move in full pixels
    pub solid: bool, // Can we collide with other entities

    // Rendering
    pub visible: bool, // Can we be seen by the player
    pub animation_controller: Option<AnimationController>, // takes care of all animations of the entity
    pub render_layer: i32,                                 // lower layers are drawn first

    // Behavior flags
    pub active: bool,
    pub can_move: bool, // Can we be moved by the player
    
    // Extended attributes for entity definitions
    #[serde(default)]
    pub has_inventory: bool, // Can this entity carry items
}

impl Default for EntityAttributes {
    fn default() -> Self {
        Self {
            health: None,
            speed: 2,
            solid: true,
            visible: true,
            animation_controller: None,
            render_layer: 0,
            active: true,
            can_move: true,
            has_inventory: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
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

    /// Update animations for all entities
    pub fn update_animations(&mut self, delta_time_ms: f32) {
        for entity in self.entities.values_mut() {
            if let Some(animation_controller) = &mut entity.attributes.animation_controller {
                animation_controller.update(delta_time_ms);
            }
        }
    }

    pub fn spawn_entity(
        &mut self,
        entity_type: EntityType,
        position: IVec2,
        size: UVec2,
        attributes: EntityAttributes,
    ) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        // Create a default collision box for solid entities
        let collision_box = if attributes.solid {
            Some(CollisionBox::solid_box(size))
        } else {
            None
        };

        let entity = Entity {
            id,
            position,
            size,
            entity_type: entity_type.clone(),
            attributes,
            collision_box,
            footstep_distance_accumulator: 0.0,
            footstep_trigger_distance: 32.0, // Trigger footstep every 32 pixels
            last_collision_state: false,
            movement_sound: None, // Default to no sound for existing entities
        };

        // Insert into main storage
        self.entities.insert(id, entity);

        // Update lookup tables
        if matches!(entity_type, EntityType::Player) {
            self.player_id = Some(id);
        }

        self.entities_by_type
            .entry(entity_type)
            .or_default()
            .insert(id);

        if self.entities.get(&id).unwrap().attributes.active {
            self.active_entities.insert(id);
        }

        id
    }
    
    /// Add an existing entity to the manager (used for scene-to-gamestate conversion)
    pub fn add_existing_entity(&mut self, entity: Entity) -> EntityId {
        let id = entity.id;
        let entity_type = entity.entity_type.clone();
        
        // Update next_id if needed to avoid conflicts
        if id >= self.next_id {
            self.next_id = id + 1;
        }
        
        // Track player entity
        if matches!(entity_type, EntityType::Player) && self.player_id.is_none() {
            self.player_id = Some(id);
        }
        
        // Update lookups
        self.entities_by_type
            .entry(entity_type)
            .or_default()
            .insert(id);
            
        self.active_entities.insert(id);
        
        // Store the entity
        self.entities.insert(id, entity);
        
        tracing::debug!("Added existing entity {} to EntityManager", id);
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
    pub fn spawn_player(&mut self, position: IVec2) -> EntityId {
        let mut controller = AnimationController::new();
        let idle_clip = AnimationClip {
            state: AnimationState::Idle,
            atlas_name: "creatures".to_string(),
            frame_tile_names: vec!["slime/idle_0".to_string(), "slime/idle_1".to_string()],
            frame_duration_ms: 300.0,
            loop_mode: LoopMode::Loop,
        };
        controller.add_clip(idle_clip);
        let walk_clip = AnimationClip {
            state: AnimationState::Walk,
            atlas_name: "creatures".to_string(),
            frame_tile_names: vec![
                "slime/walk_0".to_string(),
                "slime/walk_1".to_string(),
                "slime/walk_2".to_string(),
                "slime/walk_3".to_string(),
            ],
            frame_duration_ms: 150.0,
            loop_mode: LoopMode::Loop,
        };
        controller.add_clip(walk_clip);
        controller.play(AnimationState::Idle);
        let attributes = EntityAttributes {
            health: Some(100),
            speed: 2,
            animation_controller: Some(controller),
            ..Default::default()
        };
        self.spawn_entity(
            EntityType::Player,
            position,
            glam::UVec2::new(16, 16),
            attributes,
        )
    }

    pub fn spawn_npc(&mut self, position: glam::IVec2, animation_name: &str) -> EntityId {
        let mut controller = AnimationController::new();
        let idle_clip = AnimationClip {
            state: AnimationState::Walk,
            atlas_name: "creatures".to_string(),
            frame_tile_names: vec![
                format!("{}/walk_0", animation_name),
                format!("{}/walk_1", animation_name),
                format!("{}/walk_2", animation_name),
                format!("{}/walk_3", animation_name),
            ],
            frame_duration_ms: 150.0,
            loop_mode: LoopMode::Loop,
        };
        controller.add_clip(idle_clip);
        controller.play(AnimationState::Walk);
        let attributes = EntityAttributes {
            health: Some(50),
            speed: 1,
            can_move: false, // NPCs don't move by themselves
            animation_controller: Some(controller),
            ..Default::default()
        };
        self.spawn_entity(
            EntityType::Npc,
            position,
            glam::UVec2::new(16, 16),
            attributes,
        )
    }

    pub fn spawn_item(&mut self, position: IVec2, item_name: &str) -> EntityId {
        let mut controller = AnimationController::new();
        let idle_clip = AnimationClip {
            state: AnimationState::Idle,
            atlas_name: "objects".to_string(),
            frame_tile_names: vec![
                format!("{}_0", item_name),
                format!("{}_1", item_name),
                format!("{}_2", item_name),
                format!("{}_3", item_name),
            ],
            frame_duration_ms: 150.0,
            loop_mode: LoopMode::Loop,
        };
        controller.add_clip(idle_clip);
        controller.play(AnimationState::Idle);
        let attributes = EntityAttributes {
            health: None,    // Items don't have health
            solid: false,    // Items can be walked through
            can_move: false, // Items don't move
            animation_controller: Some(controller),
            ..Default::default()
        };

        self.spawn_entity(EntityType::Item, position, UVec2::new(16, 16), attributes)
    }

    pub fn spawn_decoration(&mut self, position: IVec2, decoration_name: &str) -> EntityId {
        let mut controller = AnimationController::new();
        let idle_clip = AnimationClip {
            state: AnimationState::Idle,
            atlas_name: "terrain".to_string(),
            frame_tile_names: vec![
                format!("{}_0", decoration_name),
                format!("{}_1", decoration_name),
                format!("{}_2", decoration_name),
                format!("{}_3", decoration_name),
            ],
            frame_duration_ms: 150.0,
            loop_mode: LoopMode::Loop,
        };
        controller.add_clip(idle_clip);
        controller.play(AnimationState::Idle);
        let attributes: EntityAttributes = EntityAttributes {
            health: None,
            solid: false, // Decorations don't block movement
            can_move: false,
            animation_controller: Some(controller),
            render_layer: -1, // Decorations render behind other entities
            ..Default::default()
        };

        self.spawn_entity(
            EntityType::Decoration,
            position,
            UVec2::new(16, 16),
            attributes,
        )
    }
}

impl Default for EntityManager {
    fn default() -> Self {
        Self::new()
    }
}

// Entity Definition JSON format structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDefinition {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub entity_type: String, // "player", "npc", "item", "decoration", "trigger"
    pub rendering: RenderingDef,
    pub attributes: AttributesDef,
    pub collision: CollisionDef,
    pub audio: AudioDef,
    pub animations: AnimationsDef,
    pub category: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingDef {
    pub size: [u32; 2],
    pub render_layer: i32,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributesDef {
    pub health: Option<u32>,
    pub speed: u32,
    pub solid: bool,
    pub active: bool,
    pub can_move: bool,
    pub has_inventory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionDef {
    pub enabled: bool,
    pub offset: [i32; 2],
    pub size: [u32; 2],
    pub trigger: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDef {
    pub footstep_trigger_distance: f32,
    pub movement_sound: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationsDef {
    pub atlas_name: String,
    pub clips: Vec<AnimationClipDef>,
    pub default_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationClipDef {
    pub state: String,
    pub frame_tiles: Vec<String>,
    pub frame_duration_ms: f32,
    pub loop_mode: String, // "loop", "once", "ping_pong"
}

// Conversion implementations
impl EntityDefinition {
    /// Create an Entity instance from this definition at the given position
    pub fn create_entity(&self, position: IVec2, entity_id: EntityId) -> Result<Entity, String> {
        // Parse entity type
        let entity_type = match self.entity_type.to_lowercase().as_str() {
            "player" => EntityType::Player,
            "npc" => EntityType::Npc,
            "item" => EntityType::Item,
            "decoration" => EntityType::Decoration,
            "trigger" => EntityType::Trigger,
            _ => return Err(format!("Unknown entity type: {}", self.entity_type)),
        };

        // Build animation controller
        let mut animation_controller = AnimationController::new();
        for clip_def in &self.animations.clips {
            let state = match clip_def.state.to_lowercase().as_str() {
                "idle" => AnimationState::Idle,
                "walk" => AnimationState::Walk,
                _ => return Err(format!("Unknown animation state: {}", clip_def.state)),
            };

            let loop_mode = match clip_def.loop_mode.to_lowercase().as_str() {
                "loop" => LoopMode::Loop,
                "once" => LoopMode::Once,
                "ping_pong" => LoopMode::PingPong,
                _ => return Err(format!("Unknown loop mode: {}", clip_def.loop_mode)),
            };

            let clip = AnimationClip {
                state,
                atlas_name: self.animations.atlas_name.clone(),
                frame_tile_names: clip_def.frame_tiles.clone(),
                frame_duration_ms: clip_def.frame_duration_ms,
                loop_mode,
            };
            animation_controller.add_clip(clip);
        }

        // Set default animation state
        let default_state = match self.animations.default_state.to_lowercase().as_str() {
            "idle" => AnimationState::Idle,
            "walk" => AnimationState::Walk,
            _ => return Err(format!("Unknown default animation state: {}", self.animations.default_state)),
        };
        animation_controller.play(default_state);

        // Build attributes
        let attributes = EntityAttributes {
            health: self.attributes.health,
            speed: self.attributes.speed,
            solid: self.attributes.solid,
            visible: self.rendering.visible,
            animation_controller: Some(animation_controller),
            render_layer: self.rendering.render_layer,
            active: self.attributes.active,
            can_move: self.attributes.can_move,
            has_inventory: self.attributes.has_inventory,
        };

        // Build collision box if enabled
        let collision_box = if self.collision.enabled {
            Some(CollisionBox::new(
                IVec2::new(self.collision.offset[0], self.collision.offset[1]),
                UVec2::new(self.collision.size[0], self.collision.size[1]),
                self.collision.trigger,
            ))
        } else {
            None
        };

        // Create entity
        Ok(Entity {
            id: entity_id,
            position,
            size: UVec2::new(self.rendering.size[0], self.rendering.size[1]),
            entity_type,
            attributes,
            collision_box,
            footstep_distance_accumulator: 0.0,
            footstep_trigger_distance: self.audio.footstep_trigger_distance,
            last_collision_state: false,
            movement_sound: Some(self.audio.movement_sound.clone()),
        })
    }
}
