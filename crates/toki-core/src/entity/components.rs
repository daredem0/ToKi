use super::model::{Entity, EntityAudioComponent, EntityId};
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntityOptionalComponents {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_projectile: Option<PrimaryProjectileDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile: Option<ProjectileState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pickup: Option<PickupDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory: Option<Inventory>,
}

impl EntityOptionalComponents {
    pub fn is_empty(&self) -> bool {
        self.primary_projectile.is_none()
            && self.projectile.is_none()
            && self.pickup.is_none()
            && self.inventory.as_ref().is_none_or(Inventory::is_empty)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntityComponentStore {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    primary_projectiles: HashMap<EntityId, PrimaryProjectileDef>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    projectiles: HashMap<EntityId, ProjectileState>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pickups: HashMap<EntityId, PickupDef>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    inventories: HashMap<EntityId, Inventory>,
}

impl EntityComponentStore {
    pub fn is_empty(&self) -> bool {
        self.primary_projectiles.is_empty()
            && self.projectiles.is_empty()
            && self.pickups.is_empty()
            && self.inventories.is_empty()
    }

    pub fn optional_components(&self, id: EntityId) -> EntityOptionalComponents {
        EntityOptionalComponents {
            primary_projectile: self.primary_projectiles.get(&id).cloned(),
            projectile: self.projectiles.get(&id).cloned(),
            pickup: self.pickups.get(&id).cloned(),
            inventory: self.inventories.get(&id).cloned(),
        }
    }

    pub fn set_optional_components(&mut self, id: EntityId, components: EntityOptionalComponents) {
        self.set_primary_projectile(id, components.primary_projectile);
        self.set_projectile(id, components.projectile);
        self.set_pickup(id, components.pickup);
        self.set_inventory(id, components.inventory);
    }

    pub fn remove_all(&mut self, id: EntityId) {
        self.primary_projectiles.remove(&id);
        self.projectiles.remove(&id);
        self.pickups.remove(&id);
        self.inventories.remove(&id);
    }

    pub fn primary_projectile(&self, id: EntityId) -> Option<&PrimaryProjectileDef> {
        self.primary_projectiles.get(&id)
    }

    pub fn primary_projectile_mut(&mut self, id: EntityId) -> Option<&mut PrimaryProjectileDef> {
        self.primary_projectiles.get_mut(&id)
    }

    pub fn set_primary_projectile(
        &mut self,
        id: EntityId,
        projectile: Option<PrimaryProjectileDef>,
    ) {
        if let Some(projectile) = projectile {
            self.primary_projectiles.insert(id, projectile);
        } else {
            self.primary_projectiles.remove(&id);
        }
    }

    pub fn projectile(&self, id: EntityId) -> Option<&ProjectileState> {
        self.projectiles.get(&id)
    }

    pub fn projectile_mut(&mut self, id: EntityId) -> Option<&mut ProjectileState> {
        self.projectiles.get_mut(&id)
    }

    pub fn set_projectile(&mut self, id: EntityId, projectile: Option<ProjectileState>) {
        if let Some(projectile) = projectile {
            self.projectiles.insert(id, projectile);
        } else {
            self.projectiles.remove(&id);
        }
    }

    pub fn pickup(&self, id: EntityId) -> Option<&PickupDef> {
        self.pickups.get(&id)
    }

    pub fn pickup_mut(&mut self, id: EntityId) -> Option<&mut PickupDef> {
        self.pickups.get_mut(&id)
    }

    pub fn set_pickup(&mut self, id: EntityId, pickup: Option<PickupDef>) {
        if let Some(pickup) = pickup {
            self.pickups.insert(id, pickup);
        } else {
            self.pickups.remove(&id);
        }
    }

    pub fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories.get(&id)
    }

    pub fn inventory_mut(&mut self, id: EntityId) -> Option<&mut Inventory> {
        self.inventories.get_mut(&id)
    }

    pub fn ensure_inventory(&mut self, id: EntityId) -> &mut Inventory {
        self.inventories.entry(id).or_default()
    }

    pub fn set_inventory(&mut self, id: EntityId, inventory: Option<Inventory>) {
        match inventory {
            Some(inventory) if !inventory.is_empty() => {
                self.inventories.insert(id, inventory);
            }
            Some(_) | None => {
                self.inventories.remove(&id);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct EntitySpawnBundle {
    pub entity: Entity,
    pub optional_components: EntityOptionalComponents,
    pub audio_component: EntityAudioComponent,
}
