use super::editor_ui::{EditorUI, MapEditorTool, SceneRulesGraphCommandData, Selection};
use super::rule_graph::{RuleGraph, RuleGraphNodeKind};
use super::undo_redo::EditorCommand;
use crate::config::EditorConfig;
use crate::project::Project;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use toki_core::animation::AnimationState;
use toki_core::assets::object_sheet::ObjectSheetMeta;
use toki_core::entity::{
    AiBehavior, ControlRole, MovementProfile, MovementSoundTrigger, ATTACK_POWER_STAT_ID,
    HEALTH_STAT_ID,
};
use toki_core::rules::{
    Rule, RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel, RuleSpawnEntityType,
    RuleTarget, RuleTrigger,
};

mod assets;
mod entities;
mod map_editor;
mod menu_editor;
mod project;
mod rules;

/// Handles inspector panel rendering for assets and entities
pub struct InspectorSystem;

#[derive(Debug, Clone)]
struct EntityPropertyDraft {
    category: String,
    static_object_sheet: Option<String>,
    static_object_name: Option<String>,
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
    attack_power_enabled: bool,
    attack_power_value: i64,
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
    show_entity_health_bars: bool,
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
        AnimationState::Attack => "Attack",
        AnimationState::IdleDown => "Idle Down",
        AnimationState::IdleUp => "Idle Up",
        AnimationState::IdleLeft => "Idle Left",
        AnimationState::IdleRight => "Idle Right",
        AnimationState::WalkDown => "Walk Down",
        AnimationState::WalkUp => "Walk Up",
        AnimationState::WalkLeft => "Walk Left",
        AnimationState::WalkRight => "Walk Right",
        AnimationState::AttackDown => "Attack Down",
        AnimationState::AttackUp => "Attack Up",
        AnimationState::AttackLeft => "Attack Left",
        AnimationState::AttackRight => "Attack Right",
    }
}

fn animation_state_options() -> [AnimationState; 15] {
    [
        AnimationState::Idle,
        AnimationState::Walk,
        AnimationState::Attack,
        AnimationState::IdleDown,
        AnimationState::IdleUp,
        AnimationState::IdleLeft,
        AnimationState::IdleRight,
        AnimationState::WalkDown,
        AnimationState::WalkUp,
        AnimationState::WalkLeft,
        AnimationState::WalkRight,
        AnimationState::AttackDown,
        AnimationState::AttackUp,
        AnimationState::AttackLeft,
        AnimationState::AttackRight,
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
    Damaged,
    Death,
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
        let (attack_power_enabled, attack_power_value) =
            match entity.attributes.current_stat(ATTACK_POWER_STAT_ID) {
                Some(value) => (true, value as i64),
                None => (false, 0),
            };

        Self {
            category: entity.category.clone(),
            static_object_sheet: entity
                .attributes
                .static_object_render
                .as_ref()
                .map(|render| render.sheet.clone()),
            static_object_name: entity
                .attributes
                .static_object_render
                .as_ref()
                .map(|render| render.object_name.clone()),
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
            attack_power_enabled,
            attack_power_value,
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
            show_entity_health_bars: project.metadata.runtime.display.show_entity_health_bars,
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
                    .show(ui, |ui| match ui_state.right_panel_tab {
                        super::editor_ui::RightPanelTab::Inspector => {
                            Self::render_selection_inspector_contents(
                                ui_state, ui, ctx, game_state, project, config,
                            );
                        }
                        super::editor_ui::RightPanelTab::Project => {
                            Self::render_project_settings_panel(ui_state, ui, project, config);
                        }
                    });
            });
    }

    fn render_selection_inspector_contents(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        game_state: Option<&toki_core::GameState>,
        project: Option<&mut Project>,
        config: Option<&EditorConfig>,
    ) {
        if ui_state.center_panel_tab == super::editor_ui::CenterPanelTab::MapEditor {
            Self::render_map_editor_command_palette(ui_state, ui, ctx, config);
            return;
        }

        if ui_state.center_panel_tab == super::editor_ui::CenterPanelTab::MenuEditor {
            Self::render_menu_editor_inspector(ui_state, ui, project);
            return;
        }

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

                    if ui.button("🗺 Add Map").clicked() {
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

                let changed = Self::render_selected_rule_graph_node_editor(
                    ui, ui_state, scene_name, node_key, config,
                );

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
                    ui.heading(format!(
                        "👥 {} Entities",
                        ui_state.selected_entity_ids.len()
                    ));
                    ui.separator();
                    entity_changed = Self::render_multi_scene_entity_editor(ui, ui_state);
                } else {
                    ui.separator();
                    ui.heading(format!("👤 Entity {}", entity_id));
                    ui.separator();
                    if let Some(scene_entity) =
                        Self::find_selected_scene_entity(ui_state, *entity_id)
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
            Some(Selection::MenuScreen(_)) | Some(Selection::MenuEntry { .. }) => {
                ui.label("Menu selection available only in Menu Editor.");
            }
            None => {
                ui.label("No selection");
                ui.separator();
                ui.label("Click on an item in the hierarchy to inspect it.");
            }
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
        std::fs::write(path, content).map_err(|err| {
            format!(
                "failed to write entity definition '{}': {err}",
                path.display()
            )
        })
    }
}

#[cfg(test)]
#[path = "inspector_tests.rs"]
mod tests;
