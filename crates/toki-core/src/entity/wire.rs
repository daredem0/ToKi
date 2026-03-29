use super::components::{
    EntityOptionalComponents, Inventory, PickupDef, PrimaryProjectileDef, ProjectileState,
};
use super::model::{
    AiConfig, ControlRole, Entity, EntityAttributes, EntityAudioSettings, EntityBehavior,
    EntityGameplay, EntityGrounding, EntityId, EntityKind, EntityRendering, EntityStats,
    MovementProfile, StaticObjectRenderDef,
};
use crate::animation::AnimationController;
use crate::collision::CollisionBox;
use crate::ids::EntityDefName;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAttributesWire {
    pub health: Option<u32>,
    #[serde(default, skip_serializing_if = "EntityStats::is_empty")]
    pub stats: EntityStats,
    pub speed: f32,
    pub solid: bool,
    pub visible: bool,
    #[serde(default = "default_has_shadow")]
    pub has_shadow: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette_override: Option<String>,
    pub animation_controller: Option<AnimationController>,
    pub render_layer: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub static_object_render: Option<StaticObjectRenderDef>,
    #[serde(default, skip_serializing_if = "EntityGrounding::is_empty")]
    pub grounding: EntityGrounding,
    pub active: bool,
    pub can_move: bool,
    #[serde(default)]
    pub interactable: bool,
    #[serde(default)]
    pub interaction_reach: u32,
    #[serde(default)]
    pub ai_config: AiConfig,
    #[serde(default)]
    pub movement_profile: MovementProfile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_projectile: Option<PrimaryProjectileDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile: Option<ProjectileState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pickup: Option<PickupDef>,
    #[serde(default, skip_serializing_if = "Inventory::is_empty")]
    pub inventory: Inventory,
    #[serde(default)]
    pub has_inventory: bool,
}

const fn default_has_shadow() -> bool {
    true
}

impl EntityAttributesWire {
    pub fn from_parts(
        attributes: &EntityAttributes,
        components: &EntityOptionalComponents,
    ) -> Self {
        Self {
            health: attributes.gameplay.health,
            stats: attributes.gameplay.stats.clone(),
            speed: attributes.gameplay.speed,
            solid: attributes.gameplay.solid,
            visible: attributes.rendering.visible,
            has_shadow: attributes.rendering.has_shadow,
            palette_override: attributes.rendering.palette_override.clone(),
            animation_controller: attributes.rendering.animation_controller.clone(),
            render_layer: attributes.rendering.render_layer,
            static_object_render: attributes.rendering.static_object_render.clone(),
            grounding: attributes.rendering.grounding.clone(),
            active: attributes.behavior.active,
            can_move: attributes.behavior.can_move,
            interactable: attributes.behavior.interactable,
            interaction_reach: attributes.behavior.interaction_reach,
            ai_config: attributes.behavior.ai_config,
            movement_profile: attributes.behavior.movement_profile,
            primary_projectile: components.primary_projectile.clone(),
            projectile: components.projectile.clone(),
            pickup: components.pickup.clone(),
            inventory: components.inventory.clone().unwrap_or_default(),
            has_inventory: attributes.behavior.has_inventory,
        }
    }

    pub fn into_parts(self) -> (EntityAttributes, EntityOptionalComponents) {
        let mut attributes = EntityAttributes {
            gameplay: EntityGameplay {
                health: self.health,
                stats: self.stats,
                speed: self.speed,
                solid: self.solid,
            },
            rendering: EntityRendering {
                visible: self.visible,
                has_shadow: self.has_shadow,
                palette_override: self.palette_override,
                animation_controller: self.animation_controller,
                render_layer: self.render_layer,
                static_object_render: self.static_object_render,
                grounding: self.grounding,
            },
            behavior: EntityBehavior {
                active: self.active,
                can_move: self.can_move,
                interactable: self.interactable,
                interaction_reach: self.interaction_reach,
                ai_config: self.ai_config,
                movement_profile: self.movement_profile,
                has_inventory: self.has_inventory,
            },
        };
        attributes.ensure_legacy_health_stat();
        let components = EntityOptionalComponents {
            primary_projectile: self.primary_projectile,
            projectile: self.projectile,
            pickup: self.pickup,
            inventory: if self.inventory.is_empty() {
                None
            } else {
                Some(self.inventory)
            },
        };
        (attributes, components)
    }
}

#[derive(Debug, Clone)]
pub struct StoredEntity {
    pub entity: Entity,
    pub components: EntityOptionalComponents,
}

impl StoredEntity {
    pub fn new(entity: Entity, components: EntityOptionalComponents) -> Self {
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
    pub attributes: EntityAttributesWire,
    pub collision_box: Option<CollisionBox>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
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
            attributes: EntityAttributesWire::from_parts(&value.entity.attributes, &value.components),
            collision_box: value.entity.collision_box,
            tags: value.entity.tags,
        }
    }
}

impl From<EntityWire> for StoredEntity {
    fn from(value: EntityWire) -> Self {
        let (attributes, components) = value.attributes.into_parts();
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
                attributes,
                collision_box: value.collision_box,
                tags: value.tags,
                movement_accumulator: glam::Vec2::ZERO,
            },
            components,
        }
    }
}
