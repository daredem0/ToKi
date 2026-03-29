//! Entity management - creating, spawning, despawning, and querying entities.

use super::definition::{EntityDefinition, EntityDefinitionError};
use super::components::{EntityComponentStore, EntityOptionalComponents, EntitySpawnBundle};
use super::model::{
    ControlRole, Entity, EntityAttributes, EntityAudioComponent, EntityAudioSettings, EntityId,
    EntityKind,
};
use super::default_category_for_kind;
use super::wire::StoredEntity;
use crate::collision::CollisionBox;
use glam::{IVec2, UVec2};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct EntityManager {
    entities: HashMap<EntityId, Entity>,
    next_id: EntityId,

    // Quick lookups
    player_id: Option<EntityId>,
    entities_by_kind: HashMap<EntityKind, HashSet<EntityId>>,

    // This is prepared for spatial queries (collission)
    active_entities: HashSet<EntityId>,

    /// Runtime audio components keyed by entity id.
    audio_components: HashMap<EntityId, EntityAudioComponent>,

    components: EntityComponentStore,
}

#[derive(Serialize, Deserialize)]
struct EntityManagerWire {
    entities: Vec<StoredEntity>,
    next_id: EntityId,
    player_id: Option<EntityId>,
    #[serde(default)]
    audio_components: HashMap<EntityId, EntityAudioComponent>,
}

impl Serialize for EntityManager {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        EntityManagerWire {
            entities: self
                .entities
                .values()
                .cloned()
                .map(|entity| {
                    let id = entity.id;
                    StoredEntity::new(entity, self.components.optional_components(id))
                })
                .collect(),
            next_id: self.next_id,
            player_id: self.player_id,
            audio_components: self.audio_components.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EntityManager {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = EntityManagerWire::deserialize(deserializer)?;
        let mut manager = EntityManager::new();
        manager.next_id = wire.next_id;
        manager.player_id = wire.player_id;
        for stored in wire.entities {
            let entity_id = stored.entity.id;
            let audio_component = wire
                .audio_components
                .get(&entity_id)
                .cloned()
                .unwrap_or_else(|| stored.entity.audio.to_component());
            manager.add_spawn_bundle(EntitySpawnBundle {
                entity: stored.entity,
                optional_components: stored.components,
                audio_component,
            });
        }
        if manager.next_id == 1 {
            manager.next_id = wire.next_id;
        }
        manager.player_id = wire.player_id.or(manager.player_id);
        Ok(manager)
    }
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
            components: EntityComponentStore::default(),
        }
    }

    /// Update animations for all entities
    pub fn update_animations(&mut self, delta_time_ms: f32) -> HashMap<EntityId, u32> {
        let mut completed_loops = HashMap::new();
        for (entity_id, entity) in &mut self.entities {
            if let Some(animation_controller) = &mut entity.attributes.rendering.animation_controller
            {
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
        let collision_box = if attributes.gameplay.solid {
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

        self.add_spawn_bundle(EntitySpawnBundle {
            entity,
            optional_components: EntityOptionalComponents::default(),
            audio_component: EntityAudioComponent::default(),
        })
    }

    /// Spawn an entity from an entity definition.
    pub fn spawn_from_definition(
        &mut self,
        definition: &EntityDefinition,
        position: IVec2,
    ) -> Result<EntityId, EntityDefinitionError> {
        let id = self.next_id;
        self.next_id += 1;

        let bundle = definition.create_spawn_bundle(position, id)?;
        Ok(self.add_spawn_bundle(bundle))
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

        let optional_components = self.components.optional_components(source_id);
        Some(self.add_spawn_bundle(EntitySpawnBundle {
            audio_component: cloned.audio.to_component(),
            entity: cloned,
            optional_components,
        }))
    }

    /// Add an existing entity to the manager (used for scene-to-gamestate conversion)
    pub fn add_existing_entity(&mut self, entity: Entity) -> EntityId {
        self.add_existing_stored_entity(StoredEntity::new(entity, EntityOptionalComponents::default()))
    }

    pub fn add_existing_stored_entity(&mut self, stored: StoredEntity) -> EntityId {
        let mut entity = stored.entity;
        entity.attributes.ensure_legacy_health_stat();
        self.add_spawn_bundle(EntitySpawnBundle {
            audio_component: entity.audio.to_component(),
            entity,
            optional_components: stored.components,
        })
    }

    pub fn despawn_entity(&mut self, id: EntityId) -> bool {
        let Some(entity) = self.entities.remove(&id) else {
            return false;
        };

        self.deregister_entity_indices(&entity, id);
        self.audio_components.remove(&id);
        self.components.remove_all(id);

        true
    }

    fn add_spawn_bundle(&mut self, mut bundle: EntitySpawnBundle) -> EntityId {
        let id = bundle.entity.id;
        let entity_kind = bundle.entity.entity_kind;
        let is_player = Self::tracks_player_role(&bundle.entity) && self.player_id.is_none();
        let is_active = bundle.entity.attributes.behavior.active;

        if id >= self.next_id {
            self.next_id = id + 1;
        }

        bundle.entity.attributes.ensure_legacy_health_stat();
        self.audio_components.insert(id, bundle.audio_component);
        self.components
            .set_optional_components(id, bundle.optional_components);
        self.entities.insert(id, bundle.entity);
        self.register_entity_indices_from_state(entity_kind, id, is_player, is_active);
        tracing::trace!("Added entity {} to EntityManager", id);
        id
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

    pub fn stored_entity(&self, id: EntityId) -> Option<StoredEntity> {
        self.entities
            .get(&id)
            .cloned()
            .map(|entity| StoredEntity::new(entity, self.components.optional_components(id)))
    }

    pub fn components(&self) -> &EntityComponentStore {
        &self.components
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

    pub fn primary_projectile(&self, id: EntityId) -> Option<&super::PrimaryProjectileDef> {
        self.components.primary_projectile(id)
    }

    pub fn primary_projectile_mut(
        &mut self,
        id: EntityId,
    ) -> Option<&mut super::PrimaryProjectileDef> {
        self.components.primary_projectile_mut(id)
    }

    pub fn set_primary_projectile(
        &mut self,
        id: EntityId,
        projectile: Option<super::PrimaryProjectileDef>,
    ) {
        self.components.set_primary_projectile(id, projectile);
    }

    pub fn projectile(&self, id: EntityId) -> Option<&super::ProjectileState> {
        self.components.projectile(id)
    }

    pub fn projectile_mut(&mut self, id: EntityId) -> Option<&mut super::ProjectileState> {
        self.components.projectile_mut(id)
    }

    pub fn set_projectile(&mut self, id: EntityId, projectile: Option<super::ProjectileState>) {
        self.components.set_projectile(id, projectile);
    }

    pub fn pickup(&self, id: EntityId) -> Option<&super::PickupDef> {
        self.components.pickup(id)
    }

    pub fn pickup_mut(&mut self, id: EntityId) -> Option<&mut super::PickupDef> {
        self.components.pickup_mut(id)
    }

    pub fn set_pickup(&mut self, id: EntityId, pickup: Option<super::PickupDef>) {
        self.components.set_pickup(id, pickup);
    }

    pub fn inventory(&self, id: EntityId) -> Option<&super::Inventory> {
        self.components.inventory(id)
    }

    pub fn inventory_mut(&mut self, id: EntityId) -> Option<&mut super::Inventory> {
        self.components.inventory_mut(id)
    }

    pub fn ensure_inventory(&mut self, id: EntityId) -> &mut super::Inventory {
        self.components.ensure_inventory(id)
    }

    pub fn set_inventory(&mut self, id: EntityId, inventory: Option<super::Inventory>) {
        self.components.set_inventory(id, inventory);
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
        if moving_box.trigger || !moving_entity.attributes.gameplay.solid {
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
            if !other_entity.attributes.gameplay.solid {
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
            if !other_entity.attributes.gameplay.solid {
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
            .filter(|(_, entity)| entity.attributes.rendering.visible)
            .map(|(id, _)| *id)
            .collect()
    }

    // Update entity active status
    pub fn set_entity_active(&mut self, id: EntityId, active: bool) {
        if let Some(entity) = self.entities.get_mut(&id) {
            let was_active = entity.attributes.behavior.active;
            entity.attributes.behavior.active = active;
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
