//! Rule action buffering.
//!
//! Contains logic for converting rule actions into buffered commands.

use crate::animation::AnimationState;
use crate::rules::{RuleAction, RuleSoundChannel, TriggerContext};
use tracing::info;

use super::{AudioChannel, RuleCommand, RuleEngine};

impl RuleEngine<'_> {
    fn resolve_and_push(
        &self,
        target: crate::rules::RuleTarget,
        context: &TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
        make_command: impl FnOnce(crate::entity::EntityId) -> RuleCommand,
    ) {
        if let Some(entity_id) = self.resolve_rule_target(target, context) {
            command_buffer.push(make_command(entity_id));
        }
    }

    pub(super) fn buffer_rule_action(
        &self,
        rule_id: &str,
        log_enabled: bool,
        action: &RuleAction,
        context: &TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        if log_enabled {
            info!(rule_id = %rule_id, action = ?action, "Rule action passed");
        }
        tracing::debug!("Buffering rule action: {:?}", action);
        match action {
            RuleAction::PlaySound { .. }
            | RuleAction::PlayMusic { .. }
            | RuleAction::PlayAnimation { .. }
            | RuleAction::SetVelocity { .. } => {
                self.buffer_audio_or_motion_action(action, context, command_buffer);
            }
            RuleAction::SwitchScene { .. }
            | RuleAction::StartDialog { .. }
            | RuleAction::ShowUi { .. }
            | RuleAction::HideUi { .. }
            | RuleAction::UpdateUiBinding { .. }
            | RuleAction::SaveGame { .. }
            | RuleAction::LoadGame { .. } => {
                self.buffer_scene_dialog_or_persistence_action(action, context, command_buffer);
            }
            RuleAction::Spawn { .. }
            | RuleAction::DestroySelf { .. }
            | RuleAction::DamageEntity { .. }
            | RuleAction::HealEntity { .. }
            | RuleAction::SetEntityActive { .. }
            | RuleAction::TeleportEntity { .. } => {
                self.buffer_entity_mutation_action(action, context, command_buffer);
            }
            RuleAction::AddInventoryItem { .. }
            | RuleAction::RemoveInventoryItem { .. }
            | RuleAction::SetFlag { .. }
            | RuleAction::IncrementFlag { .. }
            | RuleAction::ClearFlag { .. } => {
                self.buffer_inventory_or_flag_action(action, context, command_buffer);
            }
        }
    }

    fn buffer_audio_or_motion_action(
        &self,
        action: &RuleAction,
        context: &TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        match action {
            RuleAction::PlaySound { channel, sound_id } => {
                self.buffer_play_sound(channel, sound_id, command_buffer);
            }
            RuleAction::PlayMusic { track_id } => {
                self.buffer_play_music(track_id, command_buffer);
            }
            RuleAction::PlayAnimation { target, state } => {
                self.buffer_play_animation(*target, *state, context, command_buffer);
            }
            RuleAction::SetVelocity { target, velocity } => {
                self.buffer_set_velocity(*target, velocity, context, command_buffer);
            }
            _ => unreachable!("audio or motion helper only handles audio or motion actions"),
        }
    }

    fn buffer_scene_dialog_or_persistence_action(
        &self,
        action: &RuleAction,
        context: &TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        match action {
            RuleAction::SwitchScene {
                scene_name,
                spawn_point_id,
                transition,
                duration_ms,
            } => {
                tracing::info!(
                    scene_name = %scene_name,
                    spawn_point_id = %spawn_point_id,
                    "Scene switch triggered"
                );
                command_buffer.push(RuleCommand::SwitchScene {
                    scene_name: scene_name.clone(),
                    spawn_point_id: spawn_point_id.clone(),
                    transition: *transition,
                    duration_ms: *duration_ms,
                });
            }
            RuleAction::StartDialog { dialog_id } => {
                let dialog_id = dialog_id.trim();
                if !dialog_id.is_empty() {
                    command_buffer.push(RuleCommand::StartDialog {
                        dialog_id: dialog_id.into(),
                        context: crate::dialog::DialogRuntimeContext {
                            interactor: context.trigger_self,
                            speaker: context.trigger_other,
                        },
                    });
                }
            }
            RuleAction::ShowUi { ui_id } => {
                if !ui_id.as_str().trim().is_empty() {
                    command_buffer.push(RuleCommand::ShowUi {
                        ui_id: ui_id.clone(),
                    });
                }
            }
            RuleAction::HideUi { ui_id } => {
                if !ui_id.as_str().trim().is_empty() {
                    command_buffer.push(RuleCommand::HideUi {
                        ui_id: ui_id.clone(),
                    });
                }
            }
            RuleAction::UpdateUiBinding {
                ui_id,
                binding_key,
                value,
            } => {
                let binding_key = binding_key.trim();
                if !ui_id.as_str().trim().is_empty() && !binding_key.is_empty() {
                    match value.resolve(self.value_path_context(context)) {
                        Ok(value) => command_buffer.push(RuleCommand::UpdateUiBinding {
                            ui_id: ui_id.clone(),
                            binding_key: binding_key.to_string(),
                            value,
                        }),
                        Err(error) => tracing::warn!(
                            error = %error,
                            action = ?action,
                            "Failed to resolve ui binding expression"
                        ),
                    }
                }
            }
            RuleAction::SaveGame { slot } => {
                command_buffer.push(RuleCommand::SaveGame { slot: *slot });
            }
            RuleAction::LoadGame { slot } => {
                command_buffer.push(RuleCommand::LoadGame { slot: *slot });
            }
            _ => unreachable!("scene, dialog, or persistence helper only handles matching actions"),
        }
    }

    fn buffer_entity_mutation_action(
        &self,
        action: &RuleAction,
        context: &TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        match action {
            RuleAction::Spawn {
                entity_type,
                position,
            } => match position.resolve(self.value_path_context(context)) {
                Ok(position) => command_buffer.push(RuleCommand::Spawn {
                    entity_type: *entity_type,
                    position: glam::IVec2::new(position[0], position[1]),
                }),
                Err(error) => tracing::warn!(
                    error = %error,
                    action = ?action,
                    "Failed to resolve spawn position expression"
                ),
            },
            RuleAction::DestroySelf { target } => {
                self.resolve_and_push(*target, context, command_buffer, |entity_id| {
                    RuleCommand::DestroySelf { entity_id }
                });
            }
            RuleAction::DamageEntity { target, amount } => {
                match amount.resolve(self.value_path_context(context)) {
                    Ok(amount) => {
                        self.resolve_and_push(*target, context, command_buffer, |entity_id| {
                            RuleCommand::DamageEntity { entity_id, amount }
                        })
                    }
                    Err(error) => tracing::warn!(
                        error = %error,
                        action = ?action,
                        "Failed to resolve damage amount expression"
                    ),
                }
            }
            RuleAction::HealEntity { target, amount } => {
                match amount.resolve(self.value_path_context(context)) {
                    Ok(amount) => {
                        self.resolve_and_push(*target, context, command_buffer, |entity_id| {
                            RuleCommand::HealEntity { entity_id, amount }
                        })
                    }
                    Err(error) => tracing::warn!(
                        error = %error,
                        action = ?action,
                        "Failed to resolve heal amount expression"
                    ),
                }
            }
            RuleAction::SetEntityActive { target, active } => {
                self.resolve_and_push(*target, context, command_buffer, |entity_id| {
                    RuleCommand::SetEntityActive {
                        entity_id,
                        active: *active,
                    }
                });
            }
            RuleAction::TeleportEntity {
                target,
                tile_x,
                tile_y,
            } => {
                let resolved_x = tile_x.resolve(self.value_path_context(context));
                let resolved_y = tile_y.resolve(self.value_path_context(context));
                match (resolved_x, resolved_y) {
                    (Ok(tile_x), Ok(tile_y)) if tile_x >= 0 && tile_y >= 0 => {
                        self.resolve_and_push(*target, context, command_buffer, |entity_id| {
                            RuleCommand::TeleportEntity {
                                entity_id,
                                tile_x: tile_x as u32,
                                tile_y: tile_y as u32,
                            }
                        });
                    }
                    (Ok(_), Ok(_)) => tracing::warn!(
                        action = ?action,
                        "Teleport expressions resolved to negative tile coordinates"
                    ),
                    (Err(error), _) | (_, Err(error)) => tracing::warn!(
                        error = %error,
                        action = ?action,
                        "Failed to resolve teleport expression"
                    ),
                }
            }
            _ => unreachable!("entity mutation helper only handles entity mutation actions"),
        }
    }

    fn buffer_inventory_or_flag_action(
        &self,
        action: &RuleAction,
        context: &TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        match action {
            RuleAction::AddInventoryItem {
                target,
                item_id,
                count,
            } => {
                self.resolve_and_push(*target, context, command_buffer, |entity_id| {
                    RuleCommand::AddInventoryItem {
                        entity_id,
                        item_id: item_id.clone(),
                        count: *count,
                    }
                });
            }
            RuleAction::RemoveInventoryItem {
                target,
                item_id,
                count,
            } => {
                self.resolve_and_push(*target, context, command_buffer, |entity_id| {
                    RuleCommand::RemoveInventoryItem {
                        entity_id,
                        item_id: item_id.clone(),
                        count: *count,
                    }
                });
            }
            RuleAction::SetFlag { flag, value } => {
                let flag = flag.trim();
                if !flag.is_empty() {
                    match value.resolve(self.value_path_context(context)) {
                        Ok(value) => command_buffer.push(RuleCommand::SetFlag {
                            flag: flag.to_string(),
                            value,
                        }),
                        Err(error) => tracing::warn!(
                            error = %error,
                            action = ?action,
                            "Failed to resolve set-flag value expression"
                        ),
                    }
                }
            }
            RuleAction::IncrementFlag { flag, amount } => {
                let flag = flag.trim();
                if !flag.is_empty() {
                    match amount.resolve(self.value_path_context(context)) {
                        Ok(amount) => command_buffer.push(RuleCommand::IncrementFlag {
                            flag: flag.to_string(),
                            amount,
                        }),
                        Err(error) => tracing::warn!(
                            error = %error,
                            action = ?action,
                            "Failed to resolve increment-flag amount expression"
                        ),
                    }
                }
            }
            RuleAction::ClearFlag { flag } => {
                let flag = flag.trim();
                if !flag.is_empty() {
                    command_buffer.push(RuleCommand::ClearFlag {
                        flag: flag.to_string(),
                    });
                }
            }
            _ => unreachable!("inventory or flag helper only handles inventory or flag actions"),
        }
    }

    fn buffer_play_sound(
        &self,
        channel: &RuleSoundChannel,
        sound_id: &str,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let sound_id = sound_id.trim();
        if sound_id.is_empty() {
            return;
        }

        let channel = match channel {
            RuleSoundChannel::Movement => AudioChannel::Movement,
            RuleSoundChannel::Collision => AudioChannel::Collision,
        };

        command_buffer.push(RuleCommand::PlaySound {
            channel,
            sound_id: sound_id.to_string(),
        });
    }

    fn buffer_play_music(&self, track_id: &str, command_buffer: &mut Vec<RuleCommand>) {
        let track_id = track_id.trim();
        if track_id.is_empty() {
            return;
        }
        command_buffer.push(RuleCommand::PlayMusic {
            track_id: track_id.to_string(),
        });
    }

    fn buffer_play_animation(
        &self,
        target: crate::rules::RuleTarget,
        state: AnimationState,
        context: &TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.resolve_and_push(target, context, command_buffer, |entity_id| {
            RuleCommand::PlayAnimation { entity_id, state }
        });
    }

    fn buffer_set_velocity(
        &self,
        target: crate::rules::RuleTarget,
        velocity: &crate::rules::RuleVec2IntSource,
        context: &TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        match velocity.resolve(self.value_path_context(context)) {
            Ok(velocity) => self.resolve_and_push(target, context, command_buffer, |entity_id| {
                RuleCommand::SetVelocity {
                    entity_id,
                    velocity: glam::IVec2::new(velocity[0], velocity[1]),
                }
            }),
            Err(error) => tracing::warn!(
                error = %error,
                "Failed to resolve set-velocity expression"
            ),
        }
    }
}
