use super::*;
use toki_core::animation::AnimationState;
use toki_core::rules::{RuleIntSource, RuleVec2IntSource};

struct RuleActionEditorContext<'a> {
    scene_name: &'a str,
    rule_index: usize,
    action_index: usize,
}

pub(in super::super) struct RuleEditorContext<'a> {
    pub scene_name: &'a str,
    pub rule_index: usize,
    pub validation_issues: &'a [RuleValidationIssue],
    pub audio_choices: &'a RuleAudioChoices,
    pub scenes: &'a [toki_core::Scene],
    pub available_dialog_outcomes: &'a std::collections::BTreeMap<String, Vec<String>>,
    pub map_size: Option<(u32, u32)>,
}

impl RuleActionEditorContext<'_> {
    fn id_salt(&self) -> String {
        format!(
            "{}_{}_{}",
            self.scene_name, self.rule_index, self.action_index
        )
    }
}

impl InspectorSystem {
    pub(super) fn render_rule_editor(
        ui: &mut egui::Ui,
        rule: &mut Rule,
        ctx: RuleEditorContext<'_>,
    ) -> RuleEditorOutcome {
        let RuleEditorContext {
            scene_name,
            rule_index,
            validation_issues,
            audio_choices,
            scenes,
            available_dialog_outcomes,
            map_size,
        } = ctx;
        let mut outcome = RuleEditorOutcome::default();
        let has_rule_issues = validation_issues
            .iter()
            .any(|issue| issue.rule_index == rule_index && issue.action_index.is_none());

        let header = if has_rule_issues {
            format!("⚠ {} ({:?})", rule.id, rule.trigger)
        } else {
            format!("{} ({:?})", rule.id, rule.trigger)
        };
        egui::CollapsingHeader::new(header)
            .id_salt(format!("rule_header_{}_{}", scene_name, rule_index))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.small_button("⧉ Duplicate").clicked() {
                        outcome.command = Some(RuleEditorCommand::Duplicate(rule_index));
                    }
                    if ui.small_button("↑").clicked() {
                        outcome.command = Some(RuleEditorCommand::MoveUp(rule_index));
                    }
                    if ui.small_button("↓").clicked() {
                        outcome.command = Some(RuleEditorCommand::MoveDown(rule_index));
                    }
                    if ui.small_button("🗑 Remove").clicked() {
                        outcome.command = Some(RuleEditorCommand::Remove(rule_index));
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Id:");
                    outcome.changed |= ui.text_edit_singleline(&mut rule.id).changed();
                });

                ui.horizontal(|ui| {
                    outcome.changed |= ui.checkbox(&mut rule.enabled, "Enabled").changed();
                    outcome.changed |= ui.checkbox(&mut rule.once, "Once").changed();
                });

                ui.horizontal(|ui| {
                    ui.label("Priority:");
                    outcome.changed |= ui
                        .add(egui::DragValue::new(&mut rule.priority).speed(1.0))
                        .changed();
                });

                ui.horizontal(|ui| {
                    ui.label("Trigger:");
                    let mut trigger_kind = Self::trigger_kind(&rule.trigger);
                    egui::ComboBox::from_id_salt(format!(
                        "rule_trigger_{}_{}",
                        scene_name, rule_index
                    ))
                    .selected_text(Self::trigger_kind_label(trigger_kind))
                    .show_ui(ui, |ui| {
                        for candidate in RuleTriggerKind::iter() {
                            outcome.changed |= ui
                                .selectable_value(
                                    &mut trigger_kind,
                                    candidate,
                                    Self::trigger_kind_label(candidate),
                                )
                                .changed();
                        }
                    });
                    if trigger_kind != Self::trigger_kind(&rule.trigger) {
                        Self::set_rule_trigger_kind(rule, trigger_kind);
                    }
                });

                if let RuleTrigger::OnKey { key } = &mut rule.trigger {
                    ui.horizontal(|ui| {
                        ui.label("Key:");
                        egui::ComboBox::from_id_salt(format!(
                            "rule_trigger_key_{}_{}",
                            scene_name, rule_index
                        ))
                        .selected_text(Self::rule_key_label(*key))
                        .show_ui(ui, |ui| {
                            for candidate in RuleKey::iter() {
                                outcome.changed |= ui
                                    .selectable_value(
                                        key,
                                        candidate,
                                        Self::rule_key_label(candidate),
                                    )
                                    .changed();
                            }
                        });
                    });
                }

                if let RuleTrigger::OnDialogComplete {
                    dialog_id,
                    outcome_id,
                } = &mut rule.trigger
                {
                    ui.horizontal(|ui| {
                        ui.label("Dialog Id:");
                        if available_dialog_outcomes.is_empty() {
                            let mut dialog_id_value = dialog_id.to_string();
                            if ui.text_edit_singleline(&mut dialog_id_value).changed() {
                                *dialog_id = dialog_id_value.into();
                                outcome.changed = true;
                            }
                        } else {
                            egui::ComboBox::from_id_salt(format!(
                                "rule_trigger_dialog_id_{}_{}",
                                scene_name, rule_index
                            ))
                            .selected_text(if dialog_id.is_empty() {
                                "<select dialog>"
                            } else {
                                dialog_id.as_str()
                            })
                            .show_ui(ui, |ui| {
                                for candidate in available_dialog_outcomes.keys() {
                                    outcome.changed |= ui
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
                    ui.horizontal(|ui| {
                        ui.label("Outcome Id:");
                        if let Some(outcomes) = available_dialog_outcomes.get(dialog_id.as_str()) {
                            egui::ComboBox::from_id_salt(format!(
                                "rule_trigger_dialog_outcome_{}_{}",
                                scene_name, rule_index
                            ))
                            .selected_text(if outcome_id.is_empty() {
                                "<select outcome>"
                            } else {
                                outcome_id.as_str()
                            })
                            .show_ui(ui, |ui| {
                                for candidate in outcomes {
                                    outcome.changed |= ui
                                        .selectable_value(
                                            outcome_id,
                                            candidate.clone(),
                                            candidate.as_str(),
                                        )
                                        .changed();
                                }
                            });
                        } else {
                            outcome.changed |= ui.text_edit_singleline(outcome_id).changed();
                        }
                    });
                }

                if let RuleTrigger::OnTileEnter { x, y } | RuleTrigger::OnTileExit { x, y } =
                    &mut rule.trigger
                {
                    ui.horizontal(|ui| {
                        ui.label("Tile X:");
                        let mut x_val = *x as i32;
                        if ui
                            .add(egui::DragValue::new(&mut x_val).speed(1.0).range(0..=9999))
                            .changed()
                        {
                            *x = x_val.max(0) as u32;
                            outcome.changed = true;
                        }
                    });
                    // Validation warning for X coordinate
                    if let Some((map_width, _)) = map_size {
                        if *x >= map_width {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 150, 80),
                                format!(
                                    "⚠ X coordinate {} is out of bounds (map width: {})",
                                    *x, map_width
                                ),
                            );
                        }
                    }

                    ui.horizontal(|ui| {
                        ui.label("Tile Y:");
                        let mut y_val = *y as i32;
                        if ui
                            .add(egui::DragValue::new(&mut y_val).speed(1.0).range(0..=9999))
                            .changed()
                        {
                            *y = y_val.max(0) as u32;
                            outcome.changed = true;
                        }
                    });
                    // Validation warning for Y coordinate
                    if let Some((_, map_height)) = map_size {
                        if *y >= map_height {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 150, 80),
                                format!(
                                    "⚠ Y coordinate {} is out of bounds (map height: {})",
                                    *y, map_height
                                ),
                            );
                        }
                    }
                }

                if rule.conditions.is_empty() {
                    rule.conditions.push(RuleCondition::Always);
                    outcome.changed = true;
                }
                ui.separator();
                ui.label("Conditions");

                let mut remove_condition_index = None;
                for (condition_index, condition) in rule.conditions.iter_mut().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("Condition {}", condition_index + 1));
                            if ui.small_button("✕").clicked() {
                                remove_condition_index = Some(condition_index);
                            }
                        });
                        outcome.changed |= Self::render_rule_condition_editor(
                            ui,
                            scene_name,
                            rule_index,
                            condition_index,
                            condition,
                        );
                    });
                }

                if let Some(index) = remove_condition_index {
                    outcome.changed |= Self::remove_condition(rule, index);
                }

                ui.horizontal_wrapped(|ui| {
                    for kind in RuleConditionKind::iter() {
                        let label = format!("+ {}", Self::condition_kind_label(kind));
                        if ui.small_button(label).clicked() {
                            Self::add_condition(rule, kind);
                            outcome.changed = true;
                        }
                    }
                });

                for issue in validation_issues
                    .iter()
                    .filter(|issue| issue.rule_index == rule_index && issue.action_index.is_none())
                {
                    ui.colored_label(egui::Color32::from_rgb(255, 210, 80), &issue.message);
                }

                ui.separator();
                ui.label("Actions");

                let mut remove_action_index = None;
                for (action_index, action) in rule.actions.iter_mut().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("Action {}", action_index + 1));
                            if ui.small_button("✕").clicked() {
                                remove_action_index = Some(action_index);
                            }
                        });
                        outcome.changed |= Self::render_rule_action_editor(
                            ui,
                            RuleActionEditorContext {
                                scene_name,
                                rule_index,
                                action_index,
                            },
                            action,
                            validation_issues,
                            audio_choices,
                            scenes,
                            available_dialog_outcomes,
                        );
                    });
                }

                if let Some(index) = remove_action_index {
                    outcome.changed |= Self::remove_action(rule, index);
                }

                ui.horizontal_wrapped(|ui| {
                    for kind in RuleActionKind::iter() {
                        let label = format!("+ {}", Self::action_kind_label(kind));
                        if ui.small_button(label).clicked() {
                            Self::add_action(rule, kind);
                            outcome.changed = true;
                        }
                    }
                });
            });

        outcome
    }

    fn render_rule_action_editor(
        ui: &mut egui::Ui,
        ctx: RuleActionEditorContext<'_>,
        action: &mut RuleAction,
        validation_issues: &[RuleValidationIssue],
        audio_choices: &RuleAudioChoices,
        scenes: &[toki_core::Scene],
        available_dialog_outcomes: &std::collections::BTreeMap<String, Vec<String>>,
    ) -> bool {
        let mut changed = false;
        let id_salt = ctx.id_salt();

        let (selected_kind, kind_changed) =
            Self::render_rule_action_kind_picker(ui, action, &id_salt);
        changed |= kind_changed;
        let current_kind = Self::action_kind(action);
        if selected_kind != current_kind {
            Self::switch_action_kind(action, selected_kind);
        }

        match action {
            RuleAction::PlaySound { channel, sound_id } => {
                changed |= Self::render_play_sound_action_editor(
                    ui,
                    &id_salt,
                    channel,
                    sound_id,
                    audio_choices,
                );
            }
            RuleAction::PlayMusic { track_id } => {
                changed |=
                    Self::render_play_music_action_editor(ui, &id_salt, track_id, audio_choices);
            }
            RuleAction::PlayAnimation { target, state } => {
                changed |=
                    Self::render_play_animation_action_editor(ui, &ctx, &id_salt, target, state);
            }
            RuleAction::SetVelocity { target, velocity } => {
                changed |= Self::render_set_velocity_action_editor(ui, &ctx, target, velocity);
            }
            RuleAction::Spawn {
                entity_type,
                position,
            } => {
                changed |= Self::render_spawn_action_editor(ui, &id_salt, entity_type, position);
            }
            RuleAction::DestroySelf { target } => {
                changed |= Self::render_target_only_action_editor(ui, &ctx, target);
            }
            RuleAction::SwitchScene {
                scene_name,
                spawn_point_id,
                transition,
                duration_ms,
            } => {
                changed |= Self::render_switch_scene_editor(
                    ui,
                    format!(
                        "switch_scene_{}_{}_{}",
                        ctx.scene_name, ctx.rule_index, ctx.action_index
                    ),
                    scene_name,
                    spawn_point_id,
                    transition,
                    duration_ms,
                    scenes,
                );
            }
            RuleAction::StartDialog { dialog_id } => {
                changed |= Self::render_start_dialog_action_editor(
                    ui,
                    &id_salt,
                    dialog_id,
                    available_dialog_outcomes,
                );
            }
            RuleAction::DamageEntity { target, amount } => {
                changed |= Self::render_targeted_amount_action_editor(ui, &ctx, target, amount);
            }
            RuleAction::HealEntity { target, amount } => {
                changed |= Self::render_targeted_amount_action_editor(ui, &ctx, target, amount);
            }
            RuleAction::AddInventoryItem {
                target,
                item_id,
                count,
            } => {
                changed |= Self::render_inventory_action_editor(ui, &ctx, target, item_id, count);
            }
            RuleAction::RemoveInventoryItem {
                target,
                item_id,
                count,
            } => {
                changed |= Self::render_inventory_action_editor(ui, &ctx, target, item_id, count);
            }
            RuleAction::SetEntityActive { target, active } => {
                changed |= Self::render_set_active_action_editor(ui, &ctx, target, active);
            }
            RuleAction::TeleportEntity {
                target,
                tile_x,
                tile_y,
            } => {
                changed |= Self::render_teleport_action_editor(ui, &ctx, target, tile_x, tile_y);
            }
            RuleAction::SetFlag { flag, value } => {
                changed |= Self::render_flag_name_editor(ui, flag);
                changed |= Self::render_rule_flag_value_source_editor(
                    ui,
                    format!("{id_salt}_set_flag"),
                    value,
                );
            }
            RuleAction::IncrementFlag { flag, amount } => {
                changed |= Self::render_flag_name_editor(ui, flag);
                changed |= Self::render_rule_int_source_editor(
                    ui,
                    format!("{id_salt}_increment_flag"),
                    "Amount:",
                    amount,
                );
            }
            RuleAction::ClearFlag { flag } => {
                changed |= Self::render_flag_name_editor(ui, flag);
            }
            RuleAction::SaveGame { slot } | RuleAction::LoadGame { slot } => {
                changed |= Self::render_save_slot_editor(ui, slot);
            }
            RuleAction::ShowUi { ui_id } | RuleAction::HideUi { ui_id } => {
                ui.horizontal(|ui| {
                    ui.label("UI Id:");
                    let mut ui_id_value = ui_id.to_string();
                    if ui.text_edit_singleline(&mut ui_id_value).changed() {
                        *ui_id = ui_id_value.into();
                        changed = true;
                    }
                });
            }
            RuleAction::UpdateUiBinding {
                ui_id,
                binding_key,
                value,
            } => {
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
                    format!("{id_salt}_ui_binding_value"),
                    value,
                );
            }
        }

        for issue in validation_issues.iter().filter(|issue| {
            issue.rule_index == ctx.rule_index && issue.action_index == Some(ctx.action_index)
        }) {
            ui.colored_label(egui::Color32::from_rgb(255, 210, 80), &issue.message);
        }

        changed
    }

    fn render_rule_action_kind_picker(
        ui: &mut egui::Ui,
        action: &RuleAction,
        id_salt: &str,
    ) -> (RuleActionKind, bool) {
        let current_kind = Self::action_kind(action);
        let mut selected_kind = current_kind;
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_salt(format!("rule_action_kind_{id_salt}"))
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
        (selected_kind, changed)
    }

    fn render_play_sound_action_editor(
        ui: &mut egui::Ui,
        id_salt: &str,
        channel: &mut RuleSoundChannel,
        sound_id: &mut String,
        audio_choices: &RuleAudioChoices,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Channel:");
            egui::ComboBox::from_id_salt(format!("rule_sound_channel_{id_salt}"))
                .selected_text(match channel {
                    RuleSoundChannel::Movement => "Movement",
                    RuleSoundChannel::Collision => "Collision",
                })
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(channel, RuleSoundChannel::Movement, "Movement")
                        .changed();
                    changed |= ui
                        .selectable_value(channel, RuleSoundChannel::Collision, "Collision")
                        .changed();
                });
        });
        ui.horizontal(|ui| {
            ui.label("Sound Id:");
            changed |= ui.text_edit_singleline(sound_id).changed();
        });
        changed
            | Self::render_audio_choice_picker(
                ui,
                format!("rule_sfx_picker_{id_salt}"),
                "SFX",
                sound_id,
                &audio_choices.sfx,
            )
    }

    fn render_play_music_action_editor(
        ui: &mut egui::Ui,
        id_salt: &str,
        track_id: &mut String,
        audio_choices: &RuleAudioChoices,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Track Id:");
            changed |= ui.text_edit_singleline(track_id).changed();
        });
        changed
            | Self::render_audio_choice_picker(
                ui,
                format!("rule_music_picker_{id_salt}"),
                "Music",
                track_id,
                &audio_choices.music,
            )
    }

    fn render_play_animation_action_editor(
        ui: &mut egui::Ui,
        ctx: &RuleActionEditorContext<'_>,
        id_salt: &str,
        target: &mut RuleTarget,
        state: &mut AnimationState,
    ) -> bool {
        let mut changed = Self::render_rule_target_editor(
            ui,
            ctx.scene_name,
            ctx.rule_index,
            ctx.action_index,
            target,
        );
        ui.horizontal(|ui| {
            ui.label("State:");
            egui::ComboBox::from_id_salt(format!("rule_animation_state_{id_salt}"))
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

    fn render_set_velocity_action_editor(
        ui: &mut egui::Ui,
        ctx: &RuleActionEditorContext<'_>,
        target: &mut RuleTarget,
        velocity: &mut RuleVec2IntSource,
    ) -> bool {
        let mut changed = Self::render_rule_target_editor(
            ui,
            ctx.scene_name,
            ctx.rule_index,
            ctx.action_index,
            target,
        );
        changed |= Self::render_rule_vec2_source_editor(
            ui,
            (
                "rule_set_velocity",
                ctx.scene_name,
                ctx.rule_index,
                ctx.action_index,
            ),
            "Velocity:",
            velocity,
        );
        changed
    }

    fn render_spawn_action_editor(
        ui: &mut egui::Ui,
        id_salt: &str,
        entity_type: &mut RuleSpawnEntityType,
        position: &mut RuleVec2IntSource,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Entity Type:");
            egui::ComboBox::from_id_salt(format!("rule_spawn_type_{id_salt}"))
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
        changed |=
            Self::render_rule_vec2_source_editor(ui, (id_salt, "spawn"), "Position:", position);
        changed
    }

    fn render_target_only_action_editor(
        ui: &mut egui::Ui,
        ctx: &RuleActionEditorContext<'_>,
        target: &mut RuleTarget,
    ) -> bool {
        Self::render_rule_target_editor(
            ui,
            ctx.scene_name,
            ctx.rule_index,
            ctx.action_index,
            target,
        )
    }

    fn render_start_dialog_action_editor(
        ui: &mut egui::Ui,
        id_salt: &str,
        dialog_id: &mut toki_core::DialogId,
        available_dialog_outcomes: &std::collections::BTreeMap<String, Vec<String>>,
    ) -> bool {
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
                egui::ComboBox::from_id_salt(format!("rule_start_dialog_{id_salt}"))
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

    fn render_targeted_amount_action_editor(
        ui: &mut egui::Ui,
        ctx: &RuleActionEditorContext<'_>,
        target: &mut RuleTarget,
        amount: &mut RuleIntSource,
    ) -> bool {
        let mut changed = Self::render_rule_target_editor(
            ui,
            ctx.scene_name,
            ctx.rule_index,
            ctx.action_index,
            target,
        );
        changed |= Self::render_rule_int_source_editor(
            ui,
            (
                "rule_amount",
                ctx.scene_name,
                ctx.rule_index,
                ctx.action_index,
            ),
            "Amount:",
            amount,
        );
        changed
    }

    fn render_inventory_action_editor(
        ui: &mut egui::Ui,
        ctx: &RuleActionEditorContext<'_>,
        target: &mut RuleTarget,
        item_id: &mut String,
        count: &mut u32,
    ) -> bool {
        let mut changed = Self::render_rule_target_editor(
            ui,
            ctx.scene_name,
            ctx.rule_index,
            ctx.action_index,
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

    fn render_set_active_action_editor(
        ui: &mut egui::Ui,
        ctx: &RuleActionEditorContext<'_>,
        target: &mut RuleTarget,
        active: &mut bool,
    ) -> bool {
        let mut changed = Self::render_rule_target_editor(
            ui,
            ctx.scene_name,
            ctx.rule_index,
            ctx.action_index,
            target,
        );
        ui.horizontal(|ui| {
            changed |= ui.checkbox(active, "Active").changed();
        });
        changed
    }

    fn render_teleport_action_editor(
        ui: &mut egui::Ui,
        ctx: &RuleActionEditorContext<'_>,
        target: &mut RuleTarget,
        tile_x: &mut RuleIntSource,
        tile_y: &mut RuleIntSource,
    ) -> bool {
        let mut changed = Self::render_rule_target_editor(
            ui,
            ctx.scene_name,
            ctx.rule_index,
            ctx.action_index,
            target,
        );
        changed |= Self::render_rule_int_source_editor(
            ui,
            (
                "rule_teleport_x",
                ctx.scene_name,
                ctx.rule_index,
                ctx.action_index,
            ),
            "Tile X:",
            tile_x,
        );
        changed |= Self::render_rule_int_source_editor(
            ui,
            (
                "rule_teleport_y",
                ctx.scene_name,
                ctx.rule_index,
                ctx.action_index,
            ),
            "Tile Y:",
            tile_y,
        );
        changed
    }

    pub(in super::super) fn render_rule_condition_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        rule_index: usize,
        condition_index: usize,
        condition: &mut RuleCondition,
    ) -> bool {
        let mut changed = false;

        let current_kind = Self::condition_kind(condition);
        let mut selected_kind = current_kind;
        ui.horizontal(|ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_salt(format!(
                "rule_condition_kind_{}_{}_{}",
                scene_name, rule_index, condition_index
            ))
            .selected_text(Self::condition_kind_label(current_kind))
            .show_ui(ui, |ui| {
                for candidate in RuleConditionKind::iter() {
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            candidate,
                            Self::condition_kind_label(candidate),
                        )
                        .changed();
                }
            });
        });
        if selected_kind != current_kind {
            Self::switch_condition_kind(condition, selected_kind);
        }

        match condition {
            RuleCondition::Always | RuleCondition::TriggerOtherIsPlayer => {}
            RuleCondition::Expression { expression } => {
                ui.horizontal(|ui| {
                    ui.label("Expression:");
                    changed |= ui.text_edit_singleline(expression).changed();
                });
            }
            RuleCondition::TargetExists { target } => {
                changed |= Self::render_rule_condition_target_editor(
                    ui,
                    scene_name,
                    rule_index,
                    condition_index,
                    target,
                );
            }
            RuleCondition::KeyHeld { key } => {
                ui.horizontal(|ui| {
                    ui.label("Key:");
                    egui::ComboBox::from_id_salt(format!(
                        "rule_condition_key_{}_{}_{}",
                        scene_name, rule_index, condition_index
                    ))
                    .selected_text(Self::rule_key_label(*key))
                    .show_ui(ui, |ui| {
                        for candidate in RuleKey::iter() {
                            changed |= ui
                                .selectable_value(key, candidate, Self::rule_key_label(candidate))
                                .changed();
                        }
                    });
                });
            }
            RuleCondition::EntityActive { target, is_active } => {
                changed |= Self::render_rule_condition_target_editor(
                    ui,
                    scene_name,
                    rule_index,
                    condition_index,
                    target,
                );
                ui.horizontal(|ui| {
                    changed |= ui.checkbox(is_active, "Target Is Active").changed();
                });
            }
            RuleCondition::HealthBelow { target, threshold }
            | RuleCondition::HealthAbove { target, threshold } => {
                changed |= Self::render_rule_condition_target_editor(
                    ui,
                    scene_name,
                    rule_index,
                    condition_index,
                    target,
                );
                ui.horizontal(|ui| {
                    ui.label("Threshold:");
                    changed |= ui
                        .add(egui::DragValue::new(threshold).range(0..=1000))
                        .changed();
                });
            }
            RuleCondition::EntityIsKind { target, kind } => {
                changed |= Self::render_rule_condition_target_editor(
                    ui,
                    scene_name,
                    rule_index,
                    condition_index,
                    target,
                );
                changed |= Self::render_entity_kind_editor(
                    ui,
                    &format!("rule_condition_kind_{}_{}", rule_index, condition_index),
                    kind,
                );
            }
            RuleCondition::TriggerOtherIsKind { kind } => {
                changed |= Self::render_entity_kind_editor(
                    ui,
                    &format!(
                        "rule_condition_other_kind_{}_{}",
                        rule_index, condition_index
                    ),
                    kind,
                );
            }
            RuleCondition::EntityHasTag { target, tag } => {
                changed |= Self::render_rule_condition_target_editor(
                    ui,
                    scene_name,
                    rule_index,
                    condition_index,
                    target,
                );
                ui.horizontal(|ui| {
                    ui.label("Tag:");
                    changed |= ui.text_edit_singleline(tag).changed();
                });
            }
            RuleCondition::TriggerOtherHasTag { tag } => {
                ui.horizontal(|ui| {
                    ui.label("Tag:");
                    changed |= ui.text_edit_singleline(tag).changed();
                });
            }
            RuleCondition::HasInventoryItem {
                target,
                item_id,
                min_count,
            } => {
                changed |= Self::render_rule_condition_target_editor(
                    ui,
                    scene_name,
                    rule_index,
                    condition_index,
                    target,
                );
                ui.horizontal(|ui| {
                    ui.label("Item ID:");
                    changed |= ui.text_edit_singleline(item_id).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Min Count:");
                    changed |= ui
                        .add(egui::DragValue::new(min_count).range(1..=999))
                        .changed();
                });
            }
            RuleCondition::FlagEquals { flag, value } => {
                changed |= Self::render_flag_name_editor(ui, flag);
                changed |= Self::render_flag_value_editor(
                    ui,
                    format!("flag_equals_{scene_name}_{rule_index}_{condition_index}"),
                    value,
                );
            }
            RuleCondition::FlagSet { flag } => {
                changed |= Self::render_flag_name_editor(ui, flag);
            }
            RuleCondition::FlagGreaterThan { flag, value } => {
                changed |= Self::render_flag_name_editor(ui, flag);
                ui.horizontal(|ui| {
                    ui.label("Threshold:");
                    changed |= ui.add(egui::DragValue::new(value).speed(1.0)).changed();
                });
            }
        }

        changed
    }

    pub(in super::super) fn render_audio_choice_picker(
        ui: &mut egui::Ui,
        id_salt: String,
        label: &str,
        selected_name: &mut String,
        choices: &[String],
    ) -> bool {
        if choices.is_empty() {
            return false;
        }

        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(format!("{label} Picker:"));
            egui::ComboBox::from_id_salt(id_salt)
                .selected_text(if selected_name.is_empty() {
                    "(Select)".to_string()
                } else {
                    selected_name.clone()
                })
                .show_ui(ui, |ui| {
                    for choice in choices {
                        changed |= ui
                            .selectable_value(selected_name, choice.clone(), choice)
                            .changed();
                    }
                });
        });
        changed
    }

    pub(in super::super) fn render_rule_target_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        rule_index: usize,
        action_index: usize,
        target: &mut RuleTarget,
    ) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("Target:");
            egui::ComboBox::from_id_salt(format!(
                "rule_target_{}_{}_{}",
                scene_name, rule_index, action_index
            ))
            .selected_text(match target {
                RuleTarget::Player => "Player",
                RuleTarget::Entity(_) => "Entity",
                RuleTarget::RuleOwner => "RuleOwner",
                RuleTarget::TriggerSelf => "TriggerSelf",
                RuleTarget::TriggerOther => "TriggerOther",
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(matches!(target, RuleTarget::Player), "Player")
                    .clicked()
                    && !matches!(target, RuleTarget::Player)
                {
                    *target = RuleTarget::Player;
                    changed = true;
                }

                if ui
                    .selectable_label(matches!(target, RuleTarget::Entity(_)), "Entity")
                    .clicked()
                    && !matches!(target, RuleTarget::Entity(_))
                {
                    *target = RuleTarget::Entity(1);
                    changed = true;
                }

                if ui
                    .selectable_label(matches!(target, RuleTarget::TriggerSelf), "TriggerSelf")
                    .clicked()
                    && !matches!(target, RuleTarget::TriggerSelf)
                {
                    *target = RuleTarget::TriggerSelf;
                    changed = true;
                }

                if ui
                    .selectable_label(matches!(target, RuleTarget::TriggerOther), "TriggerOther")
                    .clicked()
                    && !matches!(target, RuleTarget::TriggerOther)
                {
                    *target = RuleTarget::TriggerOther;
                    changed = true;
                }

                if ui
                    .selectable_label(matches!(target, RuleTarget::RuleOwner), "RuleOwner")
                    .clicked()
                    && !matches!(target, RuleTarget::RuleOwner)
                {
                    *target = RuleTarget::RuleOwner;
                    changed = true;
                }
            });
        });

        if let RuleTarget::Entity(entity_id) = target {
            ui.horizontal(|ui| {
                ui.label("Entity Id:");
                let mut value = *entity_id as i64;
                if ui
                    .add(
                        egui::DragValue::new(&mut value)
                            .speed(1.0)
                            .range(1..=u32::MAX as i64),
                    )
                    .changed()
                {
                    *entity_id = value as u32;
                    changed = true;
                }
            });
        }

        changed
    }

    pub(in super::super) fn render_rule_condition_target_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        rule_index: usize,
        condition_index: usize,
        target: &mut RuleTarget,
    ) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("Target:");
            egui::ComboBox::from_id_salt(format!(
                "rule_condition_target_{}_{}_{}",
                scene_name, rule_index, condition_index
            ))
            .selected_text(match target {
                RuleTarget::Player => "Player",
                RuleTarget::Entity(_) => "Entity",
                RuleTarget::RuleOwner => "RuleOwner",
                RuleTarget::TriggerSelf => "TriggerSelf",
                RuleTarget::TriggerOther => "TriggerOther",
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(matches!(target, RuleTarget::Player), "Player")
                    .clicked()
                    && !matches!(target, RuleTarget::Player)
                {
                    *target = RuleTarget::Player;
                    changed = true;
                }

                if ui
                    .selectable_label(matches!(target, RuleTarget::Entity(_)), "Entity")
                    .clicked()
                    && !matches!(target, RuleTarget::Entity(_))
                {
                    *target = RuleTarget::Entity(1);
                    changed = true;
                }

                if ui
                    .selectable_label(matches!(target, RuleTarget::TriggerSelf), "TriggerSelf")
                    .clicked()
                    && !matches!(target, RuleTarget::TriggerSelf)
                {
                    *target = RuleTarget::TriggerSelf;
                    changed = true;
                }

                if ui
                    .selectable_label(matches!(target, RuleTarget::TriggerOther), "TriggerOther")
                    .clicked()
                    && !matches!(target, RuleTarget::TriggerOther)
                {
                    *target = RuleTarget::TriggerOther;
                    changed = true;
                }

                if ui
                    .selectable_label(matches!(target, RuleTarget::RuleOwner), "RuleOwner")
                    .clicked()
                    && !matches!(target, RuleTarget::RuleOwner)
                {
                    *target = RuleTarget::RuleOwner;
                    changed = true;
                }
            });
        });

        if let RuleTarget::Entity(entity_id) = target {
            ui.horizontal(|ui| {
                ui.label("Entity Id:");
                let mut value = *entity_id as i64;
                if ui
                    .add(
                        egui::DragValue::new(&mut value)
                            .speed(1.0)
                            .range(1..=u32::MAX as i64),
                    )
                    .changed()
                {
                    *entity_id = value as u32;
                    changed = true;
                }
            });
        }

        changed
    }
}
