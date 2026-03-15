use super::editor_ui::{EditorUI, SceneRulesGraphCommandData, Selection};
use super::rule_graph::{RuleGraph, RuleGraphNodeKind};
use super::undo_redo::EditorCommand;
use crate::config::EditorConfig;
use crate::project::Project;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use toki_core::animation::AnimationState;
use toki_core::entity::{AiBehavior, ControlRole, MovementProfile, MovementSoundTrigger};
use toki_core::rules::{
    Rule, RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel, RuleSpawnEntityType,
    RuleTarget, RuleTrigger,
};

/// Handles inspector panel rendering for assets and entities
pub struct InspectorSystem;

#[derive(Debug, Clone)]
struct EntityPropertyDraft {
    control_role: ControlRole,
    position_x: i32,
    position_y: i32,
    size_x: i64,
    size_y: i64,
    visible: bool,
    active: bool,
    solid: bool,
    can_move: bool,
    ai_behavior: AiBehavior,
    movement_profile: MovementProfile,
    movement_sound_trigger: MovementSoundTrigger,
    footstep_trigger_distance: f32,
    hearing_radius: u32,
    movement_sound: String,
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

#[derive(Debug, Clone)]
struct ProjectSettingsDraft {
    name: String,
    version: String,
    description: String,
    splash_duration_ms: u64,
    master_mix_percent: u8,
    music_mix_percent: u8,
    movement_mix_percent: u8,
    collision_mix_percent: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct MultiEntityCommonState {
    visible: Option<bool>,
    active: Option<bool>,
    collision_enabled: Option<bool>,
    render_layer: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct MultiEntityBatchEdit {
    set_visible: Option<bool>,
    set_active: Option<bool>,
    set_collision_enabled: Option<bool>,
    set_render_layer: Option<i32>,
    position_delta: Option<glam::IVec2>,
}

impl MultiEntityBatchEdit {
    fn is_noop(self) -> bool {
        self.set_visible.is_none()
            && self.set_active.is_none()
            && self.set_collision_enabled.is_none()
            && self.set_render_layer.is_none()
            && self.position_delta.is_none()
    }
}

fn animation_state_label(state: AnimationState) -> &'static str {
    match state {
        AnimationState::Idle => "Idle",
        AnimationState::Walk => "Walk",
        AnimationState::IdleDown => "Idle Down",
        AnimationState::IdleUp => "Idle Up",
        AnimationState::IdleLeft => "Idle Left",
        AnimationState::IdleRight => "Idle Right",
        AnimationState::WalkDown => "Walk Down",
        AnimationState::WalkUp => "Walk Up",
        AnimationState::WalkLeft => "Walk Left",
        AnimationState::WalkRight => "Walk Right",
    }
}

fn animation_state_options() -> [AnimationState; 10] {
    [
        AnimationState::Idle,
        AnimationState::Walk,
        AnimationState::IdleDown,
        AnimationState::IdleUp,
        AnimationState::IdleLeft,
        AnimationState::IdleRight,
        AnimationState::WalkDown,
        AnimationState::WalkUp,
        AnimationState::WalkLeft,
        AnimationState::WalkRight,
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleActionKind {
    PlaySound,
    PlayMusic,
    PlayAnimation,
    SetVelocity,
    Spawn,
    DestroySelf,
    SwitchScene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleConditionKind {
    Always,
    TargetExists,
    KeyHeld,
    EntityActive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleTriggerKind {
    Start,
    Update,
    PlayerMove,
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
            control_role: entity.control_role,
            position_x: entity.position.x,
            position_y: entity.position.y,
            size_x: entity.size.x as i64,
            size_y: entity.size.y as i64,
            visible: entity.attributes.visible,
            active: entity.attributes.active,
            solid: entity.attributes.solid,
            can_move: entity.attributes.can_move,
            ai_behavior: entity.attributes.ai_behavior,
            movement_profile: entity.attributes.movement_profile,
            movement_sound_trigger: entity.audio.movement_sound_trigger,
            footstep_trigger_distance: entity.audio.footstep_trigger_distance,
            hearing_radius: entity.audio.hearing_radius,
            movement_sound: entity.audio.movement_sound.clone().unwrap_or_default(),
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

impl ProjectSettingsDraft {
    fn from_project(project: &Project) -> Self {
        Self {
            name: project.metadata.project.name.clone(),
            version: project.metadata.project.version.clone(),
            description: project.metadata.project.description.clone(),
            splash_duration_ms: project.metadata.runtime.splash.duration_ms,
            master_mix_percent: project.metadata.runtime.audio.master_percent,
            music_mix_percent: project.metadata.runtime.audio.music_percent,
            movement_mix_percent: project.metadata.runtime.audio.movement_percent,
            collision_mix_percent: project.metadata.runtime.audio.collision_percent,
        }
    }
}

fn ai_behavior_label(ai_behavior: AiBehavior) -> &'static str {
    match ai_behavior {
        AiBehavior::None => "None",
        AiBehavior::Wander => "Wander",
    }
}

fn control_role_label(control_role: ControlRole) -> &'static str {
    match control_role {
        ControlRole::LegacyDefault | ControlRole::None => "None",
        ControlRole::PlayerCharacter => "Player Character",
    }
}

fn movement_profile_label(
    control_role: ControlRole,
    movement_profile: MovementProfile,
) -> &'static str {
    match movement_profile.resolved_for_control_role(control_role) {
        MovementProfile::LegacyDefault => "Legacy Default",
        MovementProfile::None => "None",
        MovementProfile::PlayerWasd => "Player WASD",
    }
}

fn movement_sound_trigger_label(trigger: MovementSoundTrigger) -> &'static str {
    match trigger {
        MovementSoundTrigger::Distance => "Distance",
        MovementSoundTrigger::AnimationLoop => "Animation Loop",
    }
}

impl InspectorSystem {
    /// Renders the main inspector panel on the right side of the screen
    pub fn render_inspector_panel(
        ui_state: &mut EditorUI,
        ctx: &egui::Context,
        game_state: Option<&toki_core::GameState>,
        project: Option<&mut Project>,
        config: Option<&EditorConfig>,
    ) {
        egui::SidePanel::right("inspector_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut ui_state.right_panel_tab,
                        super::editor_ui::RightPanelTab::Inspector,
                        "Inspector",
                    );
                    ui.selectable_value(
                        &mut ui_state.right_panel_tab,
                        super::editor_ui::RightPanelTab::Project,
                        "Project",
                    );
                });
                ui.separator();

                // Wrap all inspector content in a scrollable area
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        match ui_state.right_panel_tab {
                            super::editor_ui::RightPanelTab::Inspector => {
                                Self::render_selection_inspector_contents(
                                    ui_state, ui, game_state, config,
                                );
                            }
                            super::editor_ui::RightPanelTab::Project => {
                                Self::render_project_settings_panel(
                                    ui_state, ui, project, config,
                                );
                            }
                        }
                    });
            });
    }

    fn render_selection_inspector_contents(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        game_state: Option<&toki_core::GameState>,
        config: Option<&EditorConfig>,
    ) {
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
                    }

                    if ui.button("👤 Add Entity").clicked() {
                        tracing::info!("Add Entity to scene: {}", scene_name);
                    }
                }

                if let Some(scene_index) = ui_state
                    .scenes
                    .iter()
                    .position(|scene| scene.name == *scene_name)
                {
                    ui.separator();
                    let before_rules = ui_state.scenes[scene_index].rules.clone();
                    let mut edited_rules = before_rules.clone();
                    let rules_changed =
                        Self::render_scene_rules_editor(ui, scene_name, &mut edited_rules, config);
                    if rules_changed && edited_rules != before_rules {
                        let before_graph = ui_state.rule_graph_for_scene(scene_name).cloned();
                        let after_graph = RuleGraph::from_rule_set(&edited_rules);
                        let before_layout =
                            ui_state.graph_layouts_by_scene.get(scene_name).cloned();
                        let (zoom, pan) = ui_state.graph_view_for_scene(scene_name);
                        let _ = ui_state.execute_scene_rules_graph_command(
                            scene_name,
                            SceneRulesGraphCommandData {
                                before_rule_set: before_rules,
                                after_rule_set: edited_rules,
                                before_graph,
                                after_graph,
                                before_layout,
                                zoom,
                                pan,
                            },
                        );
                    }
                }
            }
            Some(Selection::RuleGraphNode {
                scene_name,
                node_key,
            }) => {
                ui.heading("🧩 Scene Rule Node");
                ui.label(format!("Scene: {}", scene_name));
                ui.monospace(node_key);
                ui.separator();

                let changed =
                    Self::render_selected_rule_graph_node_editor(ui, ui_state, scene_name, node_key, config);

                if changed {
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
                let mut entity_changed = false;
                if ui_state.has_multi_entity_selection() {
                    ui.heading(format!("👥 {} Entities", ui_state.selected_entity_ids.len()));
                    ui.separator();
                    entity_changed = Self::render_multi_scene_entity_editor(ui, ui_state);
                } else {
                    ui.separator();
                    ui.heading(format!("👤 Entity {}", entity_id));
                    ui.separator();
                    if let Some(scene_entity) = Self::find_selected_scene_entity(ui_state, *entity_id)
                    {
                        let mut draft = EntityPropertyDraft::from_entity(&scene_entity);
                        if Self::render_scene_entity_editor(ui, &mut draft, config) {
                            entity_changed = Self::apply_entity_property_draft_with_undo(
                                ui_state, *entity_id, &draft,
                            );
                        }
                    } else {
                        ui.label("Runtime-only entity (read-only)");
                        ui.separator();
                        Self::render_runtime_entity_read_only(ui, game_state, *entity_id);
                    }
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
    }

    fn render_project_settings_panel(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: Option<&mut Project>,
        _config: Option<&EditorConfig>,
    ) {
        let Some(project) = project else {
            ui.heading("Project");
            ui.separator();
            ui.label("No project open.");
            ui.label("Open or create a project to edit project-wide settings.");
            return;
        };

        ui.heading("Project");
        ui.separator();

        let mut draft = ProjectSettingsDraft::from_project(project);
        let mut changed = false;

        ui.collapsing("General", |ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                changed |= ui.text_edit_singleline(&mut draft.name).changed();
            });
            ui.horizontal(|ui| {
                ui.label("Version:");
                changed |= ui.text_edit_singleline(&mut draft.version).changed();
            });
            ui.label("Description:");
            changed |= ui
                .add(
                    egui::TextEdit::multiline(&mut draft.description)
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                )
                .changed();
        });

        ui.separator();
        ui.collapsing("Runtime", |ui| {
            ui.horizontal(|ui| {
                ui.label("Splash Duration (ms):");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.splash_duration_ms)
                            .speed(25.0)
                            .range(0..=u64::MAX),
                    )
                    .changed();
            });
        });

        ui.separator();
        ui.collapsing("Audio", |ui| {
            ui.label("Channel loudness is global for the whole project.");
            ui.horizontal(|ui| {
                ui.label("Master:");
                changed |= ui
                    .add(
                        egui::Slider::new(&mut draft.master_mix_percent, 0..=100)
                            .suffix("%")
                            .show_value(true),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Music:");
                changed |= ui
                    .add(
                        egui::Slider::new(&mut draft.music_mix_percent, 0..=100)
                            .suffix("%")
                            .show_value(true),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Movement:");
                changed |= ui
                    .add(
                        egui::Slider::new(&mut draft.movement_mix_percent, 0..=100)
                            .suffix("%")
                            .show_value(true),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Collision:");
                changed |= ui
                    .add(
                        egui::Slider::new(&mut draft.collision_mix_percent, 0..=100)
                            .suffix("%")
                            .show_value(true),
                    )
                    .changed();
            });
        });

        ui.separator();
        ui.collapsing("Asset Paths", |ui| {
            ui.label("These are currently fixed conventions in the editor/runtime.");
            ui.horizontal(|ui| {
                ui.label("Sprites:");
                ui.monospace(&project.metadata.assets.sprites);
            });
            ui.horizontal(|ui| {
                ui.label("Tilemaps:");
                ui.monospace(&project.metadata.assets.tilemaps);
            });
            ui.horizontal(|ui| {
                ui.label("Audio:");
                ui.monospace(&project.metadata.assets.audio);
            });
        });

        ui.separator();
        ui.collapsing("Metadata", |ui| {
            ui.horizontal(|ui| {
                ui.label("Created:");
                ui.monospace(project.metadata.project.created.to_rfc3339());
            });
            ui.horizontal(|ui| {
                ui.label("Modified:");
                ui.monospace(project.metadata.project.modified.to_rfc3339());
            });
            ui.horizontal(|ui| {
                ui.label("Editor Version:");
                ui.monospace(&project.metadata.project.toki_editor_version);
            });
        });

        if changed && Self::apply_project_settings_draft(project, &draft) {
            ui_state.set_title(&project.name);
        }
    }

    fn apply_project_settings_draft(project: &mut Project, draft: &ProjectSettingsDraft) -> bool {
        let trimmed_name = draft.name.trim();
        let trimmed_version = draft.version.trim();

        let mut changed = false;
        if !trimmed_name.is_empty() && project.metadata.project.name != trimmed_name {
            project.metadata.project.name = trimmed_name.to_string();
            project.name = trimmed_name.to_string();
            changed = true;
        }
        if !trimmed_version.is_empty() && project.metadata.project.version != trimmed_version {
            project.metadata.project.version = trimmed_version.to_string();
            changed = true;
        }
        if project.metadata.project.description != draft.description {
            project.metadata.project.description = draft.description.clone();
            changed = true;
        }
        if project.metadata.runtime.splash.duration_ms != draft.splash_duration_ms {
            project.metadata.runtime.splash.duration_ms = draft.splash_duration_ms;
            changed = true;
        }
        if project.metadata.runtime.audio.master_percent != draft.master_mix_percent {
            project.metadata.runtime.audio.master_percent = draft.master_mix_percent;
            changed = true;
        }
        if project.metadata.runtime.audio.music_percent != draft.music_mix_percent {
            project.metadata.runtime.audio.music_percent = draft.music_mix_percent;
            changed = true;
        }
        if project.metadata.runtime.audio.movement_percent != draft.movement_mix_percent {
            project.metadata.runtime.audio.movement_percent = draft.movement_mix_percent;
            changed = true;
        }
        if project.metadata.runtime.audio.collision_percent != draft.collision_mix_percent {
            project.metadata.runtime.audio.collision_percent = draft.collision_mix_percent;
            changed = true;
        }

        if changed {
            project.metadata.project.modified = Utc::now();
            project.is_dirty = true;
        }

        changed
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

    fn add_condition(rule: &mut Rule, condition_kind: RuleConditionKind) {
        rule.conditions
            .push(Self::default_condition(condition_kind));
    }

    fn remove_condition(rule: &mut Rule, condition_index: usize) -> bool {
        if condition_index >= rule.conditions.len() {
            return false;
        }
        rule.conditions.remove(condition_index);
        if rule.conditions.is_empty() {
            rule.conditions.push(RuleCondition::Always);
        }
        true
    }

    fn switch_condition_kind(condition: &mut RuleCondition, condition_kind: RuleConditionKind) {
        *condition = Self::default_condition(condition_kind);
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

    fn default_condition(condition_kind: RuleConditionKind) -> RuleCondition {
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

    fn condition_kind(condition: &RuleCondition) -> RuleConditionKind {
        match condition {
            RuleCondition::Always => RuleConditionKind::Always,
            RuleCondition::TargetExists { .. } => RuleConditionKind::TargetExists,
            RuleCondition::KeyHeld { .. } => RuleConditionKind::KeyHeld,
            RuleCondition::EntityActive { .. } => RuleConditionKind::EntityActive,
        }
    }

    fn condition_kind_label(condition_kind: RuleConditionKind) -> &'static str {
        match condition_kind {
            RuleConditionKind::Always => "Always",
            RuleConditionKind::TargetExists => "TargetExists",
            RuleConditionKind::KeyHeld => "KeyHeld",
            RuleConditionKind::EntityActive => "EntityActive",
        }
    }

    fn action_kind(action: &RuleAction) -> RuleActionKind {
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

    fn action_kind_label(action_kind: RuleActionKind) -> &'static str {
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

    fn spawn_entity_type_label(entity_type: RuleSpawnEntityType) -> &'static str {
        match entity_type {
            RuleSpawnEntityType::PlayerLikeNpc => "PlayerLikeNpc",
            RuleSpawnEntityType::Npc => "Npc",
            RuleSpawnEntityType::Item => "Item",
            RuleSpawnEntityType::Decoration => "Decoration",
            RuleSpawnEntityType::Trigger => "Trigger",
        }
    }

    fn trigger_kind(trigger: &RuleTrigger) -> RuleTriggerKind {
        match trigger {
            RuleTrigger::OnStart => RuleTriggerKind::Start,
            RuleTrigger::OnUpdate => RuleTriggerKind::Update,
            RuleTrigger::OnPlayerMove => RuleTriggerKind::PlayerMove,
            RuleTrigger::OnKey { .. } => RuleTriggerKind::Key,
            RuleTrigger::OnCollision => RuleTriggerKind::Collision,
            RuleTrigger::OnTrigger => RuleTriggerKind::Trigger,
        }
    }

    fn trigger_kind_label(kind: RuleTriggerKind) -> &'static str {
        match kind {
            RuleTriggerKind::Start => "OnStart",
            RuleTriggerKind::Update => "OnUpdate",
            RuleTriggerKind::PlayerMove => "OnPlayerMove",
            RuleTriggerKind::Key => "OnKey",
            RuleTriggerKind::Collision => "OnCollision",
            RuleTriggerKind::Trigger => "OnTrigger",
        }
    }

    fn set_rule_trigger_kind(rule: &mut Rule, kind: RuleTriggerKind) {
        rule.trigger = match kind {
            RuleTriggerKind::Start => RuleTrigger::OnStart,
            RuleTriggerKind::Update => RuleTrigger::OnUpdate,
            RuleTriggerKind::PlayerMove => RuleTrigger::OnPlayerMove,
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

    fn save_entity_definition(
        definition: &toki_core::entity::EntityDefinition,
        path: &std::path::Path,
    ) -> Result<(), String> {
        let content = serde_json::to_string_pretty(definition)
            .map_err(|err| format!("failed to serialize entity definition: {err}"))?;
        std::fs::write(path, content)
            .map_err(|err| format!("failed to write entity definition '{}': {err}", path.display()))
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

    fn render_selected_rule_graph_node_editor(
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

    fn rule_graph_node_badges(graph: &RuleGraph) -> HashMap<u64, String> {
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

    fn rule_graph_node_label_for_inspector(
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

    fn rule_graph_trigger_summary(trigger: RuleTrigger) -> String {
        match trigger {
            RuleTrigger::OnStart => "OnStart".to_string(),
            RuleTrigger::OnUpdate => "OnUpdate".to_string(),
            RuleTrigger::OnPlayerMove => "OnPlayerMove".to_string(),
            RuleTrigger::OnKey { key } => format!("OnKey({})", Self::rule_key_label(key)),
            RuleTrigger::OnCollision => "OnCollision".to_string(),
            RuleTrigger::OnTrigger => "OnTrigger".to_string(),
        }
    }

    fn rule_graph_condition_summary(condition: RuleCondition) -> String {
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

    fn rule_graph_action_summary(action: &RuleAction) -> String {
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

    fn render_rule_graph_trigger_editor(
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

    fn render_rule_graph_condition_editor(
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

    fn render_rule_graph_action_editor(
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

    fn render_rule_target_editor_with_salt(
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

    fn render_rule_key_editor_with_salt(
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

    fn render_rule_condition_editor(
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

    fn render_rule_condition_target_editor(
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

    fn render_scene_entity_editor(
        ui: &mut egui::Ui,
        draft: &mut EntityPropertyDraft,
        config: Option<&EditorConfig>,
    ) -> bool {
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
        ui.horizontal(|ui| {
            ui.label("Control Role:");
            egui::ComboBox::from_id_salt("entity_control_role")
                .selected_text(control_role_label(draft.control_role))
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(&mut draft.control_role, ControlRole::None, "None")
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut draft.control_role,
                            ControlRole::PlayerCharacter,
                            "Player Character",
                        )
                        .changed();
                });
        });
        ui.horizontal(|ui| {
            ui.label("Movement:");
            egui::ComboBox::from_id_salt("entity_movement_profile")
                .selected_text(movement_profile_label(
                    draft.control_role,
                    draft.movement_profile,
                ))
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(
                            &mut draft.movement_profile,
                            MovementProfile::None,
                            "None",
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut draft.movement_profile,
                            MovementProfile::PlayerWasd,
                            "Player WASD",
                        )
                        .changed();
                });
        });
        ui.horizontal(|ui| {
            ui.label("AI:");
            egui::ComboBox::from_id_salt("entity_ai_behavior")
                .selected_text(ai_behavior_label(draft.ai_behavior))
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(&mut draft.ai_behavior, AiBehavior::None, "None")
                        .changed();
                    changed |= ui
                        .selectable_value(&mut draft.ai_behavior, AiBehavior::Wander, "Wander")
                        .changed();
                });
        });
        changed |= ui
            .checkbox(&mut draft.has_inventory, "Has Inventory")
            .changed();

        ui.separator();
        ui.label("Audio");
        ui.horizontal(|ui| {
            ui.label("Movement Trigger:");
            egui::ComboBox::from_id_salt("entity_movement_sound_trigger")
                .selected_text(movement_sound_trigger_label(draft.movement_sound_trigger))
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(
                            &mut draft.movement_sound_trigger,
                            MovementSoundTrigger::Distance,
                            "Distance",
                        )
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut draft.movement_sound_trigger,
                            MovementSoundTrigger::AnimationLoop,
                            "Animation Loop",
                        )
                        .changed();
                });
        });
        let uses_distance_trigger = matches!(
            draft.movement_sound_trigger,
            MovementSoundTrigger::Distance
        );
        ui.horizontal(|ui| {
            ui.label("Footstep Distance:");
            ui.add_enabled_ui(uses_distance_trigger, |ui| {
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.footstep_trigger_distance)
                            .speed(0.5)
                            .range(0.0..=f32::MAX),
                    )
                    .changed();
            });
        });
        ui.horizontal(|ui| {
            ui.label("Movement Sound:");
            let mut sfx_names = config
                .and_then(|cfg| cfg.current_project_path())
                .map(|project_path| {
                    Self::discover_audio_asset_names(project_path.join("assets/audio/sfx").as_path())
                })
                .unwrap_or_default();
            if !draft.movement_sound.trim().is_empty()
                && !sfx_names.iter().any(|name| name == &draft.movement_sound)
            {
                sfx_names.push(draft.movement_sound.clone());
                sfx_names.sort();
                sfx_names.dedup();
            }
            egui::ComboBox::from_id_salt("entity_movement_sound")
                .selected_text(if draft.movement_sound.trim().is_empty() {
                    "None".to_string()
                } else {
                    draft.movement_sound.clone()
                })
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(&mut draft.movement_sound, String::new(), "None")
                        .changed();
                    for sound_name in &sfx_names {
                        changed |= ui
                            .selectable_value(
                                &mut draft.movement_sound,
                                sound_name.clone(),
                                sound_name,
                            )
                            .changed();
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Hearing Radius:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut draft.hearing_radius)
                        .speed(1.0)
                        .range(0..=u32::MAX),
                )
                .changed();
        });

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

    fn render_multi_scene_entity_editor(ui: &mut egui::Ui, ui_state: &mut EditorUI) -> bool {
        let Some(active_scene_name) = ui_state.active_scene.clone() else {
            ui.label("No active scene");
            return false;
        };

        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|scene| scene.name == active_scene_name)
        else {
            ui.label("Active scene not found");
            return false;
        };

        let selected_ids = ui_state.selected_entity_ids.clone();
        let selected_set: HashSet<_> = selected_ids.iter().copied().collect();
        let selected_entities = {
            let scene = &ui_state.scenes[scene_index];
            scene
                .entities
                .iter()
                .filter(|entity| selected_set.contains(&entity.id))
                .collect::<Vec<_>>()
        };

        if selected_entities.len() < 2 {
            ui.label("Select at least two scene entities for batch editing.");
            return false;
        }

        let common = Self::collect_multi_entity_common_state(&selected_entities);
        if ui_state.multi_entity_inspector_selection_signature != selected_ids {
            ui_state.multi_entity_inspector_selection_signature = selected_ids;
            ui_state.multi_entity_render_layer_input = common.render_layer.unwrap_or(0) as i64;
            ui_state.multi_entity_delta_x_input = 0;
            ui_state.multi_entity_delta_y_input = 0;
        }

        ui.label("Batch edit selected scene entities");
        ui.horizontal(|ui| {
            ui.label("Entities:");
            ui.label(selected_entities.len().to_string());
        });
        ui.separator();

        let mut edit = MultiEntityBatchEdit::default();
        Self::render_multi_entity_bool_row(
            ui,
            "Visible",
            common.visible,
            &mut edit.set_visible,
            "Set Visible",
            "Set Hidden",
        );
        Self::render_multi_entity_bool_row(
            ui,
            "Active",
            common.active,
            &mut edit.set_active,
            "Set Active",
            "Set Inactive",
        );
        Self::render_multi_entity_bool_row(
            ui,
            "Collision",
            common.collision_enabled,
            &mut edit.set_collision_enabled,
            "Enable Collision",
            "Disable Collision",
        );

        ui.horizontal(|ui| {
            ui.label(format!(
                "Render Layer: {}",
                common
                    .render_layer
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "Mixed".to_string())
            ));
            ui.add(egui::DragValue::new(&mut ui_state.multi_entity_render_layer_input).speed(1.0));
            if ui.button("Apply Layer").clicked() {
                edit.set_render_layer = Some(
                    ui_state
                        .multi_entity_render_layer_input
                        .clamp(i32::MIN as i64, i32::MAX as i64) as i32,
                );
            }
        });

        ui.horizontal(|ui| {
            ui.label("Position Delta:");
            ui.add(egui::DragValue::new(&mut ui_state.multi_entity_delta_x_input).speed(1.0));
            ui.add(egui::DragValue::new(&mut ui_state.multi_entity_delta_y_input).speed(1.0));
            if ui.button("Apply Delta").clicked() {
                let delta = glam::IVec2::new(
                    ui_state.multi_entity_delta_x_input,
                    ui_state.multi_entity_delta_y_input,
                );
                if delta != glam::IVec2::ZERO {
                    edit.position_delta = Some(delta);
                }
                ui_state.multi_entity_delta_x_input = 0;
                ui_state.multi_entity_delta_y_input = 0;
            }
        });

        if edit.is_noop() {
            return false;
        }

        Self::apply_multi_entity_batch_edit_with_undo(
            ui_state,
            &active_scene_name,
            &selected_set,
            edit,
        )
    }

    fn render_multi_entity_bool_row(
        ui: &mut egui::Ui,
        label: &str,
        common_value: Option<bool>,
        out_edit: &mut Option<bool>,
        true_button: &str,
        false_button: &str,
    ) {
        ui.horizontal(|ui| {
            let state_text = match common_value {
                Some(true) => "true",
                Some(false) => "false",
                None => "mixed",
            };
            ui.label(format!("{label}: {state_text}"));
            if ui.button(true_button).clicked() {
                *out_edit = Some(true);
            }
            if ui.button(false_button).clicked() {
                *out_edit = Some(false);
            }
        });
    }

    fn collect_multi_entity_common_state(
        entities: &[&toki_core::entity::Entity],
    ) -> MultiEntityCommonState {
        fn common_bool(
            entities: &[&toki_core::entity::Entity],
            accessor: impl Fn(&toki_core::entity::Entity) -> bool,
        ) -> Option<bool> {
            let first = entities.first().map(|entity| accessor(entity))?;
            if entities.iter().all(|entity| accessor(entity) == first) {
                Some(first)
            } else {
                None
            }
        }

        fn common_i32(
            entities: &[&toki_core::entity::Entity],
            accessor: impl Fn(&toki_core::entity::Entity) -> i32,
        ) -> Option<i32> {
            let first = entities.first().map(|entity| accessor(entity))?;
            if entities.iter().all(|entity| accessor(entity) == first) {
                Some(first)
            } else {
                None
            }
        }

        MultiEntityCommonState {
            visible: common_bool(entities, |entity| entity.attributes.visible),
            active: common_bool(entities, |entity| entity.attributes.active),
            collision_enabled: common_bool(entities, |entity| entity.collision_box.is_some()),
            render_layer: common_i32(entities, |entity| entity.attributes.render_layer),
        }
    }

    fn apply_multi_entity_batch_edit_with_undo(
        ui_state: &mut EditorUI,
        scene_name: &str,
        selected_set: &HashSet<toki_core::entity::EntityId>,
        edit: MultiEntityBatchEdit,
    ) -> bool {
        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|scene| scene.name == scene_name)
        else {
            return false;
        };

        let before_entities = ui_state.scenes[scene_index]
            .entities
            .iter()
            .filter(|entity| selected_set.contains(&entity.id))
            .cloned()
            .collect::<Vec<_>>();

        if before_entities.is_empty() {
            return false;
        }

        let mut changed = false;
        let mut after_entities = Vec::with_capacity(before_entities.len());
        for before_entity in &before_entities {
            let mut after_entity = before_entity.clone();
            changed |= Self::apply_multi_entity_batch_edit_to_entity(&mut after_entity, edit);
            after_entities.push(after_entity);
        }

        if !changed {
            return false;
        }

        ui_state.execute_command(EditorCommand::update_entities(
            scene_name.to_string(),
            before_entities,
            after_entities,
        ))
    }

    fn apply_multi_entity_batch_edit_to_entity(
        entity: &mut toki_core::entity::Entity,
        edit: MultiEntityBatchEdit,
    ) -> bool {
        let mut changed = false;

        if let Some(visible) = edit.set_visible {
            if entity.attributes.visible != visible {
                entity.attributes.visible = visible;
                changed = true;
            }
        }

        if let Some(active) = edit.set_active {
            if entity.attributes.active != active {
                entity.attributes.active = active;
                changed = true;
            }
        }

        if let Some(render_layer) = edit.set_render_layer {
            if entity.attributes.render_layer != render_layer {
                entity.attributes.render_layer = render_layer;
                changed = true;
            }
        }

        if let Some(delta) = edit.position_delta {
            let new_position = entity.position + delta;
            if entity.position != new_position {
                entity.position = new_position;
                changed = true;
            }
        }

        if let Some(collision_enabled) = edit.set_collision_enabled {
            if collision_enabled {
                if entity.collision_box.is_none() {
                    entity.collision_box =
                        Some(toki_core::collision::CollisionBox::solid_box(entity.size));
                    changed = true;
                }
            } else if entity.collision_box.is_some() {
                entity.collision_box = None;
                changed = true;
            }
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
                    if entity.category.is_empty() {
                        ui.label(format!("{:?}", entity.entity_kind));
                    } else {
                        ui.label(entity.category.as_str());
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Control Role:");
                    ui.label(control_role_label(entity.control_role));
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
                ui.horizontal(|ui| {
                    ui.label("AI:");
                    ui.label(ai_behavior_label(entity.attributes.ai_behavior));
                });
                ui.horizontal(|ui| {
                    ui.label("Movement:");
                    ui.label(movement_profile_label(
                        entity.control_role,
                        entity.attributes.movement_profile,
                    ));
                });

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

    fn find_selected_scene_entity(
        ui_state: &EditorUI,
        entity_id: toki_core::entity::EntityId,
    ) -> Option<toki_core::entity::Entity> {
        let active_scene_name = ui_state.active_scene.clone()?;
        let scene = ui_state
            .scenes
            .iter()
            .find(|scene| scene.name == active_scene_name)?;
        scene
            .entities
            .iter()
            .find(|entity| entity.id == entity_id)
            .cloned()
    }

    fn apply_entity_property_draft_with_undo(
        ui_state: &mut EditorUI,
        entity_id: toki_core::entity::EntityId,
        draft: &EntityPropertyDraft,
    ) -> bool {
        let Some(active_scene_name) = ui_state.active_scene.clone() else {
            return false;
        };
        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|scene| scene.name == active_scene_name)
        else {
            return false;
        };
        let Some(entity_index) = ui_state.scenes[scene_index]
            .entities
            .iter()
            .position(|entity| entity.id == entity_id)
        else {
            return false;
        };

        let before = ui_state.scenes[scene_index].entities[entity_index].clone();
        let mut after = before.clone();
        let mut changed = Self::apply_entity_property_draft(&mut after, draft);

        let mut before_entities = vec![before];
        let mut after_entities = vec![after.clone()];

        if matches!(after.control_role, ControlRole::PlayerCharacter) {
            for other in ui_state.scenes[scene_index].entities.iter() {
                if other.id == entity_id {
                    continue;
                }
                if matches!(
                    other.effective_control_role(),
                    toki_core::entity::ControlRole::PlayerCharacter
                ) {
                    let mut demoted = other.clone();
                    demoted.control_role = ControlRole::None;
                    before_entities.push(other.clone());
                    after_entities.push(demoted);
                    changed = true;
                }
            }
        }

        if !changed {
            return false;
        }

        ui_state.execute_command(EditorCommand::update_entities(
            active_scene_name,
            before_entities,
            after_entities,
        ))
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
        changed |= set_if_changed(&mut entity.control_role, draft.control_role);
        changed |= set_if_changed(&mut entity.attributes.ai_behavior, draft.ai_behavior);
        changed |= set_if_changed(
            &mut entity.attributes.movement_profile,
            draft.movement_profile,
        );
        changed |= set_if_changed(
            &mut entity.audio.movement_sound_trigger,
            draft.movement_sound_trigger,
        );
        changed |= set_if_changed(
            &mut entity.audio.footstep_trigger_distance,
            draft.footstep_trigger_distance.max(0.0),
        );
        changed |= set_if_changed(&mut entity.audio.hearing_radius, draft.hearing_radius);
        let new_movement_sound = {
            let trimmed = draft.movement_sound.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        };
        changed |= set_if_changed(&mut entity.audio.movement_sound, new_movement_sound);
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
                            if let Ok(mut definition) =
                                serde_json::from_str::<toki_core::entity::EntityDefinition>(&content)
                            {
                                ui.separator();
                                ui.label("Audio Settings:");

                                ui.horizontal(|ui| {
                                    ui.label("Movement Trigger:");
                                    let mut changed = false;
                                    egui::ComboBox::from_id_salt(format!(
                                        "entity_def_movement_trigger_{}",
                                        entity_name
                                    ))
                                    .selected_text(movement_sound_trigger_label(
                                        definition.audio.movement_sound_trigger,
                                    ))
                                    .show_ui(ui, |ui| {
                                        changed |= ui
                                            .selectable_value(
                                                &mut definition.audio.movement_sound_trigger,
                                                MovementSoundTrigger::Distance,
                                                "Distance",
                                            )
                                            .changed();
                                        changed |= ui
                                            .selectable_value(
                                                &mut definition.audio.movement_sound_trigger,
                                                MovementSoundTrigger::AnimationLoop,
                                                "Animation Loop",
                                            )
                                            .changed();
                                    });
                                    if changed {
                                        if let Err(err) =
                                            Self::save_entity_definition(&definition, &entity_file)
                                        {
                                            tracing::error!("{}", err);
                                            ui.colored_label(egui::Color32::RED, err);
                                        }
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Footstep Distance:");
                                    let mut changed = false;
                                    ui.add_enabled_ui(
                                        matches!(
                                            definition.audio.movement_sound_trigger,
                                            MovementSoundTrigger::Distance
                                        ),
                                        |ui| {
                                            changed |= ui
                                                .add(
                                                    egui::DragValue::new(
                                                        &mut definition.audio
                                                            .footstep_trigger_distance,
                                                    )
                                                    .speed(0.5)
                                                    .range(0.0..=f32::MAX),
                                                )
                                                .changed();
                                        },
                                    );
                                    if changed {
                                        definition.audio.footstep_trigger_distance =
                                            definition.audio.footstep_trigger_distance.max(0.0);
                                        if let Err(err) =
                                            Self::save_entity_definition(&definition, &entity_file)
                                        {
                                            tracing::error!("{}", err);
                                            ui.colored_label(egui::Color32::RED, err);
                                        }
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Hearing Radius:");
                                    let mut changed = false;
                                    changed |= ui
                                        .add(
                                            egui::DragValue::new(
                                                &mut definition.audio.hearing_radius,
                                            )
                                            .speed(1.0)
                                            .range(0..=u32::MAX),
                                        )
                                        .changed();
                                    if changed {
                                        if let Err(err) =
                                            Self::save_entity_definition(&definition, &entity_file)
                                        {
                                            tracing::error!("{}", err);
                                            ui.colored_label(egui::Color32::RED, err);
                                        }
                                    }
                                });

                                let mut movement_sound_options = Self::discover_audio_asset_names(
                                    project_path.join("assets/audio/sfx").as_path(),
                                );
                                if !definition.audio.movement_sound.trim().is_empty()
                                    && !movement_sound_options
                                        .iter()
                                        .any(|name| name == &definition.audio.movement_sound)
                                {
                                    movement_sound_options
                                        .push(definition.audio.movement_sound.clone());
                                    movement_sound_options.sort();
                                    movement_sound_options.dedup();
                                }

                                ui.horizontal(|ui| {
                                    ui.label("Movement Sound:");
                                    let selected_text = if definition.audio.movement_sound.trim().is_empty() {
                                        "None".to_string()
                                    } else {
                                        definition.audio.movement_sound.clone()
                                    };
                                    let mut changed = false;
                                    egui::ComboBox::from_id_salt(format!(
                                        "entity_def_movement_sound_{}",
                                        entity_name
                                    ))
                                    .selected_text(selected_text)
                                    .show_ui(ui, |ui| {
                                        changed |= ui
                                            .selectable_value(
                                                &mut definition.audio.movement_sound,
                                                String::new(),
                                                "None",
                                            )
                                            .changed();
                                        for sound_name in &movement_sound_options {
                                            changed |= ui
                                                .selectable_value(
                                                    &mut definition.audio.movement_sound,
                                                    sound_name.clone(),
                                                    sound_name,
                                                )
                                                .changed();
                                        }
                                    });
                                    if changed {
                                        if let Err(err) =
                                            Self::save_entity_definition(&definition, &entity_file)
                                        {
                                            tracing::error!("{}", err);
                                            ui.colored_label(egui::Color32::RED, err);
                                        }
                                    }
                                });
                            }

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
    use super::{
        AiBehavior, EntityPropertyDraft, InspectorSystem, MovementProfile, MultiEntityBatchEdit,
        ProjectSettingsDraft, RuleActionKind, RuleConditionKind, RuleTriggerKind,
    };
    use crate::project::Project;
    use crate::ui::EditorUI;
    use glam::{IVec2, UVec2};
    use std::fs;
    use toki_core::animation::AnimationState;
    use toki_core::collision::CollisionBox;
    use toki_core::entity::{
        ControlRole, EntityAttributes, EntityKind, EntityManager, MovementSoundTrigger,
    };
    use toki_core::rules::{
        Rule, RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel, RuleSpawnEntityType,
        RuleTarget, RuleTrigger,
    };
    use toki_core::Scene;

    fn sample_entity_with_id(id: u32) -> toki_core::entity::Entity {
        let mut manager = EntityManager::new();
        let spawned_id = manager.spawn_entity(
            EntityKind::Npc,
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
                ai_behavior: AiBehavior::Wander,
                movement_profile: MovementProfile::LegacyDefault,
                has_inventory: false,
            },
        );
        let mut entity = manager
            .get_entity(spawned_id)
            .expect("missing spawned entity")
            .clone();
        entity.id = id;
        entity.category = "creature".to_string();
        entity.control_role = ControlRole::None;
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
        draft.control_role = ControlRole::PlayerCharacter;
        draft.ai_behavior = AiBehavior::None;
        draft.movement_profile = MovementProfile::PlayerWasd;
        draft.movement_sound_trigger = MovementSoundTrigger::AnimationLoop;
        draft.footstep_trigger_distance = -5.0;
        draft.movement_sound = "sfx_custom_step".to_string();
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
        assert_eq!(entity.control_role, ControlRole::PlayerCharacter);
        assert_eq!(entity.attributes.ai_behavior, AiBehavior::None);
        assert_eq!(
            entity.attributes.movement_profile,
            MovementProfile::PlayerWasd
        );
        assert_eq!(
            entity.audio.movement_sound_trigger,
            MovementSoundTrigger::AnimationLoop
        );
        assert_eq!(entity.audio.footstep_trigger_distance, 0.0);
        assert_eq!(entity.audio.movement_sound.as_deref(), Some("sfx_custom_step"));
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
    fn apply_project_settings_draft_updates_metadata_and_marks_project_dirty() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let mut project = Project::new("Demo".to_string(), temp_dir.path().join("Demo"));
        let original_modified = project.metadata.project.modified;
        let draft = ProjectSettingsDraft {
            name: "Renamed Demo".to_string(),
            version: "2.0.0".to_string(),
            description: "Updated description".to_string(),
            splash_duration_ms: 4500,
            master_mix_percent: 85,
            music_mix_percent: 70,
            movement_mix_percent: 55,
            collision_mix_percent: 35,
        };

        let changed = InspectorSystem::apply_project_settings_draft(&mut project, &draft);

        assert!(changed);
        assert_eq!(project.name, "Renamed Demo");
        assert_eq!(project.metadata.project.name, "Renamed Demo");
        assert_eq!(project.metadata.project.version, "2.0.0");
        assert_eq!(project.metadata.project.description, "Updated description");
        assert_eq!(project.metadata.runtime.splash.duration_ms, 4500);
        assert_eq!(project.metadata.runtime.audio.master_percent, 85);
        assert_eq!(project.metadata.runtime.audio.music_percent, 70);
        assert_eq!(project.metadata.runtime.audio.movement_percent, 55);
        assert_eq!(project.metadata.runtime.audio.collision_percent, 35);
        assert!(project.is_dirty);
        assert!(project.metadata.project.modified >= original_modified);
    }

    #[test]
    fn collect_multi_entity_common_state_reports_mixed_values() {
        let mut first = sample_entity_with_id(1);
        let mut second = sample_entity_with_id(2);

        first.attributes.visible = true;
        second.attributes.visible = false;
        first.attributes.active = true;
        second.attributes.active = true;
        first.attributes.render_layer = 2;
        second.attributes.render_layer = 2;
        second.collision_box = None;

        let entities = vec![&first, &second];
        let common = InspectorSystem::collect_multi_entity_common_state(&entities);

        assert_eq!(common.visible, None);
        assert_eq!(common.active, Some(true));
        assert_eq!(common.render_layer, Some(2));
        assert_eq!(common.collision_enabled, None);
    }

    #[test]
    fn apply_multi_entity_batch_edit_updates_all_selected_entities() {
        let mut first = sample_entity_with_id(1);
        let mut second = sample_entity_with_id(2);
        second.collision_box = None;

        let edit = MultiEntityBatchEdit {
            set_visible: Some(false),
            set_active: Some(false),
            set_collision_enabled: Some(true),
            set_render_layer: Some(7),
            position_delta: Some(IVec2::new(2, -3)),
        };
        let changed = InspectorSystem::apply_multi_entity_batch_edit_to_entity(&mut first, edit)
            | InspectorSystem::apply_multi_entity_batch_edit_to_entity(&mut second, edit);

        assert!(changed);
        assert!(!first.attributes.visible);
        assert!(!second.attributes.visible);
        assert!(!first.attributes.active);
        assert!(!second.attributes.active);
        assert_eq!(first.attributes.render_layer, 7);
        assert_eq!(second.attributes.render_layer, 7);
        assert_eq!(first.position, IVec2::new(12, 17));
        assert_eq!(second.position, IVec2::new(12, 17));
        assert!(first.collision_box.is_some());
        assert!(second.collision_box.is_some());
    }

    #[test]
    fn find_selected_scene_entity_returns_entity_from_active_scene() {
        let mut ui_state = EditorUI::new();
        let entity = sample_entity_with_id(7);
        let scene = ui_state
            .scenes
            .iter_mut()
            .find(|scene| scene.name == "Main Scene")
            .expect("missing default scene");
        scene.entities.push(entity);

        let selected_entity = InspectorSystem::find_selected_scene_entity(&ui_state, 7)
            .expect("entity should be found");
        assert_eq!(selected_entity.id, 7);
        assert_eq!(selected_entity.position, IVec2::new(10, 20));
    }

    #[test]
    fn find_selected_scene_entity_returns_none_for_inactive_scene() {
        let mut ui_state = EditorUI::new();
        ui_state.scenes.push(Scene::new("Other".to_string()));
        ui_state.active_scene = Some("Other".to_string());

        let scene = ui_state
            .scenes
            .iter_mut()
            .find(|scene| scene.name == "Main Scene")
            .expect("missing default scene");
        scene.entities.push(sample_entity_with_id(42));

        assert!(InspectorSystem::find_selected_scene_entity(&ui_state, 42).is_none());
    }

    #[test]
    fn apply_entity_property_draft_with_undo_round_trips() {
        let mut ui_state = EditorUI::new();
        let entity = sample_entity_with_id(7);
        let scene = ui_state
            .scenes
            .iter_mut()
            .find(|scene| scene.name == "Main Scene")
            .expect("missing default scene");
        scene.entities.push(entity.clone());

        let mut draft = EntityPropertyDraft::from_entity(&entity);
        draft.position_x = 99;
        draft.position_y = -8;
        draft.visible = false;

        assert!(InspectorSystem::apply_entity_property_draft_with_undo(
            &mut ui_state,
            7,
            &draft
        ));
        let edited = InspectorSystem::find_selected_scene_entity(&ui_state, 7)
            .expect("entity should still exist");
        assert_eq!(edited.position, IVec2::new(99, -8));
        assert!(!edited.attributes.visible);

        assert!(ui_state.undo());
        let restored = InspectorSystem::find_selected_scene_entity(&ui_state, 7)
            .expect("entity should still exist");
        assert_eq!(restored.position, IVec2::new(10, 20));
        assert!(restored.attributes.visible);
    }

    #[test]
    fn apply_entity_property_draft_with_undo_enforces_single_player_character() {
        let mut ui_state = EditorUI::new();
        let mut first = sample_entity_with_id(1);
        first.control_role = ControlRole::PlayerCharacter;
        let second = sample_entity_with_id(2);
        let scene = ui_state
            .scenes
            .iter_mut()
            .find(|scene| scene.name == "Main Scene")
            .expect("missing default scene");
        scene.entities.push(first);
        scene.entities.push(second.clone());

        let mut draft = EntityPropertyDraft::from_entity(&second);
        draft.control_role = ControlRole::PlayerCharacter;

        assert!(InspectorSystem::apply_entity_property_draft_with_undo(
            &mut ui_state,
            2,
            &draft,
        ));

        let scene = ui_state
            .scenes
            .iter()
            .find(|scene| scene.name == "Main Scene")
            .expect("missing default scene");
        let first = scene
            .entities
            .iter()
            .find(|entity| entity.id == 1)
            .expect("first entity should exist");
        let second = scene
            .entities
            .iter()
            .find(|entity| entity.id == 2)
            .expect("second entity should exist");

        assert_eq!(first.control_role, ControlRole::None);
        assert_eq!(second.control_role, ControlRole::PlayerCharacter);
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

        InspectorSystem::set_rule_trigger_kind(&mut rule, RuleTriggerKind::PlayerMove);
        assert_eq!(rule.trigger, RuleTrigger::OnPlayerMove);

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
        InspectorSystem::add_action(&mut rule, RuleActionKind::Spawn);
        InspectorSystem::add_action(&mut rule, RuleActionKind::DestroySelf);
        InspectorSystem::add_action(&mut rule, RuleActionKind::SwitchScene);
        assert_eq!(rule.actions.len(), 7);
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
            RuleAction::Spawn {
                entity_type: RuleSpawnEntityType::Npc,
                position: [0, 0]
            }
        ));
        assert!(matches!(
            rule.actions[5],
            RuleAction::DestroySelf {
                target: RuleTarget::Player
            }
        ));
        assert!(matches!(
            rule.actions[6],
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
        InspectorSystem::switch_action_kind(&mut rule.actions[0], RuleActionKind::DestroySelf);
        assert!(matches!(
            rule.actions[0],
            RuleAction::DestroySelf {
                target: RuleTarget::Player
            }
        ));

        assert!(InspectorSystem::remove_action(&mut rule, 1));
        assert_eq!(rule.actions.len(), 6);
        assert!(!InspectorSystem::remove_action(&mut rule, 99));
    }

    #[test]
    fn add_remove_and_switch_condition_types() {
        let mut rule = sample_rule("rule_1");
        assert_eq!(rule.conditions, vec![RuleCondition::Always]);

        InspectorSystem::add_condition(&mut rule, RuleConditionKind::TargetExists);
        InspectorSystem::add_condition(&mut rule, RuleConditionKind::KeyHeld);
        InspectorSystem::add_condition(&mut rule, RuleConditionKind::EntityActive);

        assert_eq!(rule.conditions.len(), 4);
        assert!(matches!(
            rule.conditions[1],
            RuleCondition::TargetExists {
                target: RuleTarget::Player
            }
        ));
        assert!(matches!(
            rule.conditions[2],
            RuleCondition::KeyHeld { key: RuleKey::Up }
        ));
        assert!(matches!(
            rule.conditions[3],
            RuleCondition::EntityActive {
                target: RuleTarget::Player,
                is_active: true
            }
        ));

        InspectorSystem::switch_condition_kind(
            &mut rule.conditions[0],
            RuleConditionKind::EntityActive,
        );
        assert!(matches!(
            rule.conditions[0],
            RuleCondition::EntityActive {
                target: RuleTarget::Player,
                is_active: true
            }
        ));

        assert!(InspectorSystem::remove_condition(&mut rule, 2));
        assert_eq!(rule.conditions.len(), 3);
        assert!(!InspectorSystem::remove_condition(&mut rule, 99));
    }

    #[test]
    fn validate_rule_set_reports_duplicate_ids_and_invalid_action_payloads() {
        let mut first = sample_rule("dupe");
        first.conditions = vec![RuleCondition::TargetExists {
            target: RuleTarget::Entity(0),
        }];
        first.actions = vec![
            RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "   ".to_string(),
            },
            RuleAction::SetVelocity {
                target: RuleTarget::Entity(0),
                velocity: [1, 0],
            },
            RuleAction::DestroySelf {
                target: RuleTarget::Entity(0),
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
        assert!(issues.iter().any(|issue| issue
            .message
            .contains("DestroySelf entity target must be non-zero")));
        assert!(issues.iter().any(|issue| issue
            .message
            .contains("Condition 1 entity target must be non-zero")));
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

    #[test]
    fn save_entity_definition_persists_audio_updates() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let entity_file = temp_dir.path().join("player.json");

        let definition = toki_core::entity::EntityDefinition {
            name: "player".to_string(),
            display_name: "Player".to_string(),
            description: "desc".to_string(),
            rendering: toki_core::entity::RenderingDef {
                size: [16, 16],
                render_layer: 0,
                visible: true,
            },
            attributes: toki_core::entity::AttributesDef {
                health: Some(100),
                speed: 2,
                solid: true,
                active: true,
                can_move: true,
                ai_behavior: AiBehavior::None,
                movement_profile: MovementProfile::PlayerWasd,
                has_inventory: false,
            },
            collision: toki_core::entity::CollisionDef {
                enabled: true,
                offset: [0, 0],
                size: [16, 16],
                trigger: false,
            },
            audio: toki_core::entity::AudioDef {
                footstep_trigger_distance: 42.0,
                hearing_radius: 144,
                movement_sound_trigger: MovementSoundTrigger::AnimationLoop,
                movement_sound: "sfx_step".to_string(),
                collision_sound: None,
            },
            animations: toki_core::entity::AnimationsDef {
                atlas_name: "players".to_string(),
                clips: vec![],
                default_state: "idle".to_string(),
            },
            category: "human".to_string(),
            tags: vec![],
        };

        InspectorSystem::save_entity_definition(&definition, &entity_file)
            .expect("entity definition should save");

        let content =
            fs::read_to_string(&entity_file).expect("saved entity definition should be readable");
        let reloaded: toki_core::entity::EntityDefinition =
            serde_json::from_str(&content).expect("saved entity definition should parse");

        assert_eq!(reloaded.audio.footstep_trigger_distance, 42.0);
        assert_eq!(reloaded.audio.hearing_radius, 144);
        assert_eq!(
            reloaded.audio.movement_sound_trigger,
            MovementSoundTrigger::AnimationLoop
        );
        assert_eq!(reloaded.audio.movement_sound, "sfx_step");
    }
}
