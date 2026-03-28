//! Entity management - creating, spawning, despawning, and querying entities.

use super::definition::{EntityDefinition, EntityDefinitionError};
use super::types::{
    ControlRole, Entity, EntityAttributes, EntityAudioComponent, EntityAudioSettings, EntityId,
    EntityKind,
};
use super::default_category_for_kind;
use crate::collision::CollisionBox;
use glam::{IVec2, UVec2};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityManager {
    entities: HashMap<EntityId, Entity>,
    next_id: EntityId,

    // Quick lookups
    player_id: Option<EntityId>,
    entities_by_kind: HashMap<EntityKind, HashSet<EntityId>>,

    // This is prepared for spatial queries (collission)
    active_entities: HashSet<EntityId>,

    /// Runtime audio components keyed by entity id.
    #[serde(default)]
    audio_components: HashMap<EntityId, EntityAudioComponent>,
}

impl EntityManager {
    fn tracks_player_role(entity: &Entity) -> bool {
        matches!(
            entity.effective_control_role(),
            ControlRole::PlayerCharacter
        )
    }

    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_id: 1, // we start at 1 to use 0 for invalid entities
            player_id: None,
            entities_by_kind: HashMap::new(),
            active_entities: HashSet::new(),
            audio_components: HashMap::new(),
        }
    }

    /// Update animations for all entities
    pub fn update_animations(&mut self, delta_time_ms: f32) -> HashMap<EntityId, u32> {
        let mut completed_loops = HashMap::new();
        for (entity_id, entity) in &mut self.entities {
            if let Some(animation_controller) = &mut entity.attributes.animation_controller {
                let loop_count = animation_controller.update(delta_time_ms);
                if loop_count > 0 {
                    completed_loops.insert(*entity_id, loop_count);
                }
            }
        }
        completed_loops
    }

    pub fn spawn_entity(
        &mut self,
        entity_kind: EntityKind,
        position: IVec2,
        size: UVec2,
        mut attributes: EntityAttributes,
    ) -> EntityId {
        attributes.ensure_legacy_health_stat();
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
            entity_kind,
            category: default_category_for_kind(&entity_kind).to_string(),
            definition_name: None,
            persistent_across_saves: false,
            control_role: ControlRole::LegacyDefault,
            audio: EntityAudioSettings::default(),
            attributes,
            collision_box,
            tags: Vec::new(),
            movement_accumulator: glam::Vec2::ZERO,
        };

        self.audio_components
            .insert(id, EntityAudioComponent::default());

        // Insert into main storage
        self.entities.insert(id, entity);

        self.register_entity_indices_from_state(
            entity_kind,
            id,
            self.entities
                .get(&id)
                .is_some_and(Self::tracks_player_role),
            self.entities
                .get(&id)
                .is_some_and(|entity| entity.attributes.active),
        );

        id
    }

    /// Spawn an entity from an entity definition.
    pub fn spawn_from_definition(
        &mut self,
        definition: &EntityDefinition,
        position: IVec2,
    ) -> Result<EntityId, EntityDefinitionError> {
        let id = self.next_id;
        self.next_id += 1;

        let entity = definition.create_entity(position, id)?;
        let entity_kind = entity.entity_kind;
        let audio_component = definition.create_audio_component();

        self.entities.insert(id, entity);
        self.audio_components.insert(id, audio_component);
        self.register_entity_indices_from_state(
            entity_kind,
            id,
            self.entities
                .get(&id)
                .is_some_and(Self::tracks_player_role),
            self.entities
                .get(&id)
                .is_some_and(|entity| entity.attributes.active),
        );
        Ok(id)
    }

    /// Clone an existing entity at a new position.
    /// The cloned entity gets a new ID but inherits all attributes from the source.
    pub fn clone_entity(&mut self, source_id: EntityId, position: IVec2) -> Option<EntityId> {
        let source = self.entities.get(&source_id)?;
        let id = self.next_id;
        self.next_id += 1;

        let mut cloned = source.clone();
        cloned.id = id;
        cloned.position = position;

        let entity_kind = cloned.entity_kind;
        let audio_component = cloned.audio.to_component();

        self.entities.insert(id, cloned);
        self.audio_components.insert(id, audio_component);
        self.register_entity_indices_from_state(
            entity_kind,
            id,
            false,
            self.entities
                .get(&id)
                .is_some_and(|entity| entity.attributes.active),
        );
        Some(id)
    }

    /// Add an existing entity to the manager (used for scene-to-gamestate conversion)
    pub fn add_existing_entity(&mut self, mut entity: Entity) -> EntityId {
        entity.attributes.ensure_legacy_health_stat();

        let id = entity.id;
        let entity_kind = entity.entity_kind;

        // Update next_id if needed to avoid conflicts
        if id >= self.next_id {
            self.next_id = id + 1;
        }

        self.audio_components
            .insert(id, entity.audio.to_component());

        // Store the entity
        self.entities.insert(id, entity);
        self.register_entity_indices_from_state(
            entity_kind,
            id,
            self.entities
                .get(&id)
                .is_some_and(Self::tracks_player_role)
                && self.player_id.is_none(),
            true,
        );

        tracing::trace!("Added existing entity {} to EntityManager", id);
        id
    }

    pub fn despawn_entity(&mut self, id: EntityId) -> bool {
        let Some(entity) = self.entities.remove(&id) else {
            return false;
        };

        self.deregister_entity_indices(&entity, id);
        self.audio_components.remove(&id);

        true
    }

    fn register_entity_indices_from_state(
        &mut self,
        entity_kind: EntityKind,
        id: EntityId,
        is_player: bool,
        is_active: bool,
    ) {
        if is_player {
            self.player_id = Some(id);
        }
        self.entities_by_kind.entry(entity_kind).or_default().insert(id);
        if is_active {
            self.active_entities.insert(id);
        }
    }

    fn deregister_entity_indices(&mut self, entity: &Entity, id: EntityId) {
        if self.player_id.is_some_and(|player_id| player_id == id) {
            self.player_id = None;
        }
        if let Some(kind_set) = self.entities_by_kind.get_mut(&entity.entity_kind) {
            kind_set.remove(&id);
        }
        self.active_entities.remove(&id);
    }

    // Basic getters
    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    pub fn set_control_role(&mut self, id: EntityId, control_role: ControlRole) -> bool {
        let Some(entity) = self.entities.get_mut(&id) else {
            return false;
        };

        entity.control_role = control_role;
        if matches!(
            entity.effective_control_role(),
            ControlRole::PlayerCharacter
        ) {
            self.player_id = Some(id);
        } else if self.player_id == Some(id) {
            self.player_id = None;
        }
        true
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
    pub fn entities_of_kind(&self, entity_kind: &EntityKind) -> Vec<EntityId> {
        self.entities_by_kind
            .get(entity_kind)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn active_entities(&self) -> Vec<EntityId> {
        self.active_entities.iter().copied().collect()
    }

    /// Returns an iterator over active entity IDs without allocating.
    ///
    /// Prefer this over `active_entities()` when you only need to iterate.
    pub fn active_entities_iter(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.active_entities.iter().copied()
    }

    /// Returns the number of active entities without allocating.
    pub fn active_entity_count(&self) -> usize {
        self.active_entities.len()
    }

    pub fn would_collide_with_solid_entity(
        &self,
        moving_entity_id: EntityId,
        new_position: IVec2,
    ) -> bool {
        self.find_colliding_entity(moving_entity_id, new_position)
            .is_some()
    }

    /// Finds the first solid entity that would collide with `moving_entity_id`
    /// if it moved to `new_position`.
    ///
    /// Returns `Some(entity_id)` of the colliding entity, or `None` if no collision.
    pub fn find_colliding_entity(
        &self,
        moving_entity_id: EntityId,
        new_position: IVec2,
    ) -> Option<EntityId> {
        let moving_entity = self.entities.get(&moving_entity_id)?;
        let moving_box = moving_entity.collision_box.as_ref()?;
        if moving_box.trigger || !moving_entity.attributes.solid {
            return None;
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
                return Some(*other_id);
            }
        }

        None
    }

    /// Check if spawning an entity at the given position with given size would be free.
    /// Returns true if no solid entities would overlap.
    pub fn is_spawn_position_free(&self, position: IVec2, size: glam::UVec2) -> bool {
        for other_id in &self.active_entities {
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
            if crate::collision::aabb_overlap(position, size, other_pos, other_size) {
                return false;
            }
        }
        true
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
