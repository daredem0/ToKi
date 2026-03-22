use crate::assets::tilemap::TileMap;
use crate::entity::{EntityId, EntityManager, HEALTH_STAT_ID};

use super::rules::{DamageEvent, DeathEvent, RuleRuntimeState};
use super::GameState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StatChangeRequest {
    pub(super) target_entity_id: EntityId,
    pub(super) stat_id: String,
    pub(super) delta: i32,
    pub(super) source_entity_id: Option<EntityId>,
}

pub(super) struct StatEffectService<'a> {
    entity_manager: &'a mut EntityManager,
    rule_runtime: &'a mut RuleRuntimeState,
    pending_stat_changes: &'a mut Vec<StatChangeRequest>,
    pending_despawns: &'a mut Vec<EntityId>,
}

impl<'a> StatEffectService<'a> {
    pub(super) fn new(
        entity_manager: &'a mut EntityManager,
        rule_runtime: &'a mut RuleRuntimeState,
        pending_stat_changes: &'a mut Vec<StatChangeRequest>,
        pending_despawns: &'a mut Vec<EntityId>,
    ) -> Self {
        Self {
            entity_manager,
            rule_runtime,
            pending_stat_changes,
            pending_despawns,
        }
    }

    pub(super) fn queue_damage(
        &mut self,
        entity_id: EntityId,
        amount: i32,
        source_entity_id: Option<EntityId>,
    ) {
        self.pending_stat_changes.push(StatChangeRequest {
            target_entity_id: entity_id,
            stat_id: HEALTH_STAT_ID.to_string(),
            delta: -amount,
            source_entity_id,
        });
    }

    pub(super) fn queue_capped_heal(&mut self, entity_id: EntityId, amount: i32) {
        let Some(entity) = self.entity_manager.get_entity(entity_id) else {
            return;
        };
        let current = entity.attributes.current_stat(HEALTH_STAT_ID).unwrap_or(0);
        let max = entity.attributes.base_stat(HEALTH_STAT_ID).unwrap_or(0);
        let capped_heal = amount.min(max - current);
        if capped_heal > 0 {
            self.pending_stat_changes.push(StatChangeRequest {
                target_entity_id: entity_id,
                stat_id: HEALTH_STAT_ID.to_string(),
                delta: capped_heal,
                source_entity_id: None,
            });
        }
    }

    pub(super) fn add_inventory_item(&mut self, entity_id: EntityId, item_id: &str, count: u32) {
        if let Some(entity) = self.entity_manager.get_entity_mut(entity_id) {
            entity.attributes.inventory.add_item(item_id, count);
        }
    }

    pub(super) fn remove_inventory_item(&mut self, entity_id: EntityId, item_id: &str, count: u32) {
        let Some(entity) = self.entity_manager.get_entity_mut(entity_id) else {
            return;
        };
        let available = entity.attributes.inventory.item_count(item_id);
        let to_remove = count.min(available);
        if to_remove == 0 {
            return;
        }

        let new_count = available.saturating_sub(to_remove);
        if new_count == 0 {
            entity.attributes.inventory.items.remove(item_id);
        } else if let Some(entry) = entity.attributes.inventory.items.get_mut(item_id) {
            *entry = new_count;
        }
    }

    pub(super) fn set_entity_active(&mut self, entity_id: EntityId, active: bool) {
        if let Some(entity) = self.entity_manager.get_entity_mut(entity_id) {
            entity.attributes.active = active;
        }
    }

    pub(super) fn teleport_entity_to_tile(
        &mut self,
        entity_id: EntityId,
        tile_x: u32,
        tile_y: u32,
        tilemap: &TileMap,
    ) {
        if let Some(entity) = self.entity_manager.get_entity_mut(entity_id) {
            entity.position = glam::IVec2::new(
                (tile_x * tilemap.tile_size.x) as i32,
                (tile_y * tilemap.tile_size.y) as i32,
            );
        }
    }

    pub(super) fn resolve_pending_stat_changes(&mut self) {
        let pending_stat_changes = std::mem::take(self.pending_stat_changes);
        if pending_stat_changes.is_empty() {
            return;
        }

        let mut despawn_ids = Vec::new();
        for change in pending_stat_changes {
            let Some(entity) = self.entity_manager.get_entity_mut(change.target_entity_id) else {
                continue;
            };
            let previous_value = entity.attributes.current_stat(&change.stat_id);
            let Some(new_value) = entity
                .attributes
                .apply_stat_delta(&change.stat_id, change.delta)
            else {
                continue;
            };

            tracing::debug!(
                "Applied stat change: source={:?} target={} stat={} delta={} previous={:?} new={}",
                change.source_entity_id,
                change.target_entity_id,
                change.stat_id,
                change.delta,
                previous_value,
                new_value
            );

            if change.stat_id == HEALTH_STAT_ID && change.delta < 0 {
                self.rule_runtime.frame_damage_events.push(DamageEvent {
                    victim: change.target_entity_id,
                    attacker: change.source_entity_id,
                });
            }

            if change.stat_id == HEALTH_STAT_ID && new_value <= 0 {
                self.rule_runtime.frame_death_events.push(DeathEvent {
                    victim: change.target_entity_id,
                    attacker: change.source_entity_id,
                });
                tracing::info!(
                    "Entity {} reached zero {} and will be deferred for despawn",
                    change.target_entity_id,
                    change.stat_id
                );
                despawn_ids.push(change.target_entity_id);
            }
        }

        despawn_ids.sort_unstable();
        despawn_ids.dedup();
        self.pending_despawns.extend(despawn_ids);
    }

    pub(super) fn flush_pending_despawns(&mut self) {
        let pending_despawns = std::mem::take(self.pending_despawns);
        for entity_id in pending_despawns {
            self.entity_manager.despawn_entity(entity_id);
        }
    }
}

impl GameState {
    pub(super) fn stat_effect_service(&mut self) -> StatEffectService<'_> {
        StatEffectService::new(
            &mut self.entity_manager,
            &mut self.rule_runtime,
            &mut self.pending_stat_changes,
            &mut self.pending_despawns,
        )
    }

    pub(super) fn resolve_pending_stat_changes(&mut self) {
        self.stat_effect_service().resolve_pending_stat_changes();
    }

    pub(super) fn flush_pending_despawns(&mut self) {
        self.stat_effect_service().flush_pending_despawns();
    }
}

#[cfg(test)]
mod tests {
    use super::StatEffectService;
    use crate::entity::{EntityManager, EntityStats};
    use crate::game::rules::RuleRuntimeState;

    #[test]
    fn queue_capped_heal_never_exceeds_base_health() {
        let mut entity_manager = EntityManager::new();
        let entity_id = entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(0, 0),
            glam::UVec2::new(16, 16),
            crate::entity::EntityAttributes {
                health: Some(50),
                stats: EntityStats::from_legacy_health(Some(50)),
                ..crate::entity::EntityAttributes::default()
            },
        );
        entity_manager
            .get_entity_mut(entity_id)
            .expect("entity should exist")
            .attributes
            .apply_stat_delta(crate::entity::HEALTH_STAT_ID, -40);

        let mut rule_runtime = RuleRuntimeState::default();
        let mut pending_stat_changes = Vec::new();
        let mut pending_despawns = Vec::new();
        let mut service = StatEffectService::new(
            &mut entity_manager,
            &mut rule_runtime,
            &mut pending_stat_changes,
            &mut pending_despawns,
        );

        service.queue_capped_heal(entity_id, 100);

        assert_eq!(pending_stat_changes.len(), 1);
        assert_eq!(pending_stat_changes[0].delta, 40);
    }

    #[test]
    fn resolve_pending_stat_changes_records_damage_and_death_and_defers_despawn() {
        let mut entity_manager = EntityManager::new();
        let entity_id = entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(0, 0),
            glam::UVec2::new(16, 16),
            crate::entity::EntityAttributes {
                health: Some(10),
                stats: EntityStats::from_legacy_health(Some(10)),
                ..crate::entity::EntityAttributes::default()
            },
        );
        let mut rule_runtime = RuleRuntimeState::default();
        let mut pending_stat_changes = vec![super::StatChangeRequest {
            target_entity_id: entity_id,
            stat_id: crate::entity::HEALTH_STAT_ID.to_string(),
            delta: -10,
            source_entity_id: Some(99),
        }];
        let mut pending_despawns = Vec::new();
        let mut service = StatEffectService::new(
            &mut entity_manager,
            &mut rule_runtime,
            &mut pending_stat_changes,
            &mut pending_despawns,
        );

        service.resolve_pending_stat_changes();

        assert_eq!(rule_runtime.frame_damage_events.len(), 1);
        assert_eq!(rule_runtime.frame_death_events.len(), 1);
        assert_eq!(pending_despawns, vec![entity_id]);
    }
}
