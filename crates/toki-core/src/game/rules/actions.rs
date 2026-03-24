//! Rule action buffering.
//!
//! Contains logic for converting rule actions into buffered commands.

use crate::animation::AnimationState;
use crate::rules::{RuleAction, RuleSoundChannel, TriggerContext};

use super::{AudioChannel, GameState, RuleCommand};

impl GameState {
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
            RuleAction::Spawn {
                entity_type,
                position,
            } => {
                command_buffer.push(RuleCommand::Spawn {
                    entity_type: *entity_type,
                    position: glam::IVec2::new(position[0], position[1]),
                });
            }
            RuleAction::DestroySelf { target } => {
                self.resolve_and_push(*target, context, command_buffer, |entity_id| {
                    RuleCommand::DestroySelf { entity_id }
                });
            }
            RuleAction::SwitchScene {
                scene_name,
                spawn_point_id,
            } => {
                tracing::info!(
                    scene_name = %scene_name,
                    spawn_point_id = %spawn_point_id,
                    "Scene switch triggered"
                );
                command_buffer.push(RuleCommand::SwitchScene {
                    scene_name: scene_name.clone(),
                    spawn_point_id: spawn_point_id.clone(),
                });
            }
            RuleAction::StartDialog { dialog_id } => {
                let dialog_id = dialog_id.trim();
                if !dialog_id.is_empty() {
                    command_buffer.push(RuleCommand::StartDialog {
                        dialog_id: dialog_id.to_string(),
                        context: crate::dialog::DialogRuntimeContext {
                            interactor: context.trigger_self,
                            speaker: context.trigger_other,
                        },
                    });
                }
            }
            RuleAction::DamageEntity { target, amount } => {
                self.resolve_and_push(*target, context, command_buffer, |entity_id| {
                    RuleCommand::DamageEntity {
                        entity_id,
                        amount: *amount,
                    }
                });
            }
            RuleAction::HealEntity { target, amount } => {
                self.resolve_and_push(*target, context, command_buffer, |entity_id| {
                    RuleCommand::HealEntity {
                        entity_id,
                        amount: *amount,
                    }
                });
            }
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
                self.resolve_and_push(*target, context, command_buffer, |entity_id| {
                    RuleCommand::TeleportEntity {
                        entity_id,
                        tile_x: *tile_x,
                        tile_y: *tile_y,
                    }
                });
            }
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
        velocity: &[i32; 2],
        context: &TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.resolve_and_push(target, context, command_buffer, |entity_id| {
            RuleCommand::SetVelocity {
                entity_id,
                velocity: glam::IVec2::new(velocity[0], velocity[1]),
            }
        });
    }
}
