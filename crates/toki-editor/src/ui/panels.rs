use super::editor_ui::{CenterPanelTab, SceneRulesGraphCommandData, Selection};
use super::interactions::{
    CameraInteraction, MapObjectInteraction, MapPaintInteraction, PlacementInteraction,
    SelectionInteraction,
};
use super::rule_graph::{RuleGraph, RuleGraphError, RuleGraphNodeKind};
use crate::config::EditorConfig;
use crate::scene::SceneViewport;
use std::collections::{BTreeMap, HashMap, HashSet};
use toki_core::animation::AnimationState;
use toki_core::assets::{atlas::AtlasMeta, object_sheet::ObjectSheetMeta, tilemap::TileMap};
use toki_core::rules::{
    RuleAction, RuleCondition, RuleKey, RuleSoundChannel, RuleSpawnEntityType, RuleTarget,
    RuleTrigger,
};

mod map_editor;
mod scene_graph;
mod scene_graph_canvas;
mod scene_viewport;

/// Handles panel rendering for the editor (viewport and log panels)
pub struct PanelSystem;

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
enum GraphConditionKind {
    Always,
    TargetExists,
    KeyHeld,
    EntityActive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphActionKind {
    PlaySound,
    PlayMusic,
    PlayAnimation,
    SetVelocity,
    Spawn,
    DestroySelf,
    SwitchScene,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphTriggerKind {
    Start,
    Update,
    PlayerMove,
    Key,
    Collision,
    Damaged,
    Death,
    Trigger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphValidationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GraphValidationIssue {
    severity: GraphValidationSeverity,
    message: String,
    hint: String,
    fixes: Vec<GraphValidationFix>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GraphValidationFix {
    label: String,
    command: GraphValidationFixCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GraphValidationFixCommand {
    DisconnectEdges(Vec<(u64, u64)>),
    RemoveNode(u64),
}

#[derive(Debug, Clone, PartialEq)]
enum GraphCommand {
    AddTrigger,
    ResetLayout,
    SetTrigger(u64, RuleTrigger),
    AddConditionNode,
    AddActionNode,
    SetCondition(u64, RuleCondition),
    SetAction(u64, RuleAction),
    SetNodePosition(u64, [f32; 2]),
    RemoveNode(u64),
    Connect(u64, u64),
    Disconnect(u64, u64),
    DisconnectMany(Vec<(u64, u64)>),
    DisconnectNode(u64),
}

impl PanelSystem {
    /// Renders the main scene viewport panel in the center of the screen
    pub fn render_viewport(
        ui_state: &mut super::EditorUI,
        ctx: &egui::Context,
        scene_viewport: Option<&mut SceneViewport>,
        map_editor_viewport: Option<&mut SceneViewport>,
        available_map_names: Option<Vec<String>>,
        config: Option<&mut EditorConfig>,
        renderer: Option<&mut egui_wgpu::Renderer>,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ui_state.center_panel_tab,
                    CenterPanelTab::SceneViewport,
                    "Scene Viewport",
                );
                ui.selectable_value(
                    &mut ui_state.center_panel_tab,
                    CenterPanelTab::SceneGraph,
                    "Scene Graph",
                );
                ui.selectable_value(
                    &mut ui_state.center_panel_tab,
                    CenterPanelTab::SceneRules,
                    "Scene Rules",
                );
                ui.selectable_value(
                    &mut ui_state.center_panel_tab,
                    CenterPanelTab::MapEditor,
                    "Map Editor",
                );
            });
            ui.separator();

            if ui_state.center_panel_tab == CenterPanelTab::SceneGraph {
                Self::render_scene_graph(ui, ui_state, false);
                return;
            }

            if ui_state.center_panel_tab == CenterPanelTab::SceneRules {
                Self::render_scene_graph(ui, ui_state, true);
                return;
            }

            if ui_state.center_panel_tab == CenterPanelTab::MapEditor {
                Self::render_map_editor(
                    ui,
                    ui_state,
                    map_editor_viewport,
                    available_map_names,
                    config,
                    renderer,
                );
                return;
            }

            Self::render_scene_viewport_tab(ui, ui_state, scene_viewport, config, renderer);
        });
    }

    fn sanitize_grid_size_axis(value: i32) -> u32 {
        value.max(1) as u32
    }

    fn first_grid_line_at_or_before(value: i32, step: i32) -> i32 {
        let safe_step = step.max(1);
        value - value.rem_euclid(safe_step)
    }

    fn grid_world_lines(min_inclusive: i32, max_inclusive: i32, step: i32) -> Vec<i32> {
        if max_inclusive < min_inclusive {
            return Vec::new();
        }

        let safe_step = step.max(1);
        let mut current = Self::first_grid_line_at_or_before(min_inclusive, safe_step);
        let mut lines = Vec::new();
        while current <= max_inclusive {
            if current >= min_inclusive {
                lines.push(current);
            }
            current += safe_step;
        }
        lines
    }

    fn compute_viewport_display_rect(
        outer_rect: egui::Rect,
        viewport_size: (u32, u32),
        responsive: bool,
    ) -> egui::Rect {
        if responsive {
            return outer_rect;
        }

        let viewport_aspect = viewport_size.0 as f32 / viewport_size.1 as f32;
        let available_size = outer_rect.size();
        let available_aspect = available_size.x / available_size.y;

        let display_size = if available_aspect > viewport_aspect {
            egui::Vec2::new(available_size.y * viewport_aspect, available_size.y)
        } else {
            egui::Vec2::new(available_size.x, available_size.x / viewport_aspect)
        };
        let offset = (available_size - display_size) * 0.5;
        egui::Rect::from_min_size(outer_rect.min + offset, display_size)
    }

    fn effective_grid_size(viewport: &SceneViewport, config: &EditorConfig) -> glam::UVec2 {
        viewport.scene_manager().tilemap().map_or_else(
            || {
                glam::UVec2::new(
                    config.editor_settings.grid.grid_size[0],
                    config.editor_settings.grid.grid_size[1],
                )
                .max(glam::UVec2::ONE)
            },
            |tilemap| tilemap.tile_size.max(glam::UVec2::ONE),
        )
    }

    fn paint_viewport_grid_overlay(
        ui: &egui::Ui,
        outer_rect: egui::Rect,
        viewport: &SceneViewport,
        config: &EditorConfig,
    ) {
        if !config.editor_settings.grid.show_grid {
            return;
        }

        let (viewport_width, viewport_height) = viewport.viewport_size();
        let display_rect = Self::compute_viewport_display_rect(
            outer_rect,
            (viewport_width, viewport_height),
            viewport.sizing_mode() == crate::scene::viewport::ViewportSizingMode::Responsive,
        );
        let (camera_position, camera_scale) = viewport.camera_state();
        let grid_size = Self::effective_grid_size(viewport, config);

        let world_min_x = camera_position.x;
        let world_min_y = camera_position.y;
        let world_span_x = viewport_width as f32 * camera_scale;
        let world_span_y = viewport_height as f32 * camera_scale;
        let world_max_x = world_min_x + world_span_x.ceil() as i32;
        let world_max_y = world_min_y + world_span_y.ceil() as i32;

        let stroke = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(34));
        let painter = ui.painter();

        for world_x in Self::grid_world_lines(world_min_x, world_max_x, grid_size.x as i32) {
            let t = (world_x - world_min_x) as f32 / world_span_x.max(1.0);
            let screen_x = egui::lerp(display_rect.left()..=display_rect.right(), t);
            painter.line_segment(
                [
                    egui::pos2(screen_x, display_rect.top()),
                    egui::pos2(screen_x, display_rect.bottom()),
                ],
                stroke,
            );
        }

        for world_y in Self::grid_world_lines(world_min_y, world_max_y, grid_size.y as i32) {
            let t = (world_y - world_min_y) as f32 / world_span_y.max(1.0);
            let screen_y = egui::lerp(display_rect.top()..=display_rect.bottom(), t);
            painter.line_segment(
                [
                    egui::pos2(display_rect.left(), screen_y),
                    egui::pos2(display_rect.right(), screen_y),
                ],
                stroke,
            );
        }
    }

    fn paint_marquee_selection_overlay(ui: &egui::Ui, ui_state: &super::EditorUI) {
        let Some(marquee) = ui_state.marquee_selection.as_ref() else {
            return;
        };

        let selection_rect = egui::Rect::from_two_pos(marquee.start_screen, marquee.current_screen);
        if selection_rect.width() <= 0.0 || selection_rect.height() <= 0.0 {
            return;
        }

        let painter = ui.painter();
        painter.rect_filled(
            selection_rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(88, 166, 255, 36),
        );
        painter.rect_stroke(
            selection_rect,
            0.0,
            egui::Stroke::new(1.5, egui::Color32::from_rgb(88, 166, 255)),
            egui::StrokeKind::Middle,
        );
    }

    fn render_grid_toolbar(ui: &mut egui::Ui, config: &mut EditorConfig) -> bool {
        let mut changed = false;
        let grid = &mut config.editor_settings.grid;

        ui.horizontal(|ui| {
            ui.label("Grid:");

            changed |= ui.checkbox(&mut grid.show_grid, "Show Grid").changed();
            changed |= ui
                .checkbox(&mut grid.snap_to_grid, "Snap To Grid")
                .changed();

            let mut grid_x = grid.grid_size[0] as i32;
            let mut grid_y = grid.grid_size[1] as i32;

            ui.label("Grid Size");
            let x_changed = ui
                .add(
                    egui::DragValue::new(&mut grid_x)
                        .prefix("x:")
                        .range(1..=512)
                        .speed(1),
                )
                .changed();
            let y_changed = ui
                .add(
                    egui::DragValue::new(&mut grid_y)
                        .prefix("y:")
                        .range(1..=512)
                        .speed(1),
                )
                .changed();

            if x_changed || y_changed {
                let new_size = [
                    Self::sanitize_grid_size_axis(grid_x),
                    Self::sanitize_grid_size_axis(grid_y),
                ];
                if grid.grid_size != new_size {
                    grid.grid_size = new_size;
                    changed = true;
                }
            }
        });

        changed
    }

    pub fn render_log_panel(
        _ui_state: &mut super::EditorUI,
        ctx: &egui::Context,
        log_capture: Option<&crate::logging::LogCapture>,
    ) {
        egui::TopBottomPanel::bottom("log_panel")
            .resizable(true)
            .default_height(200.0)
            .show(ctx, |ui| {
                ui.heading("📝 Console");
                ui.separator();

                if let Some(capture) = log_capture {
                    let logs = capture.get_logs();
                    let scroll_area = egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true);

                    scroll_area.show(ui, |ui| {
                        for log_entry in &logs {
                            ui.horizontal(|ui| {
                                ui.label(&log_entry.timestamp);
                                ui.label(&log_entry.level);
                                ui.label(&log_entry.message);
                            });
                        }
                    });
                } else {
                    ui.label("Logs are being sent to terminal (check log_to_terminal config)");
                }
            });
    }
}

#[cfg(test)]
#[path = "panels_tests.rs"]
mod tests;
