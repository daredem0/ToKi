use super::editor_domain::{
    animation_state_label, animation_state_options, RuleActionEditorKind as RuleActionKind,
    RuleConditionEditorKind as RuleConditionKind, RuleTriggerEditorKind as RuleTriggerKind,
};
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
mod domain_inspectors;
mod entities;
mod map_editor;
mod menu_editor;
mod project;
mod rules;

pub use domain_inspectors::*;

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
        // Handle special editor modes first
        if ui_state.center_panel_tab == super::editor_ui::CenterPanelTab::MapEditor {
            Self::render_map_editor_command_palette(ui_state, ui, ctx, config);
            return;
        }

        if ui_state.center_panel_tab == super::editor_ui::CenterPanelTab::MenuEditor {
            Self::render_menu_editor_inspector(ui_state, ui, project);
            return;
        }

        // Use trait-based dispatch for selection inspectors
        use super::inspector_trait::InspectorContext;

        let current_selection = ui_state.selection.clone();
        let mut inspector = create_inspector_for_selection(current_selection.as_ref());

        let mut inspector_ctx = InspectorContext {
            ui_state,
            ctx,
            game_state,
            project,
            config,
        };

        inspector.render(ui, &mut inspector_ctx);
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
