use super::*;

impl InspectorSystem {
    pub(in super::super) fn render_selected_rule_graph_node_editor(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
        scene_name: &str,
        node_key: &str,
        config: Option<&EditorConfig>,
    ) -> bool {
        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|scene| scene.name == scene_name)
        else {
            ui.label("Scene not found.");
            return false;
        };
        let scene_rules = ui_state.scenes[scene_index].rules.clone();
        let before_rules = scene_rules.clone();
        let before_graph = ui_state.rule_graph_for_scene(scene_name).cloned();
        let before_layout = ui_state.graph.layouts_by_scene.get(scene_name).cloned();
        ui_state.sync_rule_graph_with_rule_set(scene_name, &scene_rules);

        let audio_choices = Self::load_rule_audio_choices(config);
        let validation_issues = Self::validate_rule_set(&scene_rules);
        let mut graph = ui_state
            .rule_graph_for_scene(scene_name)
            .cloned()
            .unwrap_or_else(|| RuleGraph::from_rule_set(&scene_rules));
        let node_badges = Self::rule_graph_node_badges(&graph);
        let Some(node_id) = graph.node_id_for_stable_key(node_key) else {
            ui.colored_label(
                egui::Color32::from_rgb(255, 210, 80),
                "Selected node no longer exists in this scene.",
            );
            return false;
        };
        let Some(node_kind) = graph
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .map(|node| node.kind.clone())
        else {
            ui.colored_label(
                egui::Color32::from_rgb(255, 120, 120),
                "Failed to resolve selected node.",
            );
            return false;
        };

        let mut graph_mutated = false;
        let mut operation_error = None::<String>;
        match node_kind {
            RuleGraphNodeKind::Trigger(trigger) => {
                ui.label("Trigger");
                let mut edited_trigger = trigger;
                // Extract map size for validation if available
                let map_size = ui_state.scenes.get(scene_index)
                    .and_then(|scene| scene.maps.first())
                    .and_then(|map_name| {
                        // Try to get map size from loaded map draft or pending sync
                        if ui_state.map.active_map.as_ref() == Some(map_name) {
                            ui_state.map.draft.as_ref()
                                .map(|draft| (draft.tilemap.size.x, draft.tilemap.size.y))
                                .or_else(|| ui_state.map.pending_tilemap_sync.as_ref()
                                    .map(|tm| (tm.size.x, tm.size.y)))
                        } else {
                            None
                        }
                    });
                let changed = Self::render_rule_graph_trigger_editor(
                    ui,
                    scene_name,
                    node_key,
                    &mut edited_trigger,
                    map_size,
                );
                if changed && edited_trigger != trigger {
                    if let Err(error) = graph.set_trigger_for_chain(node_id, edited_trigger) {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 120, 120),
                            format!("Failed to update trigger: {:?}", error),
                        );
                        return false;
                    }
                    graph_mutated = true;
                }
            }
            RuleGraphNodeKind::Condition(condition) => {
                ui.label("Condition");
                let mut edited_condition = condition.clone();
                let changed = Self::render_rule_graph_condition_editor(
                    ui,
                    scene_name,
                    node_key,
                    &mut edited_condition,
                );
                if changed && edited_condition != condition {
                    if let Err(error) = graph.set_condition_for_node(node_id, edited_condition) {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 120, 120),
                            format!("Failed to update condition: {:?}", error),
                        );
                        return false;
                    }
                    graph_mutated = true;
                }
            }
            RuleGraphNodeKind::Action(action) => {
                ui.label("Action");
                let mut edited_action = action.clone();
                let changed = Self::render_rule_graph_action_editor(
                    ui,
                    scene_name,
                    node_key,
                    &mut edited_action,
                    &validation_issues,
                    &audio_choices,
                    &ui_state.scenes,
                );
                if changed && edited_action != action {
                    if let Err(error) = graph.set_action_for_node(node_id, edited_action) {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 120, 120),
                            format!("Failed to update action: {:?}", error),
                        );
                        return false;
                    }
                    graph_mutated = true;
                }
            }
        }

        ui.separator();
        let mut outgoing_connected_ids = graph
            .edges
            .iter()
            .filter(|edge| edge.from == node_id)
            .map(|edge| edge.to)
            .collect::<Vec<_>>();
        outgoing_connected_ids.sort_unstable();
        outgoing_connected_ids.dedup();
        let mut incoming_connected_ids = graph
            .edges
            .iter()
            .filter(|edge| edge.to == node_id)
            .map(|edge| edge.from)
            .collect::<Vec<_>>();
        incoming_connected_ids.sort_unstable();
        incoming_connected_ids.dedup();

        let connectable_to_nodes = graph
            .nodes
            .iter()
            .filter_map(|node| {
                (node.id != node_id
                    && !outgoing_connected_ids.contains(&node.id)
                    && graph.can_connect_nodes(node_id, node.id).is_ok())
                .then_some((
                    node.id,
                    Self::rule_graph_node_label_for_inspector(&graph, &node_badges, node.id),
                ))
            })
            .filter_map(|(id, label)| label.map(|label| (id, label)))
            .collect::<Vec<_>>();
        let connectable_from_nodes = graph
            .nodes
            .iter()
            .filter_map(|node| {
                (node.id != node_id
                    && !incoming_connected_ids.contains(&node.id)
                    && graph.can_connect_nodes(node.id, node_id).is_ok())
                .then_some((
                    node.id,
                    Self::rule_graph_node_label_for_inspector(&graph, &node_badges, node.id),
                ))
            })
            .filter_map(|(id, label)| label.map(|label| (id, label)))
            .collect::<Vec<_>>();

        let mut pending_connect_to = None::<u64>;
        let mut pending_connect_from = None::<u64>;
        let mut pending_disconnect_edge = None::<(u64, u64)>;
        ui.push_id(("graph_node_action_buttons", scene_name, node_id), |ui| {
            egui::Grid::new("graph_node_action_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    if ui.button("Disconnect Node").clicked() {
                        if let Err(error) = graph.disconnect_node(node_id) {
                            operation_error =
                                Some(format!("Failed to disconnect node: {:?}", error));
                        } else {
                            graph_mutated = true;
                        }
                    }
                    if ui
                        .add(
                            egui::Button::new("Delete Node")
                                .fill(egui::Color32::from_rgb(120, 30, 30)),
                        )
                        .clicked()
                    {
                        if let Err(error) = graph.remove_node(node_id) {
                            operation_error = Some(format!("Failed to delete node: {:?}", error));
                        } else {
                            graph_mutated = true;
                        }
                    }
                    ui.end_row();

                    ui.menu_button("Connect From", |ui| {
                        if connectable_from_nodes.is_empty() {
                            ui.label("No available nodes");
                            return;
                        }
                        for (candidate_id, label) in &connectable_from_nodes {
                            if ui.button(label).clicked() {
                                pending_connect_from = Some(*candidate_id);
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button("Connect To", |ui| {
                        if connectable_to_nodes.is_empty() {
                            ui.label("No available nodes");
                            return;
                        }
                        for (candidate_id, label) in &connectable_to_nodes {
                            if ui.button(label).clicked() {
                                pending_connect_to = Some(*candidate_id);
                                ui.close();
                            }
                        }
                    });
                    ui.end_row();
                });
        });
        ui.separator();
        let outgoing_edges = graph
            .edges
            .iter()
            .filter(|edge| edge.from == node_id)
            .copied()
            .collect::<Vec<_>>();
        let incoming_edges = graph
            .edges
            .iter()
            .filter(|edge| edge.to == node_id)
            .copied()
            .collect::<Vec<_>>();
        ui.label("Connections");
        if outgoing_edges.is_empty() && incoming_edges.is_empty() {
            ui.label("None");
        } else {
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    if !outgoing_edges.is_empty() {
                        ui.label("Outgoing");
                        for edge in &outgoing_edges {
                            let label = Self::rule_graph_node_label_for_inspector(
                                &graph,
                                &node_badges,
                                edge.to,
                            )
                            .unwrap_or_else(|| format!("node {}", edge.to));
                            ui.horizontal(|ui| {
                                ui.label(format!("-> {}", label));
                                if ui.small_button("Disconnect").clicked() {
                                    pending_disconnect_edge = Some((edge.from, edge.to));
                                }
                            });
                        }
                    }
                    if !incoming_edges.is_empty() {
                        ui.label("Incoming");
                        for edge in &incoming_edges {
                            let label = Self::rule_graph_node_label_for_inspector(
                                &graph,
                                &node_badges,
                                edge.from,
                            )
                            .unwrap_or_else(|| format!("node {}", edge.from));
                            ui.horizontal(|ui| {
                                ui.label(format!("<- {}", label));
                                if ui.small_button("Disconnect").clicked() {
                                    pending_disconnect_edge = Some((edge.from, edge.to));
                                }
                            });
                        }
                    }
                });
        }
        if let Some((from, to)) = pending_disconnect_edge {
            if graph.disconnect_nodes(from, to) {
                graph_mutated = true;
            } else {
                operation_error = Some("Failed to disconnect selected connection".to_string());
            }
        }
        if let Some(connect_from) = pending_connect_from {
            if let Err(error) = graph.connect_nodes(connect_from, node_id) {
                operation_error = Some(format!("Failed to connect nodes: {:?}", error));
            } else {
                graph_mutated = true;
            }
        }
        if let Some(connect_to) = pending_connect_to {
            if let Err(error) = graph.connect_nodes(node_id, connect_to) {
                operation_error = Some(format!("Failed to connect nodes: {:?}", error));
            } else {
                graph_mutated = true;
            }
        }
        if let Some(message) = operation_error {
            ui.colored_label(egui::Color32::from_rgb(255, 120, 120), message);
        }

        if !graph_mutated {
            return false;
        }

        match graph.to_rule_set() {
            Ok(updated_rules) => {
                let (zoom, pan) = ui_state.graph_view_for_scene(scene_name);
                ui_state.execute_scene_rules_graph_command(
                    scene_name,
                    SceneRulesGraphCommandData {
                        before_rule_set: before_rules,
                        after_rule_set: updated_rules,
                        before_graph,
                        after_graph: graph,
                        before_layout,
                        zoom,
                        pan,
                    },
                )
            }
            Err(error) => {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 120, 120),
                    format!("Failed to rebuild rule set from graph: {:?}", error),
                );
                false
            }
        }
    }

    pub(in super::super) fn rule_graph_node_badges(graph: &RuleGraph) -> HashMap<u64, String> {
        let mut node_ids = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        node_ids.sort_unstable();

        let mut trigger_index = 0usize;
        let mut condition_index = 0usize;
        let mut action_index = 0usize;
        let mut badges = HashMap::new();
        for node_id in node_ids {
            let Some(node) = graph.nodes.iter().find(|candidate| candidate.id == node_id) else {
                continue;
            };
            let badge = match node.kind {
                RuleGraphNodeKind::Trigger(_) => {
                    trigger_index += 1;
                    format!("T{}", trigger_index)
                }
                RuleGraphNodeKind::Condition(_) => {
                    condition_index += 1;
                    format!("C{}", condition_index)
                }
                RuleGraphNodeKind::Action(_) => {
                    action_index += 1;
                    format!("A{}", action_index)
                }
            };
            badges.insert(node_id, badge);
        }
        badges
    }

    pub(in super::super) fn rule_graph_node_label_for_inspector(
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        node_id: u64,
    ) -> Option<String> {
        let node = graph.nodes.iter().find(|node| node.id == node_id)?;
        let badge = node_badges
            .get(&node_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        let details = match &node.kind {
            RuleGraphNodeKind::Trigger(trigger) => {
                format!("Trigger {}", Self::rule_graph_trigger_summary(*trigger))
            }
            RuleGraphNodeKind::Condition(condition) => {
                format!(
                    "Condition {}",
                    Self::rule_graph_condition_summary(condition)
                )
            }
            RuleGraphNodeKind::Action(action) => {
                format!("Action {}", Self::rule_graph_action_summary(action))
            }
        };
        Some(format!("{}: {}", badge, details))
    }

    pub(in super::super) fn rule_graph_trigger_summary(trigger: RuleTrigger) -> String {
        match trigger {
            RuleTrigger::OnStart => "OnStart".to_string(),
            RuleTrigger::OnUpdate => "OnUpdate".to_string(),
            RuleTrigger::OnPlayerMove => "OnPlayerMove".to_string(),
            RuleTrigger::OnKey { key } => format!("OnKey({})", Self::rule_key_label(key)),
            RuleTrigger::OnCollision { entity: None } => "OnCollision".to_string(),
            RuleTrigger::OnCollision { entity: Some(target) } => {
                format!("OnCollision({})", Self::rule_graph_target_summary(target))
            }
            RuleTrigger::OnDamaged { entity: None } => "OnDamaged".to_string(),
            RuleTrigger::OnDamaged { entity: Some(target) } => {
                format!("OnDamaged({})", Self::rule_graph_target_summary(target))
            }
            RuleTrigger::OnDeath { entity: None } => "OnDeath".to_string(),
            RuleTrigger::OnDeath { entity: Some(target) } => {
                format!("OnDeath({})", Self::rule_graph_target_summary(target))
            }
            RuleTrigger::OnTrigger => "OnTrigger".to_string(),
            RuleTrigger::OnInteract { .. } => "OnInteract".to_string(),
            RuleTrigger::OnTileEnter { x, y } => format!("OnTileEnter({}, {})", x, y),
            RuleTrigger::OnTileExit { x, y } => format!("OnTileExit({}, {})", x, y),
        }
    }

    pub(in super::super) fn rule_graph_condition_summary(condition: &RuleCondition) -> String {
        match condition {
            RuleCondition::Always => "Always".to_string(),
            RuleCondition::TargetExists { target } => {
                format!("TargetExists({})", Self::rule_graph_target_summary(*target))
            }
            RuleCondition::KeyHeld { key } => format!("KeyHeld({})", Self::rule_key_label(*key)),
            RuleCondition::EntityActive { target, is_active } => format!(
                "EntityActive({}, {})",
                Self::rule_graph_target_summary(*target),
                if *is_active { "true" } else { "false" }
            ),
            RuleCondition::HealthBelow { target, threshold } => format!(
                "HealthBelow({}, {})",
                Self::rule_graph_target_summary(*target),
                threshold
            ),
            RuleCondition::HealthAbove { target, threshold } => format!(
                "HealthAbove({}, {})",
                Self::rule_graph_target_summary(*target),
                threshold
            ),
            RuleCondition::TriggerOtherIsPlayer => "TriggerOtherIsPlayer".to_string(),
            RuleCondition::EntityIsKind { target, kind } => format!(
                "EntityIsKind({}, {:?})",
                Self::rule_graph_target_summary(*target),
                kind
            ),
            RuleCondition::TriggerOtherIsKind { kind } => format!("TriggerOtherIsKind({:?})", kind),
            RuleCondition::EntityHasTag { target, tag } => format!(
                "EntityHasTag({}, {})",
                Self::rule_graph_target_summary(*target),
                tag
            ),
            RuleCondition::TriggerOtherHasTag { tag } => format!("TriggerOtherHasTag({})", tag),
            RuleCondition::HasInventoryItem {
                target,
                item_id,
                min_count,
            } => format!(
                "HasInventoryItem({}, {}, {})",
                Self::rule_graph_target_summary(*target),
                item_id,
                min_count
            ),
        }
    }

    pub(in super::super) fn rule_graph_action_summary(action: &RuleAction) -> String {
        match action {
            RuleAction::PlaySound { channel, sound_id } => format!(
                "PlaySound({:?}, {})",
                channel,
                if sound_id.is_empty() {
                    "<empty>"
                } else {
                    sound_id
                }
            ),
            RuleAction::PlayMusic { track_id } => format!(
                "PlayMusic({})",
                if track_id.is_empty() {
                    "<empty>"
                } else {
                    track_id
                }
            ),
            RuleAction::PlayAnimation { target, state } => {
                format!(
                    "PlayAnimation({}, {:?})",
                    Self::rule_graph_target_summary(*target),
                    state
                )
            }
            RuleAction::SetVelocity { target, velocity } => format!(
                "SetVelocity({}, [{}, {}])",
                Self::rule_graph_target_summary(*target),
                velocity[0],
                velocity[1]
            ),
            RuleAction::Spawn {
                entity_type,
                position,
            } => format!(
                "Spawn({:?}, [{}, {}])",
                entity_type, position[0], position[1]
            ),
            RuleAction::DestroySelf { target } => {
                format!("DestroySelf({})", Self::rule_graph_target_summary(*target))
            }
            RuleAction::SwitchScene {
                scene_name,
                spawn_point_id,
            } => {
                let scene = if scene_name.is_empty() {
                    "<empty>"
                } else {
                    scene_name
                };
                let spawn = if spawn_point_id.is_empty() {
                    "<empty>"
                } else {
                    spawn_point_id
                };
                format!("SwitchScene({scene} -> {spawn})")
            }
            RuleAction::DamageEntity { target, amount } => {
                format!(
                    "DamageEntity({}, {})",
                    Self::rule_graph_target_summary(*target),
                    amount
                )
            }
            RuleAction::HealEntity { target, amount } => {
                format!(
                    "HealEntity({}, {})",
                    Self::rule_graph_target_summary(*target),
                    amount
                )
            }
            RuleAction::AddInventoryItem {
                target,
                item_id,
                count,
            } => {
                let item = if item_id.is_empty() { "<empty>" } else { item_id };
                format!(
                    "AddItem({}, {}, {})",
                    Self::rule_graph_target_summary(*target),
                    item,
                    count
                )
            }
            RuleAction::RemoveInventoryItem {
                target,
                item_id,
                count,
            } => {
                let item = if item_id.is_empty() { "<empty>" } else { item_id };
                format!(
                    "RemoveItem({}, {}, {})",
                    Self::rule_graph_target_summary(*target),
                    item,
                    count
                )
            }
            RuleAction::SetEntityActive { target, active } => {
                format!(
                    "SetActive({}, {})",
                    Self::rule_graph_target_summary(*target),
                    active
                )
            }
            RuleAction::TeleportEntity {
                target,
                tile_x,
                tile_y,
            } => {
                format!(
                    "Teleport({}, tile[{}, {}])",
                    Self::rule_graph_target_summary(*target),
                    tile_x,
                    tile_y
                )
            }
        }
    }

    fn rule_graph_target_summary(target: RuleTarget) -> String {
        match target {
            RuleTarget::Player => "Player".to_string(),
            RuleTarget::Entity(id) => format!("Entity({})", id),
            RuleTarget::RuleOwner => "RuleOwner".to_string(),
            RuleTarget::TriggerSelf => "TriggerSelf".to_string(),
            RuleTarget::TriggerOther => "TriggerOther".to_string(),
        }
    }

    pub(in super::super) fn render_rule_graph_trigger_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        trigger: &mut RuleTrigger,
        map_size: Option<(u32, u32)>,
    ) -> bool {
        let mut changed = false;
        let mut trigger_kind = Self::trigger_kind(trigger);
        ui.horizontal(|ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_salt(format!(
                "graph_node_trigger_kind_{}_{}",
                scene_name, node_key
            ))
            .selected_text(Self::trigger_kind_label(trigger_kind))
            .show_ui(ui, |ui| {
                for candidate in RuleTriggerKind::iter() {
                    changed |= ui
                        .selectable_value(
                            &mut trigger_kind,
                            candidate,
                            Self::trigger_kind_label(candidate),
                        )
                        .changed();
                }
            });
        });

        if trigger_kind != Self::trigger_kind(trigger) {
            *trigger = match trigger_kind {
                RuleTriggerKind::Start => RuleTrigger::OnStart,
                RuleTriggerKind::Update => RuleTrigger::OnUpdate,
                RuleTriggerKind::PlayerMove => RuleTrigger::OnPlayerMove,
                RuleTriggerKind::Key => RuleTrigger::OnKey { key: RuleKey::Up },
                RuleTriggerKind::Collision => RuleTrigger::OnCollision { entity: None },
                RuleTriggerKind::Damaged => RuleTrigger::OnDamaged { entity: None },
                RuleTriggerKind::Death => RuleTrigger::OnDeath { entity: None },
                RuleTriggerKind::Trigger => RuleTrigger::OnTrigger,
                RuleTriggerKind::Interact => RuleTrigger::OnInteract {
                    mode: toki_core::rules::InteractionMode::default(),
                    entity: None,
                },
                RuleTriggerKind::TileEnter => RuleTrigger::OnTileEnter { x: 0, y: 0 },
                RuleTriggerKind::TileExit => RuleTrigger::OnTileExit { x: 0, y: 0 },
            };
            changed = true;
        }

        if let RuleTrigger::OnKey { key } = trigger {
            changed |= Self::render_rule_key_editor_with_salt(
                ui,
                &format!("graph_node_trigger_key_{}_{}", scene_name, node_key),
                key,
            );
        }

        if let RuleTrigger::OnInteract { mode, .. } = trigger {
            changed |= Self::render_rule_interaction_mode_editor_with_salt(
                ui,
                &format!("graph_node_trigger_interact_mode_{}_{}", scene_name, node_key),
                mode,
            );
        }

        // Entity filter editors for triggers that support them
        if let RuleTrigger::OnCollision { entity } = trigger {
            changed |= Self::render_optional_entity_filter_editor(
                ui,
                &format!("graph_node_trigger_collision_entity_{}_{}", scene_name, node_key),
                entity,
            );
        }
        if let RuleTrigger::OnDamaged { entity } = trigger {
            changed |= Self::render_optional_entity_filter_editor(
                ui,
                &format!("graph_node_trigger_damaged_entity_{}_{}", scene_name, node_key),
                entity,
            );
        }
        if let RuleTrigger::OnDeath { entity } = trigger {
            changed |= Self::render_optional_entity_filter_editor(
                ui,
                &format!("graph_node_trigger_death_entity_{}_{}", scene_name, node_key),
                entity,
            );
        }
        if let RuleTrigger::OnInteract { entity, .. } = trigger {
            changed |= Self::render_optional_entity_filter_editor(
                ui,
                &format!("graph_node_trigger_interact_entity_{}_{}", scene_name, node_key),
                entity,
            );
        }

        // Tile coordinate editors for OnTileEnter and OnTileExit
        if let RuleTrigger::OnTileEnter { x, y } | RuleTrigger::OnTileExit { x, y } = trigger {
            ui.horizontal(|ui| {
                ui.label("Tile X:");
                let mut x_val = *x as i32;
                if ui
                    .add(egui::DragValue::new(&mut x_val).speed(1.0).range(0..=9999))
                    .changed()
                {
                    *x = x_val.max(0) as u32;
                    changed = true;
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
                if ui
                    .add(egui::DragValue::new(&mut y_val).speed(1.0).range(0..=9999))
                    .changed()
                {
                    *y = y_val.max(0) as u32;
                    changed = true;
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

        changed
    }

    pub(in super::super) fn render_rule_graph_condition_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        condition: &mut RuleCondition,
    ) -> bool {
        let mut changed = false;

        let current_kind = Self::condition_kind(condition);
        let mut selected_kind = current_kind;
        ui.horizontal(|ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_salt(format!(
                "graph_node_condition_kind_{}_{}",
                scene_name, node_key
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
            changed = true;
        }

        match condition {
            RuleCondition::Always | RuleCondition::TriggerOtherIsPlayer => {}
            RuleCondition::TargetExists { target } => {
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!("graph_node_condition_target_{}_{}", scene_name, node_key),
                    target,
                );
            }
            RuleCondition::KeyHeld { key } => {
                changed |= Self::render_rule_key_editor_with_salt(
                    ui,
                    &format!("graph_node_condition_key_{}_{}", scene_name, node_key),
                    key,
                );
            }
            RuleCondition::EntityActive { target, is_active } => {
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!(
                        "graph_node_condition_entity_target_{}_{}",
                        scene_name, node_key
                    ),
                    target,
                );
                changed |= ui.checkbox(is_active, "Target Is Active").changed();
            }
            RuleCondition::HealthBelow { target, threshold }
            | RuleCondition::HealthAbove { target, threshold } => {
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!("graph_node_condition_health_target_{}_{}", scene_name, node_key),
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
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!("graph_node_condition_entity_kind_target_{}_{}", scene_name, node_key),
                    target,
                );
                changed |= Self::render_entity_kind_editor(
                    ui,
                    &format!("graph_node_condition_entity_kind_{}_{}", scene_name, node_key),
                    kind,
                );
            }
            RuleCondition::TriggerOtherIsKind { kind } => {
                changed |= Self::render_entity_kind_editor(
                    ui,
                    &format!("graph_node_condition_other_kind_{}_{}", scene_name, node_key),
                    kind,
                );
            }
            RuleCondition::EntityHasTag { target, tag } => {
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!("graph_node_condition_tag_target_{}_{}", scene_name, node_key),
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
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!("graph_node_condition_inv_target_{}_{}", scene_name, node_key),
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

        match action {
            RuleAction::PlaySound { channel, sound_id } => {
                ui.horizontal(|ui| {
                    ui.label("Channel:");
                    egui::ComboBox::from_id_salt(format!(
                        "graph_node_sound_channel_{}_{}",
                        scene_name, node_key
                    ))
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
                    format!("graph_node_sfx_picker_{}_{}", scene_name, node_key),
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
                    format!("graph_node_music_picker_{}_{}", scene_name, node_key),
                    "Music",
                    track_id,
                    &audio_choices.music,
                );
            }
            RuleAction::PlayAnimation { target, state } => {
                changed |= Self::render_rule_target_editor_with_salt(
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
                changed |= Self::render_rule_target_editor_with_salt(
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
            }
            RuleAction::Spawn {
                entity_type,
                position,
            } => {
                ui.horizontal(|ui| {
                    ui.label("Entity Type:");
                    egui::ComboBox::from_id_salt(format!(
                        "graph_node_spawn_type_{}_{}",
                        scene_name, node_key
                    ))
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
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!("graph_node_destroy_target_{}_{}", scene_name, node_key),
                    target,
                );
            }
            RuleAction::SwitchScene {
                scene_name,
                spawn_point_id,
            } => {
                changed |= Self::render_switch_scene_editor(
                    ui,
                    format!("graph_switch_scene_{}_{}", scene_name, node_key),
                    scene_name,
                    spawn_point_id,
                    scenes,
                );
            }
            RuleAction::DamageEntity { target, amount } => {
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!("graph_node_damage_target_{}_{}", scene_name, node_key),
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
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!("graph_node_heal_target_{}_{}", scene_name, node_key),
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
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!("graph_node_add_inv_target_{}_{}", scene_name, node_key),
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
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!("graph_node_rem_inv_target_{}_{}", scene_name, node_key),
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
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!("graph_node_set_active_target_{}_{}", scene_name, node_key),
                    target,
                );
                ui.horizontal(|ui| {
                    changed |= ui.checkbox(active, "Active").changed();
                });
            }
            RuleAction::TeleportEntity {
                target,
                tile_x,
                tile_y,
            } => {
                changed |= Self::render_rule_target_editor_with_salt(
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
            }
        }

        changed
    }

    pub(in super::super) fn render_rule_target_editor_with_salt(
        ui: &mut egui::Ui,
        id_salt: &str,
        target: &mut RuleTarget,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Target:");
            egui::ComboBox::from_id_salt((id_salt, "kind"))
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

    pub(in super::super) fn render_rule_key_editor_with_salt(
        ui: &mut egui::Ui,
        id_salt: &str,
        key: &mut RuleKey,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Key:");
            egui::ComboBox::from_id_salt(id_salt)
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
        changed
    }

    fn interaction_mode_label(mode: toki_core::rules::InteractionMode) -> &'static str {
        use toki_core::rules::InteractionMode;
        match mode {
            InteractionMode::Overlap => "Overlap (Same Tile)",
            InteractionMode::Adjacent => "Adjacent (Within Reach)",
            InteractionMode::InFront => "In Front",
        }
    }

    pub(in super::super) fn render_rule_interaction_mode_editor_with_salt(
        ui: &mut egui::Ui,
        id_salt: &str,
        mode: &mut toki_core::rules::InteractionMode,
    ) -> bool {
        use toki_core::rules::InteractionMode;

        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Mode:");
            egui::ComboBox::from_id_salt(id_salt)
                .selected_text(Self::interaction_mode_label(*mode))
                .show_ui(ui, |ui| {
                    for candidate in [
                        InteractionMode::Overlap,
                        InteractionMode::Adjacent,
                        InteractionMode::InFront,
                    ] {
                        changed |= ui
                            .selectable_value(
                                mode,
                                candidate,
                                Self::interaction_mode_label(candidate),
                            )
                            .changed();
                    }
                });
        });
        changed
    }

    fn entity_kind_label(kind: toki_core::entity::EntityKind) -> &'static str {
        use toki_core::entity::EntityKind;
        match kind {
            EntityKind::Player => "Player",
            EntityKind::Npc => "NPC",
            EntityKind::Item => "Item",
            EntityKind::Decoration => "Decoration",
            EntityKind::Trigger => "Trigger",
            EntityKind::Projectile => "Projectile",
        }
    }

    pub(in super::super) fn render_entity_kind_editor(
        ui: &mut egui::Ui,
        id_salt: &str,
        kind: &mut toki_core::entity::EntityKind,
    ) -> bool {
        use toki_core::entity::EntityKind;

        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Entity Kind:");
            egui::ComboBox::from_id_salt(id_salt)
                .selected_text(Self::entity_kind_label(*kind))
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
                            .selectable_value(kind, candidate, Self::entity_kind_label(candidate))
                            .changed();
                    }
                });
        });
        changed
    }

    /// Renders an optional entity filter editor for triggers like OnDamaged, OnDeath, OnCollision.
    ///
    /// When `None`, the trigger fires for all events. When `Some(target)`, it only fires
    /// when the resolved target matches the event entity.
    pub(in super::super) fn render_optional_entity_filter_editor(
        ui: &mut egui::Ui,
        id_salt: &str,
        entity: &mut Option<RuleTarget>,
    ) -> bool {
        let mut changed = false;
        let is_filtered = entity.is_some();

        ui.horizontal(|ui| {
            ui.label("Entity Filter:");
            let filter_label = if is_filtered { "Specific Entity" } else { "All Entities" };
            egui::ComboBox::from_id_salt((id_salt, "filter_toggle"))
                .selected_text(filter_label)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(!is_filtered, "All Entities")
                        .clicked()
                        && is_filtered
                    {
                        *entity = None;
                        changed = true;
                    }
                    if ui
                        .selectable_label(is_filtered, "Specific Entity")
                        .clicked()
                        && !is_filtered
                    {
                        *entity = Some(RuleTarget::Player);
                        changed = true;
                    }
                });
        });

        if let Some(target) = entity {
            changed |= Self::render_rule_target_editor_with_salt(ui, &format!("{}_target", id_salt), target);
        }

        changed
    }
}
