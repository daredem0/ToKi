use super::*;

impl PanelSystem {
    pub(super) fn trigger_summary(trigger: RuleTrigger) -> String {
        match trigger {
            RuleTrigger::OnStart => "OnStart".to_string(),
            RuleTrigger::OnUpdate => "OnUpdate".to_string(),
            RuleTrigger::OnPlayerMove => "OnPlayerMove".to_string(),
            RuleTrigger::OnKey { key } => format!("OnKey({})", Self::key_label(key)),
            RuleTrigger::OnCollision => "OnCollision".to_string(),
            RuleTrigger::OnDamaged => "OnDamaged".to_string(),
            RuleTrigger::OnDeath => "OnDeath".to_string(),
            RuleTrigger::OnTrigger => "OnTrigger".to_string(),
        }
    }

    pub(super) fn condition_summary(condition: RuleCondition) -> String {
        match condition {
            RuleCondition::Always => "Always".to_string(),
            RuleCondition::TargetExists { target } => {
                format!("TargetExists({})", Self::target_label(target))
            }
            RuleCondition::KeyHeld { key } => format!("KeyHeld({})", Self::key_label(key)),
            RuleCondition::EntityActive { target, is_active } => {
                format!(
                    "EntityActive({}, active={})",
                    Self::target_label(target),
                    is_active
                )
            }
        }
    }

    pub(super) fn action_summary(action: &RuleAction) -> String {
        match action {
            RuleAction::PlaySound { channel, sound_id } => {
                format!(
                    "PlaySound({}, {})",
                    Self::sound_channel_label(*channel),
                    sound_id
                )
            }
            RuleAction::PlayMusic { track_id } => format!("PlayMusic({})", track_id),
            RuleAction::PlayAnimation { target, state } => {
                format!(
                    "PlayAnimation({}, {:?})",
                    Self::target_label(*target),
                    state
                )
            }
            RuleAction::SetVelocity { target, velocity } => format!(
                "SetVelocity({}, {}, {})",
                Self::target_label(*target),
                velocity[0],
                velocity[1]
            ),
            RuleAction::Spawn {
                entity_type,
                position,
            } => format!("Spawn({:?}, {}, {})", entity_type, position[0], position[1]),
            RuleAction::DestroySelf { target } => {
                format!("DestroySelf({})", Self::target_label(*target))
            }
            RuleAction::SwitchScene { scene_name } => format!("SwitchScene({})", scene_name),
        }
    }

    pub(super) fn key_label(key: RuleKey) -> &'static str {
        match key {
            RuleKey::Up => "Up",
            RuleKey::Down => "Down",
            RuleKey::Left => "Left",
            RuleKey::Right => "Right",
            RuleKey::DebugToggle => "DebugToggle",
        }
    }

    pub(super) fn sound_channel_label(channel: RuleSoundChannel) -> &'static str {
        match channel {
            RuleSoundChannel::Movement => "Movement",
            RuleSoundChannel::Collision => "Collision",
        }
    }

    pub(super) fn target_label(target: RuleTarget) -> String {
        match target {
            RuleTarget::Player => "Player".to_string(),
            RuleTarget::Entity(entity_id) => format!("Entity({})", entity_id),
        }
    }

    pub(super) fn graph_trigger_kind(trigger: RuleTrigger) -> GraphTriggerKind {
        match trigger {
            RuleTrigger::OnStart => GraphTriggerKind::Start,
            RuleTrigger::OnUpdate => GraphTriggerKind::Update,
            RuleTrigger::OnPlayerMove => GraphTriggerKind::PlayerMove,
            RuleTrigger::OnKey { .. } => GraphTriggerKind::Key,
            RuleTrigger::OnCollision => GraphTriggerKind::Collision,
            RuleTrigger::OnDamaged => GraphTriggerKind::Damaged,
            RuleTrigger::OnDeath => GraphTriggerKind::Death,
            RuleTrigger::OnTrigger => GraphTriggerKind::Trigger,
        }
    }

    pub(super) fn graph_trigger_kind_label(kind: GraphTriggerKind) -> &'static str {
        match kind {
            GraphTriggerKind::Start => "OnStart",
            GraphTriggerKind::Update => "OnUpdate",
            GraphTriggerKind::PlayerMove => "OnPlayerMove",
            GraphTriggerKind::Key => "OnKey",
            GraphTriggerKind::Collision => "OnCollision",
            GraphTriggerKind::Damaged => "OnDamaged",
            GraphTriggerKind::Death => "OnDeath",
            GraphTriggerKind::Trigger => "OnTrigger",
        }
    }

    pub(super) fn graph_default_trigger(kind: GraphTriggerKind) -> RuleTrigger {
        match kind {
            GraphTriggerKind::Start => RuleTrigger::OnStart,
            GraphTriggerKind::Update => RuleTrigger::OnUpdate,
            GraphTriggerKind::PlayerMove => RuleTrigger::OnPlayerMove,
            GraphTriggerKind::Key => RuleTrigger::OnKey { key: RuleKey::Up },
            GraphTriggerKind::Collision => RuleTrigger::OnCollision,
            GraphTriggerKind::Damaged => RuleTrigger::OnDamaged,
            GraphTriggerKind::Death => RuleTrigger::OnDeath,
            GraphTriggerKind::Trigger => RuleTrigger::OnTrigger,
        }
    }

    pub(super) fn graph_condition_kind(condition: RuleCondition) -> GraphConditionKind {
        match condition {
            RuleCondition::Always => GraphConditionKind::Always,
            RuleCondition::TargetExists { .. } => GraphConditionKind::TargetExists,
            RuleCondition::KeyHeld { .. } => GraphConditionKind::KeyHeld,
            RuleCondition::EntityActive { .. } => GraphConditionKind::EntityActive,
        }
    }

    pub(super) fn graph_condition_kind_label(kind: GraphConditionKind) -> &'static str {
        match kind {
            GraphConditionKind::Always => "Always",
            GraphConditionKind::TargetExists => "TargetExists",
            GraphConditionKind::KeyHeld => "KeyHeld",
            GraphConditionKind::EntityActive => "EntityActive",
        }
    }

    pub(super) fn graph_default_condition(kind: GraphConditionKind) -> RuleCondition {
        match kind {
            GraphConditionKind::Always => RuleCondition::Always,
            GraphConditionKind::TargetExists => RuleCondition::TargetExists {
                target: RuleTarget::Player,
            },
            GraphConditionKind::KeyHeld => RuleCondition::KeyHeld { key: RuleKey::Up },
            GraphConditionKind::EntityActive => RuleCondition::EntityActive {
                target: RuleTarget::Player,
                is_active: true,
            },
        }
    }

    pub(super) fn graph_action_kind(action: &RuleAction) -> GraphActionKind {
        match action {
            RuleAction::PlaySound { .. } => GraphActionKind::PlaySound,
            RuleAction::PlayMusic { .. } => GraphActionKind::PlayMusic,
            RuleAction::PlayAnimation { .. } => GraphActionKind::PlayAnimation,
            RuleAction::SetVelocity { .. } => GraphActionKind::SetVelocity,
            RuleAction::Spawn { .. } => GraphActionKind::Spawn,
            RuleAction::DestroySelf { .. } => GraphActionKind::DestroySelf,
            RuleAction::SwitchScene { .. } => GraphActionKind::SwitchScene,
        }
    }

    pub(super) fn graph_action_kind_label(kind: GraphActionKind) -> &'static str {
        match kind {
            GraphActionKind::PlaySound => "PlaySound",
            GraphActionKind::PlayMusic => "PlayMusic",
            GraphActionKind::PlayAnimation => "PlayAnimation",
            GraphActionKind::SetVelocity => "SetVelocity",
            GraphActionKind::Spawn => "Spawn",
            GraphActionKind::DestroySelf => "DestroySelf",
            GraphActionKind::SwitchScene => "SwitchScene",
        }
    }

    pub(super) fn graph_default_action(kind: GraphActionKind) -> RuleAction {
        match kind {
            GraphActionKind::PlaySound => RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_placeholder".to_string(),
            },
            GraphActionKind::PlayMusic => RuleAction::PlayMusic {
                track_id: "music_placeholder".to_string(),
            },
            GraphActionKind::PlayAnimation => RuleAction::PlayAnimation {
                target: RuleTarget::Player,
                state: AnimationState::Idle,
            },
            GraphActionKind::SetVelocity => RuleAction::SetVelocity {
                target: RuleTarget::Player,
                velocity: [0, 0],
            },
            GraphActionKind::Spawn => RuleAction::Spawn {
                entity_type: RuleSpawnEntityType::Npc,
                position: [0, 0],
            },
            GraphActionKind::DestroySelf => RuleAction::DestroySelf {
                target: RuleTarget::Player,
            },
            GraphActionKind::SwitchScene => RuleAction::SwitchScene {
                scene_name: String::new(),
            },
        }
    }

    pub(super) fn edit_graph_condition_payload(
        ui: &mut egui::Ui,
        condition: &mut RuleCondition,
        id_prefix: &str,
    ) -> bool {
        match condition {
            RuleCondition::Always => false,
            RuleCondition::TargetExists { target } => {
                Self::edit_rule_target(ui, target, &format!("{id_prefix}::target"))
            }
            RuleCondition::KeyHeld { key } => {
                Self::edit_rule_key(ui, key, &format!("{id_prefix}::key"))
            }
            RuleCondition::EntityActive { target, is_active } => {
                let mut changed =
                    Self::edit_rule_target(ui, target, &format!("{id_prefix}::entity_target"));
                changed |= ui.checkbox(is_active, "Active").changed();
                changed
            }
        }
    }

    pub(super) fn edit_graph_action_payload(
        ui: &mut egui::Ui,
        action: &mut RuleAction,
        id_prefix: &str,
    ) -> bool {
        match action {
            RuleAction::PlaySound { channel, sound_id } => {
                let mut changed = false;
                egui::ComboBox::from_id_salt((id_prefix, "channel"))
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
                changed |= ui.text_edit_singleline(sound_id).changed();
                changed
            }
            RuleAction::PlayMusic { track_id } => ui.text_edit_singleline(track_id).changed(),
            RuleAction::PlayAnimation { target, state } => {
                let mut changed =
                    Self::edit_rule_target(ui, target, &format!("{id_prefix}::anim_target"));
                egui::ComboBox::from_id_salt((id_prefix, "anim_state"))
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
                changed
            }
            RuleAction::SetVelocity { target, velocity } => {
                let mut changed =
                    Self::edit_rule_target(ui, target, &format!("{id_prefix}::vel_target"));
                changed |= ui
                    .add(egui::DragValue::new(&mut velocity[0]).speed(1.0))
                    .changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut velocity[1]).speed(1.0))
                    .changed();
                changed
            }
            RuleAction::Spawn {
                entity_type,
                position,
            } => {
                let mut changed = false;
                egui::ComboBox::from_id_salt((id_prefix, "spawn_type"))
                    .selected_text(match entity_type {
                        RuleSpawnEntityType::PlayerLikeNpc => "PlayerLikeNpc",
                        RuleSpawnEntityType::Npc => "Npc",
                        RuleSpawnEntityType::Item => "Item",
                        RuleSpawnEntityType::Decoration => "Decoration",
                        RuleSpawnEntityType::Trigger => "Trigger",
                    })
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
                                    match candidate {
                                        RuleSpawnEntityType::PlayerLikeNpc => "PlayerLikeNpc",
                                        RuleSpawnEntityType::Npc => "Npc",
                                        RuleSpawnEntityType::Item => "Item",
                                        RuleSpawnEntityType::Decoration => "Decoration",
                                        RuleSpawnEntityType::Trigger => "Trigger",
                                    },
                                )
                                .changed();
                        }
                    });
                changed |= ui
                    .add(egui::DragValue::new(&mut position[0]).speed(1.0))
                    .changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut position[1]).speed(1.0))
                    .changed();
                changed
            }
            RuleAction::DestroySelf { target } => {
                Self::edit_rule_target(ui, target, &format!("{id_prefix}::destroy_target"))
            }
            RuleAction::SwitchScene { scene_name } => ui.text_edit_singleline(scene_name).changed(),
        }
    }

    pub(super) fn edit_rule_target(
        ui: &mut egui::Ui,
        target: &mut RuleTarget,
        id_salt: &str,
    ) -> bool {
        let mut changed = false;
        egui::ComboBox::from_id_salt((id_salt, "kind"))
            .selected_text(match target {
                RuleTarget::Player => "Player",
                RuleTarget::Entity(_) => "Entity",
            })
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(target, RuleTarget::Player, "Player")
                    .changed();
                let entity_label = match target {
                    RuleTarget::Entity(entity_id) => format!("Entity({})", entity_id),
                    RuleTarget::Player => "Entity(0)".to_string(),
                };
                if ui
                    .selectable_label(matches!(target, RuleTarget::Entity(_)), entity_label)
                    .clicked()
                    && !matches!(target, RuleTarget::Entity(_))
                {
                    *target = RuleTarget::Entity(0);
                    changed = true;
                }
            });

        if let RuleTarget::Entity(entity_id) = target {
            let mut entity_id_i64 = *entity_id as i64;
            let id_changed = ui
                .add(
                    egui::DragValue::new(&mut entity_id_i64)
                        .speed(1.0)
                        .range(0_i64..=u32::MAX as i64),
                )
                .changed();
            if id_changed {
                *entity_id = entity_id_i64 as u32;
                changed = true;
            }
        }
        changed
    }

    pub(super) fn edit_rule_key(ui: &mut egui::Ui, key: &mut RuleKey, id_salt: &str) -> bool {
        let mut changed = false;
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(match key {
                RuleKey::Up => "Up",
                RuleKey::Down => "Down",
                RuleKey::Left => "Left",
                RuleKey::Right => "Right",
                RuleKey::DebugToggle => "DebugToggle",
            })
            .show_ui(ui, |ui| {
                for candidate in [
                    RuleKey::Up,
                    RuleKey::Down,
                    RuleKey::Left,
                    RuleKey::Right,
                    RuleKey::DebugToggle,
                ] {
                    changed |= ui
                        .selectable_value(
                            key,
                            candidate,
                            match candidate {
                                RuleKey::Up => "Up",
                                RuleKey::Down => "Down",
                                RuleKey::Left => "Left",
                                RuleKey::Right => "Right",
                                RuleKey::DebugToggle => "DebugToggle",
                            },
                        )
                        .changed();
                }
            });
        changed
    }
}
