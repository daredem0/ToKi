//! Rule action buffering.
//!
//! Contains logic for converting rule actions into buffered commands.

use crate::animation::AnimationState;
use crate::rules::{RuleAction, RuleSoundChannel, TriggerContext};
use tracing::info;

use super::{
    AnimationCommand, AudioChannel, AudioCommand, EntityCommand, InventoryCommand, MotionCommand,
    ProgressCommand, RuleCommand, RuleEngine, SceneCommand, UiCommand,
};

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
                command_buffer.push(RuleCommand::Scene(SceneCommand::SwitchScene {
                    scene_name: scene_name.clone(),
                    spawn_point_id: spawn_point_id.clone(),
                    transition: *transition,
                    duration_ms: *duration_ms,
                }));
            }
            RuleAction::StartDialog { dialog_id } => {
                let dialog_id = dialog_id.trim();
                if !dialog_id.is_empty() {
                    command_buffer.push(RuleCommand::Scene(SceneCommand::StartDialog {
                        dialog_id: dialog_id.into(),
                        context: crate::dialog::DialogRuntimeContext {
                            interactor: context.trigger_self,
                            speaker: context.trigger_other,
                        },
                    }));
                }
            }
            RuleAction::ShowUi { ui_id } => {
                if !ui_id.as_str().trim().is_empty() {
                    command_buffer.push(RuleCommand::Ui(UiCommand::ShowUi {
                        ui_id: ui_id.clone(),
                    }));
                }
            }
            RuleAction::HideUi { ui_id } => {
                if !ui_id.as_str().trim().is_empty() {
                    command_buffer.push(RuleCommand::Ui(UiCommand::HideUi {
                        ui_id: ui_id.clone(),
                    }));
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
                        Ok(value) => {
                            command_buffer.push(RuleCommand::Ui(UiCommand::UpdateUiBinding {
                                ui_id: ui_id.clone(),
                                binding_key: binding_key.to_string(),
                                value,
                            }))
                        }
                        Err(error) => tracing::warn!(
                            error = %error,
                            action = ?action,
                            "Failed to resolve ui binding expression"
                        ),
                    }
                }
            }
            RuleAction::SaveGame { slot } => {
                command_buffer.push(RuleCommand::Progress(ProgressCommand::SaveGame {
                    slot: *slot,
                }));
            }
            RuleAction::LoadGame { slot } => {
                command_buffer.push(RuleCommand::Progress(ProgressCommand::LoadGame {
                    slot: *slot,
                }));
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
                Ok(position) => command_buffer.push(RuleCommand::Entity(EntityCommand::Spawn {
                    entity_type: *entity_type,
                    position: glam::IVec2::new(position[0], position[1]),
                })),
                Err(error) => tracing::warn!(
                    error = %error,
                    action = ?action,
                    "Failed to resolve spawn position expression"
                ),
            },
            RuleAction::DestroySelf { target } => {
                self.resolve_and_push(*target, context, command_buffer, |entity_id| {
                    RuleCommand::Entity(EntityCommand::DestroySelf { entity_id })
                });
            }
            RuleAction::DamageEntity { target, amount } => {
                match amount.resolve(self.value_path_context(context)) {
                    Ok(amount) => {
                        self.resolve_and_push(*target, context, command_buffer, |entity_id| {
                            RuleCommand::Entity(EntityCommand::DamageEntity { entity_id, amount })
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
                            RuleCommand::Entity(EntityCommand::HealEntity { entity_id, amount })
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
                    RuleCommand::Entity(EntityCommand::SetEntityActive {
                        entity_id,
                        active: *active,
                    })
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
                            RuleCommand::Entity(EntityCommand::TeleportEntity {
                                entity_id,
                                tile_x: tile_x as u32,
                                tile_y: tile_y as u32,
                            })
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
                    RuleCommand::Inventory(InventoryCommand::AddInventoryItem {
                        entity_id,
                        item_id: item_id.clone(),
                        count: *count,
                    })
                });
            }
            RuleAction::RemoveInventoryItem {
                target,
                item_id,
                count,
            } => {
                self.resolve_and_push(*target, context, command_buffer, |entity_id| {
                    RuleCommand::Inventory(InventoryCommand::RemoveInventoryItem {
                        entity_id,
                        item_id: item_id.clone(),
                        count: *count,
                    })
                });
            }
            RuleAction::SetFlag { flag, value } => {
                let flag = flag.trim();
                if !flag.is_empty() {
                    match value.resolve(self.value_path_context(context)) {
                        Ok(value) => {
                            command_buffer.push(RuleCommand::Progress(ProgressCommand::SetFlag {
                                flag: flag.to_string(),
                                value,
                            }))
                        }
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
                        Ok(amount) => command_buffer.push(RuleCommand::Progress(
                            ProgressCommand::IncrementFlag {
                                flag: flag.to_string(),
                                amount,
                            },
                        )),
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
                    command_buffer.push(RuleCommand::Progress(ProgressCommand::ClearFlag {
                        flag: flag.to_string(),
                    }));
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

        command_buffer.push(RuleCommand::Audio(AudioCommand::PlaySound {
            channel,
            sound_id: sound_id.to_string(),
        }));
    }

    fn buffer_play_music(&self, track_id: &str, command_buffer: &mut Vec<RuleCommand>) {
        let track_id = track_id.trim();
        if track_id.is_empty() {
            return;
        }
        command_buffer.push(RuleCommand::Audio(AudioCommand::PlayMusic {
            track_id: track_id.to_string(),
        }));
    }

    fn buffer_play_animation(
        &self,
        target: crate::rules::RuleTarget,
        state: AnimationState,
        context: &TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.resolve_and_push(target, context, command_buffer, |entity_id| {
            RuleCommand::Animation(AnimationCommand::PlayAnimation { entity_id, state })
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
                RuleCommand::Motion(MotionCommand::SetVelocity {
                    entity_id,
                    velocity: glam::IVec2::new(velocity[0], velocity[1]),
                })
            }),
            Err(error) => tracing::warn!(
                error = %error,
                "Failed to resolve set-velocity expression"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::engine::{RuleEngine, RuleEngineContext};
    use super::{
        AudioCommand, EntityCommand, InventoryCommand, ProgressCommand, RuleCommand, SceneCommand,
        UiCommand,
    };
    use crate::entity::{
        EntityId, EntityKind, EntityManager, EntityRendering, OptionalEntityComponents,
    };
    use crate::flags::{FlagValue, GameFlags};
    use crate::game::RuleRuntimeState;
    use crate::rules::{
        RuleAction, RuleFlagValueSource, RuleIntSource, RuleSet, RuleSoundChannel,
        RuleSpawnEntityType, RuleTarget, RuleVec2IntSource, TriggerContext,
    };
    use glam::{IVec2, UVec2};

    fn make_engine<'a>(
        manager: &'a EntityManager,
        rules: &'a RuleSet,
        flags: &'a GameFlags,
        runtime: &'a mut RuleRuntimeState,
    ) -> RuleEngine<'a> {
        RuleEngine::new(
            RuleEngineContext {
                entity_manager: manager,
                player_id: None,
                held_keys: &[],
                game_flags: flags,
                rules,
            },
            runtime,
        )
    }

    fn buffer(
        manager: &EntityManager,
        action: &RuleAction,
        context: &TriggerContext,
    ) -> Vec<RuleCommand> {
        let rules = RuleSet::default();
        let flags = GameFlags::default();
        let mut runtime = RuleRuntimeState::default();
        let engine = make_engine(manager, &rules, &flags, &mut runtime);
        let mut commands = Vec::new();
        engine.buffer_rule_action("r", false, action, context, &mut commands);
        commands
    }

    fn spawn_npc(manager: &mut EntityManager) -> EntityId {
        manager.spawn_entity(
            EntityKind::Npc,
            IVec2::ZERO,
            UVec2::new(16, 16),
            EntityRendering::default(),
            false,
            true,
            OptionalEntityComponents::default(),
        )
    }

    // --- Scene / Dialog ---

    #[test]
    fn start_dialog_pushes_scene_command() {
        let manager = EntityManager::new();
        let action = RuleAction::StartDialog {
            dialog_id: "intro".into(),
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Scene(SceneCommand::StartDialog { dialog_id, .. })] if dialog_id.as_str() == "intro"
        ));
    }

    #[test]
    fn start_dialog_empty_id_produces_no_command() {
        let manager = EntityManager::new();
        let action = RuleAction::StartDialog {
            dialog_id: "".into(),
        };
        assert!(buffer(&manager, &action, &TriggerContext::empty()).is_empty());
    }

    #[test]
    fn start_dialog_whitespace_id_produces_no_command() {
        let manager = EntityManager::new();
        let action = RuleAction::StartDialog {
            dialog_id: "   ".into(),
        };
        assert!(buffer(&manager, &action, &TriggerContext::empty()).is_empty());
    }

    #[test]
    fn switch_scene_pushes_scene_command() {
        let manager = EntityManager::new();
        let action = RuleAction::SwitchScene {
            scene_name: "overworld".into(),
            spawn_point_id: "start".to_string(),
            transition: None,
            duration_ms: None,
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Scene(SceneCommand::SwitchScene { scene_name, .. })] if scene_name.as_str() == "overworld"
        ));
    }

    // --- Audio ---

    #[test]
    fn play_sound_pushes_audio_command() {
        let manager = EntityManager::new();
        let action = RuleAction::PlaySound {
            channel: RuleSoundChannel::Movement,
            sound_id: "footstep".to_string(),
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Audio(AudioCommand::PlaySound { sound_id, .. })] if sound_id == "footstep"
        ));
    }

    #[test]
    fn play_sound_empty_id_produces_no_command() {
        let manager = EntityManager::new();
        let action = RuleAction::PlaySound {
            channel: RuleSoundChannel::Movement,
            sound_id: "".to_string(),
        };
        assert!(buffer(&manager, &action, &TriggerContext::empty()).is_empty());
    }

    #[test]
    fn play_music_pushes_audio_command() {
        let manager = EntityManager::new();
        let action = RuleAction::PlayMusic {
            track_id: "battle_theme".to_string(),
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Audio(AudioCommand::PlayMusic { track_id })] if track_id == "battle_theme"
        ));
    }

    #[test]
    fn play_music_empty_id_produces_no_command() {
        let manager = EntityManager::new();
        let action = RuleAction::PlayMusic {
            track_id: "".to_string(),
        };
        assert!(buffer(&manager, &action, &TriggerContext::empty()).is_empty());
    }

    // --- Progress / Flags ---

    #[test]
    fn set_flag_pushes_progress_command() {
        let manager = EntityManager::new();
        let action = RuleAction::SetFlag {
            flag: "done".to_string(),
            value: RuleFlagValueSource::Literal(FlagValue::Bool(true)),
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Progress(ProgressCommand::SetFlag { flag, value: FlagValue::Bool(true) })] if flag == "done"
        ));
    }

    #[test]
    fn set_flag_empty_name_produces_no_command() {
        let manager = EntityManager::new();
        let action = RuleAction::SetFlag {
            flag: "  ".to_string(),
            value: RuleFlagValueSource::Literal(FlagValue::Bool(true)),
        };
        assert!(buffer(&manager, &action, &TriggerContext::empty()).is_empty());
    }

    #[test]
    fn increment_flag_pushes_progress_command() {
        let manager = EntityManager::new();
        let action = RuleAction::IncrementFlag {
            flag: "coins".to_string(),
            amount: RuleIntSource::literal(5),
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Progress(ProgressCommand::IncrementFlag { flag, amount: 5 })] if flag == "coins"
        ));
    }

    #[test]
    fn clear_flag_pushes_progress_command() {
        let manager = EntityManager::new();
        let action = RuleAction::ClearFlag {
            flag: "temp".to_string(),
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Progress(ProgressCommand::ClearFlag { flag })] if flag == "temp"
        ));
    }

    #[test]
    fn clear_flag_empty_name_produces_no_command() {
        let manager = EntityManager::new();
        let action = RuleAction::ClearFlag {
            flag: "".to_string(),
        };
        assert!(buffer(&manager, &action, &TriggerContext::empty()).is_empty());
    }

    // --- Entity mutations ---

    #[test]
    fn damage_entity_pushes_entity_command() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager);
        let action = RuleAction::DamageEntity {
            target: RuleTarget::Entity(id),
            amount: RuleIntSource::literal(10),
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Entity(EntityCommand::DamageEntity { entity_id, amount: 10 })]
                if *entity_id == id
        ));
    }

    #[test]
    fn damage_entity_unresolvable_target_produces_no_command() {
        // RuleTarget::TriggerSelf with no trigger context resolves to None → no command.
        let manager = EntityManager::new();
        let action = RuleAction::DamageEntity {
            target: RuleTarget::TriggerSelf,
            amount: RuleIntSource::literal(10),
        };
        assert!(buffer(&manager, &action, &TriggerContext::empty()).is_empty());
    }

    #[test]
    fn set_entity_active_pushes_entity_command() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager);
        let action = RuleAction::SetEntityActive {
            target: RuleTarget::Entity(id),
            active: false,
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Entity(EntityCommand::SetEntityActive { entity_id, active: false })]
                if *entity_id == id
        ));
    }

    #[test]
    fn heal_entity_pushes_entity_command() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager);
        let action = RuleAction::HealEntity {
            target: RuleTarget::Entity(id),
            amount: RuleIntSource::literal(20),
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Entity(EntityCommand::HealEntity { entity_id, amount: 20 })]
                if *entity_id == id
        ));
    }

    // --- Inventory ---

    #[test]
    fn add_inventory_item_pushes_inventory_command() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager);
        let action = RuleAction::AddInventoryItem {
            target: RuleTarget::Entity(id),
            item_id: "key".to_string(),
            count: 1,
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Inventory(InventoryCommand::AddInventoryItem { entity_id, item_id, count: 1 })]
                if *entity_id == id && item_id == "key"
        ));
    }

    #[test]
    fn remove_inventory_item_pushes_inventory_command() {
        let mut manager = EntityManager::new();
        let id = spawn_npc(&mut manager);
        let action = RuleAction::RemoveInventoryItem {
            target: RuleTarget::Entity(id),
            item_id: "key".to_string(),
            count: 1,
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Inventory(InventoryCommand::RemoveInventoryItem { entity_id, item_id, count: 1 })]
                if *entity_id == id && item_id == "key"
        ));
    }

    // --- Spawn ---

    #[test]
    fn spawn_pushes_entity_command() {
        let manager = EntityManager::new();
        let action = RuleAction::Spawn {
            entity_type: RuleSpawnEntityType::Npc,
            position: RuleVec2IntSource::literal([4, 8]),
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Entity(EntityCommand::Spawn {
                entity_type: RuleSpawnEntityType::Npc,
                position,
            })] if position.x == 4 && position.y == 8
        ));
    }

    // --- UI ---

    #[test]
    fn show_ui_pushes_ui_command() {
        let manager = EntityManager::new();
        let action = RuleAction::ShowUi {
            ui_id: "hud".into(),
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Ui(UiCommand::ShowUi { ui_id })] if ui_id.as_str() == "hud"
        ));
    }

    #[test]
    fn hide_ui_pushes_ui_command() {
        let manager = EntityManager::new();
        let action = RuleAction::HideUi {
            ui_id: "hud".into(),
        };
        let cmds = buffer(&manager, &action, &TriggerContext::empty());
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Ui(UiCommand::HideUi { ui_id })] if ui_id.as_str() == "hud"
        ));
    }

    #[test]
    fn trigger_self_context_attached_to_start_dialog() {
        let mut manager = EntityManager::new();
        let self_id = spawn_npc(&mut manager);
        let action = RuleAction::StartDialog {
            dialog_id: "chat".into(),
        };
        let ctx = TriggerContext::with_self_only(self_id);
        let cmds = buffer(&manager, &action, &ctx);
        assert!(matches!(
            cmds.as_slice(),
            [RuleCommand::Scene(SceneCommand::StartDialog { dialog_id, context })]
                if dialog_id.as_str() == "chat" && context.interactor == Some(self_id)
        ));
    }
}
