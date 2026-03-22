use super::*;

impl PanelSystem {
    pub(super) fn trigger_summary(trigger: RuleTrigger) -> String {
        match trigger {
            RuleTrigger::OnStart => "OnStart".to_string(),
            RuleTrigger::OnUpdate => "OnUpdate".to_string(),
            RuleTrigger::OnPlayerMove => "OnPlayerMove".to_string(),
            RuleTrigger::OnKey { key } => format!("OnKey({})", Self::key_label(key)),
            RuleTrigger::OnCollision { entity: None } => "OnCollision".to_string(),
            RuleTrigger::OnCollision {
                entity: Some(target),
            } => {
                format!("OnCollision({})", Self::target_label(target))
            }
            RuleTrigger::OnDamaged { entity: None } => "OnDamaged".to_string(),
            RuleTrigger::OnDamaged {
                entity: Some(target),
            } => {
                format!("OnDamaged({})", Self::target_label(target))
            }
            RuleTrigger::OnDeath { entity: None } => "OnDeath".to_string(),
            RuleTrigger::OnDeath {
                entity: Some(target),
            } => {
                format!("OnDeath({})", Self::target_label(target))
            }
            RuleTrigger::OnTrigger => "OnTrigger".to_string(),
            RuleTrigger::OnInteract { entity: None, .. } => "OnInteract".to_string(),
            RuleTrigger::OnInteract {
                entity: Some(target),
                ..
            } => {
                format!("OnInteract({})", Self::target_label(target))
            }
            RuleTrigger::OnTileEnter { x, y } => format!("OnTileEnter({}, {})", x, y),
            RuleTrigger::OnTileExit { x, y } => format!("OnTileExit({}, {})", x, y),
        }
    }

    pub(super) fn condition_summary(condition: &RuleCondition) -> String {
        match condition {
            RuleCondition::Always => "Always".to_string(),
            RuleCondition::TargetExists { target } => {
                format!("TargetExists({})", Self::target_label(*target))
            }
            RuleCondition::KeyHeld { key } => format!("KeyHeld({})", Self::key_label(*key)),
            RuleCondition::EntityActive { target, is_active } => {
                format!(
                    "EntityActive({}, active={})",
                    Self::target_label(*target),
                    is_active
                )
            }
            RuleCondition::HealthBelow { target, threshold } => {
                format!(
                    "HealthBelow({}, {})",
                    Self::target_label(*target),
                    threshold
                )
            }
            RuleCondition::HealthAbove { target, threshold } => {
                format!(
                    "HealthAbove({}, {})",
                    Self::target_label(*target),
                    threshold
                )
            }
            RuleCondition::TriggerOtherIsPlayer => "TriggerOtherIsPlayer".to_string(),
            RuleCondition::EntityIsKind { target, kind } => {
                format!("EntityIsKind({}, {:?})", Self::target_label(*target), kind)
            }
            RuleCondition::TriggerOtherIsKind { kind } => {
                format!("TriggerOtherIsKind({:?})", kind)
            }
            RuleCondition::EntityHasTag { target, tag } => {
                format!("EntityHasTag({}, {})", Self::target_label(*target), tag)
            }
            RuleCondition::TriggerOtherHasTag { tag } => {
                format!("TriggerOtherHasTag({})", tag)
            }
            RuleCondition::HasInventoryItem {
                target,
                item_id,
                min_count,
            } => {
                format!(
                    "HasInventoryItem({}, {}, {})",
                    Self::target_label(*target),
                    item_id,
                    min_count
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
            RuleAction::SwitchScene {
                scene_name,
                spawn_point_id,
            } => format!("SwitchScene({} -> {})", scene_name, spawn_point_id),
            RuleAction::DamageEntity { target, amount } => {
                format!("DamageEntity({}, {})", Self::target_label(*target), amount)
            }
            RuleAction::HealEntity { target, amount } => {
                format!("HealEntity({}, {})", Self::target_label(*target), amount)
            }
            RuleAction::AddInventoryItem {
                target,
                item_id,
                count,
            } => format!(
                "AddItem({}, {}, {})",
                Self::target_label(*target),
                item_id,
                count
            ),
            RuleAction::RemoveInventoryItem {
                target,
                item_id,
                count,
            } => format!(
                "RemoveItem({}, {}, {})",
                Self::target_label(*target),
                item_id,
                count
            ),
            RuleAction::SetEntityActive { target, active } => {
                format!("SetActive({}, {})", Self::target_label(*target), active)
            }
            RuleAction::TeleportEntity {
                target,
                tile_x,
                tile_y,
            } => format!(
                "Teleport({}, tile[{}, {}])",
                Self::target_label(*target),
                tile_x,
                tile_y
            ),
        }
    }

    pub(super) fn key_label(key: RuleKey) -> &'static str {
        shared_rule_key_label(key)
    }

    pub(super) fn sound_channel_label(channel: RuleSoundChannel) -> &'static str {
        shared_rule_sound_channel_label(channel)
    }

    pub(super) fn target_label(target: RuleTarget) -> String {
        shared_rule_target_label(target)
    }

    pub(super) fn graph_trigger_kind(trigger: RuleTrigger) -> GraphTriggerKind {
        shared_rule_trigger_kind(&trigger)
    }

    pub(super) fn graph_trigger_kind_label(kind: GraphTriggerKind) -> &'static str {
        shared_rule_trigger_kind_label(kind)
    }

    pub(super) fn graph_default_trigger(kind: GraphTriggerKind) -> RuleTrigger {
        shared_default_rule_trigger(kind)
    }

    pub(super) fn graph_condition_kind(condition: &RuleCondition) -> GraphConditionKind {
        shared_rule_condition_kind(condition)
    }

    pub(super) fn graph_condition_kind_label(kind: GraphConditionKind) -> &'static str {
        shared_rule_condition_kind_label(kind)
    }

    pub(super) fn graph_default_condition(kind: GraphConditionKind) -> RuleCondition {
        shared_default_rule_condition(kind)
    }

    pub(super) fn graph_action_kind(action: &RuleAction) -> GraphActionKind {
        shared_rule_action_kind(action)
    }

    pub(super) fn graph_action_kind_label(kind: GraphActionKind) -> &'static str {
        shared_rule_action_kind_label(kind)
    }

    pub(super) fn graph_default_action(kind: GraphActionKind) -> RuleAction {
        match kind {
            GraphActionKind::DamageEntity => RuleAction::DamageEntity {
                target: RuleTarget::TriggerOther,
                amount: 10,
            },
            GraphActionKind::SetEntityActive => RuleAction::SetEntityActive {
                target: RuleTarget::TriggerOther,
                active: false,
            },
            _ => shared_default_rule_action(kind),
        }
    }

    pub(super) fn edit_graph_condition_payload(
        ui: &mut egui::Ui,
        condition: &mut RuleCondition,
        id_prefix: &str,
    ) -> bool {
        match condition {
            RuleCondition::Always | RuleCondition::TriggerOtherIsPlayer => false,
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
            RuleCondition::HealthBelow { target, threshold }
            | RuleCondition::HealthAbove { target, threshold } => {
                let mut changed =
                    Self::edit_rule_target(ui, target, &format!("{id_prefix}::target"));
                ui.horizontal(|ui| {
                    ui.label("Threshold:");
                    changed |= ui.add(egui::DragValue::new(threshold)).changed();
                });
                changed
            }
            RuleCondition::EntityIsKind { target, kind } => {
                let mut changed =
                    Self::edit_rule_target(ui, target, &format!("{id_prefix}::target"));
                changed |= Self::edit_entity_kind(ui, kind, &format!("{id_prefix}::kind"));
                changed
            }
            RuleCondition::TriggerOtherIsKind { kind } => {
                Self::edit_entity_kind(ui, kind, &format!("{id_prefix}::kind"))
            }
            RuleCondition::EntityHasTag { target, tag } => {
                let mut changed =
                    Self::edit_rule_target(ui, target, &format!("{id_prefix}::target"));
                ui.horizontal(|ui| {
                    ui.label("Tag:");
                    changed |= ui.text_edit_singleline(tag).changed();
                });
                changed
            }
            RuleCondition::TriggerOtherHasTag { tag } => {
                let mut changed = false;
                ui.horizontal(|ui| {
                    ui.label("Tag:");
                    changed |= ui.text_edit_singleline(tag).changed();
                });
                changed
            }
            RuleCondition::HasInventoryItem {
                target,
                item_id,
                min_count,
            } => {
                let mut changed =
                    Self::edit_rule_target(ui, target, &format!("{id_prefix}::target"));
                ui.horizontal(|ui| {
                    ui.label("Item ID:");
                    changed |= ui.text_edit_singleline(item_id).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Min Count:");
                    let mut count_i32 = *min_count as i32;
                    if ui
                        .add(egui::DragValue::new(&mut count_i32).range(0..=i32::MAX))
                        .changed()
                    {
                        *min_count = count_i32.max(0) as u32;
                        changed = true;
                    }
                });
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
            RuleAction::SwitchScene {
                scene_name,
                spawn_point_id,
            } => {
                let mut changed = ui.text_edit_singleline(scene_name).changed();
                ui.end_row();
                ui.label("Spawn Point");
                changed |= ui.text_edit_singleline(spawn_point_id).changed();
                changed
            }
            RuleAction::DamageEntity { target, amount } => {
                let mut changed =
                    Self::edit_rule_target(ui, target, &format!("{id_prefix}::damage_target"));
                changed |= ui.add(egui::DragValue::new(amount).speed(1.0)).changed();
                changed
            }
            RuleAction::HealEntity { target, amount } => {
                let mut changed =
                    Self::edit_rule_target(ui, target, &format!("{id_prefix}::heal_target"));
                changed |= ui.add(egui::DragValue::new(amount).speed(1.0)).changed();
                changed
            }
            RuleAction::AddInventoryItem {
                target,
                item_id,
                count,
            } => {
                let mut changed =
                    Self::edit_rule_target(ui, target, &format!("{id_prefix}::add_inv_target"));
                changed |= ui.text_edit_singleline(item_id).changed();
                changed |= ui.add(egui::DragValue::new(count).speed(1.0)).changed();
                changed
            }
            RuleAction::RemoveInventoryItem {
                target,
                item_id,
                count,
            } => {
                let mut changed =
                    Self::edit_rule_target(ui, target, &format!("{id_prefix}::rem_inv_target"));
                changed |= ui.text_edit_singleline(item_id).changed();
                changed |= ui.add(egui::DragValue::new(count).speed(1.0)).changed();
                changed
            }
            RuleAction::SetEntityActive { target, active } => {
                let mut changed =
                    Self::edit_rule_target(ui, target, &format!("{id_prefix}::active_target"));
                changed |= ui.checkbox(active, "Active").changed();
                changed
            }
            RuleAction::TeleportEntity {
                target,
                tile_x,
                tile_y,
            } => {
                let mut changed =
                    Self::edit_rule_target(ui, target, &format!("{id_prefix}::teleport_target"));
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
                RuleTarget::RuleOwner => "RuleOwner",
                RuleTarget::TriggerSelf => "TriggerSelf",
                RuleTarget::TriggerOther => "TriggerOther",
            })
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(target, RuleTarget::Player, "Player")
                    .changed();
                let entity_label = match target {
                    RuleTarget::Entity(entity_id) => format!("Entity({})", entity_id),
                    RuleTarget::Player
                    | RuleTarget::RuleOwner
                    | RuleTarget::TriggerSelf
                    | RuleTarget::TriggerOther => "Entity(0)".to_string(),
                };
                if ui
                    .selectable_label(matches!(target, RuleTarget::Entity(_)), entity_label)
                    .clicked()
                    && !matches!(target, RuleTarget::Entity(_))
                {
                    *target = RuleTarget::Entity(0);
                    changed = true;
                }
                changed |= ui
                    .selectable_value(target, RuleTarget::TriggerSelf, "TriggerSelf")
                    .changed();
                changed |= ui
                    .selectable_value(target, RuleTarget::TriggerOther, "TriggerOther")
                    .changed();
                changed |= ui
                    .selectable_value(target, RuleTarget::RuleOwner, "RuleOwner")
                    .changed();
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
            .selected_text(Self::key_label(*key))
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
                        .selectable_value(key, candidate, Self::key_label(candidate))
                        .changed();
                }
            });
        changed
    }

    pub(super) fn edit_entity_kind(
        ui: &mut egui::Ui,
        kind: &mut EntityKind,
        id_salt: &str,
    ) -> bool {
        let mut changed = false;
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(format!("{:?}", kind))
            .show_ui(ui, |ui| {
                for candidate in [
                    EntityKind::Player,
                    EntityKind::Npc,
                    EntityKind::Item,
                    EntityKind::Decoration,
                    EntityKind::Trigger,
                    EntityKind::Projectile,
                ] {
                    changed |= ui
                        .selectable_value(kind, candidate, format!("{:?}", candidate))
                        .changed();
                }
            });
        changed
    }

    pub(super) fn render_graph_selected_node_editor(
        ui: &mut egui::Ui,
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        node_id: u64,
        scene_name: &str,
    ) -> Option<GraphCommand> {
        let Some(node_kind) = graph
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .map(|node| node.kind.clone())
        else {
            ui.label("Selected node no longer exists.");
            return None;
        };

        let display_label = Self::rule_graph_node_label(graph, node_badges, node_id)
            .unwrap_or_else(|| format!("node {node_id}"));
        ui.horizontal(|ui| {
            ui.monospace(display_label);
        });

        let mut command = None;
        ui.horizontal(|ui| {
            if ui.button("Disconnect All").clicked() {
                command = Some(GraphCommand::DisconnectNode(node_id));
            }
            if ui.button("Delete Node").clicked() {
                command = Some(GraphCommand::RemoveNode(node_id));
            }
        });

        ui.separator();

        match node_kind {
            RuleGraphNodeKind::Trigger(trigger) => {
                let mut trigger_value = trigger;
                let mut kind = Self::graph_trigger_kind(trigger);
                let kind_salt = format!("graph_canvas_trigger_kind::{scene_name}::{node_id}");
                egui::ComboBox::from_id_salt(kind_salt)
                    .selected_text(Self::graph_trigger_kind_label(kind))
                    .show_ui(ui, |ui| {
                        for candidate in [
                            GraphTriggerKind::Start,
                            GraphTriggerKind::Update,
                            GraphTriggerKind::PlayerMove,
                            GraphTriggerKind::Key,
                            GraphTriggerKind::Collision,
                            GraphTriggerKind::Damaged,
                            GraphTriggerKind::Death,
                            GraphTriggerKind::Trigger,
                        ] {
                            ui.selectable_value(
                                &mut kind,
                                candidate,
                                Self::graph_trigger_kind_label(candidate),
                            );
                        }
                    });
                if kind != Self::graph_trigger_kind(trigger) {
                    trigger_value = Self::graph_default_trigger(kind);
                }
                if let RuleTrigger::OnKey { key } = &mut trigger_value {
                    let key_salt = format!("graph_canvas_trigger_key::{scene_name}::{node_id}");
                    let _ = Self::edit_rule_key(ui, key, &key_salt);
                }
                if command.is_none() && trigger_value != trigger {
                    command = Some(GraphCommand::SetTrigger(node_id, trigger_value));
                }
            }
            RuleGraphNodeKind::Condition(condition) => {
                let mut edited_condition = condition.clone();
                let mut kind = Self::graph_condition_kind(&condition);
                let kind_salt = format!("graph_canvas_condition_kind::{scene_name}::{node_id}");
                egui::ComboBox::from_id_salt(kind_salt)
                    .selected_text(Self::graph_condition_kind_label(kind))
                    .show_ui(ui, |ui| {
                        for candidate in [
                            GraphConditionKind::Always,
                            GraphConditionKind::TargetExists,
                            GraphConditionKind::KeyHeld,
                            GraphConditionKind::EntityActive,
                            GraphConditionKind::HealthBelow,
                            GraphConditionKind::HealthAbove,
                            GraphConditionKind::TriggerOtherIsPlayer,
                            GraphConditionKind::EntityIsKind,
                            GraphConditionKind::TriggerOtherIsKind,
                            GraphConditionKind::EntityHasTag,
                            GraphConditionKind::TriggerOtherHasTag,
                            GraphConditionKind::HasInventoryItem,
                        ] {
                            ui.selectable_value(
                                &mut kind,
                                candidate,
                                Self::graph_condition_kind_label(candidate),
                            );
                        }
                    });
                if kind != Self::graph_condition_kind(&condition) {
                    edited_condition = Self::graph_default_condition(kind);
                }
                let _ = Self::edit_graph_condition_payload(
                    ui,
                    &mut edited_condition,
                    &format!("graph_canvas_condition_payload::{scene_name}::{node_id}"),
                );
                if command.is_none() && edited_condition != condition {
                    command = Some(GraphCommand::SetCondition(node_id, edited_condition));
                }
            }
            RuleGraphNodeKind::Action(action) => {
                let mut edited_action = action.clone();
                let mut kind = Self::graph_action_kind(&action);
                let kind_salt = format!("graph_canvas_action_kind::{scene_name}::{node_id}");
                egui::ComboBox::from_id_salt(kind_salt)
                    .selected_text(Self::graph_action_kind_label(kind))
                    .show_ui(ui, |ui| {
                        for candidate in [
                            GraphActionKind::PlaySound,
                            GraphActionKind::PlayMusic,
                            GraphActionKind::PlayAnimation,
                            GraphActionKind::SetVelocity,
                            GraphActionKind::Spawn,
                            GraphActionKind::DestroySelf,
                            GraphActionKind::SwitchScene,
                            GraphActionKind::DamageEntity,
                            GraphActionKind::HealEntity,
                            GraphActionKind::AddInventoryItem,
                            GraphActionKind::RemoveInventoryItem,
                            GraphActionKind::SetEntityActive,
                            GraphActionKind::TeleportEntity,
                        ] {
                            ui.selectable_value(
                                &mut kind,
                                candidate,
                                Self::graph_action_kind_label(candidate),
                            );
                        }
                    });
                if kind != Self::graph_action_kind(&action) {
                    edited_action = Self::graph_default_action(kind);
                }
                let _ = Self::edit_graph_action_payload(
                    ui,
                    &mut edited_action,
                    &format!("graph_canvas_action_payload::{scene_name}::{node_id}"),
                );
                if command.is_none() && edited_action != action {
                    command = Some(GraphCommand::SetAction(node_id, edited_action));
                }
            }
        }

        command
    }
}
