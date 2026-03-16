use super::*;

impl InspectorSystem {
    pub(super) fn next_rule_id(rule_set: &RuleSet) -> String {
        let mut index = 1usize;
        loop {
            let candidate = format!("rule_{}", index);
            if !rule_set.rules.iter().any(|rule| rule.id == candidate) {
                return candidate;
            }
            index += 1;
        }
    }

    pub(super) fn add_default_rule(rule_set: &mut RuleSet) -> String {
        let id = Self::next_rule_id(rule_set);
        let rule = Rule {
            id: id.clone(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_placeholder".to_string(),
            }],
        };
        rule_set.rules.push(rule);
        id
    }

    pub(super) fn duplicate_rule(rule_set: &mut RuleSet, rule_index: usize) -> Option<usize> {
        let source_rule = rule_set.rules.get(rule_index)?.clone();
        let mut duplicated = source_rule;
        duplicated.id = Self::next_rule_id(rule_set);
        let insert_index = (rule_index + 1).min(rule_set.rules.len());
        rule_set.rules.insert(insert_index, duplicated);
        Some(insert_index)
    }

    pub(super) fn remove_rule(rule_set: &mut RuleSet, rule_index: usize) -> Option<usize> {
        if rule_index >= rule_set.rules.len() {
            return None;
        }

        rule_set.rules.remove(rule_index);
        if rule_set.rules.is_empty() {
            None
        } else if rule_index < rule_set.rules.len() {
            Some(rule_index)
        } else {
            Some(rule_set.rules.len() - 1)
        }
    }

    pub(super) fn move_rule_up(rule_set: &mut RuleSet, rule_index: usize) -> Option<usize> {
        if rule_index >= rule_set.rules.len() {
            return None;
        }
        if rule_index == 0 {
            return Some(0);
        }

        rule_set.rules.swap(rule_index - 1, rule_index);
        Some(rule_index - 1)
    }

    pub(super) fn move_rule_down(rule_set: &mut RuleSet, rule_index: usize) -> Option<usize> {
        if rule_index >= rule_set.rules.len() {
            return None;
        }
        if rule_index + 1 >= rule_set.rules.len() {
            return Some(rule_index);
        }

        rule_set.rules.swap(rule_index, rule_index + 1);
        Some(rule_index + 1)
    }

    pub(super) fn add_action(rule: &mut Rule, action_kind: RuleActionKind) {
        rule.actions.push(Self::default_action(action_kind));
    }

    pub(super) fn add_condition(rule: &mut Rule, condition_kind: RuleConditionKind) {
        rule.conditions
            .push(Self::default_condition(condition_kind));
    }

    pub(super) fn remove_condition(rule: &mut Rule, condition_index: usize) -> bool {
        if condition_index >= rule.conditions.len() {
            return false;
        }
        rule.conditions.remove(condition_index);
        if rule.conditions.is_empty() {
            rule.conditions.push(RuleCondition::Always);
        }
        true
    }

    pub(super) fn switch_condition_kind(
        condition: &mut RuleCondition,
        condition_kind: RuleConditionKind,
    ) {
        *condition = Self::default_condition(condition_kind);
    }

    pub(super) fn remove_action(rule: &mut Rule, action_index: usize) -> bool {
        if action_index >= rule.actions.len() {
            return false;
        }
        rule.actions.remove(action_index);
        true
    }

    pub(super) fn switch_action_kind(action: &mut RuleAction, action_kind: RuleActionKind) {
        *action = Self::default_action(action_kind);
    }

    pub(super) fn validate_rule_set(rule_set: &RuleSet) -> Vec<RuleValidationIssue> {
        let mut issues = Vec::new();

        let mut id_to_indices: HashMap<&str, Vec<usize>> = HashMap::new();
        for (rule_index, rule) in rule_set.rules.iter().enumerate() {
            id_to_indices
                .entry(rule.id.as_str())
                .or_default()
                .push(rule_index);
        }

        for (rule_id, indices) in id_to_indices {
            if indices.len() > 1 {
                for rule_index in indices {
                    issues.push(RuleValidationIssue {
                        rule_index,
                        action_index: None,
                        message: format!("Duplicate rule id '{rule_id}'"),
                    });
                }
            }
        }

        for (rule_index, rule) in rule_set.rules.iter().enumerate() {
            if rule.id.trim().is_empty() {
                issues.push(RuleValidationIssue {
                    rule_index,
                    action_index: None,
                    message: "Rule id must not be empty".to_string(),
                });
            }

            for (condition_index, condition) in rule.conditions.iter().enumerate() {
                match condition {
                    RuleCondition::Always => {}
                    RuleCondition::TargetExists { target }
                    | RuleCondition::EntityActive { target, .. } => {
                        if let RuleTarget::Entity(entity_id) = target {
                            if *entity_id == 0 {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: None,
                                    message: format!(
                                        "Condition {} entity target must be non-zero",
                                        condition_index + 1
                                    ),
                                });
                            }
                        }
                    }
                    RuleCondition::KeyHeld { .. } => {}
                }
            }

            for (action_index, action) in rule.actions.iter().enumerate() {
                match action {
                    RuleAction::PlaySound { sound_id, .. } => {
                        if sound_id.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "PlaySound requires a non-empty sound id".to_string(),
                            });
                        }
                    }
                    RuleAction::PlayMusic { track_id } => {
                        if track_id.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "PlayMusic requires a non-empty track id".to_string(),
                            });
                        }
                    }
                    RuleAction::PlayAnimation { .. } => {}
                    RuleAction::SetVelocity { target, .. } => {
                        if let RuleTarget::Entity(entity_id) = target {
                            if *entity_id == 0 {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: Some(action_index),
                                    message: "SetVelocity entity target must be non-zero"
                                        .to_string(),
                                });
                            }
                        }
                    }
                    RuleAction::Spawn { .. } => {}
                    RuleAction::DestroySelf { target } => {
                        if let RuleTarget::Entity(entity_id) = target {
                            if *entity_id == 0 {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: Some(action_index),
                                    message: "DestroySelf entity target must be non-zero"
                                        .to_string(),
                                });
                            }
                        }
                    }
                    RuleAction::SwitchScene { scene_name } => {
                        if scene_name.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "SwitchScene requires a scene name".to_string(),
                            });
                        }
                    }
                }
            }
        }

        issues
    }

    pub(super) fn default_action(action_kind: RuleActionKind) -> RuleAction {
        match action_kind {
            RuleActionKind::PlaySound => RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_placeholder".to_string(),
            },
            RuleActionKind::PlayMusic => RuleAction::PlayMusic {
                track_id: "music_placeholder".to_string(),
            },
            RuleActionKind::PlayAnimation => RuleAction::PlayAnimation {
                target: RuleTarget::Player,
                state: AnimationState::Idle,
            },
            RuleActionKind::SetVelocity => RuleAction::SetVelocity {
                target: RuleTarget::Player,
                velocity: [0, 0],
            },
            RuleActionKind::Spawn => RuleAction::Spawn {
                entity_type: RuleSpawnEntityType::Npc,
                position: [0, 0],
            },
            RuleActionKind::DestroySelf => RuleAction::DestroySelf {
                target: RuleTarget::Player,
            },
            RuleActionKind::SwitchScene => RuleAction::SwitchScene {
                scene_name: String::new(),
            },
        }
    }

    pub(super) fn default_condition(condition_kind: RuleConditionKind) -> RuleCondition {
        match condition_kind {
            RuleConditionKind::Always => RuleCondition::Always,
            RuleConditionKind::TargetExists => RuleCondition::TargetExists {
                target: RuleTarget::Player,
            },
            RuleConditionKind::KeyHeld => RuleCondition::KeyHeld { key: RuleKey::Up },
            RuleConditionKind::EntityActive => RuleCondition::EntityActive {
                target: RuleTarget::Player,
                is_active: true,
            },
        }
    }

    pub(super) fn condition_kind(condition: &RuleCondition) -> RuleConditionKind {
        match condition {
            RuleCondition::Always => RuleConditionKind::Always,
            RuleCondition::TargetExists { .. } => RuleConditionKind::TargetExists,
            RuleCondition::KeyHeld { .. } => RuleConditionKind::KeyHeld,
            RuleCondition::EntityActive { .. } => RuleConditionKind::EntityActive,
        }
    }

    pub(super) fn condition_kind_label(condition_kind: RuleConditionKind) -> &'static str {
        match condition_kind {
            RuleConditionKind::Always => "Always",
            RuleConditionKind::TargetExists => "TargetExists",
            RuleConditionKind::KeyHeld => "KeyHeld",
            RuleConditionKind::EntityActive => "EntityActive",
        }
    }

    pub(super) fn action_kind(action: &RuleAction) -> RuleActionKind {
        match action {
            RuleAction::PlaySound { .. } => RuleActionKind::PlaySound,
            RuleAction::PlayMusic { .. } => RuleActionKind::PlayMusic,
            RuleAction::PlayAnimation { .. } => RuleActionKind::PlayAnimation,
            RuleAction::SetVelocity { .. } => RuleActionKind::SetVelocity,
            RuleAction::Spawn { .. } => RuleActionKind::Spawn,
            RuleAction::DestroySelf { .. } => RuleActionKind::DestroySelf,
            RuleAction::SwitchScene { .. } => RuleActionKind::SwitchScene,
        }
    }

    pub(super) fn action_kind_label(action_kind: RuleActionKind) -> &'static str {
        match action_kind {
            RuleActionKind::PlaySound => "PlaySound",
            RuleActionKind::PlayMusic => "PlayMusic",
            RuleActionKind::PlayAnimation => "PlayAnimation",
            RuleActionKind::SetVelocity => "SetVelocity",
            RuleActionKind::Spawn => "Spawn",
            RuleActionKind::DestroySelf => "DestroySelf",
            RuleActionKind::SwitchScene => "SwitchScene",
        }
    }

    pub(super) fn spawn_entity_type_label(entity_type: RuleSpawnEntityType) -> &'static str {
        match entity_type {
            RuleSpawnEntityType::PlayerLikeNpc => "PlayerLikeNpc",
            RuleSpawnEntityType::Npc => "Npc",
            RuleSpawnEntityType::Item => "Item",
            RuleSpawnEntityType::Decoration => "Decoration",
            RuleSpawnEntityType::Trigger => "Trigger",
        }
    }

    pub(super) fn trigger_kind(trigger: &RuleTrigger) -> RuleTriggerKind {
        match trigger {
            RuleTrigger::OnStart => RuleTriggerKind::Start,
            RuleTrigger::OnUpdate => RuleTriggerKind::Update,
            RuleTrigger::OnPlayerMove => RuleTriggerKind::PlayerMove,
            RuleTrigger::OnKey { .. } => RuleTriggerKind::Key,
            RuleTrigger::OnCollision => RuleTriggerKind::Collision,
            RuleTrigger::OnDamaged => RuleTriggerKind::Damaged,
            RuleTrigger::OnDeath => RuleTriggerKind::Death,
            RuleTrigger::OnTrigger => RuleTriggerKind::Trigger,
        }
    }

    pub(super) fn trigger_kind_label(kind: RuleTriggerKind) -> &'static str {
        match kind {
            RuleTriggerKind::Start => "OnStart",
            RuleTriggerKind::Update => "OnUpdate",
            RuleTriggerKind::PlayerMove => "OnPlayerMove",
            RuleTriggerKind::Key => "OnKey",
            RuleTriggerKind::Collision => "OnCollision",
            RuleTriggerKind::Damaged => "OnDamaged",
            RuleTriggerKind::Death => "OnDeath",
            RuleTriggerKind::Trigger => "OnTrigger",
        }
    }

    pub(super) fn set_rule_trigger_kind(rule: &mut Rule, kind: RuleTriggerKind) {
        rule.trigger = match kind {
            RuleTriggerKind::Start => RuleTrigger::OnStart,
            RuleTriggerKind::Update => RuleTrigger::OnUpdate,
            RuleTriggerKind::PlayerMove => RuleTrigger::OnPlayerMove,
            RuleTriggerKind::Key => RuleTrigger::OnKey { key: RuleKey::Up },
            RuleTriggerKind::Collision => RuleTrigger::OnCollision,
            RuleTriggerKind::Damaged => RuleTrigger::OnDamaged,
            RuleTriggerKind::Death => RuleTrigger::OnDeath,
            RuleTriggerKind::Trigger => RuleTrigger::OnTrigger,
        };
    }

    pub(super) fn rule_key_label(key: RuleKey) -> &'static str {
        match key {
            RuleKey::Up => "Up",
            RuleKey::Down => "Down",
            RuleKey::Left => "Left",
            RuleKey::Right => "Right",
            RuleKey::DebugToggle => "DebugToggle",
        }
    }

    pub(super) fn load_rule_audio_choices(config: Option<&EditorConfig>) -> RuleAudioChoices {
        let Some(project_path) = config.and_then(|cfg| cfg.current_project_path()) else {
            return RuleAudioChoices::default();
        };

        RuleAudioChoices {
            sfx: Self::discover_audio_asset_names(project_path.join("assets/audio/sfx").as_path()),
            music: Self::discover_audio_asset_names(
                project_path.join("assets/audio/music").as_path(),
            ),
        }
    }

    pub(super) fn render_scene_rules_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        rule_set: &mut RuleSet,
        config: Option<&EditorConfig>,
    ) -> bool {
        let mut changed = false;
        let validation_issues = Self::validate_rule_set(rule_set);
        let audio_choices = Self::load_rule_audio_choices(config);

        ui.label("Visual Rules");
        ui.horizontal(|ui| {
            ui.label("Count:");
            ui.label(rule_set.rules.len().to_string());
        });

        if ui.button("➕ Add Rule").clicked() {
            let rule_id = Self::add_default_rule(rule_set);
            tracing::info!("Added rule '{}' to scene '{}'", rule_id, scene_name);
            changed = true;
        }

        if !validation_issues.is_empty() {
            ui.colored_label(
                egui::Color32::from_rgb(255, 210, 80),
                format!("⚠ {} validation issues", validation_issues.len()),
            );
        }

        if rule_set.rules.is_empty() {
            ui.label("No rules configured");
            return changed;
        }

        let mut pending_command = None;
        for (rule_index, rule) in rule_set.rules.iter_mut().enumerate() {
            let outcome = Self::render_rule_editor(
                ui,
                scene_name,
                rule_index,
                rule,
                &validation_issues,
                &audio_choices,
            );
            changed |= outcome.changed;
            if pending_command.is_none() {
                pending_command = outcome.command;
            }
        }

        if let Some(command) = pending_command {
            match command {
                RuleEditorCommand::Remove(rule_index) => {
                    if Self::remove_rule(rule_set, rule_index).is_some() {
                        changed = true;
                    }
                }
                RuleEditorCommand::Duplicate(rule_index) => {
                    if Self::duplicate_rule(rule_set, rule_index).is_some() {
                        changed = true;
                    }
                }
                RuleEditorCommand::MoveUp(rule_index) => {
                    if let Some(new_index) = Self::move_rule_up(rule_set, rule_index) {
                        changed |= new_index != rule_index;
                    }
                }
                RuleEditorCommand::MoveDown(rule_index) => {
                    if let Some(new_index) = Self::move_rule_down(rule_set, rule_index) {
                        changed |= new_index != rule_index;
                    }
                }
            }
        }

        changed
    }

    pub(super) fn render_selected_rule_graph_node_editor(
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
        let before_layout = ui_state.graph_layouts_by_scene.get(scene_name).cloned();
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
                let changed = Self::render_rule_graph_trigger_editor(
                    ui,
                    scene_name,
                    node_key,
                    &mut edited_trigger,
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
                let mut edited_condition = condition;
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

    pub(super) fn rule_graph_node_badges(graph: &RuleGraph) -> HashMap<u64, String> {
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

    pub(super) fn rule_graph_node_label_for_inspector(
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
                    Self::rule_graph_condition_summary(*condition)
                )
            }
            RuleGraphNodeKind::Action(action) => {
                format!("Action {}", Self::rule_graph_action_summary(action))
            }
        };
        Some(format!("{}: {}", badge, details))
    }

    pub(super) fn rule_graph_trigger_summary(trigger: RuleTrigger) -> String {
        match trigger {
            RuleTrigger::OnStart => "OnStart".to_string(),
            RuleTrigger::OnUpdate => "OnUpdate".to_string(),
            RuleTrigger::OnPlayerMove => "OnPlayerMove".to_string(),
            RuleTrigger::OnKey { key } => format!("OnKey({})", Self::rule_key_label(key)),
            RuleTrigger::OnCollision => "OnCollision".to_string(),
            RuleTrigger::OnDamaged => "OnDamaged".to_string(),
            RuleTrigger::OnDeath => "OnDeath".to_string(),
            RuleTrigger::OnTrigger => "OnTrigger".to_string(),
        }
    }

    pub(super) fn rule_graph_condition_summary(condition: RuleCondition) -> String {
        match condition {
            RuleCondition::Always => "Always".to_string(),
            RuleCondition::TargetExists { target } => {
                format!("TargetExists({})", Self::rule_graph_target_summary(target))
            }
            RuleCondition::KeyHeld { key } => format!("KeyHeld({})", Self::rule_key_label(key)),
            RuleCondition::EntityActive { target, is_active } => format!(
                "EntityActive({}, {})",
                Self::rule_graph_target_summary(target),
                if is_active { "true" } else { "false" }
            ),
        }
    }

    pub(super) fn rule_graph_action_summary(action: &RuleAction) -> String {
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
            RuleAction::SwitchScene { scene_name } => format!(
                "SwitchScene({})",
                if scene_name.is_empty() {
                    "<empty>"
                } else {
                    scene_name
                }
            ),
        }
    }

    fn rule_graph_target_summary(target: RuleTarget) -> String {
        match target {
            RuleTarget::Player => "Player".to_string(),
            RuleTarget::Entity(id) => format!("Entity({})", id),
        }
    }

    pub(super) fn render_rule_graph_trigger_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        trigger: &mut RuleTrigger,
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
                for candidate in [
                    RuleTriggerKind::Start,
                    RuleTriggerKind::Update,
                    RuleTriggerKind::PlayerMove,
                    RuleTriggerKind::Key,
                    RuleTriggerKind::Collision,
                    RuleTriggerKind::Damaged,
                    RuleTriggerKind::Death,
                    RuleTriggerKind::Trigger,
                ] {
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
                RuleTriggerKind::Collision => RuleTrigger::OnCollision,
                RuleTriggerKind::Damaged => RuleTrigger::OnDamaged,
                RuleTriggerKind::Death => RuleTrigger::OnDeath,
                RuleTriggerKind::Trigger => RuleTrigger::OnTrigger,
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

        changed
    }

    pub(super) fn render_rule_graph_condition_editor(
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
                for candidate in [
                    RuleConditionKind::Always,
                    RuleConditionKind::TargetExists,
                    RuleConditionKind::KeyHeld,
                    RuleConditionKind::EntityActive,
                ] {
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
            RuleCondition::Always => {}
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
        }

        changed
    }

    pub(super) fn render_rule_graph_action_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        action: &mut RuleAction,
        _validation_issues: &[RuleValidationIssue],
        audio_choices: &RuleAudioChoices,
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
                for candidate in [
                    RuleActionKind::PlaySound,
                    RuleActionKind::PlayMusic,
                    RuleActionKind::PlayAnimation,
                    RuleActionKind::SetVelocity,
                    RuleActionKind::Spawn,
                    RuleActionKind::DestroySelf,
                    RuleActionKind::SwitchScene,
                ] {
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
            RuleAction::SwitchScene { scene_name } => {
                ui.horizontal(|ui| {
                    ui.label("Scene:");
                    changed |= ui.text_edit_singleline(scene_name).changed();
                });
            }
        }

        changed
    }

    pub(super) fn render_rule_target_editor_with_salt(
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

    pub(super) fn render_rule_key_editor_with_salt(
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
                    ] {
                        changed |= ui
                            .selectable_value(key, candidate, Self::rule_key_label(candidate))
                            .changed();
                    }
                });
        });
        changed
    }

    pub(super) fn render_rule_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        rule_index: usize,
        rule: &mut Rule,
        validation_issues: &[RuleValidationIssue],
        audio_choices: &RuleAudioChoices,
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
                            scene_name,
                            rule_index,
                            action_index,
                            action,
                            validation_issues,
                            audio_choices,
                        );
                    });
                }

                if let Some(index) = remove_action_index {
                    outcome.changed |= Self::remove_action(rule, index);
                }

                ui.horizontal(|ui| {
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
                });
            });

        outcome
    }

    pub(super) fn render_rule_action_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        rule_index: usize,
        action_index: usize,
        action: &mut RuleAction,
        validation_issues: &[RuleValidationIssue],
        audio_choices: &RuleAudioChoices,
    ) -> bool {
        let mut changed = false;

        let current_kind = Self::action_kind(action);
        let mut selected_kind = current_kind;
        ui.horizontal(|ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_salt(format!(
                "rule_action_kind_{}_{}_{}",
                scene_name, rule_index, action_index
            ))
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
            });
        });
        if selected_kind != current_kind {
            Self::switch_action_kind(action, selected_kind);
        }

        match action {
            RuleAction::PlaySound { channel, sound_id } => {
                ui.horizontal(|ui| {
                    ui.label("Channel:");
                    egui::ComboBox::from_id_salt(format!(
                        "rule_sound_channel_{}_{}_{}",
                        scene_name, rule_index, action_index
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
                    format!(
                        "rule_sfx_picker_{}_{}_{}",
                        scene_name, rule_index, action_index
                    ),
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
                    format!(
                        "rule_music_picker_{}_{}_{}",
                        scene_name, rule_index, action_index
                    ),
                    "Music",
                    track_id,
                    &audio_choices.music,
                );
            }
            RuleAction::PlayAnimation { target, state } => {
                changed |= Self::render_rule_target_editor(
                    ui,
                    scene_name,
                    rule_index,
                    action_index,
                    target,
                );

                ui.horizontal(|ui| {
                    ui.label("State:");
                    egui::ComboBox::from_id_salt(format!(
                        "rule_animation_state_{}_{}_{}",
                        scene_name, rule_index, action_index
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
                changed |= Self::render_rule_target_editor(
                    ui,
                    scene_name,
                    rule_index,
                    action_index,
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
                        "rule_spawn_type_{}_{}_{}",
                        scene_name, rule_index, action_index
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
                changed |= Self::render_rule_target_editor(
                    ui,
                    scene_name,
                    rule_index,
                    action_index,
                    target,
                );
            }
            RuleAction::SwitchScene { scene_name } => {
                ui.horizontal(|ui| {
                    ui.label("Scene:");
                    changed |= ui.text_edit_singleline(scene_name).changed();
                });
            }
        }

        for issue in validation_issues.iter().filter(|issue| {
            issue.rule_index == rule_index && issue.action_index == Some(action_index)
        }) {
            ui.colored_label(egui::Color32::from_rgb(255, 210, 80), &issue.message);
        }

        changed
    }

    pub(super) fn render_rule_condition_editor(
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
            RuleCondition::Always => {}
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
        }

        changed
    }

    pub(super) fn render_audio_choice_picker(
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

    pub(super) fn render_rule_target_editor(
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

    pub(super) fn render_rule_condition_target_editor(
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
