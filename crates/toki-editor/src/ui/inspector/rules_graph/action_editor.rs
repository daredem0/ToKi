//! Action node editing UI.

use super::*;
use toki_core::rules::{RuleIntSource, RuleVec2IntSource};

pub(in super::super) struct RuleGraphActionEditorContext<'a> {
    pub scene_name: &'a str,
    pub node_key: &'a str,
    pub audio_choices: &'a RuleAudioChoices,
    pub available_dialog_outcomes: &'a std::collections::BTreeMap<String, Vec<String>>,
    pub scenes: &'a [toki_core::Scene],
}

impl InspectorSystem {
    pub(super) fn render_rule_graph_action_editor(
        ui: &mut egui::Ui,
        action: &mut RuleAction,
        ctx: RuleGraphActionEditorContext<'_>,
    ) -> bool {
        let mut changed = false;
        let current_kind = Self::action_kind(action);
        let mut selected_kind = current_kind;
        ui.horizontal(|ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_salt(format!(
                "graph_node_action_kind_{}_{}",
                ctx.scene_name, ctx.node_key
            ))
            .selected_text(Self::action_kind_label(current_kind))
            .show_ui(ui, |ui| {
                for candidate in RuleActionKind::iter() {
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            candidate,
                            Self::action_kind_label(candidate),
                        )
                        .changed();
                }
            });
        });
        if selected_kind != current_kind {
            Self::switch_action_kind(action, selected_kind);
            changed = true;
        }

        changed |= Self::render_action_parameters(
            ui,
            ctx.scene_name,
            ctx.node_key,
            action,
            ctx.audio_choices,
            ctx.available_dialog_outcomes,
            ctx.scenes,
        );

        changed
    }

    fn render_action_parameters(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        action: &mut RuleAction,
        audio_choices: &RuleAudioChoices,
        available_dialog_outcomes: &std::collections::BTreeMap<String, Vec<String>>,
        scenes: &[toki_core::Scene],
    ) -> bool {
        match action {
            RuleAction::PlaySound { channel, sound_id } => Self::render_play_sound_params(
                ui,
                scene_name,
                node_key,
                channel,
                sound_id,
                audio_choices,
            ),
            RuleAction::PlayMusic { track_id } => {
                Self::render_play_music_params(ui, scene_name, node_key, track_id, audio_choices)
            }
            RuleAction::PlayAnimation { target, state } => {
                Self::render_play_animation_params(ui, scene_name, node_key, target, state)
            }
            RuleAction::SetVelocity { target, velocity } => {
                Self::render_set_velocity_params(ui, scene_name, node_key, target, velocity)
            }
            RuleAction::Spawn {
                entity_type,
                position,
            } => Self::render_spawn_params(ui, scene_name, node_key, entity_type, position),
            RuleAction::DestroySelf { target } => Self::render_rule_target_editor_with_salt(
                ui,
                &format!("graph_node_destroy_target_{}_{}", scene_name, node_key),
                target,
            ),
            RuleAction::SwitchScene {
                scene_name: scene,
                spawn_point_id,
                transition,
                duration_ms,
            } => Self::render_switch_scene_editor(
                ui,
                format!("graph_switch_scene_{}_{}", scene_name, node_key),
                scene,
                spawn_point_id,
                transition,
                duration_ms,
                scenes,
            ),
            RuleAction::StartDialog { dialog_id } => {
                let mut changed = false;
                ui.horizontal(|ui| {
                    ui.label("Dialog Id:");
                    if available_dialog_outcomes.is_empty() {
                        let mut dialog_id_value = dialog_id.to_string();
                        if ui.text_edit_singleline(&mut dialog_id_value).changed() {
                            *dialog_id = dialog_id_value.into();
                            changed = true;
                        }
                    } else {
                        egui::ComboBox::from_id_salt(format!(
                            "graph_start_dialog_{}_{}",
                            scene_name, node_key
                        ))
                        .selected_text(if dialog_id.is_empty() {
                            "<select dialog>"
                        } else {
                            dialog_id.as_str()
                        })
                        .show_ui(ui, |ui| {
                            for candidate in available_dialog_outcomes.keys() {
                                changed |= ui
                                    .selectable_value(
                                        dialog_id,
                                        candidate.clone().into(),
                                        candidate.as_str(),
                                    )
                                    .changed();
                            }
                        });
                    }
                });
                changed
            }
            RuleAction::DamageEntity { target, amount } => {
                Self::render_damage_heal_params(ui, scene_name, node_key, target, amount, "damage")
            }
            RuleAction::HealEntity { target, amount } => {
                Self::render_damage_heal_params(ui, scene_name, node_key, target, amount, "heal")
            }
            RuleAction::AddInventoryItem {
                target,
                item_id,
                count,
            } => Self::render_inventory_params(
                ui, scene_name, node_key, target, item_id, count, "add_inv",
            ),
            RuleAction::RemoveInventoryItem {
                target,
                item_id,
                count,
            } => Self::render_inventory_params(
                ui, scene_name, node_key, target, item_id, count, "rem_inv",
            ),
            RuleAction::SetEntityActive { target, active } => {
                Self::render_set_active_params(ui, scene_name, node_key, target, active)
            }
            RuleAction::TeleportEntity {
                target,
                tile_x,
                tile_y,
            } => Self::render_teleport_params(ui, scene_name, node_key, target, tile_x, tile_y),
            RuleAction::SetFlag { flag, value } => {
                let mut changed = Self::render_flag_name_editor(ui, flag);
                changed |= Self::render_rule_flag_value_source_editor(
                    ui,
                    format!("graph_node_set_flag_{}_{}", scene_name, node_key),
                    value,
                );
                changed
            }
            RuleAction::IncrementFlag { flag, amount } => {
                let mut changed = Self::render_flag_name_editor(ui, flag);
                changed |= Self::render_rule_int_source_editor(
                    ui,
                    format!("graph_node_inc_flag_{}_{}", scene_name, node_key),
                    "Amount:",
                    amount,
                );
                changed
            }
            RuleAction::ClearFlag { flag } => Self::render_flag_name_editor(ui, flag),
            RuleAction::SaveGame { slot } | RuleAction::LoadGame { slot } => {
                Self::render_save_slot_editor(ui, slot)
            }
            RuleAction::ShowUi { ui_id } | RuleAction::HideUi { ui_id } => {
                let mut changed = false;
                ui.horizontal(|ui| {
                    ui.label("UI Id:");
                    let mut ui_id_value = ui_id.to_string();
                    if ui.text_edit_singleline(&mut ui_id_value).changed() {
                        *ui_id = ui_id_value.into();
                        changed = true;
                    }
                });
                changed
            }
            RuleAction::UpdateUiBinding {
                ui_id,
                binding_key,
                value,
            } => {
                let mut changed = false;
                ui.horizontal(|ui| {
                    ui.label("UI Id:");
                    let mut ui_id_value = ui_id.to_string();
                    if ui.text_edit_singleline(&mut ui_id_value).changed() {
                        *ui_id = ui_id_value.into();
                        changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Binding Key:");
                    changed |= ui.text_edit_singleline(binding_key).changed();
                });
                changed |= Self::render_rule_flag_value_source_editor(
                    ui,
                    format!("graph_node_ui_binding_{}_{}", scene_name, node_key),
                    value,
                );
                changed
            }
        }
    }

    fn render_play_sound_params(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        channel: &mut RuleSoundChannel,
        sound_id: &mut String,
        audio_choices: &RuleAudioChoices,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Channel:");
            egui::ComboBox::from_id_salt(format!(
                "graph_node_sound_channel_{}_{}",
                scene_name, node_key
            ))
            .selected_text(Self::sound_channel_label(*channel))
            .show_ui(ui, |ui| {
                for candidate in RuleSoundChannel::iter() {
                    changed |= ui
                        .selectable_value(channel, candidate, Self::sound_channel_label(candidate))
                        .changed();
                }
            });
        });
        ui.horizontal(|ui| {
            ui.label("Sound Id:");
            changed |= ui.text_edit_singleline(sound_id).changed();
        });
        changed |= Self::render_audio_choice_picker(
            ui,
            format!("graph_node_sfx_picker_{}_{}", scene_name, node_key),
            "SFX",
            sound_id,
            &audio_choices.sfx,
        );
        changed
    }

    fn render_play_music_params(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        track_id: &mut String,
        audio_choices: &RuleAudioChoices,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Track Id:");
            changed |= ui.text_edit_singleline(track_id).changed();
        });
        changed |= Self::render_audio_choice_picker(
            ui,
            format!("graph_node_music_picker_{}_{}", scene_name, node_key),
            "Music",
            track_id,
            &audio_choices.music,
        );
        changed
    }

    fn render_play_animation_params(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        target: &mut toki_core::rules::RuleTarget,
        state: &mut toki_core::animation::AnimationState,
    ) -> bool {
        let mut changed = Self::render_rule_target_editor_with_salt(
            ui,
            &format!("graph_node_anim_target_{}_{}", scene_name, node_key),
            target,
        );
        ui.horizontal(|ui| {
            ui.label("State:");
            egui::ComboBox::from_id_salt(format!(
                "graph_node_anim_state_{}_{}",
                scene_name, node_key
            ))
            .selected_text(animation_state_label(*state))
            .show_ui(ui, |ui| {
                for candidate in animation_state_options() {
                    changed |= ui
                        .selectable_value(state, candidate, animation_state_label(candidate))
                        .changed();
                }
            });
        });
        changed
    }

    fn render_set_velocity_params(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        target: &mut toki_core::rules::RuleTarget,
        velocity: &mut RuleVec2IntSource,
    ) -> bool {
        let mut changed = Self::render_rule_target_editor_with_salt(
            ui,
            &format!("graph_node_velocity_target_{}_{}", scene_name, node_key),
            target,
        );
        changed |= Self::render_rule_vec2_source_editor(
            ui,
            ("graph_velocity", scene_name, node_key),
            "Velocity:",
            velocity,
        );
        changed
    }

    fn render_spawn_params(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        entity_type: &mut RuleSpawnEntityType,
        position: &mut RuleVec2IntSource,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Entity Type:");
            egui::ComboBox::from_id_salt(format!(
                "graph_node_spawn_type_{}_{}",
                scene_name, node_key
            ))
            .selected_text(Self::spawn_entity_type_label(*entity_type))
            .show_ui(ui, |ui| {
                for candidate in RuleSpawnEntityType::iter() {
                    changed |= ui
                        .selectable_value(
                            entity_type,
                            candidate,
                            Self::spawn_entity_type_label(candidate),
                        )
                        .changed();
                }
            });
        });
        changed |= Self::render_rule_vec2_source_editor(
            ui,
            ("graph_spawn", scene_name, node_key),
            "Position:",
            position,
        );
        changed
    }

    fn render_damage_heal_params(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        target: &mut toki_core::rules::RuleTarget,
        amount: &mut RuleIntSource,
        prefix: &str,
    ) -> bool {
        let mut changed = Self::render_rule_target_editor_with_salt(
            ui,
            &format!("graph_node_{}_target_{}_{}", prefix, scene_name, node_key),
            target,
        );
        changed |= Self::render_rule_int_source_editor(
            ui,
            ("graph_amount", scene_name, node_key, prefix),
            "Amount:",
            amount,
        );
        changed
    }

    fn render_inventory_params(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        target: &mut toki_core::rules::RuleTarget,
        item_id: &mut String,
        count: &mut u32,
        prefix: &str,
    ) -> bool {
        let mut changed = Self::render_rule_target_editor_with_salt(
            ui,
            &format!("graph_node_{}_target_{}_{}", prefix, scene_name, node_key),
            target,
        );
        ui.horizontal(|ui| {
            ui.label("Item Id:");
            changed |= ui.text_edit_singleline(item_id).changed();
        });
        ui.horizontal(|ui| {
            ui.label("Count:");
            changed |= ui
                .add(egui::DragValue::new(count).speed(1.0).range(1..=9999))
                .changed();
        });
        changed
    }

    fn render_set_active_params(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        target: &mut toki_core::rules::RuleTarget,
        active: &mut bool,
    ) -> bool {
        let mut changed = Self::render_rule_target_editor_with_salt(
            ui,
            &format!("graph_node_set_active_target_{}_{}", scene_name, node_key),
            target,
        );
        ui.horizontal(|ui| {
            changed |= ui.checkbox(active, "Active").changed();
        });
        changed
    }

    fn render_teleport_params(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        target: &mut toki_core::rules::RuleTarget,
        tile_x: &mut RuleIntSource,
        tile_y: &mut RuleIntSource,
    ) -> bool {
        let mut changed = Self::render_rule_target_editor_with_salt(
            ui,
            &format!("graph_node_teleport_target_{}_{}", scene_name, node_key),
            target,
        );
        changed |= Self::render_rule_int_source_editor(
            ui,
            ("graph_tile_x", scene_name, node_key),
            "Tile X:",
            tile_x,
        );
        changed |= Self::render_rule_int_source_editor(
            ui,
            ("graph_tile_y", scene_name, node_key),
            "Tile Y:",
            tile_y,
        );
        changed
    }
}
