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
    /// Source entity definition name used to instantiate this entity.
    /// This lets editor workflows (e.g. drag-to-move) re-enter placement mode without guessing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_name: Option<String>,
    pub attributes: EntityAttributes,
    pub collision_box: Option<CollisionBox>,
}

/// Runtime audio component attached to an entity.
///
/// This keeps transient audio behavior out of the core `Entity` model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityAudioComponent {
    pub footstep_distance_accumulator: f32,
    pub footstep_trigger_distance: f32,
    pub last_collision_state: bool,
    #[serde(default)]
    pub movement_sound: Option<String>,
    #[serde(default)]
    pub collision_sound: Option<String>,
}

impl Default for EntityAudioComponent {
    fn default() -> Self {
        Self {
            footstep_distance_accumulator: 0.0,
            footstep_trigger_distance: 32.0,
            last_collision_state: false,
            movement_sound: None,
            collision_sound: None,
        }
    }
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

    /// Runtime audio components keyed by entity id.
    #[serde(default)]
    audio_components: HashMap<EntityId, EntityAudioComponent>,
}

impl EntityManager {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_id: 1, // we start at 1 to use 0 for invalid entities
            player_id: None,
            entities_by_type: HashMap::new(),
            active_entities: HashSet::new(),
            audio_components: HashMap::new(),
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
            definition_name: None,
            attributes,
            collision_box,
        };
        self.audio_components
            .insert(id, EntityAudioComponent::default());

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

    /// Spawn an entity from an entity definition.
    pub fn spawn_from_definition(
        &mut self,
        definition: &EntityDefinition,
        position: IVec2,
    ) -> Result<EntityId, String> {
        let id = self.next_id;
        self.next_id += 1;

        let entity = definition.create_entity(position, id)?;
        let entity_type = entity.entity_type.clone();
        let audio_component = definition.create_audio_component();

        if matches!(entity_type, EntityType::Player) {
            self.player_id = Some(id);
        }

        self.entities_by_type
            .entry(entity_type)
            .or_default()
            .insert(id);

        if entity.attributes.active {
            self.active_entities.insert(id);
        }

        self.entities.insert(id, entity);
        self.audio_components.insert(id, audio_component);
        Ok(id)
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
        self.audio_components.entry(id).or_default();

        // Store the entity
        self.entities.insert(id, entity);

        tracing::trace!("Added existing entity {} to EntityManager", id);
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
        self.audio_components.remove(&id);

        true
    }

    // Basic getters
    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    pub fn audio_component(&self, id: EntityId) -> Option<&EntityAudioComponent> {
        self.audio_components.get(&id)
    }

    pub fn audio_component_mut(&mut self, id: EntityId) -> Option<&mut EntityAudioComponent> {
        self.audio_components.get_mut(&id)
    }

    pub fn get_entity_with_audio_mut(
        &mut self,
        id: EntityId,
    ) -> Option<(&mut Entity, &mut EntityAudioComponent)> {
        let (entities, audio_components) = (&mut self.entities, &mut self.audio_components);
        let entity = entities.get_mut(&id)?;
        let audio_component = audio_components.entry(id).or_default();
        Some((entity, audio_component))
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

    pub fn would_collide_with_solid_entity(
        &self,
        moving_entity_id: EntityId,
        new_position: IVec2,
    ) -> bool {
        let Some(moving_entity) = self.entities.get(&moving_entity_id) else {
            return false;
        };
        let Some(moving_box) = &moving_entity.collision_box else {
            return false;
        };
        if moving_box.trigger || !moving_entity.attributes.solid {
            return false;
        }

        let (moving_pos, moving_size) = moving_box.world_bounds(new_position);

        for other_id in &self.active_entities {
            if *other_id == moving_entity_id {
                continue;
            }

            let Some(other_entity) = self.entities.get(other_id) else {
                continue;
            };
            if !other_entity.attributes.solid {
                continue;
            }

            let Some(other_box) = &other_entity.collision_box else {
                continue;
            };
            if other_box.trigger {
                continue;
            }

            let (other_pos, other_size) = other_box.world_bounds(other_entity.position);
            if crate::collision::aabb_overlap(moving_pos, moving_size, other_pos, other_size) {
                return true;
            }
        }

        false
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
    #[serde(default)]
    pub collision_sound: Option<String>,
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
    fn parse_animation_state(state: &str) -> Result<AnimationState, String> {
        match state.to_lowercase().as_str() {
            "idle" => Ok(AnimationState::Idle),
            "walk" => Ok(AnimationState::Walk),
            "idle_down" => Ok(AnimationState::IdleDown),
            "idle_up" => Ok(AnimationState::IdleUp),
            "idle_left" => Ok(AnimationState::IdleLeft),
            "idle_right" => Ok(AnimationState::IdleRight),
            "walk_down" => Ok(AnimationState::WalkDown),
            "walk_up" => Ok(AnimationState::WalkUp),
            "walk_left" => Ok(AnimationState::WalkLeft),
            "walk_right" => Ok(AnimationState::WalkRight),
            _ => Err(format!("Unknown animation state: {state}")),
        }
    }

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
            let state = Self::parse_animation_state(&clip_def.state)?;

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
        let default_state = Self::parse_animation_state(&self.animations.default_state)?;
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
            definition_name: Some(self.name.clone()),
            attributes,
            collision_box,
        })
    }

    /// Build a runtime audio component from this definition.
    pub fn create_audio_component(&self) -> EntityAudioComponent {
        let movement_sound = {
            let trimmed = self.audio.movement_sound.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        };
        let collision_sound = self
            .audio
            .collision_sound
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);

        EntityAudioComponent {
            footstep_distance_accumulator: 0.0,
            footstep_trigger_distance: self.audio.footstep_trigger_distance,
            last_collision_state: false,
            movement_sound,
            collision_sound,
        }
    }

    /// Get collision box from entity definition without creating full entity.
    /// Useful for placement validation.
    pub fn get_collision_box(&self) -> Option<CollisionBox> {
        if self.collision.enabled {
            Some(CollisionBox::new(
                IVec2::new(self.collision.offset[0], self.collision.offset[1]),
                UVec2::new(self.collision.size[0], self.collision.size[1]),
                self.collision.trigger,
            ))
        } else {
            None
        }
    }
}
