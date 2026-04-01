use super::model::{
    ControlRole, Entity, EntityAudioSettings, EntityId, EntityKind, EntityRendering,
};
use super::storage::OptionalEntityComponents;
use crate::collision::CollisionBox;
use crate::ids::EntityDefName;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub type EntityComponentsWire = OptionalEntityComponents;

#[derive(Debug, Clone)]
pub struct StoredEntity {
    pub entity: Entity,
    pub components: OptionalEntityComponents,
}

impl StoredEntity {
    pub fn new(entity: Entity, components: OptionalEntityComponents) -> Self {
        Self { entity, components }
    }
}

impl Serialize for StoredEntity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        EntityWire::from(self.clone()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StoredEntity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(EntityWire::deserialize(deserializer)?.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityWire {
    pub id: EntityId,
    pub position: glam::IVec2,
    pub size: glam::UVec2,
    #[serde(alias = "entity_type")]
    pub entity_kind: EntityKind,
    #[serde(default)]
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_name: Option<EntityDefName>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub persistent_across_saves: bool,
    #[serde(default, skip_serializing_if = "ControlRole::is_legacy_default")]
    pub control_role: ControlRole,
    #[serde(default, skip_serializing_if = "EntityAudioSettings::is_default")]
    pub audio: EntityAudioSettings,
    pub rendering: EntityRendering,
    pub collision_box: Option<CollisionBox>,
    pub solid: bool,
    pub active: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "OptionalEntityComponents::is_empty")]
    pub components: EntityComponentsWire,
}

const fn is_false(value: &bool) -> bool {
    !*value
}

impl From<StoredEntity> for EntityWire {
    fn from(value: StoredEntity) -> Self {
        Self {
            id: value.entity.id,
            position: value.entity.position,
            size: value.entity.size,
            entity_kind: value.entity.entity_kind,
            category: value.entity.category,
            definition_name: value.entity.definition_name,
            persistent_across_saves: value.entity.persistent_across_saves,
            control_role: value.entity.control_role,
            audio: value.entity.audio,
            rendering: value.entity.rendering,
            collision_box: value.entity.collision_box,
            solid: value.entity.solid,
            active: value.entity.active,
            tags: value.entity.tags,
            components: value.components,
        }
    }
}

impl From<EntityWire> for StoredEntity {
    fn from(value: EntityWire) -> Self {
        Self {
            entity: Entity {
                id: value.id,
                position: value.position,
                size: value.size,
                entity_kind: value.entity_kind,
                category: value.category,
                definition_name: value.definition_name,
                persistent_across_saves: value.persistent_across_saves,
                control_role: value.control_role,
                audio: value.audio,
                rendering: value.rendering,
                collision_box: value.collision_box,
                solid: value.solid,
                active: value.active,
                tags: value.tags,
                movement_accumulator: glam::Vec2::ZERO,
            },
            components: value.components,
        }
    }
}
