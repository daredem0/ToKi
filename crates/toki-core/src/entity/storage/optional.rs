use super::bundle::{
    Inventory, OptionalEntityComponents, PickupDef, PrimaryProjectileDef, ProjectileState,
};
use super::sparse_map::SparseComponentMap;
use crate::entity::{
    AiComponent, CombatComponent, EntityId, InteractionComponent, MovementComponent,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct OptionalComponentRegistry {
    #[serde(default, skip_serializing_if = "SparseComponentMap::is_empty")]
    movements: SparseComponentMap<MovementComponent>,
    #[serde(default, skip_serializing_if = "SparseComponentMap::is_empty")]
    ai_components: SparseComponentMap<AiComponent>,
    #[serde(default, skip_serializing_if = "SparseComponentMap::is_empty")]
    interactions: SparseComponentMap<InteractionComponent>,
    #[serde(default, skip_serializing_if = "SparseComponentMap::is_empty")]
    combats: SparseComponentMap<CombatComponent>,
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
        self.movements.is_empty()
            && self.ai_components.is_empty()
            && self.interactions.is_empty()
            && self.combats.is_empty()
            && self.primary_projectiles.is_empty()
            && self.projectiles.is_empty()
            && self.pickups.is_empty()
            && self.inventories.is_empty()
    }

    pub fn optional_components(&self, id: EntityId) -> OptionalEntityComponents {
        OptionalEntityComponents {
            movement: self.movements.get(id).cloned(),
            ai: self.ai_components.get(id).cloned(),
            interaction: self.interactions.get(id).cloned(),
            combat: self.combats.get(id).cloned(),
            primary_projectile: self.primary_projectiles.get(id).cloned(),
            projectile: self.projectiles.get(id).cloned(),
            pickup: self.pickups.get(id).cloned(),
            inventory: self.inventories.get(id).cloned(),
        }
    }

    pub fn apply_optional_components(
        &mut self,
        id: EntityId,
        components: OptionalEntityComponents,
    ) {
        self.set_movement(id, components.movement);
        self.set_ai(id, components.ai);
        self.set_interaction(id, components.interaction);
        self.set_combat(id, components.combat);
        self.set_primary_projectile(id, components.primary_projectile);
        self.set_projectile(id, components.projectile);
        self.set_pickup(id, components.pickup);
        self.set_inventory(id, components.inventory);
    }

    pub fn remove_all(&mut self, id: EntityId) {
        self.movements.remove(id);
        self.ai_components.remove(id);
        self.interactions.remove(id);
        self.combats.remove(id);
        self.primary_projectiles.remove(id);
        self.projectiles.remove(id);
        self.pickups.remove(id);
        self.inventories.remove(id);
    }

    pub fn movement(&self, id: EntityId) -> Option<&MovementComponent> {
        self.movements.get(id)
    }

    pub fn movement_mut(&mut self, id: EntityId) -> Option<&mut MovementComponent> {
        self.movements.get_mut(id)
    }

    pub fn set_movement(&mut self, id: EntityId, movement: Option<MovementComponent>) {
        Self::set_optional(&mut self.movements, id, movement);
    }

    pub fn ai(&self, id: EntityId) -> Option<&AiComponent> {
        self.ai_components.get(id)
    }

    pub fn ai_mut(&mut self, id: EntityId) -> Option<&mut AiComponent> {
        self.ai_components.get_mut(id)
    }

    pub fn set_ai(&mut self, id: EntityId, ai: Option<AiComponent>) {
        Self::set_optional(&mut self.ai_components, id, ai);
    }

    pub fn interaction(&self, id: EntityId) -> Option<&InteractionComponent> {
        self.interactions.get(id)
    }

    pub fn interaction_mut(&mut self, id: EntityId) -> Option<&mut InteractionComponent> {
        self.interactions.get_mut(id)
    }

    pub fn set_interaction(&mut self, id: EntityId, interaction: Option<InteractionComponent>) {
        Self::set_optional(&mut self.interactions, id, interaction);
    }

    pub fn combat(&self, id: EntityId) -> Option<&CombatComponent> {
        self.combats.get(id)
    }

    pub fn combat_mut(&mut self, id: EntityId) -> Option<&mut CombatComponent> {
        self.combats.get_mut(id)
    }

    pub fn set_combat(&mut self, id: EntityId, combat: Option<CombatComponent>) {
        Self::set_optional(&mut self.combats, id, combat);
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
        Self::set_optional(&mut self.inventories, id, inventory);
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

    pub fn interaction_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.interactions.ids()
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
                movement: None,
                ai: None,
                interaction: None,
                combat: None,
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
        assert_eq!(
            registry.primary_projectile_ids().collect::<Vec<_>>(),
            vec![7]
        );
        assert_eq!(registry.pickup_ids().collect::<Vec<_>>(), vec![7]);
        assert_eq!(registry.inventory_ids().collect::<Vec<_>>(), vec![7]);
        assert!(registry.interaction_ids().collect::<Vec<_>>().is_empty());
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
