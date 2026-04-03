use super::GameState;

impl GameState {
    pub fn player_inventory_entries(&self) -> Vec<crate::menu::InventoryEntry> {
        let Some(player) = self
            .world
            .player_id()
            .and_then(|player_id| self.world.entity_manager.get_entity(player_id))
        else {
            return Vec::new();
        };

        let mut entries = self
            .world
            .entity_manager
            .storage()
            .components()
            .inventory(player.id)
            .into_iter()
            .flat_map(|inventory| inventory.items.iter())
            .map(|(item_id, count)| crate::menu::InventoryEntry {
                item_id: item_id.clone(),
                count: *count,
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| a.item_id.cmp(&b.item_id));
        entries
    }
}
