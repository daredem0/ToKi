//! Rule command application.
//!
//! Contains logic for applying buffered rule commands to game state.

use std::collections::HashMap;

use crate::animation::AnimationState;
use crate::assets::tilemap::TileMap;
use crate::entity::EntityId;
use crate::events::GameUpdateResult;
use crate::events::PersistenceRequest;

use super::super::stat_effects::StatEffectService;
use super::{
    AnimationCommand, AppliedRuleCommandResult, AudioCommand, AudioEvent, EntityCommand,
    InventoryCommand, MotionCommand, PendingDialogStart, PendingSceneSwitch, ProgressCommand,
    RuleCommand, RuleEvaluationService, SceneCommand, UiCommand,
};

impl RuleEvaluationService<'_> {
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
        let mut pending_ui_requests = Vec::new();

        for command in &commands {
            match command {
                RuleCommand::Audio(command) => self.apply_audio_command(
                    command,
                    result,
                ),
                RuleCommand::Animation(command) => {
                    self.apply_animation_command(command, &mut buffered_animations)
                }
                RuleCommand::Motion(command) => {
                    self.apply_motion_command(command, &mut buffered_velocities)
                }
                RuleCommand::Scene(command) => self.apply_scene_command(
                    command,
                    &mut pending_scene_switch,
                    &mut pending_dialog_start,
                ),
                RuleCommand::Ui(command) => self.apply_ui_command(command, &mut pending_ui_requests),
                RuleCommand::Progress(command) => {
                    self.apply_progress_side_effects(command);
                    self.apply_progress_command(command, &mut pending_persistence)
                }
                RuleCommand::Entity(command) => self.apply_entity_command(command, tilemap),
                RuleCommand::Inventory(command) => self.apply_inventory_command(command),
            };
        }

        for (entity_id, velocity) in buffered_velocities {
            self.runtime.rules.velocities.insert(entity_id, velocity);
        }

        let mut pending_animations = buffered_animations.into_iter().collect::<Vec<_>>();
        pending_animations.sort_by_key(|(entity_id, _)| *entity_id);
        if let Some(request) = &pending_dialog_start {
            result.request_dialog_start(request.dialog_id.clone(), request.context);
        }
        AppliedRuleCommandResult {
            pending_animations,
            pending_scene_switch,
            pending_persistence,
            pending_ui_requests,
        }
    }

    fn apply_destroy_self(&mut self, entity_id: EntityId) {
        let removed = self.world.entity_manager.despawn_entity(entity_id);
        if removed {
            self.runtime.rules.velocities.remove(&entity_id);
        }
    }

    fn stat_effect_service(&mut self) -> StatEffectService<'_> {
        StatEffectService::new(
            &mut self.world.entity_manager,
            &mut self.runtime.rules,
            &mut self.runtime.effects.pending_stat_changes,
            &mut self.runtime.effects.pending_despawns,
        )
    }

    fn apply_audio_command(
        &mut self,
        command: &AudioCommand,
        result: &mut GameUpdateResult<AudioEvent>,
    ) {
        match command {
            AudioCommand::PlaySound { channel, sound_id } => {
                result.add_event(AudioEvent::PlaySound {
                    channel: *channel,
                    sound_id: sound_id.clone(),
                    source_position: None,
                    hearing_radius: None,
                });
            }
            AudioCommand::PlayMusic { track_id } => {
                result.add_event(AudioEvent::BackgroundMusic(track_id.clone()));
            }
        }
    }

    fn apply_animation_command(
        &mut self,
        command: &AnimationCommand,
        buffered_animations: &mut HashMap<EntityId, AnimationState>,
    ) {
        match command {
            AnimationCommand::PlayAnimation { entity_id, state } => {
                buffered_animations.entry(*entity_id).or_insert(*state);
            }
        }
    }

    fn apply_motion_command(
        &mut self,
        command: &MotionCommand,
        buffered_velocities: &mut HashMap<EntityId, glam::IVec2>,
    ) {
        match command {
            MotionCommand::SetVelocity { entity_id, velocity } => {
                buffered_velocities.entry(*entity_id).or_insert(*velocity);
            }
        }
    }

    fn apply_scene_command(
        &mut self,
        command: &SceneCommand,
        pending_scene_switch: &mut Option<PendingSceneSwitch>,
        pending_dialog_start: &mut Option<PendingDialogStart>,
    ) {
        match command {
            SceneCommand::SwitchScene {
                scene_name,
                spawn_point_id,
                transition,
                duration_ms,
            } => {
                self.apply_switch_scene(
                    scene_name.as_str(),
                    spawn_point_id,
                    *transition,
                    *duration_ms,
                    pending_scene_switch,
                );
            }
            SceneCommand::StartDialog { dialog_id, context } => {
                if pending_dialog_start.is_none() {
                    *pending_dialog_start = Some(crate::events::DialogStartRequest {
                        dialog_id: dialog_id.clone(),
                        context: *context,
                    });
                }
            }
        }
    }

    fn apply_ui_command(
        &mut self,
        command: &UiCommand,
        pending_ui_requests: &mut Vec<crate::ui_layout::UiRequest>,
    ) {
        match command {
            UiCommand::ShowUi { ui_id } => {
                pending_ui_requests.push(crate::ui_layout::UiRequest::ShowUi {
                    ui_id: ui_id.clone(),
                });
            }
            UiCommand::HideUi { ui_id } => {
                pending_ui_requests.push(crate::ui_layout::UiRequest::HideUi {
                    ui_id: ui_id.clone(),
                });
            }
            UiCommand::UpdateUiBinding {
                ui_id,
                binding_key,
                value,
            } => {
                pending_ui_requests.push(crate::ui_layout::UiRequest::UpdateUiBinding {
                    ui_id: ui_id.clone(),
                    binding_key: binding_key.clone(),
                    value: value.clone(),
                });
            }
        }
    }

    fn apply_progress_command(
        &mut self,
        command: &ProgressCommand,
        pending_persistence: &mut Option<PersistenceRequest>,
    ) {
        match command {
            ProgressCommand::SaveGame { slot } => {
                if pending_persistence.is_none() {
                    *pending_persistence = Some(PersistenceRequest::SaveSlot { slot: *slot });
                }
            }
            ProgressCommand::LoadGame { slot } => {
                if pending_persistence.is_none() {
                    *pending_persistence = Some(PersistenceRequest::LoadSlot { slot: *slot });
                }
            }
            ProgressCommand::SetFlag { .. }
            | ProgressCommand::IncrementFlag { .. }
            | ProgressCommand::ClearFlag { .. } => {}
        }
    }

    fn apply_entity_command(&mut self, command: &EntityCommand, tilemap: &TileMap) {
        match command {
            EntityCommand::Spawn { entity_type, position } => {
                self.spawn_entity_from_rule(*entity_type, *position);
            }
            EntityCommand::DestroySelf { entity_id } => {
                self.apply_destroy_self(*entity_id);
            }
            EntityCommand::DamageEntity { entity_id, amount } => {
                self.stat_effect_service()
                    .queue_damage(*entity_id, *amount, None);
            }
            EntityCommand::HealEntity { entity_id, amount } => {
                self.stat_effect_service()
                    .queue_capped_heal(*entity_id, *amount);
            }
            EntityCommand::SetEntityActive { entity_id, active } => {
                self.stat_effect_service()
                    .set_entity_active(*entity_id, *active);
            }
            EntityCommand::TeleportEntity {
                entity_id,
                tile_x,
                tile_y,
            } => {
                self.stat_effect_service()
                    .teleport_entity_to_tile(*entity_id, *tile_x, *tile_y, tilemap);
            }
        }
    }

    fn apply_inventory_command(&mut self, command: &InventoryCommand) {
        match command {
            InventoryCommand::AddInventoryItem {
                entity_id,
                item_id,
                count,
            } => {
                self.stat_effect_service()
                    .add_inventory_item(*entity_id, item_id, *count);
            }
            InventoryCommand::RemoveInventoryItem {
                entity_id,
                item_id,
                count,
            } => {
                self.stat_effect_service()
                    .remove_inventory_item(*entity_id, item_id, *count);
            }
        }
    }

    fn apply_progress_side_effects(&mut self, command: &ProgressCommand) {
        match command {
            ProgressCommand::SetFlag { flag, value } => {
                self.progress.game_flags.set(flag.clone(), value.clone());
            }
            ProgressCommand::IncrementFlag { flag, amount } => {
                self.progress.game_flags.increment(flag.clone(), *amount);
            }
            ProgressCommand::ClearFlag { flag } => {
                self.progress.game_flags.clear(flag);
            }
            ProgressCommand::SaveGame { .. } | ProgressCommand::LoadGame { .. } => {}
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
