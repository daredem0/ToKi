use strum::IntoEnumIterator;

use super::editor_domain::{
    animation_state_label, animation_state_options, RuleActionEditorKind as RuleActionKind,
    RuleConditionEditorKind as RuleConditionKind, RuleTriggerEditorKind as RuleTriggerKind,
};
use super::editor_ui::{EditorUI, MapEditorTool, SceneRulesGraphCommandData, Selection};
use super::rule_graph::{RuleGraph, RuleGraphEdge, RuleGraphNodeKind};
use crate::config::EditorConfig;
use crate::project::Project;
pub(crate) use crate::project::ProjectSettingsDraft;
use std::collections::HashMap;
use toki_core::assets::object_sheet::ObjectSheetMeta;
use toki_core::entity::EntityKind;
use toki_core::entity::{
    AiBehavior, AiConfig, ControlRole, MovementProfile, MovementSoundTrigger, ATTACK_POWER_STAT_ID,
    HEALTH_STAT_ID,
};
use toki_core::rules::{
    InteractionMode, Rule, RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel,
    RuleSpawnEntityType, RuleTarget, RuleTrigger,
};

mod animation_editor;
mod assets;
#[path = "inspector/domain_inspectors/mod.rs"]
mod domain_inspectors;
#[path = "inspector/entities/mod.rs"]
mod entities;
mod entity_editor;
mod map_editor;
#[path = "inspector/menu_editor/mod.rs"]
mod menu_editor;
mod project;
mod rules;
mod sprite_editor;

pub use domain_inspectors::*;

/// Handles inspector panel rendering for assets and entities
pub struct InspectorSystem;

/// Value object grouping collision-related entity properties
#[derive(Debug, Clone)]
pub(super) struct CollisionDraft {
    pub(super) enabled: bool,
    pub(super) offset_x: i32,
    pub(super) offset_y: i32,
    pub(super) size_x: i64,
    pub(super) size_y: i64,
    pub(super) trigger: bool,
}

#[derive(Debug, Clone)]
pub(super) struct EntityPropertyDraft {
    pub(super) category: String,
    pub(super) static_object_sheet: Option<String>,
    pub(super) static_object_name: Option<String>,
    pub(super) control_role: ControlRole,
    pub(super) position_x: i32,
    pub(super) position_y: i32,
    pub(super) size_x: i64,
    pub(super) size_y: i64,
    pub(super) visible: bool,
    pub(super) active: bool,
    pub(super) solid: bool,
    pub(super) interactable: bool,
    pub(super) interaction_reach: u32,
    pub(super) can_move: bool,
    pub(super) ai_config: AiConfig,
    pub(super) movement_profile: MovementProfile,
    pub(super) movement_sound_trigger: MovementSoundTrigger,
    pub(super) footstep_trigger_distance: f32,
    pub(super) hearing_radius: u32,
    pub(super) movement_sound: String,
    pub(super) has_inventory: bool,
    pub(super) speed: f64,
    pub(super) render_layer: i32,
    pub(super) health_enabled: bool,
    pub(super) health_value: i64,
    pub(super) attack_power_enabled: bool,
    pub(super) attack_power_value: i64,
    pub(super) collision: CollisionDraft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct MultiEntityCommonState {
    pub(super) visible: Option<bool>,
    pub(super) active: Option<bool>,
    pub(super) collision_enabled: Option<bool>,
    pub(super) render_layer: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct MultiEntityBatchEdit {
    pub(super) set_visible: Option<bool>,
    pub(super) set_active: Option<bool>,
    pub(super) set_collision_enabled: Option<bool>,
    pub(super) set_render_layer: Option<i32>,
    pub(super) position_delta: Option<glam::IVec2>,
}

impl MultiEntityBatchEdit {
    pub(super) fn is_noop(self) -> bool {
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
        let collision = if let Some(collision_box) = &entity.collision_box {
            CollisionDraft {
                enabled: true,
                offset_x: collision_box.offset.x,
                offset_y: collision_box.offset.y,
                size_x: collision_box.size.x as i64,
                size_y: collision_box.size.y as i64,
                trigger: collision_box.trigger,
            }
        } else {
            CollisionDraft {
                enabled: false,
                offset_x: 0,
                offset_y: 0,
                size_x: entity.size.x as i64,
                size_y: entity.size.y as i64,
                trigger: false,
            }
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
            interactable: entity.attributes.interactable,
            interaction_reach: entity.attributes.interaction_reach,
            can_move: entity.attributes.can_move,
            ai_config: entity.attributes.ai_config,
            movement_profile: entity.attributes.movement_profile,
            movement_sound_trigger: entity.audio.movement_sound_trigger,
            footstep_trigger_distance: entity.audio.footstep_trigger_distance,
            hearing_radius: entity.audio.hearing_radius,
            movement_sound: entity.audio.movement_sound.clone().unwrap_or_default(),
            has_inventory: entity.attributes.has_inventory,
            speed: entity.attributes.speed as f64,
            render_layer: entity.attributes.render_layer,
            health_enabled,
            health_value,
            attack_power_enabled,
            attack_power_value,
            collision,
        }
    }

    fn from_entity_definition(definition: &toki_core::entity::EntityDefinition) -> Self {
        let collision = if definition.collision.enabled {
            CollisionDraft {
                enabled: true,
                offset_x: definition.collision.offset[0],
                offset_y: definition.collision.offset[1],
                size_x: definition.collision.size[0] as i64,
                size_y: definition.collision.size[1] as i64,
                trigger: definition.collision.trigger,
            }
        } else {
            CollisionDraft {
                enabled: false,
                offset_x: 0,
                offset_y: 0,
                size_x: definition.rendering.size[0] as i64,
                size_y: definition.rendering.size[1] as i64,
                trigger: false,
            }
        };

        let (health_enabled, health_value) = match definition.attributes.health {
            Some(value) => (true, value as i64),
            None => (false, 0),
        };
        let (attack_power_enabled, attack_power_value) = match definition
            .attributes
            .stats
            .get(ATTACK_POWER_STAT_ID)
            .copied()
        {
            Some(value) => (true, value as i64),
            None => (false, 0),
        };

        Self {
            category: definition.category.clone(),
            static_object_sheet: definition
                .rendering
                .static_object
                .as_ref()
                .map(|render| render.sheet.clone()),
            static_object_name: definition
                .rendering
                .static_object
                .as_ref()
                .map(|render| render.object_name.clone()),
            control_role: ControlRole::PlayerCharacter,
            position_x: 0,
            position_y: 0,
            size_x: definition.rendering.size[0] as i64,
            size_y: definition.rendering.size[1] as i64,
            visible: definition.rendering.visible,
            active: definition.attributes.active,
            solid: definition.attributes.solid,
            interactable: definition.attributes.interactable,
            interaction_reach: definition.attributes.interaction_reach,
            can_move: definition.attributes.can_move,
            ai_config: definition.attributes.ai_config,
            movement_profile: definition.attributes.movement_profile,
            movement_sound_trigger: definition.audio.movement_sound_trigger,
            footstep_trigger_distance: definition.audio.footstep_trigger_distance,
            hearing_radius: definition.audio.hearing_radius,
            movement_sound: definition.audio.movement_sound.clone(),
            has_inventory: definition.attributes.has_inventory,
            speed: definition.attributes.speed as f64,
            render_layer: definition.rendering.render_layer,
            health_enabled,
            health_value,
            attack_power_enabled,
            attack_power_value,
            collision,
        }
    }
}

pub(super) fn ai_behavior_label(ai_behavior: AiBehavior) -> &'static str {
    match ai_behavior {
        AiBehavior::None => "None",
        AiBehavior::Wander => "Wander",
        AiBehavior::Chase => "Chase",
        AiBehavior::Run => "Run",
        AiBehavior::RunAndMultiply => "Run And Multiply",
    }
}

pub(super) fn ai_behavior_needs_detection_radius(behavior: AiBehavior) -> bool {
    matches!(
        behavior,
        AiBehavior::Chase | AiBehavior::Run | AiBehavior::RunAndMultiply
    )
}

pub(super) fn control_role_label(control_role: ControlRole) -> &'static str {
    match control_role {
        ControlRole::LegacyDefault | ControlRole::None => "None",
        ControlRole::PlayerCharacter => "Player Character",
    }
}

pub(super) fn movement_profile_label(
    control_role: ControlRole,
    movement_profile: MovementProfile,
) -> &'static str {
    match movement_profile.resolved_for_control_role(control_role) {
        MovementProfile::LegacyDefault => "Legacy Default",
        MovementProfile::None => "None",
        MovementProfile::PlayerWasd => "Player WASD",
    }
}

pub(super) fn movement_sound_trigger_label(trigger: MovementSoundTrigger) -> &'static str {
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

        if ui_state.center_panel_tab == super::editor_ui::CenterPanelTab::SpriteEditor {
            Self::render_sprite_editor_inspector(ui_state, ui, ctx);
            return;
        }

        if ui_state.center_panel_tab == super::editor_ui::CenterPanelTab::AnimationEditor {
            Self::render_animation_editor_inspector(ui_state, ui);
            return;
        }

        if ui_state.center_panel_tab == super::editor_ui::CenterPanelTab::EntityEditor {
            Self::render_entity_editor_inspector(ui_state, ui);
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
