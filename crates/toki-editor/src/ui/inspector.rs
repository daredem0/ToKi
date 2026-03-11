use super::editor_ui::{EditorUI, Selection};
use crate::config::EditorConfig;
use std::collections::HashMap;
use toki_core::animation::AnimationState;
use toki_core::rules::{
    Rule, RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel, RuleTarget, RuleTrigger,
};

/// Handles inspector panel rendering for assets and entities
pub struct InspectorSystem;

#[derive(Debug, Clone)]
struct EntityPropertyDraft {
    position_x: i32,
    position_y: i32,
    size_x: i64,
    size_y: i64,
    visible: bool,
    active: bool,
    solid: bool,
    can_move: bool,
    has_inventory: bool,
    speed: i64,
    render_layer: i32,
    health_enabled: bool,
    health_value: i64,
    collision_enabled: bool,
    collision_offset_x: i32,
    collision_offset_y: i32,
    collision_size_x: i64,
    collision_size_y: i64,
    collision_trigger: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleActionKind {
    PlaySound,
    PlayMusic,
    PlayAnimation,
    SetVelocity,
    SwitchScene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleTriggerKind {
    Start,
    Update,
    Key,
    Collision,
    Trigger,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuleValidationIssue {
    rule_index: usize,
    action_index: Option<usize>,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleEditorCommand {
    Remove(usize),
    Duplicate(usize),
    MoveUp(usize),
    MoveDown(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct RuleEditorOutcome {
    changed: bool,
    command: Option<RuleEditorCommand>,
}

#[derive(Debug, Default, Clone)]
struct RuleAudioChoices {
    sfx: Vec<String>,
    music: Vec<String>,
}

impl EntityPropertyDraft {
    fn from_entity(entity: &toki_core::entity::Entity) -> Self {
        let (
            collision_enabled,
            collision_offset_x,
            collision_offset_y,
            collision_size_x,
            collision_size_y,
            collision_trigger,
        ) = if let Some(collision_box) = &entity.collision_box {
            (
                true,
                collision_box.offset.x,
                collision_box.offset.y,
                collision_box.size.x as i64,
                collision_box.size.y as i64,
                collision_box.trigger,
            )
        } else {
            (
                false,
                0,
                0,
                entity.size.x as i64,
                entity.size.y as i64,
                false,
            )
        };

        let (health_enabled, health_value) = match entity.attributes.health {
            Some(value) => (true, value as i64),
            None => (false, 0),
        };

        Self {
            position_x: entity.position.x,
            position_y: entity.position.y,
            size_x: entity.size.x as i64,
            size_y: entity.size.y as i64,
            visible: entity.attributes.visible,
            active: entity.attributes.active,
            solid: entity.attributes.solid,
            can_move: entity.attributes.can_move,
            has_inventory: entity.attributes.has_inventory,
            speed: entity.attributes.speed as i64,
            render_layer: entity.attributes.render_layer,
            health_enabled,
            health_value,
            collision_enabled,
            collision_offset_x,
            collision_offset_y,
            collision_size_x,
            collision_size_y,
            collision_trigger,
        }
    }
}

impl InspectorSystem {
    /// Renders the main inspector panel on the right side of the screen
    pub fn render_inspector_panel(
        ui_state: &mut EditorUI,
        ctx: &egui::Context,
        game_state: Option<&toki_core::GameState>,
        config: Option<&EditorConfig>,
    ) {
        egui::SidePanel::right("inspector_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("🔍 Inspector");
                ui.separator();

                // Wrap all inspector content in a scrollable area
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        let current_selection = ui_state.selection.clone();
                        match current_selection.as_ref() {
                            Some(Selection::Scene(scene_name)) => {
                                ui.heading(format!("🎬 {}", scene_name));
                                ui.separator();

                                if let Some(scene) = ui_state.get_scene(scene_name) {
                                    ui.horizontal(|ui| {
                                        ui.label("Maps:");
                                        ui.label(format!("{}", scene.maps.len()));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Entities:");
                                        ui.label(format!("{}", scene.entities.len()));
                                    });

                                    ui.separator();
                                    ui.label("Scene Actions:");

                                    if ui.button("🗺️ Add Map").clicked() {
                                        tracing::info!("Add Map to scene: {}", scene_name);
                                        // Maps are added via the hierarchy panel, this could open a dialog
                                    }

                                    if ui.button("👤 Add Entity").clicked() {
                                        tracing::info!("Add Entity to scene: {}", scene_name);
                                        // TODO: Entity creation
                                    }
                                }

                                let mut rules_changed = false;
                                if let Some(scene) = Self::find_scene_mut(ui_state, scene_name) {
                                    ui.separator();
                                    rules_changed = Self::render_scene_rules_editor(
                                        ui,
                                        scene_name,
                                        &mut scene.rules,
                                        config,
                                    );
                                }

                                if rules_changed {
                                    ui_state.scene_content_changed = true;
                                }
                            }

                            Some(Selection::Map(scene_name, map_name)) => {
                                ui.heading(format!("🗺️ {}", map_name));
                                ui.label(format!("Scene: {}", scene_name));
                                ui.separator();

                                Self::render_map_details(
                                    ui,
                                    map_name,
                                    config,
                                    Some(scene_name),
                                    &mut ui_state.map_load_requested,
                                );
                            }

                            Some(Selection::Entity(entity_id)) => {
                                ui.heading(format!("👤 Entity {}", entity_id));
                                ui.separator();
                                let mut entity_changed = false;
                                if let Some(scene_entity) =
                                    Self::find_selected_scene_entity_mut(ui_state, *entity_id)
                                {
                                    let mut draft = EntityPropertyDraft::from_entity(scene_entity);
                                    if Self::render_scene_entity_editor(ui, &mut draft) {
                                        entity_changed =
                                            Self::apply_entity_property_draft(scene_entity, &draft);
                                    }
                                } else {
                                    ui.label("Runtime-only entity (read-only)");
                                    ui.separator();
                                    Self::render_runtime_entity_read_only(
                                        ui, game_state, *entity_id,
                                    );
                                }

                                if entity_changed {
                                    ui_state.scene_content_changed = true;
                                }
                            }

                            Some(Selection::StandaloneMap(map_name)) => {
                                ui.heading(format!("🗺️ {}", map_name));
                                ui.label("(Standalone map - not in scene)");
                                ui.separator();

                                Self::render_map_details(
                                    ui,
                                    map_name,
                                    config,
                                    None,
                                    &mut ui_state.map_load_requested,
                                );
                            }

                            Some(Selection::EntityDefinition(entity_name)) => {
                                ui.heading(format!("🤖 {}", entity_name));
                                ui.label("Entity Definition");
                                ui.separator();

                                Self::render_entity_definition_details(ui, entity_name, config);
                            }

                            None => {
                                ui.label("No selection");
                                ui.separator();
                                ui.label("Click on an item in the hierarchy to inspect it.");
                            }
                        }
                    });
            });
    }

    fn find_scene_mut<'a>(
        ui_state: &'a mut EditorUI,
        scene_name: &str,
    ) -> Option<&'a mut toki_core::Scene> {
        ui_state
            .scenes
            .iter_mut()
            .find(|scene| scene.name == scene_name)
    }

    fn next_rule_id(rule_set: &RuleSet) -> String {
        let mut index = 1usize;
        loop {
            let candidate = format!("rule_{}", index);
            if !rule_set.rules.iter().any(|rule| rule.id == candidate) {
                return candidate;
            }
            index += 1;
        }
    }

    fn add_default_rule(rule_set: &mut RuleSet) -> String {
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

    fn duplicate_rule(rule_set: &mut RuleSet, rule_index: usize) -> Option<usize> {
        let source_rule = rule_set.rules.get(rule_index)?.clone();
        let mut duplicated = source_rule;
        duplicated.id = Self::next_rule_id(rule_set);
        let insert_index = (rule_index + 1).min(rule_set.rules.len());
        rule_set.rules.insert(insert_index, duplicated);
        Some(insert_index)
    }

    fn remove_rule(rule_set: &mut RuleSet, rule_index: usize) -> Option<usize> {
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

    fn move_rule_up(rule_set: &mut RuleSet, rule_index: usize) -> Option<usize> {
        if rule_index >= rule_set.rules.len() {
            return None;
        }
        if rule_index == 0 {
            return Some(0);
        }

        rule_set.rules.swap(rule_index - 1, rule_index);
        Some(rule_index - 1)
    }

    fn move_rule_down(rule_set: &mut RuleSet, rule_index: usize) -> Option<usize> {
        if rule_index >= rule_set.rules.len() {
            return None;
        }
        if rule_index + 1 >= rule_set.rules.len() {
            return Some(rule_index);
        }

        rule_set.rules.swap(rule_index, rule_index + 1);
        Some(rule_index + 1)
    }

    fn add_action(rule: &mut Rule, action_kind: RuleActionKind) {
        rule.actions.push(Self::default_action(action_kind));
    }

    fn remove_action(rule: &mut Rule, action_index: usize) -> bool {
        if action_index >= rule.actions.len() {
            return false;
        }
        rule.actions.remove(action_index);
        true
    }

    fn switch_action_kind(action: &mut RuleAction, action_kind: RuleActionKind) {
        *action = Self::default_action(action_kind);
    }

    fn validate_rule_set(rule_set: &RuleSet) -> Vec<RuleValidationIssue> {
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

    fn default_action(action_kind: RuleActionKind) -> RuleAction {
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
            RuleActionKind::SwitchScene => RuleAction::SwitchScene {
                scene_name: String::new(),
            },
        }
    }

    fn action_kind(action: &RuleAction) -> RuleActionKind {
        match action {
            RuleAction::PlaySound { .. } => RuleActionKind::PlaySound,
            RuleAction::PlayMusic { .. } => RuleActionKind::PlayMusic,
            RuleAction::PlayAnimation { .. } => RuleActionKind::PlayAnimation,
            RuleAction::SetVelocity { .. } => RuleActionKind::SetVelocity,
            RuleAction::SwitchScene { .. } => RuleActionKind::SwitchScene,
        }
    }

    fn action_kind_label(action_kind: RuleActionKind) -> &'static str {
        match action_kind {
            RuleActionKind::PlaySound => "PlaySound",
            RuleActionKind::PlayMusic => "PlayMusic",
            RuleActionKind::PlayAnimation => "PlayAnimation",
            RuleActionKind::SetVelocity => "SetVelocity",
            RuleActionKind::SwitchScene => "SwitchScene",
        }
    }

    fn trigger_kind(trigger: &RuleTrigger) -> RuleTriggerKind {
        match trigger {
            RuleTrigger::OnStart => RuleTriggerKind::Start,
            RuleTrigger::OnUpdate => RuleTriggerKind::Update,
            RuleTrigger::OnKey { .. } => RuleTriggerKind::Key,
            RuleTrigger::OnCollision => RuleTriggerKind::Collision,
            RuleTrigger::OnTrigger => RuleTriggerKind::Trigger,
        }
    }

    fn trigger_kind_label(kind: RuleTriggerKind) -> &'static str {
        match kind {
            RuleTriggerKind::Start => "OnStart",
            RuleTriggerKind::Update => "OnUpdate",
            RuleTriggerKind::Key => "OnKey",
            RuleTriggerKind::Collision => "OnCollision",
            RuleTriggerKind::Trigger => "OnTrigger",
        }
    }

    fn set_rule_trigger_kind(rule: &mut Rule, kind: RuleTriggerKind) {
        rule.trigger = match kind {
            RuleTriggerKind::Start => RuleTrigger::OnStart,
            RuleTriggerKind::Update => RuleTrigger::OnUpdate,
            RuleTriggerKind::Key => RuleTrigger::OnKey { key: RuleKey::Up },
            RuleTriggerKind::Collision => RuleTrigger::OnCollision,
            RuleTriggerKind::Trigger => RuleTrigger::OnTrigger,
        };
    }

    fn rule_key_label(key: RuleKey) -> &'static str {
        match key {
            RuleKey::Up => "Up",
            RuleKey::Down => "Down",
            RuleKey::Left => "Left",
            RuleKey::Right => "Right",
            RuleKey::DebugToggle => "DebugToggle",
        }
    }

    fn load_rule_audio_choices(config: Option<&EditorConfig>) -> RuleAudioChoices {
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

    fn discover_audio_asset_names(dir: &std::path::Path) -> Vec<String> {
        if !dir.exists() {
            return Vec::new();
        }

        let mut names = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
                    continue;
                };
                let supported = matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "ogg" | "wav" | "mp3"
                );
                if !supported {
                    continue;
                }
                if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }

        names.sort();
        names.dedup();
        names
    }

    fn render_scene_rules_editor(
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

    fn render_rule_editor(
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
                ui.label("Conditions: Always");
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
                    if ui.small_button("+ SwitchScene").clicked() {
                        Self::add_action(rule, RuleActionKind::SwitchScene);
                        outcome.changed = true;
                    }
                });
            });

        outcome
    }

    fn render_rule_action_editor(
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
                    .selected_text(match state {
                        AnimationState::Idle => "Idle",
                        AnimationState::Walk => "Walk",
                    })
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(state, AnimationState::Idle, "Idle")
                            .changed();
                        changed |= ui
                            .selectable_value(state, AnimationState::Walk, "Walk")
                            .changed();
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

    fn render_audio_choice_picker(
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

    fn render_rule_target_editor(
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

    fn render_scene_entity_editor(ui: &mut egui::Ui, draft: &mut EntityPropertyDraft) -> bool {
        let mut changed = false;

        ui.label("Scene Entity Properties");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Position:");
            changed |= ui
                .add(egui::DragValue::new(&mut draft.position_x).speed(1.0))
                .changed();
            changed |= ui
                .add(egui::DragValue::new(&mut draft.position_y).speed(1.0))
                .changed();
        });

        ui.horizontal(|ui| {
            ui.label("Size:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut draft.size_x)
                        .speed(1.0)
                        .range(1..=i64::MAX),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut draft.size_y)
                        .speed(1.0)
                        .range(1..=i64::MAX),
                )
                .changed();
        });

        ui.horizontal(|ui| {
            ui.label("Render Layer:");
            changed |= ui
                .add(egui::DragValue::new(&mut draft.render_layer).speed(1.0))
                .changed();
        });

        ui.horizontal(|ui| {
            ui.label("Speed:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut draft.speed)
                        .speed(1.0)
                        .range(0..=i64::MAX),
                )
                .changed();
        });

        changed |= ui.checkbox(&mut draft.visible, "Visible").changed();
        changed |= ui.checkbox(&mut draft.active, "Active").changed();
        changed |= ui.checkbox(&mut draft.solid, "Solid").changed();
        changed |= ui.checkbox(&mut draft.can_move, "Can Move").changed();
        changed |= ui
            .checkbox(&mut draft.has_inventory, "Has Inventory")
            .changed();

        ui.separator();
        ui.label("Health");
        changed |= ui.checkbox(&mut draft.health_enabled, "Enabled").changed();
        if draft.health_enabled {
            ui.horizontal(|ui| {
                ui.label("Value:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.health_value)
                            .speed(1.0)
                            .range(0..=i64::MAX),
                    )
                    .changed();
            });
        }

        ui.separator();
        ui.label("Collision");
        changed |= ui
            .checkbox(&mut draft.collision_enabled, "Enabled")
            .changed();
        if draft.collision_enabled {
            ui.horizontal(|ui| {
                ui.label("Offset:");
                changed |= ui
                    .add(egui::DragValue::new(&mut draft.collision_offset_x).speed(1.0))
                    .changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut draft.collision_offset_y).speed(1.0))
                    .changed();
            });

            ui.horizontal(|ui| {
                ui.label("Size:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.collision_size_x)
                            .speed(1.0)
                            .range(1..=i64::MAX),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.collision_size_y)
                            .speed(1.0)
                            .range(1..=i64::MAX),
                    )
                    .changed();
            });

            changed |= ui
                .checkbox(&mut draft.collision_trigger, "Trigger")
                .changed();
        }

        changed
    }

    fn render_runtime_entity_read_only(
        ui: &mut egui::Ui,
        game_state: Option<&toki_core::GameState>,
        entity_id: toki_core::entity::EntityId,
    ) {
        if let Some(game_state) = game_state {
            if let Some(entity) = game_state.entity_manager().get_entity(entity_id) {
                ui.horizontal(|ui| {
                    ui.label("Position:");
                    ui.label(format!("({}, {})", entity.position.x, entity.position.y));
                });

                ui.horizontal(|ui| {
                    ui.label("Size:");
                    ui.label(format!("{}x{}", entity.size.x, entity.size.y));
                });

                ui.horizontal(|ui| {
                    ui.label("Type:");
                    ui.label(format!("{:?}", entity.entity_type));
                });

                ui.horizontal(|ui| {
                    ui.label("Visible:");
                    ui.label(format!("{}", entity.attributes.visible));
                });

                ui.horizontal(|ui| {
                    ui.label("Active:");
                    ui.label(format!("{}", entity.attributes.active));
                });

                if let Some(health) = entity.attributes.health {
                    ui.horizontal(|ui| {
                        ui.label("Health:");
                        ui.label(format!("{}", health));
                    });
                }

                if entity.attributes.has_inventory {
                    ui.horizontal(|ui| {
                        ui.label("Has Inventory:");
                        ui.label("Yes");
                    });
                }

                if let Some(collision_box) = &entity.collision_box {
                    ui.separator();
                    ui.label("Collision Box:");
                    ui.horizontal(|ui| {
                        ui.label("Offset:");
                        ui.label(format!(
                            "({}, {})",
                            collision_box.offset.x, collision_box.offset.y
                        ));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Size:");
                        ui.label(format!("{}x{}", collision_box.size.x, collision_box.size.y));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Trigger:");
                        ui.label(format!("{}", collision_box.trigger));
                    });
                }

                if let Some(animation_controller) = &entity.attributes.animation_controller {
                    ui.separator();
                    ui.label("Animation:");
                    ui.horizontal(|ui| {
                        ui.label("Current State:");
                        ui.label(format!("{:?}", animation_controller.current_clip_state));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Frame:");
                        ui.label(format!("{}", animation_controller.current_frame_index));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Finished:");
                        ui.label(format!("{}", animation_controller.is_finished));
                    });
                }
            } else {
                ui.label("❌ Entity not found in game state");
            }
        } else {
            ui.label("❌ No game state available");
        }
    }

    fn find_selected_scene_entity_mut(
        ui_state: &mut EditorUI,
        entity_id: toki_core::entity::EntityId,
    ) -> Option<&mut toki_core::entity::Entity> {
        let active_scene_name = ui_state.active_scene.clone()?;
        let scene = ui_state
            .scenes
            .iter_mut()
            .find(|scene| scene.name == active_scene_name)?;
        scene
            .entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
    }

    fn apply_entity_property_draft(
        entity: &mut toki_core::entity::Entity,
        draft: &EntityPropertyDraft,
    ) -> bool {
        fn set_if_changed<T: PartialEq>(target: &mut T, value: T) -> bool {
            if *target != value {
                *target = value;
                true
            } else {
                false
            }
        }

        fn clamp_to_non_negative_u32(value: i64) -> u32 {
            value.clamp(0, u32::MAX as i64) as u32
        }

        fn clamp_to_min_one_u32(value: i64) -> u32 {
            value.clamp(1, u32::MAX as i64) as u32
        }

        let mut changed = false;

        let new_position = glam::IVec2::new(draft.position_x, draft.position_y);
        changed |= set_if_changed(&mut entity.position, new_position);

        let new_size = glam::UVec2::new(
            clamp_to_min_one_u32(draft.size_x),
            clamp_to_min_one_u32(draft.size_y),
        );
        changed |= set_if_changed(&mut entity.size, new_size);

        changed |= set_if_changed(&mut entity.attributes.visible, draft.visible);
        changed |= set_if_changed(&mut entity.attributes.active, draft.active);
        changed |= set_if_changed(&mut entity.attributes.solid, draft.solid);
        changed |= set_if_changed(&mut entity.attributes.can_move, draft.can_move);
        changed |= set_if_changed(&mut entity.attributes.has_inventory, draft.has_inventory);
        changed |= set_if_changed(
            &mut entity.attributes.speed,
            clamp_to_non_negative_u32(draft.speed),
        );
        changed |= set_if_changed(&mut entity.attributes.render_layer, draft.render_layer);

        let new_health = if draft.health_enabled {
            Some(clamp_to_non_negative_u32(draft.health_value))
        } else {
            None
        };
        changed |= set_if_changed(&mut entity.attributes.health, new_health);

        if draft.collision_enabled {
            if entity.collision_box.is_none() {
                entity.collision_box =
                    Some(toki_core::collision::CollisionBox::solid_box(entity.size));
                changed = true;
            }

            if let Some(collision_box) = entity.collision_box.as_mut() {
                changed |= set_if_changed(
                    &mut collision_box.offset,
                    glam::IVec2::new(draft.collision_offset_x, draft.collision_offset_y),
                );
                changed |= set_if_changed(
                    &mut collision_box.size,
                    glam::UVec2::new(
                        clamp_to_min_one_u32(draft.collision_size_x),
                        clamp_to_min_one_u32(draft.collision_size_y),
                    ),
                );
                changed |= set_if_changed(&mut collision_box.trigger, draft.collision_trigger);
            }
        } else if entity.collision_box.is_some() {
            entity.collision_box = None;
            changed = true;
        }

        changed
    }

    /// Renders detailed information about a specific map
    pub fn render_map_details(
        ui: &mut egui::Ui,
        map_name: &str,
        config: Option<&EditorConfig>,
        scene_name: Option<&str>,
        map_load_requested: &mut Option<(String, String)>,
    ) {
        // Try to load and show map details
        if let Some(config) = config {
            if let Some(project_path) = config.current_project_path() {
                let map_file = project_path
                    .join("assets")
                    .join("tilemaps")
                    .join(format!("{}.json", map_name));

                if map_file.exists() {
                    // Try to read the tilemap file
                    match std::fs::read_to_string(&map_file) {
                        Ok(content) => {
                            // Try to parse as JSON to show basic info
                            match serde_json::from_str::<serde_json::Value>(&content) {
                                Ok(json) => {
                                    // Show file info
                                    ui.horizontal(|ui| {
                                        ui.label("File:");
                                        ui.label(format!("{}.json", map_name));
                                    });

                                    // Show file size
                                    ui.horizontal(|ui| {
                                        ui.label("Size:");
                                        ui.label(format!("{} bytes", content.len()));
                                    });

                                    // Show JSON properties and values
                                    if let Some(obj) = json.as_object() {
                                        ui.horizontal(|ui| {
                                            ui.label("Properties:");
                                            ui.label(format!("{}", obj.keys().count()));
                                        });

                                        ui.separator();
                                        ui.label("Map Properties:");

                                        egui::ScrollArea::vertical()
                                            .id_salt("map_properties_scroll")
                                            .max_height(200.0)
                                            .show(ui, |ui| {
                                                for (key, value) in obj {
                                                    ui.horizontal(|ui| {
                                                        ui.label(format!("{}:", key));

                                                        // Format value based on type
                                                        let value_str = match value {
                                                            serde_json::Value::String(s) => {
                                                                format!("\"{}\"", s)
                                                            }
                                                            serde_json::Value::Number(n) => {
                                                                n.to_string()
                                                            }
                                                            serde_json::Value::Bool(b) => {
                                                                b.to_string()
                                                            }
                                                            serde_json::Value::Array(arr) => {
                                                                format!("[{} items]", arr.len())
                                                            }
                                                            serde_json::Value::Object(obj) => {
                                                                format!(
                                                                    "{{{}}} properties",
                                                                    obj.keys().count()
                                                                )
                                                            }
                                                            serde_json::Value::Null => {
                                                                "null".to_string()
                                                            }
                                                        };

                                                        ui.label(value_str);
                                                    });
                                                }
                                            });
                                    }

                                    ui.separator();
                                    ui.label("Map Actions:");

                                    if let Some(scene_name) = scene_name {
                                        if ui.button("📂 Load in Viewport").clicked() {
                                            tracing::info!(
                                                "Load Map '{}' from scene '{}' clicked",
                                                map_name,
                                                scene_name
                                            );
                                            *map_load_requested = Some((
                                                scene_name.to_string(),
                                                map_name.to_string(),
                                            ));
                                        }
                                    } else {
                                        ui.label("(Not associated with a scene)");
                                    }
                                }
                                Err(e) => {
                                    ui.label("❌ Invalid JSON file");
                                    ui.label(format!("Error: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            ui.label("❌ Could not read map file");
                            ui.label(format!("Error: {}", e));
                        }
                    }
                } else {
                    ui.label("❌ Map file not found");
                }
            }
        }
    }

    /// Renders detailed information about an entity definition
    pub fn render_entity_definition_details(
        ui: &mut egui::Ui,
        entity_name: &str,
        config: Option<&EditorConfig>,
    ) {
        // Try to load and show entity definition details
        if let Some(config) = config {
            if let Some(project_path) = config.current_project_path() {
                let entity_file = project_path
                    .join("entities")
                    .join(format!("{}.json", entity_name));

                if entity_file.exists() {
                    // Try to read the entity definition file
                    match std::fs::read_to_string(&entity_file) {
                        Ok(content) => {
                            // Try to parse as JSON to show detailed info
                            match serde_json::from_str::<serde_json::Value>(&content) {
                                Ok(json) => {
                                    // Show file info
                                    ui.horizontal(|ui| {
                                        ui.label("File:");
                                        ui.label(format!("{}.json", entity_name));
                                    });

                                    if let Some(obj) = json.as_object() {
                                        // Show basic entity information
                                        if let Some(display_name) =
                                            obj.get("display_name").and_then(|v| v.as_str())
                                        {
                                            ui.horizontal(|ui| {
                                                ui.label("Display Name:");
                                                ui.label(display_name);
                                            });
                                        }

                                        if let Some(description) =
                                            obj.get("description").and_then(|v| v.as_str())
                                        {
                                            ui.horizontal(|ui| {
                                                ui.label("Description:");
                                                ui.label(description);
                                            });
                                        }

                                        if let Some(entity_type) =
                                            obj.get("entity_type").and_then(|v| v.as_str())
                                        {
                                            ui.horizontal(|ui| {
                                                ui.label("Type:");
                                                ui.label(entity_type);
                                            });
                                        }

                                        if let Some(category) =
                                            obj.get("category").and_then(|v| v.as_str())
                                        {
                                            ui.horizontal(|ui| {
                                                ui.label("Category:");
                                                ui.label(category);
                                            });
                                        }

                                        // Show rendering properties
                                        if let Some(rendering) =
                                            obj.get("rendering").and_then(|v| v.as_object())
                                        {
                                            ui.separator();
                                            ui.label("Rendering:");

                                            if let Some(size) =
                                                rendering.get("size").and_then(|v| v.as_array())
                                            {
                                                if size.len() == 2 {
                                                    if let (Some(w), Some(h)) =
                                                        (size[0].as_u64(), size[1].as_u64())
                                                    {
                                                        ui.horizontal(|ui| {
                                                            ui.label("Size:");
                                                            ui.label(format!("{}x{}", w, h));
                                                        });
                                                    }
                                                }
                                            }

                                            if let Some(visible) =
                                                rendering.get("visible").and_then(|v| v.as_bool())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Visible:");
                                                    ui.label(format!("{}", visible));
                                                });
                                            }

                                            if let Some(render_layer) = rendering
                                                .get("render_layer")
                                                .and_then(|v| v.as_u64())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Render Layer:");
                                                    ui.label(format!("{}", render_layer));
                                                });
                                            }
                                        }

                                        // Show attributes
                                        if let Some(attributes) =
                                            obj.get("attributes").and_then(|v| v.as_object())
                                        {
                                            ui.separator();
                                            ui.label("Attributes:");

                                            if let Some(health) =
                                                attributes.get("health").and_then(|v| v.as_u64())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Health:");
                                                    ui.label(format!("{}", health));
                                                });
                                            }

                                            if let Some(speed) =
                                                attributes.get("speed").and_then(|v| v.as_u64())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Speed:");
                                                    ui.label(format!("{}", speed));
                                                });
                                            }

                                            if let Some(solid) =
                                                attributes.get("solid").and_then(|v| v.as_bool())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Solid:");
                                                    ui.label(format!("{}", solid));
                                                });
                                            }

                                            if let Some(active) =
                                                attributes.get("active").and_then(|v| v.as_bool())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Active:");
                                                    ui.label(format!("{}", active));
                                                });
                                            }

                                            if let Some(can_move) =
                                                attributes.get("can_move").and_then(|v| v.as_bool())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Can Move:");
                                                    ui.label(format!("{}", can_move));
                                                });
                                            }

                                            if let Some(has_inventory) = attributes
                                                .get("has_inventory")
                                                .and_then(|v| v.as_bool())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Has Inventory:");
                                                    ui.label(format!("{}", has_inventory));
                                                });
                                            }
                                        }

                                        // Show collision information
                                        if let Some(collision) =
                                            obj.get("collision").and_then(|v| v.as_object())
                                        {
                                            ui.separator();
                                            ui.label("Collision:");

                                            if let Some(enabled) =
                                                collision.get("enabled").and_then(|v| v.as_bool())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Enabled:");
                                                    ui.label(format!("{}", enabled));
                                                });
                                            }

                                            if let Some(offset) =
                                                collision.get("offset").and_then(|v| v.as_array())
                                            {
                                                if offset.len() == 2 {
                                                    if let (Some(x), Some(y)) =
                                                        (offset[0].as_i64(), offset[1].as_i64())
                                                    {
                                                        ui.horizontal(|ui| {
                                                            ui.label("Offset:");
                                                            ui.label(format!("({}, {})", x, y));
                                                        });
                                                    }
                                                }
                                            }

                                            if let Some(size) =
                                                collision.get("size").and_then(|v| v.as_array())
                                            {
                                                if size.len() == 2 {
                                                    if let (Some(w), Some(h)) =
                                                        (size[0].as_u64(), size[1].as_u64())
                                                    {
                                                        ui.horizontal(|ui| {
                                                            ui.label("Size:");
                                                            ui.label(format!("{}x{}", w, h));
                                                        });
                                                    }
                                                }
                                            }

                                            if let Some(trigger) =
                                                collision.get("trigger").and_then(|v| v.as_bool())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Trigger:");
                                                    ui.label(format!("{}", trigger));
                                                });
                                            }
                                        }

                                        // Show audio information
                                        if let Some(audio) =
                                            obj.get("audio").and_then(|v| v.as_object())
                                        {
                                            ui.separator();
                                            ui.label("Audio:");

                                            if let Some(distance) = audio
                                                .get("footstep_trigger_distance")
                                                .and_then(|v| v.as_f64())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Footstep Distance:");
                                                    ui.label(format!("{:.1}", distance));
                                                });
                                            }

                                            if let Some(movement_sound) =
                                                audio.get("movement_sound").and_then(|v| v.as_str())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Movement Sound:");
                                                    ui.label(movement_sound);
                                                });
                                            }

                                            if let Some(collision_sound) = audio
                                                .get("collision_sound")
                                                .and_then(|v| v.as_str())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Collision Sound:");
                                                    ui.label(collision_sound);
                                                });
                                            }
                                        }

                                        // Show animation information
                                        if let Some(animations) =
                                            obj.get("animations").and_then(|v| v.as_object())
                                        {
                                            ui.separator();
                                            ui.label("Animations:");

                                            if let Some(atlas_name) = animations
                                                .get("atlas_name")
                                                .and_then(|v| v.as_str())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Atlas:");
                                                    ui.label(atlas_name);
                                                });
                                            }

                                            if let Some(default_state) = animations
                                                .get("default_state")
                                                .and_then(|v| v.as_str())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Default State:");
                                                    ui.label(default_state);
                                                });
                                            }

                                            if let Some(clips) =
                                                animations.get("clips").and_then(|v| v.as_array())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Available Clips:");
                                                    ui.label(format!("{}", clips.len()));
                                                });

                                                ui.indent("animation_clips", |ui| {
                                                    for clip in clips.iter() {
                                                        if let Some(clip_obj) = clip.as_object() {
                                                            let state = clip_obj
                                                                .get("state")
                                                                .and_then(|v| v.as_str())
                                                                .unwrap_or("unknown");
                                                            let loop_mode = clip_obj
                                                                .get("loop_mode")
                                                                .and_then(|v| v.as_str())
                                                                .unwrap_or("unknown");
                                                            let frame_duration = clip_obj
                                                                .get("frame_duration_ms")
                                                                .and_then(|v| v.as_f64())
                                                                .unwrap_or(0.0);
                                                            let frame_count = clip_obj
                                                                .get("frame_tiles")
                                                                .and_then(|v| v.as_array())
                                                                .map(|arr| arr.len())
                                                                .unwrap_or(0);

                                                            ui.horizontal(|ui| {
                                                                ui.label(format!(
                                                                    "• {}: {} frames, {:.0}ms, {}",
                                                                    state,
                                                                    frame_count,
                                                                    frame_duration,
                                                                    loop_mode
                                                                ));
                                                            });
                                                        }
                                                    }
                                                });
                                            }
                                        }

                                        ui.separator();
                                        ui.label("Entity Actions:");

                                        if ui.button("🎮 Place in Scene").clicked() {
                                            tracing::info!(
                                                "Place entity '{}' button clicked",
                                                entity_name
                                            );
                                            // TODO: Implement entity placement functionality
                                        }
                                    }
                                }
                                Err(e) => {
                                    ui.label("❌ Invalid JSON file");
                                    ui.label(format!("Error: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            ui.label("❌ Could not read entity definition file");
                            ui.label(format!("Error: {}", e));
                        }
                    }
                } else {
                    ui.label("❌ Entity definition file not found");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EntityPropertyDraft, InspectorSystem, RuleActionKind, RuleTriggerKind};
    use crate::ui::EditorUI;
    use glam::{IVec2, UVec2};
    use std::fs;
    use toki_core::animation::AnimationState;
    use toki_core::collision::CollisionBox;
    use toki_core::entity::{EntityAttributes, EntityManager, EntityType};
    use toki_core::rules::{
        Rule, RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel, RuleTarget,
        RuleTrigger,
    };
    use toki_core::Scene;

    fn sample_entity_with_id(id: u32) -> toki_core::entity::Entity {
        let mut manager = EntityManager::new();
        let spawned_id = manager.spawn_entity(
            EntityType::Npc,
            IVec2::new(10, 20),
            UVec2::new(16, 16),
            EntityAttributes {
                health: Some(25),
                speed: 3,
                solid: true,
                visible: true,
                animation_controller: None,
                render_layer: 1,
                active: true,
                can_move: true,
                has_inventory: false,
            },
        );
        let mut entity = manager
            .get_entity(spawned_id)
            .expect("missing spawned entity")
            .clone();
        entity.id = id;
        entity.collision_box = Some(CollisionBox::new(
            IVec2::new(0, 0),
            UVec2::new(16, 16),
            false,
        ));
        entity
    }

    fn sample_rule(id: &str) -> Rule {
        Rule {
            id: id.to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_step".to_string(),
            }],
        }
    }

    #[test]
    fn apply_entity_property_draft_clamps_and_sets_values() {
        let mut entity = sample_entity_with_id(1);
        let mut draft = EntityPropertyDraft::from_entity(&entity);
        draft.position_x = 100;
        draft.position_y = 200;
        draft.size_x = 0;
        draft.size_y = -5;
        draft.visible = false;
        draft.active = false;
        draft.solid = false;
        draft.can_move = false;
        draft.has_inventory = true;
        draft.speed = -10;
        draft.render_layer = 8;
        draft.health_enabled = true;
        draft.health_value = -4;
        draft.collision_enabled = true;
        draft.collision_offset_x = 3;
        draft.collision_offset_y = -2;
        draft.collision_size_x = 0;
        draft.collision_size_y = -7;
        draft.collision_trigger = true;

        let changed = InspectorSystem::apply_entity_property_draft(&mut entity, &draft);

        assert!(changed);
        assert_eq!(entity.position, IVec2::new(100, 200));
        assert_eq!(entity.size, UVec2::new(1, 1));
        assert!(!entity.attributes.visible);
        assert!(!entity.attributes.active);
        assert!(!entity.attributes.solid);
        assert!(!entity.attributes.can_move);
        assert!(entity.attributes.has_inventory);
        assert_eq!(entity.attributes.speed, 0);
        assert_eq!(entity.attributes.render_layer, 8);
        assert_eq!(entity.attributes.health, Some(0));

        let collision = entity
            .collision_box
            .as_ref()
            .expect("collision should be enabled");
        assert_eq!(collision.offset, IVec2::new(3, -2));
        assert_eq!(collision.size, UVec2::new(1, 1));
        assert!(collision.trigger);
    }

    #[test]
    fn apply_entity_property_draft_disables_health_and_collision() {
        let mut entity = sample_entity_with_id(1);
        let mut draft = EntityPropertyDraft::from_entity(&entity);
        draft.health_enabled = false;
        draft.collision_enabled = false;

        let changed = InspectorSystem::apply_entity_property_draft(&mut entity, &draft);

        assert!(changed);
        assert_eq!(entity.attributes.health, None);
        assert!(entity.collision_box.is_none());
    }

    #[test]
    fn find_selected_scene_entity_mut_returns_entity_from_active_scene() {
        let mut ui_state = EditorUI::new();
        let entity = sample_entity_with_id(7);
        let scene = ui_state
            .scenes
            .iter_mut()
            .find(|scene| scene.name == "Main Scene")
            .expect("missing default scene");
        scene.entities.push(entity);

        let selected_entity = InspectorSystem::find_selected_scene_entity_mut(&mut ui_state, 7)
            .expect("entity should be found");
        selected_entity.position = IVec2::new(50, 60);

        let scene = ui_state
            .scenes
            .iter()
            .find(|scene| scene.name == "Main Scene")
            .expect("missing default scene");
        let entity = scene
            .entities
            .iter()
            .find(|entity| entity.id == 7)
            .expect("entity should still exist");
        assert_eq!(entity.position, IVec2::new(50, 60));
    }

    #[test]
    fn find_selected_scene_entity_mut_returns_none_for_inactive_scene() {
        let mut ui_state = EditorUI::new();
        ui_state.scenes.push(Scene::new("Other".to_string()));
        ui_state.active_scene = Some("Other".to_string());

        let scene = ui_state
            .scenes
            .iter_mut()
            .find(|scene| scene.name == "Main Scene")
            .expect("missing default scene");
        scene.entities.push(sample_entity_with_id(42));

        assert!(InspectorSystem::find_selected_scene_entity_mut(&mut ui_state, 42).is_none());
    }

    #[test]
    fn next_rule_id_fills_first_available_gap() {
        let rules = RuleSet {
            rules: vec![
                toki_core::rules::Rule {
                    id: "rule_1".to_string(),
                    enabled: true,
                    priority: 0,
                    once: false,
                    trigger: RuleTrigger::OnUpdate,
                    conditions: vec![RuleCondition::Always],
                    actions: vec![],
                },
                toki_core::rules::Rule {
                    id: "rule_3".to_string(),
                    enabled: true,
                    priority: 0,
                    once: false,
                    trigger: RuleTrigger::OnUpdate,
                    conditions: vec![RuleCondition::Always],
                    actions: vec![],
                },
            ],
        };

        let next = InspectorSystem::next_rule_id(&rules);
        assert_eq!(next, "rule_2");
    }

    #[test]
    fn add_default_rule_appends_editable_placeholder_rule() {
        let mut rules = RuleSet::default();
        let id = InspectorSystem::add_default_rule(&mut rules);

        assert_eq!(id, "rule_1");
        assert_eq!(rules.rules.len(), 1);

        let rule = &rules.rules[0];
        assert_eq!(rule.id, "rule_1");
        assert!(rule.enabled);
        assert_eq!(rule.priority, 0);
        assert!(!rule.once);
        assert_eq!(rule.trigger, RuleTrigger::OnUpdate);
        assert_eq!(rule.conditions, vec![RuleCondition::Always]);
        assert_eq!(rule.actions.len(), 1);
        assert_eq!(
            rule.actions[0],
            RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_placeholder".to_string(),
            }
        );
    }

    #[test]
    fn set_rule_trigger_kind_sets_expected_trigger_payload() {
        let mut rule = sample_rule("rule_1");

        InspectorSystem::set_rule_trigger_kind(&mut rule, RuleTriggerKind::Start);
        assert_eq!(rule.trigger, RuleTrigger::OnStart);

        InspectorSystem::set_rule_trigger_kind(&mut rule, RuleTriggerKind::Update);
        assert_eq!(rule.trigger, RuleTrigger::OnUpdate);

        InspectorSystem::set_rule_trigger_kind(&mut rule, RuleTriggerKind::Collision);
        assert_eq!(rule.trigger, RuleTrigger::OnCollision);

        InspectorSystem::set_rule_trigger_kind(&mut rule, RuleTriggerKind::Trigger);
        assert_eq!(rule.trigger, RuleTrigger::OnTrigger);

        InspectorSystem::set_rule_trigger_kind(&mut rule, RuleTriggerKind::Key);
        assert_eq!(rule.trigger, RuleTrigger::OnKey { key: RuleKey::Up });
    }

    #[test]
    fn duplicate_rule_clones_payload_with_new_id_and_insert_position() {
        let mut rules = RuleSet {
            rules: vec![sample_rule("rule_1"), sample_rule("rule_2")],
        };

        let inserted_index =
            InspectorSystem::duplicate_rule(&mut rules, 0).expect("duplicate should succeed");

        assert_eq!(inserted_index, 1);
        assert_eq!(rules.rules.len(), 3);
        assert_eq!(rules.rules[0].id, "rule_1");
        assert_eq!(rules.rules[1].id, "rule_3");
        assert_eq!(rules.rules[2].id, "rule_2");
        assert_eq!(rules.rules[1].actions, rules.rules[0].actions);
    }

    #[test]
    fn remove_rule_returns_next_selection_or_previous_for_last() {
        let mut rules = RuleSet {
            rules: vec![
                sample_rule("rule_1"),
                sample_rule("rule_2"),
                sample_rule("rule_3"),
            ],
        };

        let selected_after_middle =
            InspectorSystem::remove_rule(&mut rules, 1).expect("selection should stay valid");
        assert_eq!(selected_after_middle, 1);
        assert_eq!(
            rules
                .rules
                .iter()
                .map(|rule| rule.id.as_str())
                .collect::<Vec<_>>(),
            vec!["rule_1", "rule_3"]
        );

        let selected_after_last =
            InspectorSystem::remove_rule(&mut rules, 1).expect("last removal should select prev");
        assert_eq!(selected_after_last, 0);
        assert_eq!(
            rules
                .rules
                .iter()
                .map(|rule| rule.id.as_str())
                .collect::<Vec<_>>(),
            vec!["rule_1"]
        );

        let selected_after_final = InspectorSystem::remove_rule(&mut rules, 0);
        assert!(selected_after_final.is_none());
        assert!(rules.rules.is_empty());
    }

    #[test]
    fn move_rule_up_and_down_reorders_and_handles_boundaries() {
        let mut rules = RuleSet {
            rules: vec![
                sample_rule("rule_1"),
                sample_rule("rule_2"),
                sample_rule("rule_3"),
            ],
        };

        let up_index = InspectorSystem::move_rule_up(&mut rules, 1).expect("move up should work");
        assert_eq!(up_index, 0);
        assert_eq!(
            rules
                .rules
                .iter()
                .map(|rule| rule.id.as_str())
                .collect::<Vec<_>>(),
            vec!["rule_2", "rule_1", "rule_3"]
        );

        let noop_up = InspectorSystem::move_rule_up(&mut rules, 0).expect("boundary no-op");
        assert_eq!(noop_up, 0);
        assert_eq!(
            rules
                .rules
                .iter()
                .map(|rule| rule.id.as_str())
                .collect::<Vec<_>>(),
            vec!["rule_2", "rule_1", "rule_3"]
        );

        let down_index =
            InspectorSystem::move_rule_down(&mut rules, 1).expect("move down should work");
        assert_eq!(down_index, 2);
        assert_eq!(
            rules
                .rules
                .iter()
                .map(|rule| rule.id.as_str())
                .collect::<Vec<_>>(),
            vec!["rule_2", "rule_3", "rule_1"]
        );

        let noop_down = InspectorSystem::move_rule_down(&mut rules, 2).expect("boundary no-op");
        assert_eq!(noop_down, 2);
    }

    #[test]
    fn add_remove_and_switch_action_types() {
        let mut rule = sample_rule("rule_1");
        assert_eq!(rule.actions.len(), 1);

        InspectorSystem::add_action(&mut rule, RuleActionKind::PlayMusic);
        InspectorSystem::add_action(&mut rule, RuleActionKind::PlayAnimation);
        InspectorSystem::add_action(&mut rule, RuleActionKind::SetVelocity);
        InspectorSystem::add_action(&mut rule, RuleActionKind::SwitchScene);
        assert_eq!(rule.actions.len(), 5);
        assert!(matches!(
            rule.actions[1],
            RuleAction::PlayMusic { ref track_id } if track_id == "music_placeholder"
        ));
        assert!(matches!(
            rule.actions[2],
            RuleAction::PlayAnimation {
                target: RuleTarget::Player,
                state: AnimationState::Idle
            }
        ));
        assert!(matches!(
            rule.actions[3],
            RuleAction::SetVelocity {
                target: RuleTarget::Player,
                velocity: [0, 0]
            }
        ));
        assert!(matches!(
            rule.actions[4],
            RuleAction::SwitchScene { ref scene_name } if scene_name.is_empty()
        ));

        InspectorSystem::switch_action_kind(&mut rule.actions[0], RuleActionKind::SetVelocity);
        assert!(matches!(
            rule.actions[0],
            RuleAction::SetVelocity {
                target: RuleTarget::Player,
                velocity: [0, 0]
            }
        ));
        InspectorSystem::switch_action_kind(&mut rule.actions[0], RuleActionKind::PlaySound);
        assert!(matches!(
            rule.actions[0],
            RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                ref sound_id,
            } if sound_id == "sfx_placeholder"
        ));

        assert!(InspectorSystem::remove_action(&mut rule, 1));
        assert_eq!(rule.actions.len(), 4);
        assert!(!InspectorSystem::remove_action(&mut rule, 99));
    }

    #[test]
    fn validate_rule_set_reports_duplicate_ids_and_invalid_action_payloads() {
        let mut first = sample_rule("dupe");
        first.actions = vec![
            RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "   ".to_string(),
            },
            RuleAction::SetVelocity {
                target: RuleTarget::Entity(0),
                velocity: [1, 0],
            },
        ];

        let second = Rule {
            id: "dupe".to_string(),
            enabled: true,
            priority: 1,
            once: false,
            trigger: RuleTrigger::OnStart,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::SwitchScene {
                scene_name: "   ".to_string(),
            }],
        };

        let rules = RuleSet {
            rules: vec![first, second],
        };

        let issues = InspectorSystem::validate_rule_set(&rules);
        assert!(issues
            .iter()
            .any(|issue| issue.message.contains("Duplicate rule id 'dupe'")));
        assert!(issues.iter().any(|issue| issue
            .message
            .contains("PlaySound requires a non-empty sound id")));
        assert!(issues.iter().any(|issue| issue
            .message
            .contains("SetVelocity entity target must be non-zero")));
        assert!(issues
            .iter()
            .any(|issue| issue.message.contains("SwitchScene requires a scene name")));
    }

    #[test]
    fn validate_rule_set_reports_empty_play_music_track() {
        let rules = RuleSet {
            rules: vec![Rule {
                id: "music-rule".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnUpdate,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::PlayMusic {
                    track_id: "   ".to_string(),
                }],
            }],
        };

        let issues = InspectorSystem::validate_rule_set(&rules);
        assert!(issues.iter().any(|issue| issue
            .message
            .contains("PlayMusic requires a non-empty track id")));
    }

    #[test]
    fn discover_audio_asset_names_reads_supported_audio_files() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        fs::write(temp_dir.path().join("battle_theme.ogg"), "x").expect("ogg file write");
        fs::write(temp_dir.path().join("ambience.mp3"), "x").expect("mp3 file write");
        fs::write(temp_dir.path().join("impact.wav"), "x").expect("wav file write");
        fs::write(temp_dir.path().join("ignore.txt"), "x").expect("txt file write");
        fs::create_dir(temp_dir.path().join("sub")).expect("subdir create");
        fs::write(temp_dir.path().join("sub").join("nested.ogg"), "x").expect("nested write");

        let names = InspectorSystem::discover_audio_asset_names(temp_dir.path());
        assert_eq!(names, vec!["ambience", "battle_theme", "impact"]);
    }
}
