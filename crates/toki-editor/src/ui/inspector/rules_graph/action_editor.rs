//! Action node editing UI.

use super::*;

impl InspectorSystem {
    #[expect(clippy::too_many_arguments)]
    pub(in super::super) fn render_rule_graph_action_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        action: &mut RuleAction,
        _validation_issues: &[RuleValidationIssue],
        audio_choices: &RuleAudioChoices,
        scenes: &[toki_core::Scene],
    ) -> bool {
        let mut changed = false;
        let current_kind = Self::action_kind(action);
        let mut selected_kind = current_kind;
        ui.horizontal(|ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_salt(format!(
                "graph_node_action_kind_{}_{}",
                scene_name, node_key
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
            scene_name,
            node_key,
            action,
            audio_choices,
            scenes,
        );

        changed
    }

    fn render_action_parameters(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        action: &mut RuleAction,
        audio_choices: &RuleAudioChoices,
        scenes: &[toki_core::Scene],
    ) -> bool {
        match action {
            RuleAction::PlaySound { channel, sound_id } => {
                Self::render_play_sound_params(ui, scene_name, node_key, channel, sound_id, audio_choices)
            }
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
            RuleAction::DestroySelf { target } => {
                Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!("graph_node_destroy_target_{}_{}", scene_name, node_key),
                    target,
                )
            }
            RuleAction::SwitchScene {
                scene_name: scene,
                spawn_point_id,
            } => Self::render_switch_scene_editor(
                ui,
                format!("graph_switch_scene_{}_{}", scene_name, node_key),
                scene,
                spawn_point_id,
                scenes,
            ),
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
            } => Self::render_inventory_params(ui, scene_name, node_key, target, item_id, count, "add_inv"),
            RuleAction::RemoveInventoryItem {
                target,
                item_id,
                count,
            } => Self::render_inventory_params(ui, scene_name, node_key, target, item_id, count, "rem_inv"),
            RuleAction::SetEntityActive { target, active } => {
                Self::render_set_active_params(ui, scene_name, node_key, target, active)
            }
            RuleAction::TeleportEntity {
                target,
                tile_x,
                tile_y,
            } => Self::render_teleport_params(ui, scene_name, node_key, target, tile_x, tile_y),
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
                        .selectable_value(
                            channel,
                            candidate,
                            Self::sound_channel_label(candidate),
                        )
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
        velocity: &mut [i32; 2],
    ) -> bool {
        let mut changed = Self::render_rule_target_editor_with_salt(
            ui,
            &format!("graph_node_velocity_target_{}_{}", scene_name, node_key),
            target,
        );
        ui.horizontal(|ui| {
            ui.label("Velocity:");
            changed |= ui
                .add(egui::DragValue::new(&mut velocity[0]).speed(1.0))
                .changed();
            changed |= ui
                .add(egui::DragValue::new(&mut velocity[1]).speed(1.0))
                .changed();
        });
        changed
    }

    fn render_spawn_params(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        entity_type: &mut RuleSpawnEntityType,
        position: &mut [i32; 2],
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
        ui.horizontal(|ui| {
            ui.label("Position:");
            changed |= ui
                .add(egui::DragValue::new(&mut position[0]).speed(1.0))
                .changed();
            changed |= ui
                .add(egui::DragValue::new(&mut position[1]).speed(1.0))
                .changed();
        });
        changed
    }

    fn render_damage_heal_params(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        target: &mut toki_core::rules::RuleTarget,
        amount: &mut i32,
        prefix: &str,
    ) -> bool {
        let mut changed = Self::render_rule_target_editor_with_salt(
            ui,
            &format!("graph_node_{}_target_{}_{}", prefix, scene_name, node_key),
            target,
        );
        ui.horizontal(|ui| {
            ui.label("Amount:");
            changed |= ui
                .add(egui::DragValue::new(amount).speed(1.0).range(0..=9999))
                .changed();
        });
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
        tile_x: &mut u32,
        tile_y: &mut u32,
    ) -> bool {
        let mut changed = Self::render_rule_target_editor_with_salt(
            ui,
            &format!("graph_node_teleport_target_{}_{}", scene_name, node_key),
            target,
        );
        ui.horizontal(|ui| {
            ui.label("Tile X:");
            let mut x_val = *tile_x as i32;
            if ui
                .add(egui::DragValue::new(&mut x_val).speed(1.0).range(0..=9999))
                .changed()
            {
                *tile_x = x_val.max(0) as u32;
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Tile Y:");
            let mut y_val = *tile_y as i32;
            if ui
                .add(egui::DragValue::new(&mut y_val).speed(1.0).range(0..=9999))
                .changed()
            {
                *tile_y = y_val.max(0) as u32;
                changed = true;
            }
        });
        changed
    }
}
