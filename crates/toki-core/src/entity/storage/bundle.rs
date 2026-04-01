use super::sparse_map::SparseComponentMap;
use crate::entity::{
    AiComponent, CombatComponent, Entity, EntityAudioComponent, EntityId, InteractionComponent,
    MovementComponent,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PickupDef {
    pub item_id: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Inventory {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub items: HashMap<String, u32>,
}

impl Inventory {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn item_count(&self, item_id: &str) -> u32 {
        self.items.get(item_id).copied().unwrap_or(0)
    }

    pub fn add_item(&mut self, item_id: &str, count: u32) {
        if item_id.is_empty() || count == 0 {
            return;
        }

        let entry = self.items.entry(item_id.to_string()).or_insert(0);
        *entry = entry.saturating_add(count);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimaryProjectileDef {
    pub sheet: String,
    pub object_name: String,
    pub size: [u32; 2],
    pub speed: u32,
    pub damage: i32,
    pub lifetime_ticks: u32,
    #[serde(default)]
    pub spawn_offset: [i32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectileState {
    pub sheet: String,
    pub object_name: String,
    pub size: [u32; 2],
    pub velocity: [i32; 2],
    pub remaining_ticks: u32,
    pub damage: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<EntityId>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct OptionalEntityComponents {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub movement: Option<MovementComponent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<AiComponent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction: Option<InteractionComponent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combat: Option<CombatComponent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_projectile: Option<PrimaryProjectileDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile: Option<ProjectileState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pickup: Option<PickupDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory: Option<Inventory>,
}

impl OptionalEntityComponents {
    pub fn is_empty(&self) -> bool {
        self.movement.is_none()
            && self.ai.is_none()
            && self.interaction.is_none()
            && self.combat.is_none()
            && self.primary_projectile.is_none()
            && self.projectile.is_none()
            && self.pickup.is_none()
            && self.inventory.is_none()
    }
}

#[derive(Debug, Clone)]
pub struct EntitySpawnBundle {
    pub entity: Entity,
    pub optional_components: OptionalEntityComponents,
    pub audio_component: EntityAudioComponent,
}
