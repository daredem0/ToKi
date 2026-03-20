use super::*;

struct RuleActionEditorContext<'a> {
    scene_name: &'a str,
    rule_index: usize,
    action_index: usize,
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
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn render_rule_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        rule_index: usize,
        rule: &mut Rule,
        validation_issues: &[RuleValidationIssue],
        audio_choices: &RuleAudioChoices,
        scenes: &[toki_core::Scene],
        map_size: Option<(u32, u32)>,
    ) -> RuleEditorOutcome {
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
                        outcome.changed |= ui
                            .selectable_value(
                                &mut trigger_kind,
                                RuleTriggerKind::Start,
                                Self::trigger_kind_label(RuleTriggerKind::Start),
                            )
                            .changed();
                        outcome.changed |= ui
                            .selectable_value(
                                &mut trigger_kind,
                                RuleTriggerKind::Update,
                                Self::trigger_kind_label(RuleTriggerKind::Update),
                            )
                            .changed();
                        outcome.changed |= ui
                            .selectable_value(
                                &mut trigger_kind,
                                RuleTriggerKind::PlayerMove,
                                Self::trigger_kind_label(RuleTriggerKind::PlayerMove),
                            )
                            .changed();
                        outcome.changed |= ui
                            .selectable_value(
                                &mut trigger_kind,
                                RuleTriggerKind::Key,
                                Self::trigger_kind_label(RuleTriggerKind::Key),
                            )
                            .changed();
                        outcome.changed |= ui
                            .selectable_value(
                                &mut trigger_kind,
                                RuleTriggerKind::Collision,
                                Self::trigger_kind_label(RuleTriggerKind::Collision),
                            )
                            .changed();
                        outcome.changed |= ui
                            .selectable_value(
                                &mut trigger_kind,
                                RuleTriggerKind::Damaged,
                                Self::trigger_kind_label(RuleTriggerKind::Damaged),
                            )
                            .changed();
                        outcome.changed |= ui
                            .selectable_value(
                                &mut trigger_kind,
                                RuleTriggerKind::Death,
                                Self::trigger_kind_label(RuleTriggerKind::Death),
                            )
                            .changed();
                        outcome.changed |= ui
                            .selectable_value(
                                &mut trigger_kind,
                                RuleTriggerKind::Trigger,
                                Self::trigger_kind_label(RuleTriggerKind::Trigger),
                            )
                            .changed();
                        outcome.changed |= ui
                            .selectable_value(
                                &mut trigger_kind,
                                RuleTriggerKind::Interact,
                                Self::trigger_kind_label(RuleTriggerKind::Interact),
                            )
                            .changed();
                        outcome.changed |= ui
                            .selectable_value(
                                &mut trigger_kind,
                                RuleTriggerKind::TileEnter,
                                Self::trigger_kind_label(RuleTriggerKind::TileEnter),
                            )
                            .changed();
                        outcome.changed |= ui
                            .selectable_value(
                                &mut trigger_kind,
                                RuleTriggerKind::TileExit,
                                Self::trigger_kind_label(RuleTriggerKind::TileExit),
                            )
                            .changed();
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
                            for candidate in [
                                RuleKey::Up,
                                RuleKey::Down,
                                RuleKey::Left,
                                RuleKey::Right,
                                RuleKey::DebugToggle,
                                RuleKey::Interact,
                                RuleKey::AttackPrimary,
                                RuleKey::AttackSecondary,
                                RuleKey::Inventory,
                                RuleKey::Pause,
                            ] {
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

                if let RuleTrigger::OnTileEnter { x, y } | RuleTrigger::OnTileExit { x, y } = &mut rule.trigger {
                    ui.horizontal(|ui| {
                        ui.label("Tile X:");
                        let mut x_val = *x as i32;
                        if ui.add(egui::DragValue::new(&mut x_val).speed(1.0).range(0..=9999)).changed() {
                            *x = x_val.max(0) as u32;
                            outcome.changed = true;
                        }
                    });
                    // Validation warning for X coordinate
                    if let Some((map_width, _)) = map_size {
                        if *x >= map_width {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 150, 80),
                                format!("⚠ X coordinate {} is out of bounds (map width: {})", *x, map_width),
                            );
                        }
                    }

                    ui.horizontal(|ui| {
                        ui.label("Tile Y:");
                        let mut y_val = *y as i32;
                        if ui.add(egui::DragValue::new(&mut y_val).speed(1.0).range(0..=9999)).changed() {
                            *y = y_val.max(0) as u32;
                            outcome.changed = true;
                        }
                    });
                    // Validation warning for Y coordinate
                    if let Some((_, map_height)) = map_size {
                        if *y >= map_height {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 150, 80),
                                format!("⚠ Y coordinate {} is out of bounds (map height: {})", *y, map_height),
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

                ui.horizontal(|ui| {
                    if ui.small_button("+ Always").clicked() {
                        Self::add_condition(rule, RuleConditionKind::Always);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ TargetExists").clicked() {
                        Self::add_condition(rule, RuleConditionKind::TargetExists);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ KeyHeld").clicked() {
                        Self::add_condition(rule, RuleConditionKind::KeyHeld);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ EntityActive").clicked() {
                        Self::add_condition(rule, RuleConditionKind::EntityActive);
                        outcome.changed = true;
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
                        );
                    });
                }

                if let Some(index) = remove_action_index {
                    outcome.changed |= Self::remove_action(rule, index);
                }

                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("+ PlaySound").clicked() {
                        Self::add_action(rule, RuleActionKind::PlaySound);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ PlayMusic").clicked() {
                        Self::add_action(rule, RuleActionKind::PlayMusic);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ PlayAnimation").clicked() {
                        Self::add_action(rule, RuleActionKind::PlayAnimation);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ SetVelocity").clicked() {
                        Self::add_action(rule, RuleActionKind::SetVelocity);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ Spawn").clicked() {
                        Self::add_action(rule, RuleActionKind::Spawn);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ DestroySelf").clicked() {
                        Self::add_action(rule, RuleActionKind::DestroySelf);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ SwitchScene").clicked() {
                        Self::add_action(rule, RuleActionKind::SwitchScene);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ DamageEntity").clicked() {
                        Self::add_action(rule, RuleActionKind::DamageEntity);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ HealEntity").clicked() {
                        Self::add_action(rule, RuleActionKind::HealEntity);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ AddInventoryItem").clicked() {
                        Self::add_action(rule, RuleActionKind::AddInventoryItem);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ RemoveInventoryItem").clicked() {
                        Self::add_action(rule, RuleActionKind::RemoveInventoryItem);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ SetEntityActive").clicked() {
                        Self::add_action(rule, RuleActionKind::SetEntityActive);
                        outcome.changed = true;
                    }
                    if ui.small_button("+ TeleportEntity").clicked() {
                        Self::add_action(rule, RuleActionKind::TeleportEntity);
                        outcome.changed = true;
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
    ) -> bool {
        let mut changed = false;
        let id_salt = ctx.id_salt();

        let current_kind = Self::action_kind(action);
        let mut selected_kind = current_kind;
        ui.horizontal(|ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_salt(format!("rule_action_kind_{id_salt}"))
                .selected_text(Self::action_kind_label(current_kind))
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            RuleActionKind::PlaySound,
                            Self::action_kind_label(RuleActionKind::PlaySound),
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            RuleActionKind::PlayMusic,
                            Self::action_kind_label(RuleActionKind::PlayMusic),
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            RuleActionKind::PlayAnimation,
                            Self::action_kind_label(RuleActionKind::PlayAnimation),
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            RuleActionKind::SetVelocity,
                            Self::action_kind_label(RuleActionKind::SetVelocity),
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            RuleActionKind::Spawn,
                            Self::action_kind_label(RuleActionKind::Spawn),
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            RuleActionKind::DestroySelf,
                            Self::action_kind_label(RuleActionKind::DestroySelf),
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            RuleActionKind::SwitchScene,
                            Self::action_kind_label(RuleActionKind::SwitchScene),
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            RuleActionKind::DamageEntity,
                            Self::action_kind_label(RuleActionKind::DamageEntity),
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            RuleActionKind::HealEntity,
                            Self::action_kind_label(RuleActionKind::HealEntity),
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            RuleActionKind::AddInventoryItem,
                            Self::action_kind_label(RuleActionKind::AddInventoryItem),
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            RuleActionKind::RemoveInventoryItem,
                            Self::action_kind_label(RuleActionKind::RemoveInventoryItem),
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            RuleActionKind::SetEntityActive,
                            Self::action_kind_label(RuleActionKind::SetEntityActive),
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            RuleActionKind::TeleportEntity,
                            Self::action_kind_label(RuleActionKind::TeleportEntity),
                        )
                        .changed();
                });
        });
        if selected_kind != current_kind {
            Self::switch_action_kind(action, selected_kind);
        }

        match action {
            RuleAction::PlaySound { channel, sound_id } => {
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

                changed |= Self::render_audio_choice_picker(
                    ui,
                    format!("rule_sfx_picker_{id_salt}"),
                    "SFX",
                    sound_id,
                    &audio_choices.sfx,
                );
            }
            RuleAction::PlayMusic { track_id } => {
                ui.horizontal(|ui| {
                    ui.label("Track Id:");
                    changed |= ui.text_edit_singleline(track_id).changed();
                });

                changed |= Self::render_audio_choice_picker(
                    ui,
                    format!("rule_music_picker_{id_salt}"),
                    "Music",
                    track_id,
                    &audio_choices.music,
                );
            }
            RuleAction::PlayAnimation { target, state } => {
                changed |= Self::render_rule_target_editor(
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
                                    .selectable_value(
                                        state,
                                        candidate,
                                        animation_state_label(candidate),
                                    )
                                    .changed();
                            }
                        });
                });
            }
            RuleAction::SetVelocity { target, velocity } => {
                changed |= Self::render_rule_target_editor(
                    ui,
                    ctx.scene_name,
                    ctx.rule_index,
                    ctx.action_index,
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
            }
            RuleAction::Spawn {
                entity_type,
                position,
            } => {
                ui.horizontal(|ui| {
                    ui.label("Entity Type:");
                    egui::ComboBox::from_id_salt(format!("rule_spawn_type_{id_salt}"))
                        .selected_text(Self::spawn_entity_type_label(*entity_type))
                        .show_ui(ui, |ui| {
                            for candidate in [
                                RuleSpawnEntityType::PlayerLikeNpc,
                                RuleSpawnEntityType::Npc,
                                RuleSpawnEntityType::Item,
                                RuleSpawnEntityType::Decoration,
                                RuleSpawnEntityType::Trigger,
                            ] {
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
            }
            RuleAction::DestroySelf { target } => {
                changed |= Self::render_rule_target_editor(
                    ui,
                    ctx.scene_name,
                    ctx.rule_index,
                    ctx.action_index,
                    target,
                );
            }
            RuleAction::SwitchScene {
                scene_name,
                spawn_point_id,
            } => {
                changed |= Self::render_switch_scene_editor(
                    ui,
                    format!(
                        "switch_scene_{}_{}_{}",
                        ctx.scene_name, ctx.rule_index, ctx.action_index
                    ),
                    scene_name,
                    spawn_point_id,
                    scenes,
                );
            }
            RuleAction::DamageEntity { target, amount } => {
                changed |= Self::render_rule_target_editor(
                    ui,
                    ctx.scene_name,
                    ctx.rule_index,
                    ctx.action_index,
                    target,
                );

                ui.horizontal(|ui| {
                    ui.label("Amount:");
                    changed |= ui
                        .add(egui::DragValue::new(amount).speed(1.0).range(0..=9999))
                        .changed();
                });
            }
            RuleAction::HealEntity { target, amount } => {
                changed |= Self::render_rule_target_editor(
                    ui,
                    ctx.scene_name,
                    ctx.rule_index,
                    ctx.action_index,
                    target,
                );

                ui.horizontal(|ui| {
                    ui.label("Amount:");
                    changed |= ui
                        .add(egui::DragValue::new(amount).speed(1.0).range(0..=9999))
                        .changed();
                });
            }
            RuleAction::AddInventoryItem {
                target,
                item_id,
                count,
            } => {
                changed |= Self::render_rule_target_editor(
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
            }
            RuleAction::RemoveInventoryItem {
                target,
                item_id,
                count,
            } => {
                changed |= Self::render_rule_target_editor(
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
            }
            RuleAction::SetEntityActive { target, active } => {
                changed |= Self::render_rule_target_editor(
                    ui,
                    ctx.scene_name,
                    ctx.rule_index,
                    ctx.action_index,
                    target,
                );

                ui.horizontal(|ui| {
                    changed |= ui.checkbox(active, "Active").changed();
                });
            }
            RuleAction::TeleportEntity { target, position } => {
                changed |= Self::render_rule_target_editor(
                    ui,
                    ctx.scene_name,
                    ctx.rule_index,
                    ctx.action_index,
                    target,
                );

                ui.horizontal(|ui| {
                    ui.label("Position:");
                    changed |= ui
                        .add(egui::DragValue::new(&mut position[0]).speed(1.0))
                        .changed();
                    changed |= ui
                        .add(egui::DragValue::new(&mut position[1]).speed(1.0))
                        .changed();
                });
            }
        }

        for issue in validation_issues.iter().filter(|issue| {
            issue.rule_index == ctx.rule_index && issue.action_index == Some(ctx.action_index)
        }) {
            ui.colored_label(egui::Color32::from_rgb(255, 210, 80), &issue.message);
        }

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
                changed |= ui
                    .selectable_value(
                        &mut selected_kind,
                        RuleConditionKind::Always,
                        Self::condition_kind_label(RuleConditionKind::Always),
                    )
                    .changed();
                changed |= ui
                    .selectable_value(
                        &mut selected_kind,
                        RuleConditionKind::TargetExists,
                        Self::condition_kind_label(RuleConditionKind::TargetExists),
                    )
                    .changed();
                changed |= ui
                    .selectable_value(
                        &mut selected_kind,
                        RuleConditionKind::KeyHeld,
                        Self::condition_kind_label(RuleConditionKind::KeyHeld),
                    )
                    .changed();
                changed |= ui
                    .selectable_value(
                        &mut selected_kind,
                        RuleConditionKind::EntityActive,
                        Self::condition_kind_label(RuleConditionKind::EntityActive),
                    )
                    .changed();
            });
        });
        if selected_kind != current_kind {
            Self::switch_condition_kind(condition, selected_kind);
        }

        match condition {
            RuleCondition::Always | RuleCondition::TriggerOtherIsPlayer => {}
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
                        for candidate in [
                            RuleKey::Up,
                            RuleKey::Down,
                            RuleKey::Left,
                            RuleKey::Right,
                            RuleKey::DebugToggle,
                            RuleKey::Interact,
                            RuleKey::AttackPrimary,
                            RuleKey::AttackSecondary,
                            RuleKey::Inventory,
                            RuleKey::Pause,
                        ] {
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
                    &format!("rule_condition_other_kind_{}_{}", rule_index, condition_index),
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
