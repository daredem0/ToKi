use crate::collision;

use super::GameState;

impl GameState {
    pub fn player_inventory_entries(&self) -> Vec<crate::menu::InventoryEntry> {
        let Some(player) = self.player_entity() else {
            return Vec::new();
        };

        let mut entries = player
            .attributes
            .inventory
            .items
            .iter()
            .map(|(item_id, count)| crate::menu::InventoryEntry {
                item_id: item_id.clone(),
                count: *count,
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| a.item_id.cmp(&b.item_id));
        entries
    }

    pub(super) fn collect_overlapping_pickups(&mut self) {
        let mut collector_ids = self
            .world
            .entity_manager
            .active_entities()
            .into_iter()
            .filter(|&entity_id| {
                self.world.entity_manager
                    .get_entity(entity_id)
                    .is_some_and(|entity| entity.attributes.has_inventory)
            })
            .collect::<Vec<_>>();
        collector_ids.sort_unstable();

        let mut pickup_ids = self
            .world
            .entity_manager
            .active_entities()
            .into_iter()
            .filter(|&entity_id| {
                self.world.entity_manager
                    .get_entity(entity_id)
                    .and_then(|entity| entity.attributes.pickup.as_ref())
                    .is_some()
            })
            .collect::<Vec<_>>();
        pickup_ids.sort_unstable();

        let mut collected = Vec::new();
        for collector_id in collector_ids {
            let Some(collector) = self.world.entity_manager.get_entity(collector_id) else {
                continue;
            };
            let (collector_pos, collector_size) = collector.interaction_bounds();

            for &pickup_id in &pickup_ids {
                let Some(pickup_entity) = self.world.entity_manager.get_entity(pickup_id) else {
                    continue;
                };
                let Some(pickup) = pickup_entity.attributes.pickup.as_ref() else {
                    continue;
                };
                if pickup.count == 0 || pickup.item_id.is_empty() {
                    continue;
                }

                let (pickup_pos, pickup_size) = pickup_entity.interaction_bounds();
                if !collision::aabb_overlap(collector_pos, collector_size, pickup_pos, pickup_size)
                {
                    continue;
                }

                collected.push((
                    collector_id,
                    pickup_id,
                    pickup.item_id.clone(),
                    pickup.count,
                ));
            }
        }

        collected.sort_unstable_by_key(|(_, pickup_id, _, _)| *pickup_id);
        collected.dedup_by_key(|(_, pickup_id, _, _)| *pickup_id);

        for (collector_id, pickup_id, item_id, count) in collected {
            let Some(collector) = self.world.entity_manager.get_entity_mut(collector_id) else {
                continue;
            };
            if !collector.attributes.has_inventory {
                continue;
            }

            collector.attributes.inventory.add_item(&item_id, count);
            tracing::debug!(
                "Entity {} collected pickup {} item_id={} count={} new_count={}",
                collector_id,
                pickup_id,
                item_id,
                count,
                collector.attributes.inventory.item_count(&item_id)
            );

            self.world.entity_manager.despawn_entity(pickup_id);
        }
    }
}
