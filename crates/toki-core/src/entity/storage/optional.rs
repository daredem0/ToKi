use super::bundle::{
    Inventory, OptionalEntityComponents, PickupDef, PrimaryProjectileDef, ProjectileState,
};
use super::sparse_map::SparseComponentMap;
use crate::entity::EntityId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OptionalComponentRegistry {
    #[serde(default, skip_serializing_if = "SparseComponentMap::is_empty")]
    primary_projectiles: SparseComponentMap<PrimaryProjectileDef>,
    #[serde(default, skip_serializing_if = "SparseComponentMap::is_empty")]
    projectiles: SparseComponentMap<ProjectileState>,
    #[serde(default, skip_serializing_if = "SparseComponentMap::is_empty")]
    pickups: SparseComponentMap<PickupDef>,
    #[serde(default, skip_serializing_if = "SparseComponentMap::is_empty")]
    inventories: SparseComponentMap<Inventory>,
}

impl OptionalComponentRegistry {
    fn set_optional<T>(map: &mut SparseComponentMap<T>, id: EntityId, value: Option<T>) {
        if let Some(value) = value {
            map.insert(id, value);
        } else {
            map.remove(id);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.primary_projectiles.is_empty()
            && self.projectiles.is_empty()
            && self.pickups.is_empty()
            && self.inventories.is_empty()
    }

    pub fn optional_components(&self, id: EntityId) -> OptionalEntityComponents {
        OptionalEntityComponents {
            primary_projectile: self.primary_projectiles.get(id).cloned(),
            projectile: self.projectiles.get(id).cloned(),
            pickup: self.pickups.get(id).cloned(),
            inventory: self.inventories.get(id).cloned(),
        }
    }

    pub fn apply_optional_components(&mut self, id: EntityId, components: OptionalEntityComponents) {
        self.set_primary_projectile(id, components.primary_projectile);
        self.set_projectile(id, components.projectile);
        self.set_pickup(id, components.pickup);
        self.set_inventory(id, components.inventory);
    }

    pub fn remove_all(&mut self, id: EntityId) {
        self.primary_projectiles.remove(id);
        self.projectiles.remove(id);
        self.pickups.remove(id);
        self.inventories.remove(id);
    }

    pub fn primary_projectile(&self, id: EntityId) -> Option<&PrimaryProjectileDef> {
        self.primary_projectiles.get(id)
    }

    pub fn primary_projectile_mut(&mut self, id: EntityId) -> Option<&mut PrimaryProjectileDef> {
        self.primary_projectiles.get_mut(id)
    }

    pub fn set_primary_projectile(
        &mut self,
        id: EntityId,
        projectile: Option<PrimaryProjectileDef>,
    ) {
        Self::set_optional(&mut self.primary_projectiles, id, projectile);
    }

    pub fn projectile(&self, id: EntityId) -> Option<&ProjectileState> {
        self.projectiles.get(id)
    }

    pub fn projectile_mut(&mut self, id: EntityId) -> Option<&mut ProjectileState> {
        self.projectiles.get_mut(id)
    }

    pub fn set_projectile(&mut self, id: EntityId, projectile: Option<ProjectileState>) {
        Self::set_optional(&mut self.projectiles, id, projectile);
    }

    pub fn pickup(&self, id: EntityId) -> Option<&PickupDef> {
        self.pickups.get(id)
    }

    pub fn pickup_mut(&mut self, id: EntityId) -> Option<&mut PickupDef> {
        self.pickups.get_mut(id)
    }

    pub fn set_pickup(&mut self, id: EntityId, pickup: Option<PickupDef>) {
        Self::set_optional(&mut self.pickups, id, pickup);
    }

    pub fn inventory(&self, id: EntityId) -> Option<&Inventory> {
        self.inventories.get(id)
    }

    pub fn inventory_mut(&mut self, id: EntityId) -> Option<&mut Inventory> {
        self.inventories.get_mut(id)
    }

    pub fn ensure_inventory(&mut self, id: EntityId) -> &mut Inventory {
        if !self.inventories.contains(id) {
            self.inventories.insert(id, Inventory::default());
        }
        self.inventories
            .get_mut(id)
            .expect("inventory should exist after insertion")
    }

    pub fn set_inventory(&mut self, id: EntityId, inventory: Option<Inventory>) {
        match inventory {
            Some(inventory) if !inventory.is_empty() => {
                Self::set_optional(&mut self.inventories, id, Some(inventory));
            }
            Some(_) | None => {
                Self::set_optional(&mut self.inventories, id, None);
            }
        }
    }

    pub fn primary_projectile_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.primary_projectiles.ids()
    }

    pub fn projectile_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.projectiles.ids()
    }

    pub fn pickup_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.pickups.ids()
    }

    pub fn inventory_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.inventories.ids()
    }

    pub fn iter_projectiles(&self) -> impl Iterator<Item = (EntityId, &ProjectileState)> + '_ {
        self.projectiles.iter()
    }

    pub fn iter_pickups(&self) -> impl Iterator<Item = (EntityId, &PickupDef)> + '_ {
        self.pickups.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_apply_extract_and_remove_all() {
        let mut registry = OptionalComponentRegistry::default();
        let mut inventory = Inventory::default();
        inventory.add_item("coin", 3);
        registry.apply_optional_components(
            7,
            OptionalEntityComponents {
                primary_projectile: Some(PrimaryProjectileDef {
                    sheet: "fx".to_string(),
                    object_name: "bolt".to_string(),
                    size: [8, 8],
                    speed: 4,
                    damage: 2,
                    lifetime_ticks: 6,
                    spawn_offset: [0, 0],
                }),
                projectile: None,
                pickup: Some(PickupDef {
                    item_id: "gem".to_string(),
                    count: 2,
                }),
                inventory: Some(inventory),
            },
        );
        assert_eq!(registry.primary_projectile_ids().collect::<Vec<_>>(), vec![7]);
        assert_eq!(registry.pickup_ids().collect::<Vec<_>>(), vec![7]);
        assert_eq!(registry.inventory_ids().collect::<Vec<_>>(), vec![7]);
        assert_eq!(
            registry
                .optional_components(7)
                .inventory
                .expect("inventory should exist")
                .item_count("coin"),
            3
        );
        registry.remove_all(7);
        assert!(registry.is_empty());
    }
}
