//! Rule command application.
//!
//! Contains logic for applying buffered rule commands to game state.

use std::collections::HashMap;

use crate::animation::AnimationState;
use crate::assets::tilemap::TileMap;
use crate::entity::{EntityId, HEALTH_STAT_ID};
use crate::events::GameUpdateResult;

use crate::game::combat::StatChangeRequest;
use super::{AudioEvent, GameState, PendingSceneSwitch, RuleCommand};

impl GameState {
    pub(in crate::game) fn apply_rule_commands(
        &mut self,
        commands: Vec<RuleCommand>,
        result: &mut GameUpdateResult<AudioEvent>,
        tilemap: &TileMap,
    ) -> (Vec<(EntityId, AnimationState)>, Option<PendingSceneSwitch>) {
        let mut buffered_velocities = HashMap::new();
        let mut buffered_animations = HashMap::new();
        let mut pending_scene_switch = None;

        for command in commands {
            match command {
                RuleCommand::PlaySound { channel, sound_id } => {
                    result.add_event(AudioEvent::PlaySound {
                        channel,
                        sound_id,
                        source_position: None,
                        hearing_radius: None,
                    });
                }
                RuleCommand::PlayMusic { track_id } => {
                    result.add_event(AudioEvent::BackgroundMusic(track_id));
                }
                RuleCommand::SetVelocity {
                    entity_id,
                    velocity,
                } => {
                    buffered_velocities.entry(entity_id).or_insert(velocity);
                }
                RuleCommand::PlayAnimation { entity_id, state } => {
                    buffered_animations.entry(entity_id).or_insert(state);
                }
                RuleCommand::Spawn {
                    entity_type,
                    position,
                } => {
                    self.spawn_entity_from_rule(entity_type, position);
                }
                RuleCommand::DestroySelf { entity_id } => {
                    self.apply_destroy_self(entity_id);
                }
                RuleCommand::SwitchScene {
                    scene_name,
                    spawn_point_id,
                } => {
                    self.apply_switch_scene(&scene_name, &spawn_point_id, &mut pending_scene_switch);
                }
                RuleCommand::DamageEntity { entity_id, amount } => {
                    self.apply_damage_entity(entity_id, amount);
                }
                RuleCommand::HealEntity { entity_id, amount } => {
                    self.apply_heal_entity(entity_id, amount);
                }
                RuleCommand::AddInventoryItem {
                    entity_id,
                    item_id,
                    count,
                } => {
                    if let Some(entity) = self.entity_manager.get_entity_mut(entity_id) {
                        entity.attributes.inventory.add_item(&item_id, count);
                    }
                }
                RuleCommand::RemoveInventoryItem {
                    entity_id,
                    item_id,
                    count,
                } => {
                    self.apply_remove_inventory_item(entity_id, &item_id, count);
                }
                RuleCommand::SetEntityActive { entity_id, active } => {
                    if let Some(entity) = self.entity_manager.get_entity_mut(entity_id) {
                        entity.attributes.active = active;
                    }
                }
                RuleCommand::TeleportEntity {
                    entity_id,
                    tile_x,
                    tile_y,
                } => {
                    self.apply_teleport_entity(entity_id, tile_x, tile_y, tilemap);
                }
            }
        }

        for (entity_id, velocity) in buffered_velocities {
            self.rule_runtime.velocities.insert(entity_id, velocity);
        }

        let mut pending_animations = buffered_animations.into_iter().collect::<Vec<_>>();
        pending_animations.sort_by_key(|(entity_id, _)| *entity_id);
        (pending_animations, pending_scene_switch)
    }

    fn apply_destroy_self(&mut self, entity_id: EntityId) {
        let removed = self.entity_manager.despawn_entity(entity_id);
        if removed {
            if self.player_id == Some(entity_id) {
                self.player_id = None;
            }
            self.rule_runtime.velocities.remove(&entity_id);
        }
    }

    fn apply_switch_scene(
        &self,
        scene_name: &str,
        spawn_point_id: &str,
        pending_scene_switch: &mut Option<PendingSceneSwitch>,
    ) {
        let target = scene_name.trim();
        let spawn = spawn_point_id.trim();
        if !target.is_empty() && !spawn.is_empty() && pending_scene_switch.is_none() {
            *pending_scene_switch = Some((target.to_string(), spawn.to_string()));
        }
    }

    fn apply_damage_entity(&mut self, entity_id: EntityId, amount: i32) {
        self.pending_stat_changes.push(StatChangeRequest {
            target_entity_id: entity_id,
            stat_id: HEALTH_STAT_ID.to_string(),
            delta: -amount,
            source_entity_id: None,
        });
    }

    fn apply_heal_entity(&mut self, entity_id: EntityId, amount: i32) {
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

    fn apply_remove_inventory_item(&mut self, entity_id: EntityId, item_id: &str, count: u32) {
        let Some(entity) = self.entity_manager.get_entity_mut(entity_id) else {
            return;
        };
        let available = entity.attributes.inventory.item_count(item_id);
        let to_remove = count.min(available);
        if to_remove > 0 {
            let new_count = available.saturating_sub(to_remove);
            if new_count == 0 {
                entity.attributes.inventory.items.remove(item_id);
            } else if let Some(entry) = entity.attributes.inventory.items.get_mut(item_id) {
                *entry = new_count;
            }
        }
    }

    fn apply_teleport_entity(
        &mut self,
        entity_id: EntityId,
        tile_x: u32,
        tile_y: u32,
        tilemap: &TileMap,
    ) {
        if let Some(entity) = self.entity_manager.get_entity_mut(entity_id) {
            // Convert tile coordinates to pixel coordinates (top-left of tile)
            let pixel_x = (tile_x * tilemap.tile_size.x) as i32;
            let pixel_y = (tile_y * tilemap.tile_size.y) as i32;
            entity.position = glam::IVec2::new(pixel_x, pixel_y);
        }
    }
}
