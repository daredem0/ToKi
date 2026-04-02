//! Entity management - creating, spawning, despawning, and querying entities.

use super::components::{AiComponent, CombatComponent, InteractionComponent, MovementComponent};
use super::default_category_for_kind;
use super::definition::{EntityDefinition, EntityDefinitionError};
use super::model::{
    ControlRole, Entity, EntityAudioComponent, EntityAudioSettings, EntityId, EntityKind,
    EntityRendering,
};
use super::storage::{EntitySpawnBundle, EntityStorage, OptionalEntityComponents};
use super::wire::StoredEntity;
use crate::collision::CollisionBox;
use glam::{IVec2, UVec2};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct EntityManager {
    storage: EntityStorage,
    next_id: EntityId,
    player_id: Option<EntityId>,
    entities_by_kind: HashMap<EntityKind, HashSet<EntityId>>,
    active_entities: HashSet<EntityId>,
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
                .storage
                .entities()
                .values()
                .filter_map(|entity| self.storage.stored_entity(entity.id))
                .collect(),
            next_id: self.next_id,
            player_id: self.player_id,
            audio_components: self
                .storage
                .entities()
                .keys()
                .filter_map(|id| {
                    self.storage
                        .audio_component(*id)
                        .cloned()
                        .map(|audio| (*id, audio))
                })
                .collect(),
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
        manager.next_id = wire.next_id.max(manager.next_id);
        manager.player_id = manager
            .storage
            .entities()
            .values()
            .find(|entity| Self::tracks_player_role(entity))
            .map(|entity| entity.id)
            .or(wire.player_id);
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
            storage: EntityStorage::new(),
            next_id: 1,
            player_id: None,
            entities_by_kind: HashMap::new(),
            active_entities: HashSet::new(),
        }
    }

    pub fn storage(&self) -> &EntityStorage {
        &self.storage
    }

    pub fn storage_mut(&mut self) -> &mut EntityStorage {
        &mut self.storage
    }

    pub fn update_animations(&mut self, delta_time_ms: f32) -> HashMap<EntityId, u32> {
        let mut completed_loops = HashMap::new();
        for (entity_id, entity) in self.storage.entities_mut() {
            if let Some(animation_controller) = &mut entity.rendering.animation_controller {
                let loop_count = animation_controller.update(delta_time_ms);
                if loop_count > 0 {
                    completed_loops.insert(entity_id, loop_count);
                }
            }
        }
        completed_loops
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_entity(
        &mut self,
        entity_kind: EntityKind,
        position: IVec2,
        size: UVec2,
        rendering: EntityRendering,
        solid: bool,
        active: bool,
        optional_components: OptionalEntityComponents,
    ) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        let collision_box = if solid {
            Some(CollisionBox::solid_box(size))
        } else {
            None
        };

        let entity = Entity {
            id,
            position,
            size,
            entity_kind,
            category: default_category_for_kind(entity_kind).to_string(),
            definition_name: None,
            persistent_across_saves: false,
            control_role: ControlRole::LegacyDefault,
            audio: EntityAudioSettings::default(),
            rendering,
            collision_box,
            solid,
            active,
            tags: Vec::new(),
            movement_accumulator: glam::Vec2::ZERO,
        };

        self.add_spawn_bundle(EntitySpawnBundle {
            entity,
            optional_components,
            audio_component: EntityAudioComponent::default(),
        })
    }

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

    pub fn clone_entity(&mut self, source_id: EntityId, position: IVec2) -> Option<EntityId> {
        let id = self.next_id;
        self.next_id += 1;
        let bundle = self.storage.clone_spawn_bundle(source_id, id, position)?;
        Some(self.add_spawn_bundle(bundle))
    }

    pub fn add_existing_entity(&mut self, entity: Entity) -> EntityId {
        self.add_existing_stored_entity(StoredEntity::new(
            entity,
            OptionalEntityComponents::default(),
        ))
    }

    pub fn add_existing_stored_entity(&mut self, stored: StoredEntity) -> EntityId {
        let entity = stored.entity;
        self.add_spawn_bundle(EntitySpawnBundle {
            audio_component: entity.audio.to_component(),
            entity,
            optional_components: stored.components,
        })
    }

    pub fn despawn_entity(&mut self, id: EntityId) -> bool {
        let Some(entity) = self.storage.remove_entity(id) else {
            return false;
        };
        self.deregister_entity_indices(&entity, id);
        true
    }

    fn add_spawn_bundle(&mut self, mut bundle: EntitySpawnBundle) -> EntityId {
        let id = bundle.entity.id;
        let entity_kind = bundle.entity.entity_kind;
        let is_player = Self::tracks_player_role(&bundle.entity) && self.player_id.is_none();
        let is_active = bundle.entity.active;

        if id >= self.next_id {
            self.next_id = id + 1;
        }

        if let Some(combat) = bundle.optional_components.combat.as_mut() {
            combat.ensure_health_stat();
        }
        self.storage.insert_spawn_bundle(
            bundle.entity,
            bundle.audio_component,
            bundle.optional_components,
        );
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
        self.entities_by_kind
            .entry(entity_kind)
            .or_default()
            .insert(id);
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

    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.storage.get_entity(id)
    }

    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.storage.get_entity_mut(id)
    }

    pub fn stored_entity(&self, id: EntityId) -> Option<StoredEntity> {
        self.storage.stored_entity(id)
    }

    pub fn set_control_role(&mut self, id: EntityId, control_role: ControlRole) -> bool {
        let Some(entity) = self.storage.get_entity_mut(id) else {
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

    pub fn get_entity_with_audio_mut(
        &mut self,
        id: EntityId,
    ) -> Option<(&mut Entity, &mut EntityAudioComponent)> {
        self.storage.get_entity_with_audio_mut(id)
    }

    pub fn get_player(&self) -> Option<&Entity> {
        self.player_id.and_then(|id| self.storage.get_entity(id))
    }

    pub fn get_player_mut(&mut self) -> Option<&mut Entity> {
        self.player_id
            .and_then(|id| self.storage.get_entity_mut(id))
    }

    pub fn get_player_id(&self) -> Option<EntityId> {
        self.player_id
    }

    pub fn entities_of_kind(&self, entity_kind: &EntityKind) -> Vec<EntityId> {
        self.entities_by_kind
            .get(entity_kind)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn active_entities(&self) -> Vec<EntityId> {
        self.active_entities.iter().copied().collect()
    }

    pub fn active_entities_iter(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.active_entities.iter().copied()
    }

    pub fn active_entity_count(&self) -> usize {
        self.active_entities.len()
    }

    pub fn entity_ids(&self) -> Vec<EntityId> {
        self.storage.entities().keys().copied().collect()
    }

    pub fn entity_ids_iter(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.storage.entities().keys().copied()
    }

    pub fn would_collide_with_solid_entity(
        &self,
        moving_entity_id: EntityId,
        new_position: IVec2,
    ) -> bool {
        self.find_colliding_entity(moving_entity_id, new_position)
            .is_some()
    }

    pub fn find_colliding_entity(
        &self,
        moving_entity_id: EntityId,
        new_position: IVec2,
    ) -> Option<EntityId> {
        let moving_entity = self.storage.get_entity(moving_entity_id)?;
        let moving_box = moving_entity.collision_box.as_ref()?;
        if moving_box.trigger || !moving_entity.solid {
            return None;
        }

        let (moving_pos, moving_size) = moving_box.world_bounds(new_position);

        for other_id in self.entity_ids_iter() {
            if other_id == moving_entity_id {
                continue;
            }

            let Some(other_entity) = self.storage.get_entity(other_id) else {
                continue;
            };
            if !Self::is_entity_collidable_candidate(other_entity) {
                continue;
            }

            let other_box = other_entity
                .collision_box
                .as_ref()
                .expect("collidable entity should have a collision box");
            let (other_pos, other_size) = other_box.world_bounds(other_entity.position);
            if crate::collision::aabb_overlap(moving_pos, moving_size, other_pos, other_size) {
                return Some(other_id);
            }
        }

        None
    }

    pub fn is_spawn_position_free(&self, position: IVec2, size: glam::UVec2) -> bool {
        for other_id in self.entity_ids_iter() {
            let Some(other_entity) = self.storage.get_entity(other_id) else {
                continue;
            };
            if !Self::is_entity_collidable_candidate(other_entity) {
                continue;
            }

            let other_box = other_entity
                .collision_box
                .as_ref()
                .expect("collidable entity should have a collision box");
            let (other_pos, other_size) = other_box.world_bounds(other_entity.position);
            if crate::collision::aabb_overlap(position, size, other_pos, other_size) {
                return false;
            }
        }
        true
    }

    fn is_entity_collidable_candidate(entity: &Entity) -> bool {
        entity.solid
            && entity
                .collision_box
                .as_ref()
                .is_some_and(|collision_box| !collision_box.trigger)
    }

    pub fn visible_entities(&self) -> Vec<EntityId> {
        self.storage
            .entities()
            .iter()
            .filter(|(_, entity)| entity.rendering.visible)
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn set_entity_active(&mut self, id: EntityId, active: bool) {
        if let Some(entity) = self.storage.get_entity_mut(id) {
            let was_active = entity.active;
            entity.active = active;
            if active && !was_active {
                self.active_entities.insert(id);
            } else if !active && was_active {
                self.active_entities.remove(&id);
            }
        }
    }

    pub fn movement(&self, id: EntityId) -> Option<&MovementComponent> {
        self.storage.components().movement(id)
    }

    pub fn movement_mut(&mut self, id: EntityId) -> Option<&mut MovementComponent> {
        self.storage.components_mut().movement_mut(id)
    }

    pub fn set_movement(&mut self, id: EntityId, movement: Option<MovementComponent>) {
        self.storage.components_mut().set_movement(id, movement);
    }

    pub fn ai(&self, id: EntityId) -> Option<&AiComponent> {
        self.storage.components().ai(id)
    }

    pub fn ai_mut(&mut self, id: EntityId) -> Option<&mut AiComponent> {
        self.storage.components_mut().ai_mut(id)
    }

    pub fn set_ai(&mut self, id: EntityId, ai: Option<AiComponent>) {
        self.storage.components_mut().set_ai(id, ai);
    }

    pub fn interaction(&self, id: EntityId) -> Option<&InteractionComponent> {
        self.storage.components().interaction(id)
    }

    pub fn interaction_mut(&mut self, id: EntityId) -> Option<&mut InteractionComponent> {
        self.storage.components_mut().interaction_mut(id)
    }

    pub fn set_interaction(&mut self, id: EntityId, interaction: Option<InteractionComponent>) {
        self.storage
            .components_mut()
            .set_interaction(id, interaction);
    }

    pub fn combat(&self, id: EntityId) -> Option<&CombatComponent> {
        self.storage.components().combat(id)
    }

    pub fn combat_mut(&mut self, id: EntityId) -> Option<&mut CombatComponent> {
        self.storage.components_mut().combat_mut(id)
    }

    pub fn set_combat(&mut self, id: EntityId, combat: Option<CombatComponent>) {
        self.storage.components_mut().set_combat(id, combat);
    }
}

impl Default for EntityManager {
    fn default() -> Self {
        Self::new()
    }
}
