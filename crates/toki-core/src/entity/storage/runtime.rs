use super::{EntitySpawnBundle, OptionalComponentRegistry, OptionalEntityComponents};
use crate::entity::{Entity, EntityAudioComponent, EntityAudioSettings, EntityId, StoredEntity};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct EntityStorage {
    entities: HashMap<EntityId, Entity>,
    audio_components: HashMap<EntityId, EntityAudioComponent>,
    components: OptionalComponentRegistry,
}

impl EntityStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entities(&self) -> &HashMap<EntityId, Entity> {
        &self.entities
    }

    pub fn entities_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut Entity)> + '_ {
        self.entities.iter_mut().map(|(id, entity)| (*id, entity))
    }

    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    pub fn insert_spawn_bundle(
        &mut self,
        entity: Entity,
        audio_component: EntityAudioComponent,
        optional_components: OptionalEntityComponents,
    ) -> EntityId {
        let id = entity.id;
        self.audio_components.insert(id, audio_component);
        self.components
            .apply_optional_components(id, optional_components);
        self.entities.insert(id, entity);
        id
    }

    pub fn insert_stored_entity(
        &mut self,
        stored: StoredEntity,
        audio_component: Option<EntityAudioComponent>,
    ) -> EntityId {
        let id = stored.entity.id;
        self.insert_spawn_bundle(
            stored.entity.clone(),
            audio_component.unwrap_or_else(|| stored.entity.audio.to_component()),
            stored.components,
        );
        id
    }

    pub fn remove_entity(&mut self, id: EntityId) -> Option<Entity> {
        self.audio_components.remove(&id);
        self.components.remove_all(id);
        self.entities.remove(&id)
    }

    pub fn stored_entity(&self, id: EntityId) -> Option<StoredEntity> {
        self.get_entity(id)
            .cloned()
            .map(|entity| StoredEntity::new(entity, self.components.optional_components(id)))
    }

    pub fn clone_spawn_bundle(
        &mut self,
        source_id: EntityId,
        new_id: EntityId,
        position: glam::IVec2,
    ) -> Option<EntitySpawnBundle> {
        let source = self.entities.get(&source_id)?;
        let mut cloned = source.clone();
        cloned.id = new_id;
        cloned.position = position;
        Some(EntitySpawnBundle {
            entity: cloned,
            audio_component: self
                .audio_components
                .get(&source_id)
                .cloned()
                .unwrap_or_else(|| source.audio.to_component()),
            optional_components: self.components.optional_components(source_id),
        })
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
        let audio_component = audio_components
            .entry(id)
            .or_insert_with(|| EntityAudioSettings::default().to_component());
        Some((entity, audio_component))
    }

    pub fn components(&self) -> &OptionalComponentRegistry {
        &self.components
    }

    pub fn components_mut(&mut self) -> &mut OptionalComponentRegistry {
        &mut self.components
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{ControlRole, EntityAttributes, EntityKind};
    use glam::{IVec2, UVec2};

    fn sample_entity(id: EntityId) -> Entity {
        Entity {
            id,
            position: IVec2::new(1, 2),
            size: UVec2::new(16, 16),
            entity_kind: EntityKind::Npc,
            category: "creature".to_string(),
            definition_name: None,
            persistent_across_saves: false,
            control_role: ControlRole::None,
            audio: EntityAudioSettings::default(),
            attributes: EntityAttributes::default(),
            collision_box: None,
            movement_accumulator: glam::Vec2::ZERO,
            tags: Vec::new(),
        }
    }

    #[test]
    fn storage_clone_and_remove_keep_components_in_sync() {
        let mut storage = EntityStorage::new();
        storage.insert_spawn_bundle(
            sample_entity(1),
            EntityAudioComponent::default(),
            OptionalEntityComponents {
                pickup: Some(super::super::PickupDef {
                    item_id: "coin".to_string(),
                    count: 1,
                }),
                ..OptionalEntityComponents::default()
            },
        );
        let bundle = storage
            .clone_spawn_bundle(1, 2, IVec2::new(9, 9))
            .expect("clone bundle should exist");
        storage.insert_spawn_bundle(
            bundle.entity,
            bundle.audio_component,
            bundle.optional_components,
        );
        assert!(storage.components().pickup(2).is_some());
        storage.remove_entity(1);
        assert!(storage.components().pickup(1).is_none());
    }
}
