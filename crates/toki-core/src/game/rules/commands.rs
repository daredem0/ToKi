//! Rule command application.
//!
//! Contains logic for applying buffered rule commands to game state.

use std::collections::HashMap;

use crate::animation::AnimationState;
use crate::assets::tilemap::TileMap;
use crate::entity::EntityId;
use crate::events::GameUpdateResult;

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
                    self.apply_switch_scene(
                        &scene_name,
                        &spawn_point_id,
                        &mut pending_scene_switch,
                    );
                }
                RuleCommand::DamageEntity { entity_id, amount } => {
                    self.stat_effect_service()
                        .queue_damage(entity_id, amount, None);
                }
                RuleCommand::HealEntity { entity_id, amount } => {
                    self.stat_effect_service()
                        .queue_capped_heal(entity_id, amount);
                }
                RuleCommand::AddInventoryItem {
                    entity_id,
                    item_id,
                    count,
                } => {
                    self.stat_effect_service()
                        .add_inventory_item(entity_id, &item_id, count);
                }
                RuleCommand::RemoveInventoryItem {
                    entity_id,
                    item_id,
                    count,
                } => {
                    self.stat_effect_service()
                        .remove_inventory_item(entity_id, &item_id, count);
                }
                RuleCommand::SetEntityActive { entity_id, active } => {
                    self.stat_effect_service()
                        .set_entity_active(entity_id, active);
                }
                RuleCommand::TeleportEntity {
                    entity_id,
                    tile_x,
                    tile_y,
                } => {
                    self.stat_effect_service()
                        .teleport_entity_to_tile(entity_id, tile_x, tile_y, tilemap);
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
}
