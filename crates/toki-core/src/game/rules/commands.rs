//! Rule command application.
//!
//! Contains logic for applying buffered rule commands to game state.

use std::collections::HashMap;

use crate::animation::AnimationState;
use crate::assets::tilemap::TileMap;
use crate::entity::EntityId;
use crate::events::GameUpdateResult;
use crate::events::PersistenceRequest;

use super::{
    AppliedRuleCommandResult, AudioEvent, GameState, PendingDialogStart, PendingSceneSwitch,
    RuleCommand,
};

impl GameState {
    pub(in crate::game) fn apply_rule_commands(
        &mut self,
        commands: Vec<RuleCommand>,
        result: &mut GameUpdateResult<AudioEvent>,
        tilemap: &TileMap,
    ) -> AppliedRuleCommandResult {
        let mut buffered_velocities = HashMap::new();
        let mut buffered_animations = HashMap::new();
        let mut pending_scene_switch = None;
        let mut pending_dialog_start = None;
        let mut pending_persistence = None;

        for command in commands {
            if self.apply_audio_or_motion_command(
                command.clone(),
                result,
                &mut buffered_velocities,
                &mut buffered_animations,
            ) || self.apply_scene_dialog_or_persistence_command(
                command.clone(),
                &mut pending_scene_switch,
                &mut pending_dialog_start,
                &mut pending_persistence,
            ) || self.apply_entity_mutation_command(command.clone(), tilemap)
                || self.apply_inventory_or_flag_command(command, &mut pending_persistence)
            {
                continue;
            }
        }

        for (entity_id, velocity) in buffered_velocities {
            self.runtime.rules.velocities.insert(entity_id, velocity);
        }

        let mut pending_animations = buffered_animations.into_iter().collect::<Vec<_>>();
        pending_animations.sort_by_key(|(entity_id, _)| *entity_id);
        if let Some(request) = &pending_dialog_start {
            result.request_dialog_start(request.dialog_id.clone(), request.context);
        }
        (
            pending_animations,
            pending_scene_switch,
            pending_dialog_start,
            pending_persistence,
        )
    }

    fn apply_destroy_self(&mut self, entity_id: EntityId) {
        let removed = self.world.entity_manager.despawn_entity(entity_id);
        if removed {
            if self.world.player_id == Some(entity_id) {
                self.world.player_id = None;
            }
            self.runtime.rules.velocities.remove(&entity_id);
        }
    }

    fn apply_audio_or_motion_command(
        &mut self,
        command: RuleCommand,
        result: &mut GameUpdateResult<AudioEvent>,
        buffered_velocities: &mut HashMap<EntityId, glam::IVec2>,
        buffered_animations: &mut HashMap<EntityId, AnimationState>,
    ) -> bool {
        match command {
            RuleCommand::PlaySound { channel, sound_id } => {
                result.add_event(AudioEvent::PlaySound {
                    channel,
                    sound_id,
                    source_position: None,
                    hearing_radius: None,
                });
                true
            }
            RuleCommand::PlayMusic { track_id } => {
                result.add_event(AudioEvent::BackgroundMusic(track_id));
                true
            }
            RuleCommand::SetVelocity {
                entity_id,
                velocity,
            } => {
                buffered_velocities.entry(entity_id).or_insert(velocity);
                true
            }
            RuleCommand::PlayAnimation { entity_id, state } => {
                buffered_animations.entry(entity_id).or_insert(state);
                true
            }
            _ => false,
        }
    }

    fn apply_scene_dialog_or_persistence_command(
        &mut self,
        command: RuleCommand,
        pending_scene_switch: &mut Option<PendingSceneSwitch>,
        pending_dialog_start: &mut Option<PendingDialogStart>,
        pending_persistence: &mut Option<PersistenceRequest>,
    ) -> bool {
        match command {
            RuleCommand::Spawn {
                entity_type,
                position,
            } => {
                self.spawn_entity_from_rule(entity_type, position);
                true
            }
            RuleCommand::DestroySelf { entity_id } => {
                self.apply_destroy_self(entity_id);
                true
            }
            RuleCommand::SwitchScene {
                scene_name,
                spawn_point_id,
                transition,
                duration_ms,
            } => {
                self.apply_switch_scene(
                    &scene_name,
                    &spawn_point_id,
                    transition,
                    duration_ms,
                    pending_scene_switch,
                );
                true
            }
            RuleCommand::StartDialog { dialog_id, context } => {
                if pending_dialog_start.is_none() {
                    *pending_dialog_start =
                        Some(crate::events::DialogStartRequest { dialog_id, context });
                }
                true
            }
            RuleCommand::SaveGame { slot } => {
                if pending_persistence.is_none() {
                    *pending_persistence = Some(PersistenceRequest::SaveSlot { slot });
                }
                true
            }
            RuleCommand::LoadGame { slot } => {
                if pending_persistence.is_none() {
                    *pending_persistence = Some(PersistenceRequest::LoadSlot { slot });
                }
                true
            }
            _ => false,
        }
    }

    fn apply_entity_mutation_command(&mut self, command: RuleCommand, tilemap: &TileMap) -> bool {
        match command {
            RuleCommand::DamageEntity { entity_id, amount } => {
                self.stat_effect_service()
                    .queue_damage(entity_id, amount, None);
                true
            }
            RuleCommand::HealEntity { entity_id, amount } => {
                self.stat_effect_service()
                    .queue_capped_heal(entity_id, amount);
                true
            }
            RuleCommand::SetEntityActive { entity_id, active } => {
                self.stat_effect_service()
                    .set_entity_active(entity_id, active);
                true
            }
            RuleCommand::TeleportEntity {
                entity_id,
                tile_x,
                tile_y,
            } => {
                self.stat_effect_service()
                    .teleport_entity_to_tile(entity_id, tile_x, tile_y, tilemap);
                true
            }
            _ => false,
        }
    }

    fn apply_inventory_or_flag_command(
        &mut self,
        command: RuleCommand,
        _pending_persistence: &mut Option<PersistenceRequest>,
    ) -> bool {
        match command {
            RuleCommand::AddInventoryItem {
                entity_id,
                item_id,
                count,
            } => {
                self.stat_effect_service()
                    .add_inventory_item(entity_id, &item_id, count);
                true
            }
            RuleCommand::RemoveInventoryItem {
                entity_id,
                item_id,
                count,
            } => {
                self.stat_effect_service()
                    .remove_inventory_item(entity_id, &item_id, count);
                true
            }
            RuleCommand::SetFlag { flag, value } => {
                self.set_flag(flag, value);
                true
            }
            RuleCommand::IncrementFlag { flag, amount } => {
                self.increment_flag(flag, amount);
                true
            }
            RuleCommand::ClearFlag { flag } => {
                self.clear_flag(&flag);
                true
            }
            _ => false,
        }
    }

    fn apply_switch_scene(
        &self,
        scene_name: &str,
        spawn_point_id: &str,
        transition: Option<crate::project_runtime::SceneTransitionEffect>,
        duration_ms: Option<u32>,
        pending_scene_switch: &mut Option<PendingSceneSwitch>,
    ) {
        let target = scene_name.trim();
        let spawn = spawn_point_id.trim();
        if !target.is_empty() && !spawn.is_empty() && pending_scene_switch.is_none() {
            *pending_scene_switch = Some(crate::events::SceneSwitchRequest {
                scene_name: target.into(),
                spawn_point_id: spawn.to_string(),
                transition,
                duration_ms,
            });
        }
    }
}
