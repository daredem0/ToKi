use super::editor_ui::{CenterPanelTab, SceneRulesGraphCommandData, Selection};
use super::interactions::{
    CameraInteraction, MapPaintInteraction, PlacementInteraction, SelectionInteraction,
};
use super::rule_graph::{RuleGraph, RuleGraphError, RuleGraphNodeKind};
use crate::config::EditorConfig;
use crate::scene::SceneViewport;
use std::collections::{BTreeMap, HashMap, HashSet};
use toki_core::animation::AnimationState;
use toki_core::assets::{atlas::AtlasMeta, tilemap::TileMap};
use toki_core::rules::{
    RuleAction, RuleCondition, RuleKey, RuleSoundChannel, RuleSpawnEntityType, RuleTarget,
    RuleTrigger,
};

/// Handles panel rendering for the editor (viewport and log panels)
pub struct PanelSystem;

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
        mut config: Option<&mut EditorConfig>,
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

            // Update and render the scene viewport
            if let Some(viewport) = scene_viewport {
                // Collect stats before updating viewport to avoid borrowing conflicts
                let entity_count = viewport
                    .scene_manager()
                    .game_state()
                    .entity_manager()
                    .active_entities()
                    .len();
                let selected_entity = viewport.selected_entity();

                // Update the viewport systems
                if let Err(e) = viewport.update() {
                    tracing::error!("Scene viewport update error: {e}");
                }

                if let Some(cfg) = config.as_deref_mut() {
                    if Self::render_grid_toolbar(ui, cfg) {
                        viewport.mark_dirty();
                    }
                    ui.separator();
                }

                // Handle viewport interactions
                let available_size = ui.available_size();
                let (rect, response) = ui.allocate_exact_size(
                    available_size,
                    egui::Sense::click_and_drag().union(egui::Sense::hover()),
                );

                // Safety reset: don't keep entities hidden when no move drag is active.
                if !ui_state.is_entity_move_drag_active() {
                    viewport.clear_suppressed_entity_rendering();
                }

                // Start entity move drag if dragging began over an entity.
                let ctrl_pressed = ui.input(|i| i.modifiers.ctrl);
                if response.drag_started() {
                    if let Some(drag_start_pos) = response.interact_pointer_pos() {
                        if ctrl_pressed && !ui_state.is_in_placement_mode() {
                            SelectionInteraction::handle_marquee_drag_start(
                                ui_state,
                                drag_start_pos,
                            );
                            viewport.stop_camera_drag();
                        } else {
                            SelectionInteraction::handle_drag_start(
                                ui_state,
                                viewport,
                                drag_start_pos,
                                rect,
                                config.as_deref(),
                                ctrl_pressed,
                            );
                        }
                    }
                }

                if ui_state.is_marquee_selection_active() && response.dragged() {
                    if let Some(drag_pos) = response
                        .interact_pointer_pos()
                        .or_else(|| response.hover_pos())
                    {
                        SelectionInteraction::handle_marquee_drag_update(ui_state, drag_pos);
                    }
                }

                // Handle drag release for entity move operations.
                if response.drag_stopped() {
                    if ui_state.is_marquee_selection_active() {
                        SelectionInteraction::handle_marquee_drag_release(
                            ui_state, viewport, rect, true,
                        );
                        viewport.stop_camera_drag();
                    } else {
                        let drop_pos = response
                            .interact_pointer_pos()
                            .or_else(|| response.hover_pos());
                        SelectionInteraction::handle_drag_release(
                            ui_state,
                            viewport,
                            drop_pos,
                            rect,
                            config.as_deref(),
                        );
                    }
                }

                // Handle camera panning with drag (disabled while moving an entity).
                if !ui_state.is_entity_move_drag_active() && !ui_state.is_marquee_selection_active()
                {
                    CameraInteraction::handle_drag(viewport, &response, config.as_deref());
                } else {
                    viewport.stop_camera_drag();
                }

                // Handle placement mode hover logic
                PlacementInteraction::handle_hover(
                    ui_state,
                    viewport,
                    &response,
                    rect,
                    config.as_deref(),
                );

                // Handle viewport clicks (entity placement or selection)
                if response.clicked() {
                    if let Some(click_pos) = response.hover_pos() {
                        // Check if we're in placement mode
                        if ui_state.is_in_placement_mode() {
                            PlacementInteraction::handle_click(
                                ui_state,
                                viewport,
                                click_pos,
                                rect,
                                config.as_deref(),
                            );
                        } else {
                            // Normal entity selection
                            SelectionInteraction::handle_click(
                                ui_state,
                                viewport,
                                click_pos,
                                rect,
                                ctrl_pressed,
                            );
                        }
                    }
                }

                // Render the scene content
                let project_path = config.as_deref().and_then(|c| c.current_project_path());
                viewport.render(ui, rect, project_path.map(|p| p.as_path()), renderer);
                if let Some(cfg) = config.as_deref() {
                    Self::paint_viewport_grid_overlay(ui, rect, viewport, cfg);
                }
                Self::paint_marquee_selection_overlay(ui, ui_state);

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("📊 Stats:");
                        ui.label(format!(
                            "Entities: {} | Selected: {:?}",
                            entity_count, selected_entity
                        ));
                        ui.label("Press F1/F2 to toggle panels");
                    });
                });
            } else {
                // Show placeholder when no viewport
                let available_size = ui.available_size();
                ui.allocate_response(available_size, egui::Sense::click())
                    .on_hover_text("Scene viewport not initialized");
            }
        });
    }

    fn render_map_editor(
        ui: &mut egui::Ui,
        ui_state: &mut super::EditorUI,
        map_editor_viewport: Option<&mut SceneViewport>,
        available_map_names: Option<Vec<String>>,
        mut config: Option<&mut EditorConfig>,
        renderer: Option<&mut egui_wgpu::Renderer>,
    ) {
        if let Some(names) = &available_map_names {
            ui_state.sync_map_editor_selection(names);
        } else {
            ui_state.sync_map_editor_selection(&[]);
        }

        let project_path = config
            .as_deref()
            .and_then(|cfg| cfg.current_project_path())
            .cloned();
        let available_tiles = project_path
            .as_deref()
            .and_then(|path| {
                map_editor_viewport
                    .as_ref()
                    .and_then(|viewport| viewport.scene_manager().tilemap())
                    .and_then(|tilemap| Self::load_map_editor_tile_names(path, tilemap).ok())
            })
            .unwrap_or_default();
        ui_state.sync_map_editor_brush_selection(&available_tiles);

        ui.horizontal(|ui| {
            ui.heading("Map Editor");
            ui.separator();
            if ui.button("New Map").clicked() {
                ui_state.begin_new_map_dialog();
            }
            if ui
                .add_enabled(
                    ui_state.has_unsaved_map_editor_changes(),
                    egui::Button::new("Save Map"),
                )
                .clicked()
            {
                ui_state.map_editor_save_requested = true;
            }
            ui.separator();
            ui.label("Map:");

            let selected_label = ui_state.map_editor_selected_label();
            egui::ComboBox::from_id_salt("map_editor_map_selector")
                .selected_text(selected_label)
                .show_ui(ui, |ui| {
                    if let Some(map_names) = &available_map_names {
                        if ui_state.has_unsaved_map_editor_changes() {
                            ui.label("Save the current draft before switching maps.");
                            return;
                        }
                        for map_name in map_names {
                            let is_selected = ui_state.map_editor_active_map.as_deref()
                                == Some(map_name.as_str());
                            if ui.selectable_label(is_selected, map_name).clicked() && !is_selected
                            {
                                ui_state.map_editor_active_map = Some(map_name.clone());
                                ui_state.map_editor_map_load_requested = Some(map_name.clone());
                            }
                        }
                    }
                });

            if ui_state.has_unsaved_map_editor_draft() {
                ui.label("Unsaved draft");
            } else if ui_state.map_editor_dirty {
                ui.label("Unsaved changes");
            } else if let Some(active_map) = ui_state.map_editor_active_map.as_deref() {
                ui.label(format!("Editing asset: {}", active_map));
            }
        });
        ui.horizontal(|ui| {
            ui.label("Tool:");
            ui.label(match ui_state.map_editor_tool {
                super::editor_ui::MapEditorTool::Drag => "Drag",
                super::editor_ui::MapEditorTool::Brush => "Brush",
                super::editor_ui::MapEditorTool::Fill => "Fill",
            });
        });
        ui.separator();

        if ui_state.map_editor_show_new_map_dialog {
            let mut open = ui_state.map_editor_show_new_map_dialog;
            let mut create_clicked = false;
            let mut cancel_clicked = false;
            egui::Window::new("New Map")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut ui_state.map_editor_new_map_name);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Width");
                        ui.add(
                            egui::DragValue::new(&mut ui_state.map_editor_new_map_width)
                                .range(1..=512)
                                .speed(1),
                        );
                        ui.label("Height");
                        ui.add(
                            egui::DragValue::new(&mut ui_state.map_editor_new_map_height)
                                .range(1..=512)
                                .speed(1),
                        );
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            create_clicked = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel_clicked = true;
                        }
                    });
                });

            if create_clicked {
                ui_state.submit_new_map_request();
                open = false;
            }
            if cancel_clicked {
                open = false;
            }
            ui_state.map_editor_show_new_map_dialog = open;
        }

        let Some(viewport) = map_editor_viewport else {
            ui.label("Map editor viewport not initialized.");
            return;
        };

        if let Some(cfg) = config.as_deref_mut() {
            if Self::render_grid_toolbar(ui, cfg) {
                viewport.mark_dirty();
            }
            ui.separator();
        }

        let available_size = ui.available_size();
        let requested_viewport_size = (
            available_size.x.max(1.0).round() as u32,
            available_size.y.max(1.0).round() as u32,
        );
        viewport.request_viewport_size(requested_viewport_size);

        if let Err(error) = viewport.update() {
            tracing::error!("Map editor viewport update error: {error}");
        }

        let (rect, response) =
            ui.allocate_exact_size(available_size, egui::Sense::click_and_drag());

        match ui_state.map_editor_tool {
            super::editor_ui::MapEditorTool::Drag => {
                ui_state.cancel_map_editor_edit();
                Self::handle_map_editor_primary_drag(viewport, &response, config.as_deref());
            }
            super::editor_ui::MapEditorTool::Brush => {
                Self::handle_map_editor_secondary_drag(ui, viewport, &response, config.as_deref());
            }
            super::editor_ui::MapEditorTool::Fill => {
                ui_state.cancel_map_editor_edit();
                Self::handle_map_editor_secondary_drag(ui, viewport, &response, config.as_deref());
            }
        }

        viewport.render(ui, rect, project_path.as_deref(), renderer);
        if let Some(cfg) = config.as_deref() {
            Self::paint_viewport_grid_overlay(ui, rect, viewport, cfg);
        }
        if let Some(project_path) = project_path.as_deref() {
            Self::paint_map_editor_brush_preview(ui, ui_state, viewport, rect, project_path);
        }

        match ui_state.map_editor_tool {
            super::editor_ui::MapEditorTool::Drag => {
                if let Some(project_path) = project_path.as_deref() {
                    if let Some(tile_info) = Self::handle_map_editor_tile_inspect(
                        ui,
                        viewport,
                        &response,
                        rect,
                        project_path,
                    ) {
                        ui_state.map_editor_selected_tile_info = tile_info;
                    }
                }
            }
            super::editor_ui::MapEditorTool::Brush => {
                let primary_down = ui.input(|input| input.pointer.primary_down());
                if !primary_down {
                    if let Some(tilemap) = viewport.scene_manager().tilemap() {
                        ui_state.finish_map_editor_edit(tilemap);
                    } else {
                        ui_state.cancel_map_editor_edit();
                    }
                }
                if let Some(selected_tile) = ui_state.map_editor_selected_tile.clone() {
                    if Self::handle_map_editor_brush_paint(
                        ui,
                        ui_state,
                        viewport,
                        &response,
                        rect,
                        &selected_tile,
                        ui_state.map_editor_brush_size_tiles,
                    ) {
                        ui_state.mark_map_editor_dirty();
                    }
                }
            }
            super::editor_ui::MapEditorTool::Fill => {
                if let Some(selected_tile) = ui_state.map_editor_selected_tile.clone() {
                    if Self::handle_map_editor_fill_paint(
                        ui,
                        ui_state,
                        viewport,
                        &response,
                        &selected_tile,
                    ) {
                        ui_state.mark_map_editor_dirty();
                    }
                }
            }
        }
    }

    fn handle_map_editor_primary_drag(
        viewport: &mut SceneViewport,
        response: &egui::Response,
        config: Option<&EditorConfig>,
    ) {
        CameraInteraction::handle_drag(viewport, response, config);
    }

    fn handle_map_editor_secondary_drag(
        ui: &egui::Ui,
        viewport: &mut SceneViewport,
        response: &egui::Response,
        config: Option<&EditorConfig>,
    ) {
        let pan_speed = config
            .map(|c| c.editor_settings.camera.pan_speed)
            .unwrap_or(1.0);

        if response.hovered() && ui.input(|input| input.pointer.secondary_pressed()) {
            if let Some(start_pos) = ui.input(|input| input.pointer.interact_pos()) {
                viewport.start_camera_drag(glam::Vec2::new(start_pos.x, start_pos.y));
            }
        } else if response.hovered() && ui.input(|input| input.pointer.secondary_down()) {
            if let Some(drag_pos) = ui.input(|input| input.pointer.interact_pos()) {
                viewport.update_camera_drag(glam::Vec2::new(drag_pos.x, drag_pos.y), pan_speed);
            }
        } else if ui.input(|input| input.pointer.secondary_released()) {
            viewport.stop_camera_drag();
        }
    }

    fn handle_map_editor_brush_paint(
        ui: &egui::Ui,
        ui_state: &mut super::EditorUI,
        viewport: &mut SceneViewport,
        response: &egui::Response,
        rect: egui::Rect,
        selected_tile: &str,
        brush_size_tiles: u32,
    ) -> bool {
        let wants_paint = response.hovered()
            && ui.input(|input| input.pointer.primary_down() || input.pointer.primary_pressed());
        if !wants_paint {
            return false;
        }

        let Some(pointer_pos) = ui.input(|input| input.pointer.interact_pos()) else {
            return false;
        };

        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let Some(tilemap) = viewport.scene_manager_mut().tilemap_mut() else {
            return false;
        };
        if ui.input(|input| input.pointer.primary_pressed()) {
            ui_state.begin_map_editor_edit(tilemap);
        }
        let Some(tile_pos) = MapPaintInteraction::tile_position_at_world(tilemap, world_pos) else {
            return false;
        };

        if MapPaintInteraction::paint_brush(tilemap, tile_pos, selected_tile, brush_size_tiles) {
            viewport.mark_dirty();
            return true;
        }

        false
    }

    fn handle_map_editor_fill_paint(
        ui: &egui::Ui,
        ui_state: &mut super::EditorUI,
        viewport: &mut SceneViewport,
        response: &egui::Response,
        selected_tile: &str,
    ) -> bool {
        let wants_fill = response.hovered() && ui.input(|input| input.pointer.primary_clicked());
        if !wants_fill {
            return false;
        }

        let Some(tilemap) = viewport.scene_manager_mut().tilemap_mut() else {
            return false;
        };
        ui_state.begin_map_editor_edit(tilemap);

        if MapPaintInteraction::fill_all(tilemap, selected_tile) {
            ui_state.finish_map_editor_edit(tilemap);
            viewport.mark_dirty();
            return true;
        }

        ui_state.cancel_map_editor_edit();
        false
    }

    fn paint_map_editor_brush_preview(
        ui: &egui::Ui,
        ui_state: &mut super::EditorUI,
        viewport: &SceneViewport,
        rect: egui::Rect,
        project_path: &std::path::Path,
    ) {
        if ui_state.map_editor_tool != super::editor_ui::MapEditorTool::Brush {
            return;
        }
        let Some(selected_tile) = ui_state.map_editor_selected_tile.clone() else {
            return;
        };
        let Some(pointer_pos) = ui.input(|input| input.pointer.hover_pos()) else {
            return;
        };
        if !rect.contains(pointer_pos) {
            return;
        }
        let Some(tilemap) = viewport.scene_manager().tilemap() else {
            return;
        };
        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let Some(center_tile) = MapPaintInteraction::tile_position_at_world(tilemap, world_pos)
        else {
            return;
        };
        let Some((start_tile, end_tile)) = MapPaintInteraction::brush_footprint_bounds(
            tilemap,
            center_tile,
            ui_state.map_editor_brush_size_tiles,
        ) else {
            return;
        };
        let Some((atlas, texture_path)) =
            Self::load_map_editor_preview_assets(project_path, tilemap).ok()
        else {
            return;
        };
        let Some(texture) =
            Self::ensure_map_editor_brush_preview_texture(ui_state, ui.ctx(), &texture_path)
        else {
            return;
        };
        let Some(texture_size) = atlas.image_size() else {
            return;
        };
        let Some(tile_rect_px) = atlas.get_tile_rect(&selected_tile) else {
            return;
        };
        let uv_rect = egui::Rect::from_min_max(
            egui::pos2(
                tile_rect_px[0] as f32 / texture_size.x as f32,
                tile_rect_px[1] as f32 / texture_size.y as f32,
            ),
            egui::pos2(
                (tile_rect_px[0] + tile_rect_px[2]) as f32 / texture_size.x as f32,
                (tile_rect_px[1] + tile_rect_px[3]) as f32 / texture_size.y as f32,
            ),
        );
        let (viewport_width, viewport_height) = viewport.viewport_size();
        let display_rect = Self::compute_viewport_display_rect(
            rect,
            (viewport_width, viewport_height),
            viewport.sizing_mode() == crate::scene::viewport::ViewportSizingMode::Responsive,
        );
        let (camera_position, camera_scale) = viewport.camera_state();
        let painter = ui.painter().with_clip_rect(display_rect);
        let preview_tint = egui::Color32::from_white_alpha(170);
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(150));

        for tile_y in start_tile.y..end_tile.y {
            for tile_x in start_tile.x..end_tile.x {
                let Some(tile_screen_rect) = Self::map_editor_tile_screen_rect(
                    display_rect,
                    (viewport_width, viewport_height),
                    camera_position,
                    camera_scale,
                    tilemap.tile_size,
                    glam::UVec2::new(tile_x, tile_y),
                ) else {
                    continue;
                };
                painter.image(texture.id(), tile_screen_rect, uv_rect, preview_tint);
                painter.rect_stroke(tile_screen_rect, 0.0, stroke, egui::StrokeKind::Inside);
            }
        }
    }

    fn handle_map_editor_tile_inspect(
        ui: &egui::Ui,
        viewport: &mut SceneViewport,
        response: &egui::Response,
        rect: egui::Rect,
        project_path: &std::path::Path,
    ) -> Option<Option<super::editor_ui::MapEditorTileInfo>> {
        let clicked = response.hovered() && ui.input(|input| input.pointer.primary_clicked());
        if !clicked {
            return None;
        }

        let Some(pointer_pos) = ui.input(|input| input.pointer.interact_pos()) else {
            return Some(None);
        };
        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let Some(tilemap) = viewport.scene_manager().tilemap() else {
            return Some(None);
        };
        let Some(tile_pos) = MapPaintInteraction::tile_position_at_world(tilemap, world_pos) else {
            return Some(None);
        };
        let Some(tile_name) = tilemap
            .get_tile_name(tile_pos.x, tile_pos.y)
            .ok()
            .map(ToString::to_string)
        else {
            return Some(None);
        };
        let Some(atlas) = Self::load_map_editor_atlas(project_path, tilemap).ok() else {
            return Some(None);
        };
        let Some(properties) = atlas.get_tile_properties(&tile_name) else {
            return Some(None);
        };

        Some(Some(super::editor_ui::MapEditorTileInfo {
            tile_x: tile_pos.x,
            tile_y: tile_pos.y,
            tile_name,
            solid: properties.solid,
            trigger: properties.trigger,
        }))
    }

    fn load_map_editor_tile_names(
        project_path: &std::path::Path,
        tilemap: &TileMap,
    ) -> anyhow::Result<Vec<String>> {
        let atlas = Self::load_map_editor_atlas(project_path, tilemap)?;
        let mut tile_names = atlas.tiles.keys().cloned().collect::<Vec<_>>();
        tile_names.sort();
        Ok(tile_names)
    }

    fn load_map_editor_atlas(
        project_path: &std::path::Path,
        tilemap: &TileMap,
    ) -> anyhow::Result<AtlasMeta> {
        let atlas_path = {
            let tilemaps_path = project_path
                .join("assets")
                .join("tilemaps")
                .join(&tilemap.atlas);
            if tilemaps_path.exists() {
                tilemaps_path
            } else {
                project_path
                    .join("assets")
                    .join("sprites")
                    .join(&tilemap.atlas)
            }
        };
        AtlasMeta::load_from_file(&atlas_path)
            .map_err(|e| anyhow::anyhow!("Failed to load atlas '{}': {}", atlas_path.display(), e))
    }

    fn load_map_editor_preview_assets(
        project_path: &std::path::Path,
        tilemap: &TileMap,
    ) -> anyhow::Result<(AtlasMeta, std::path::PathBuf)> {
        let atlas_path = {
            let tilemaps_path = project_path
                .join("assets")
                .join("tilemaps")
                .join(&tilemap.atlas);
            if tilemaps_path.exists() {
                tilemaps_path
            } else {
                project_path
                    .join("assets")
                    .join("sprites")
                    .join(&tilemap.atlas)
            }
        };
        let atlas = AtlasMeta::load_from_file(&atlas_path).map_err(|e| {
            anyhow::anyhow!("Failed to load atlas '{}': {}", atlas_path.display(), e)
        })?;
        let texture_path = atlas_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Atlas path '{}' has no parent", atlas_path.display()))?
            .join(&atlas.image);
        Ok((atlas, texture_path))
    }

    fn ensure_map_editor_brush_preview_texture(
        ui_state: &mut super::EditorUI,
        ctx: &egui::Context,
        texture_path: &std::path::Path,
    ) -> Option<egui::TextureHandle> {
        if ui_state.map_editor_brush_preview_image_path.as_deref() == Some(texture_path)
            && ui_state.map_editor_brush_preview_texture.is_some()
        {
            return ui_state.map_editor_brush_preview_texture.clone();
        }

        let decoded = toki_core::graphics::image::load_image_rgba8(texture_path).ok()?;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.data,
        );
        let key = format!("map_editor_brush_preview:{}", texture_path.display());
        let texture = ctx.load_texture(key, color_image, egui::TextureOptions::NEAREST);
        ui_state.map_editor_brush_preview_image_path = Some(texture_path.to_path_buf());
        ui_state.map_editor_brush_preview_texture = Some(texture.clone());
        Some(texture)
    }

    fn map_editor_tile_screen_rect(
        display_rect: egui::Rect,
        viewport_size: (u32, u32),
        camera_position: glam::IVec2,
        camera_scale: f32,
        tile_size: glam::UVec2,
        tile_pos: glam::UVec2,
    ) -> Option<egui::Rect> {
        let world_span_x = viewport_size.0 as f32 * camera_scale;
        let world_span_y = viewport_size.1 as f32 * camera_scale;
        if world_span_x <= 0.0 || world_span_y <= 0.0 {
            return None;
        }

        let world_min_x = camera_position.x as f32;
        let world_min_y = camera_position.y as f32;
        let world_left = tile_pos.x as f32 * tile_size.x as f32;
        let world_top = tile_pos.y as f32 * tile_size.y as f32;
        let world_right = world_left + tile_size.x as f32;
        let world_bottom = world_top + tile_size.y as f32;

        let left_t = (world_left - world_min_x) / world_span_x;
        let top_t = (world_top - world_min_y) / world_span_y;
        let right_t = (world_right - world_min_x) / world_span_x;
        let bottom_t = (world_bottom - world_min_y) / world_span_y;

        Some(egui::Rect::from_min_max(
            egui::pos2(
                egui::lerp(display_rect.left()..=display_rect.right(), left_t),
                egui::lerp(display_rect.top()..=display_rect.bottom(), top_t),
            ),
            egui::pos2(
                egui::lerp(display_rect.left()..=display_rect.right(), right_t),
                egui::lerp(display_rect.top()..=display_rect.bottom(), bottom_t),
            ),
        ))
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

    fn render_scene_graph(
        ui: &mut egui::Ui,
        ui_state: &mut super::EditorUI,
        show_scene_rules: bool,
    ) {
        if show_scene_rules {
            ui.heading("Active Scene Rules");
        } else {
            ui.heading("Active Scene Graph");
        }
        ui.separator();

        let Some(active_scene_name) = ui_state.active_scene.clone() else {
            ui.label("No active scene selected.");
            return;
        };

        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|scene| scene.name == active_scene_name)
        else {
            ui.label(format!(
                "Active scene '{}' is not loaded.",
                active_scene_name
            ));
            return;
        };

        let mut connect_from = ui_state.graph_connect_from_node;
        let mut connect_to = ui_state.graph_connect_to_node;
        let (mut graph_zoom, mut graph_pan) = ui_state.graph_view_for_scene(&active_scene_name);
        let before_rule_set = ui_state.scenes[scene_index].rules.clone();
        let before_graph_snapshot = ui_state.rule_graph_for_scene(&active_scene_name).cloned();
        let before_layout_snapshot = ui_state
            .graph_layouts_by_scene
            .get(&active_scene_name)
            .cloned();
        let mut scene_changed = false;
        let mut graph_changed = false;
        let mut layout_changed = false;
        let mut operation_error: Option<String> = None;
        let mut selected_graph_node: Option<u64> = None;

        {
            let scene_rules = before_rule_set.clone();
            ui_state.sync_rule_graph_with_rule_set(&active_scene_name, &scene_rules);
            let mut graph = ui_state
                .rule_graph_for_scene(&active_scene_name)
                .cloned()
                .unwrap_or_else(|| RuleGraph::from_rule_set(&scene_rules));
            let mut pending_command: Option<GraphCommand> = None;

            let node_ids = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
            for node_id in node_ids {
                let Some(node_key) = graph.stable_node_key(node_id) else {
                    continue;
                };
                let Some(position) = ui_state.graph_layout_position(&active_scene_name, &node_key)
                else {
                    continue;
                };
                let _ = graph.set_node_position(node_id, position);
            }

            if let Some(Selection::RuleGraphNode {
                scene_name,
                node_key,
            }) = &ui_state.selection
            {
                if scene_name == &active_scene_name {
                    selected_graph_node = graph.node_id_for_stable_key(node_key);
                }
            }
            let node_badges = Self::rule_graph_node_badges(&graph);

            if !show_scene_rules {
                ui.horizontal(|ui| {
                    if !ui.ctx().wants_keyboard_input() {
                        if ui.input(|input| {
                            input.key_pressed(egui::Key::Plus)
                                || input.key_pressed(egui::Key::Equals)
                        }) {
                            graph_zoom = (graph_zoom * 1.1).clamp(0.4, 4.0);
                        }
                        if ui.input(|input| input.key_pressed(egui::Key::Minus)) {
                            graph_zoom = (graph_zoom / 1.1).clamp(0.4, 4.0);
                        }
                    }
                    ui.label(format!("Zoom: {:.0}%", graph_zoom * 100.0));
                    ui.label("Tip: Drag Empty Space To Pan");
                    if ui.button("➕ Add Trigger").clicked() {
                        pending_command = Some(GraphCommand::AddTrigger);
                    }
                    if ui.button("➕ Add Condition").clicked() {
                        pending_command = Some(GraphCommand::AddConditionNode);
                    }
                    if ui.button("➕ Add Action").clicked() {
                        pending_command = Some(GraphCommand::AddActionNode);
                    }
                    if ui.button("↺ Reset Auto Layout").clicked() {
                        pending_command = Some(GraphCommand::ResetLayout);
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    if ui.button("➕ Add Trigger").clicked() {
                        pending_command = Some(GraphCommand::AddTrigger);
                    }
                    if ui.button("➕ Add Condition").clicked() {
                        pending_command = Some(GraphCommand::AddConditionNode);
                    }
                    if ui.button("➕ Add Action").clicked() {
                        pending_command = Some(GraphCommand::AddActionNode);
                    }
                });
            }

            if connect_from.is_some_and(|id| !graph.nodes.iter().any(|node| node.id == id)) {
                connect_from = None;
            }
            if connect_to.is_some_and(|id| !graph.nodes.iter().any(|node| node.id == id)) {
                connect_to = None;
            }

            if !show_scene_rules {
                ui.horizontal(|ui| {
                    ui.label("Connect:");

                    egui::ComboBox::from_id_salt(format!("graph_connect_from_{}", scene_index))
                        .selected_text(
                            connect_from
                                .and_then(|id| {
                                    Self::rule_graph_node_label(&graph, &node_badges, id)
                                })
                                .unwrap_or_else(|| "<source>".to_string()),
                        )
                        .show_ui(ui, |ui| {
                            for node in &graph.nodes {
                                ui.selectable_value(
                                    &mut connect_from,
                                    Some(node.id),
                                    Self::rule_graph_node_label(&graph, &node_badges, node.id)
                                        .unwrap_or_else(|| format!("{}", node.id)),
                                );
                            }
                        });

                    egui::ComboBox::from_id_salt(format!("graph_connect_to_{}", scene_index))
                        .selected_text(
                            connect_to
                                .and_then(|id| {
                                    Self::rule_graph_node_label(&graph, &node_badges, id)
                                })
                                .unwrap_or_else(|| "<target>".to_string()),
                        )
                        .show_ui(ui, |ui| {
                            for node in &graph.nodes {
                                ui.selectable_value(
                                    &mut connect_to,
                                    Some(node.id),
                                    Self::rule_graph_node_label(&graph, &node_badges, node.id)
                                        .unwrap_or_else(|| format!("{}", node.id)),
                                );
                            }
                        });

                    if ui.button("Connect").clicked() {
                        if let (Some(from), Some(to)) = (connect_from, connect_to) {
                            pending_command = Some(GraphCommand::Connect(from, to));
                        }
                    }
                });
            }

            ui.label(format!(
                "Chains: {} | Nodes: {} | Edges: {}",
                graph.chains.len(),
                graph.nodes.len(),
                graph.edges.len()
            ));
            let validation_issues = Self::collect_graph_validation_issues(&graph, &node_badges);
            if pending_command.is_none() {
                if let Some(fix_command) =
                    Self::render_graph_validation_summary(ui, &validation_issues)
                {
                    pending_command = Some(match fix_command {
                        GraphValidationFixCommand::DisconnectEdges(edges) => {
                            GraphCommand::DisconnectMany(edges)
                        }
                        GraphValidationFixCommand::RemoveNode(node_id) => {
                            GraphCommand::RemoveNode(node_id)
                        }
                    });
                }
            } else {
                let _ = Self::render_graph_validation_summary(ui, &validation_issues);
            }
            if !show_scene_rules {
                if pending_command.is_none() {
                    let (moved_node, clicked_node) = Self::render_graph_canvas(
                        ui,
                        &graph,
                        &node_badges,
                        graph_zoom,
                        &mut graph_pan,
                    );
                    if let Some((node_id, position)) = moved_node {
                        pending_command = Some(GraphCommand::SetNodePosition(node_id, position));
                    }
                    if let Some(node_id) = clicked_node {
                        selected_graph_node = Some(node_id);
                    }
                }

                if graph.nodes.is_empty() {
                    ui.label("No rules in active scene. Add a rule chain to start authoring.");
                } else if let Some(node_id) = selected_graph_node {
                    ui.separator();
                    ui.strong("Selected Node");
                    if pending_command.is_none() {
                        pending_command = Self::render_graph_selected_node_editor(
                            ui,
                            &graph,
                            &node_badges,
                            node_id,
                            &active_scene_name,
                        );
                    } else {
                        let _ = Self::render_graph_selected_node_editor(
                            ui,
                            &graph,
                            &node_badges,
                            node_id,
                            &active_scene_name,
                        );
                    }
                }
            }

            if show_scene_rules {
                let node_by_id = graph
                    .nodes
                    .iter()
                    .map(|node| (node.id, node))
                    .collect::<HashMap<_, _>>();
                let mut outgoing = HashMap::<u64, Vec<u64>>::new();
                for edge in &graph.edges {
                    outgoing.entry(edge.from).or_default().push(edge.to);
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (rule_index, chain) in graph.chains.iter().enumerate() {
                        ui.push_id(("graph_chain", chain.trigger_node_id), |ui| {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.strong(format!("Rule {}: {}", rule_index + 1, chain.rule_id));
                                    if !chain.enabled {
                                        ui.label("(disabled)");
                                    }
                                    if ui.small_button("🗑 Rule").clicked() {
                                        pending_command =
                                            Some(GraphCommand::RemoveNode(chain.trigger_node_id));
                                    }
                                });

                                let sequence = match graph.chain_node_sequence(chain.trigger_node_id) {
                                    Ok(sequence) => sequence,
                                    Err(error) => {
                                        ui.colored_label(
                                            egui::Color32::from_rgb(255, 120, 120),
                                            format!("Invalid chain: {:?}", error),
                                        );
                                        Vec::new()
                                    }
                                };
                                let sequence_set = sequence.iter().copied().collect::<HashSet<_>>();

                                for node_id in sequence {
                                    let Some(node) = node_by_id.get(&node_id) else {
                                        continue;
                                    };
                                    ui.push_id(("graph_node", node_id), |ui| {
                                        ui.horizontal(|ui| match &node.kind {
                                            RuleGraphNodeKind::Trigger(trigger) => {
                                                let badge = node_badges
                                                    .get(&node_id)
                                                    .cloned()
                                                    .unwrap_or_else(|| "T?".to_string());
                                                let node_label = format!(
                                                    "{} Trigger: {}",
                                                    badge,
                                                    Self::trigger_summary(*trigger)
                                                );
                                                let is_selected = selected_graph_node == Some(node_id);
                                                if ui.selectable_label(is_selected, node_label).clicked() {
                                                    selected_graph_node = Some(node_id);
                                                }
                                                let mut trigger_value = *trigger;
                                                let mut kind = Self::graph_trigger_kind(*trigger);
                                                egui::ComboBox::from_id_salt((
                                                    "graph_trigger_kind",
                                                    &active_scene_name,
                                                    node_id,
                                                ))
                                                    .selected_text(Self::graph_trigger_kind_label(kind))
                                                    .show_ui(ui, |ui| {
                                                        for candidate in [
                                                            GraphTriggerKind::Start,
                                                            GraphTriggerKind::Update,
                                                            GraphTriggerKind::PlayerMove,
                                                            GraphTriggerKind::Key,
                                                            GraphTriggerKind::Collision,
                                                            GraphTriggerKind::Trigger,
                                                        ] {
                                                            ui.selectable_value(
                                                                &mut kind,
                                                                candidate,
                                                                Self::graph_trigger_kind_label(candidate),
                                                            );
                                                        }
                                                    });
                                                if kind != Self::graph_trigger_kind(*trigger) {
                                                    trigger_value = Self::graph_default_trigger(kind);
                                                }
                                                if let RuleTrigger::OnKey { key } = &mut trigger_value {
                                                    let _ = Self::edit_rule_key(
                                                        ui,
                                                        key,
                                                        &format!(
                                                            "graph_trigger_key::{}::{}",
                                                            active_scene_name, node_id
                                                        ),
                                                    );
                                                }
                                                if trigger_value != *trigger {
                                                    pending_command =
                                                        Some(GraphCommand::SetTrigger(node_id, trigger_value));
                                                }
                                            }
                                            RuleGraphNodeKind::Condition(condition) => {
                                                let badge = node_badges
                                                    .get(&node_id)
                                                    .cloned()
                                                    .unwrap_or_else(|| "C?".to_string());
                                                let node_label = format!(
                                                    "{} Condition: {}",
                                                    badge,
                                                    Self::condition_summary(*condition)
                                                );
                                                let is_selected = selected_graph_node == Some(node_id);
                                                if ui.selectable_label(is_selected, node_label).clicked() {
                                                    selected_graph_node = Some(node_id);
                                                }
                                                let mut kind = Self::graph_condition_kind(*condition);
                                                egui::ComboBox::from_id_salt((
                                                    "graph_condition_kind",
                                                    &active_scene_name,
                                                    node_id,
                                                ))
                                                    .selected_text(Self::graph_condition_kind_label(kind))
                                                    .show_ui(ui, |ui| {
                                                        for candidate in [
                                                            GraphConditionKind::Always,
                                                            GraphConditionKind::TargetExists,
                                                            GraphConditionKind::KeyHeld,
                                                            GraphConditionKind::EntityActive,
                                                        ] {
                                                            ui.selectable_value(
                                                                &mut kind,
                                                                candidate,
                                                                Self::graph_condition_kind_label(candidate),
                                                            );
                                                        }
                                                    });
                                                let mut edited_condition = *condition;
                                                if kind != Self::graph_condition_kind(*condition) {
                                                    edited_condition =
                                                        Self::graph_default_condition(kind);
                                                }
                                                let payload_changed =
                                                    Self::edit_graph_condition_payload(
                                                        ui,
                                                        &mut edited_condition,
                                                        &format!(
                                                            "graph_condition_payload::{}::{}",
                                                            active_scene_name, node_id
                                                        ),
                                                    );
                                                if edited_condition != *condition || payload_changed {
                                                    pending_command = Some(GraphCommand::SetCondition(
                                                        node_id,
                                                        edited_condition,
                                                    ));
                                                }
                                                if ui.small_button("✕").clicked() {
                                                    pending_command =
                                                        Some(GraphCommand::RemoveNode(node_id));
                                                }
                                            }
                                            RuleGraphNodeKind::Action(action) => {
                                                let badge = node_badges
                                                    .get(&node_id)
                                                    .cloned()
                                                    .unwrap_or_else(|| "A?".to_string());
                                                let node_label =
                                                    format!(
                                                        "{} Action: {}",
                                                        badge,
                                                        Self::action_summary(action)
                                                    );
                                                let is_selected = selected_graph_node == Some(node_id);
                                                if ui.selectable_label(is_selected, node_label).clicked() {
                                                    selected_graph_node = Some(node_id);
                                                }
                                                let mut kind = Self::graph_action_kind(action);
                                                egui::ComboBox::from_id_salt((
                                                    "graph_action_kind",
                                                    &active_scene_name,
                                                    node_id,
                                                ))
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
                                                        ] {
                                                            ui.selectable_value(
                                                                &mut kind,
                                                                candidate,
                                                                Self::graph_action_kind_label(candidate),
                                                            );
                                                        }
                                                    });
                                                let mut edited_action = action.clone();
                                                if kind != Self::graph_action_kind(action) {
                                                    edited_action = Self::graph_default_action(kind);
                                                }
                                                let payload_changed = Self::edit_graph_action_payload(
                                                    ui,
                                                    &mut edited_action,
                                                    &format!(
                                                        "graph_action_payload::{}::{}",
                                                        active_scene_name, node_id
                                                    ),
                                                );
                                                if edited_action != *action || payload_changed {
                                                    pending_command = Some(GraphCommand::SetAction(
                                                        node_id,
                                                        edited_action,
                                                    ));
                                                }
                                                if ui.small_button("✕").clicked() {
                                                    pending_command =
                                                        Some(GraphCommand::RemoveNode(node_id));
                                                }
                                            }
                                        });
                                    });
                                }

                                let edge_list = graph
                                    .edges
                                    .iter()
                                    .filter(|edge| {
                                        sequence_set.contains(&edge.from)
                                            || sequence_set.contains(&edge.to)
                                    })
                                    .copied()
                                    .collect::<Vec<_>>();

                                if !edge_list.is_empty() {
                                    egui::CollapsingHeader::new("Edges")
                                        .id_salt(("graph_edges", chain.trigger_node_id))
                                        .show(ui, |ui| {
                                            for edge in edge_list {
                                                ui.horizontal(|ui| {
                                                    let from_label = Self::rule_graph_node_label(
                                                        &graph,
                                                        &node_badges,
                                                        edge.from,
                                                    )
                                                    .unwrap_or_else(|| format!("node {}", edge.from));
                                                    let to_label = Self::rule_graph_node_label(
                                                        &graph,
                                                        &node_badges,
                                                        edge.to,
                                                    )
                                                    .unwrap_or_else(|| format!("node {}", edge.to));
                                                    ui.monospace(format!(
                                                        "{} -> {}",
                                                        from_label, to_label
                                                    ));
                                                    if ui.small_button("Disconnect").clicked() {
                                                        pending_command = Some(
                                                            GraphCommand::Disconnect(edge.from, edge.to),
                                                        );
                                                    }
                                                });
                                            }
                                        });
                                }

                                if let Some(next_nodes) = outgoing.get(&chain.trigger_node_id) {
                                    if next_nodes.is_empty() {
                                        ui.colored_label(
                                            egui::Color32::from_rgb(255, 210, 80),
                                            "Trigger has no outgoing edge. Connect it to continue chain.",
                                        );
                                    }
                                }
                            });
                        });
                        ui.add_space(6.0);
                    }
                });
            }

            if let Some(command) = pending_command {
                let is_layout_command = matches!(command, GraphCommand::SetNodePosition(_, _));
                let is_reset_layout = matches!(command, GraphCommand::ResetLayout);
                let is_draft_only_command = matches!(
                    command,
                    GraphCommand::AddConditionNode | GraphCommand::AddActionNode
                );
                let remembered_layout = Self::remember_graph_layout(&graph);
                let command_result = match command {
                    GraphCommand::AddTrigger => graph.add_trigger_chain().map(|_| ()),
                    GraphCommand::ResetLayout => {
                        let auto_positions =
                            Self::compute_auto_layout_positions(ui, &graph, &node_badges);
                        auto_positions
                            .into_iter()
                            .try_for_each(|(node_id, position)| {
                                graph.set_node_position(node_id, position)
                            })
                    }
                    GraphCommand::AddConditionNode => {
                        graph.add_condition_node(RuleCondition::Always).map(|_| ())
                    }
                    GraphCommand::SetTrigger(trigger_node_id, trigger) => {
                        graph.set_trigger_for_chain(trigger_node_id, trigger)
                    }
                    GraphCommand::AddActionNode => graph
                        .add_action_node(RuleAction::PlaySound {
                            channel: RuleSoundChannel::Movement,
                            sound_id: "sfx_placeholder".to_string(),
                        })
                        .map(|_| ()),
                    GraphCommand::SetCondition(node_id, condition) => {
                        graph.set_condition_for_node(node_id, condition)
                    }
                    GraphCommand::SetAction(node_id, action) => {
                        graph.set_action_for_node(node_id, action)
                    }
                    GraphCommand::SetNodePosition(node_id, position) => {
                        graph.set_node_position(node_id, position)
                    }
                    GraphCommand::RemoveNode(node_id) => graph.remove_node(node_id),
                    GraphCommand::Connect(from, to) => graph.connect_nodes(from, to),
                    GraphCommand::Disconnect(from, to) => {
                        graph.disconnect_nodes(from, to);
                        Ok(())
                    }
                    GraphCommand::DisconnectMany(edges) => {
                        for (from, to) in edges {
                            graph.disconnect_nodes(from, to);
                        }
                        Ok(())
                    }
                    GraphCommand::DisconnectNode(node_id) => graph.disconnect_node(node_id),
                };

                match command_result {
                    Ok(()) => {
                        graph_changed = true;
                        if is_reset_layout {
                            // Keep a visible border gap when snapping to auto layout.
                            graph_pan = [16.0, 16.0];
                            Self::enforce_graph_border_gap(&graph, graph_zoom, &mut graph_pan);
                        }
                        if !is_layout_command && !is_reset_layout {
                            Self::restore_graph_layout(&mut graph, &remembered_layout);
                        }
                        if is_layout_command || is_reset_layout || is_draft_only_command {
                            layout_changed = true;
                        } else {
                            scene_changed = true;
                        }
                    }
                    Err(error) => {
                        operation_error = Some(format!("Graph edit failed: {:?}", error));
                    }
                }
            }

            let mut after_rule_set = before_rule_set.clone();
            if scene_changed {
                match graph.to_rule_set() {
                    Ok(rule_set) => {
                        if rule_set != before_rule_set {
                            after_rule_set = rule_set;
                        } else {
                            scene_changed = false;
                        }
                    }
                    Err(error) => {
                        scene_changed = false;
                        let issue = Self::rule_graph_error_issue(&graph, &node_badges, &error);
                        operation_error = Some(format!(
                            "{} Scene JSON was not updated. Hint: {}",
                            issue.message, issue.hint
                        ));
                    }
                }
            }

            let state_changed = graph_changed || scene_changed || layout_changed;
            if state_changed {
                if !ui_state.execute_scene_rules_graph_command(
                    &active_scene_name,
                    SceneRulesGraphCommandData {
                        before_rule_set: before_rule_set.clone(),
                        after_rule_set,
                        before_graph: before_graph_snapshot.clone(),
                        after_graph: graph.clone(),
                        before_layout: before_layout_snapshot.clone(),
                        zoom: graph_zoom,
                        pan: graph_pan,
                    },
                ) {
                    operation_error =
                        Some("Failed to record scene graph change in undo history.".to_string());
                }
            } else if ui_state.rule_graph_for_scene(&active_scene_name).is_none() {
                ui_state.set_rule_graph_for_scene(active_scene_name.clone(), graph.clone());
            }

            if let Some(node_id) = selected_graph_node {
                if let Some(node_key) = graph.stable_node_key(node_id) {
                    ui_state.set_selection(Selection::RuleGraphNode {
                        scene_name: active_scene_name.clone(),
                        node_key,
                    });
                }
            }
        }

        ui_state.graph_connect_from_node = connect_from;
        ui_state.graph_connect_to_node = connect_to;
        ui_state.graph_canvas_zoom = graph_zoom;
        ui_state.graph_canvas_pan = graph_pan;
        ui_state.set_graph_view_for_scene(&active_scene_name, graph_zoom, graph_pan);
        if scene_changed {
            ui_state.scene_content_changed = true;
        }
        if let Some(error) = operation_error {
            ui.colored_label(egui::Color32::from_rgb(255, 120, 120), error);
        }
    }

    fn rule_graph_node_label(
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        node_id: u64,
    ) -> Option<String> {
        let node = graph.nodes.iter().find(|node| node.id == node_id)?;
        let badge = node_badges
            .get(&node_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        Some(format!(
            "{}: {}",
            badge,
            Self::rule_graph_node_kind_compact_label(&node.kind)
        ))
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

    fn collect_graph_validation_issues(
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
    ) -> Vec<GraphValidationIssue> {
        let mut issues = Vec::<GraphValidationIssue>::new();
        let graph_serialization_error = graph.to_rule_set().err();
        if let Some(error) = graph_serialization_error.as_ref() {
            issues.push(Self::rule_graph_error_issue(graph, node_badges, error));
        }

        let mut serialized_nodes = HashSet::<u64>::new();
        for chain in &graph.chains {
            if let Ok(sequence) = graph.chain_node_sequence(chain.trigger_node_id) {
                serialized_nodes.extend(sequence);
            }
        }

        for node in &graph.nodes {
            if matches!(
                node.kind,
                RuleGraphNodeKind::Condition(_) | RuleGraphNodeKind::Action(_)
            ) && !serialized_nodes.contains(&node.id)
            {
                let node_label = Self::rule_graph_node_label(graph, node_badges, node.id)
                    .unwrap_or_else(|| format!("node {}", node.id));
                issues.push(GraphValidationIssue {
                    severity: GraphValidationSeverity::Warning,
                    message: format!("{node_label} is detached from all trigger chains."),
                    hint: "Connect it into a trigger chain, or delete it if it is no longer needed. Detached nodes stay in editor drafts but are not exported to scene JSON/runtime.".to_string(),
                    fixes: vec![GraphValidationFix {
                        label: format!("Delete {}", node_label),
                        command: GraphValidationFixCommand::RemoveNode(node.id),
                    }],
                });
            }
        }

        issues
    }

    fn rule_graph_error_issue(
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        error: &RuleGraphError,
    ) -> GraphValidationIssue {
        match error {
            RuleGraphError::MissingTriggerNode { rule_id, node_id } => {
                let node_label = Self::rule_graph_node_label(graph, node_badges, *node_id)
                    .unwrap_or_else(|| format!("node {}", node_id));
                GraphValidationIssue {
                    severity: GraphValidationSeverity::Error,
                    message: format!(
                        "Rule '{}' references missing trigger {}.",
                        rule_id, node_label
                    ),
                    hint: "Delete and recreate the affected trigger chain.".to_string(),
                    fixes: Vec::new(),
                }
            }
            RuleGraphError::TriggerNodeKindMismatch { rule_id, node_id } => {
                let node_label = Self::rule_graph_node_label(graph, node_badges, *node_id)
                    .unwrap_or_else(|| format!("node {}", node_id));
                GraphValidationIssue {
                    severity: GraphValidationSeverity::Error,
                    message: format!(
                        "Rule '{}' trigger node has invalid kind at {}.",
                        rule_id, node_label
                    ),
                    hint: "Replace the node with a proper trigger node for this chain.".to_string(),
                    fixes: Vec::new(),
                }
            }
            RuleGraphError::MissingNode { rule_id, node_id } => {
                let node_label = Self::rule_graph_node_label(graph, node_badges, *node_id)
                    .unwrap_or_else(|| format!("node {}", node_id));
                GraphValidationIssue {
                    severity: GraphValidationSeverity::Error,
                    message: format!("Rule '{}' references missing {}.", rule_id, node_label),
                    hint: "Disconnect stale edges or remove/recreate the broken chain segment."
                        .to_string(),
                    fixes: Vec::new(),
                }
            }
            RuleGraphError::NonLinearChain { rule_id, node_id } => {
                let node_label = Self::rule_graph_node_label(graph, node_badges, *node_id)
                    .unwrap_or_else(|| format!("node {}", node_id));
                let extra_edges = Self::non_linear_extra_edges(graph, *node_id);
                let mut fixes = Vec::new();
                if !extra_edges.is_empty() {
                    fixes.push(GraphValidationFix {
                        label: format!("Disconnect extra branch edge(s) from {}", node_label),
                        command: GraphValidationFixCommand::DisconnectEdges(extra_edges),
                    });
                }
                GraphValidationIssue {
                    severity: GraphValidationSeverity::Error,
                    message: format!(
                        "Rule '{}' branches at {} (multiple outgoing edges).",
                        rule_id, node_label
                    ),
                    hint:
                        "Disconnect extra outgoing edges from this node, or split logic into separate trigger chains."
                            .to_string(),
                    fixes,
                }
            }
            RuleGraphError::CycleDetected { rule_id, node_id } => {
                let node_label = Self::rule_graph_node_label(graph, node_badges, *node_id)
                    .unwrap_or_else(|| format!("node {}", node_id));
                GraphValidationIssue {
                    severity: GraphValidationSeverity::Error,
                    message: format!("Rule '{}' contains a cycle at {}.", rule_id, node_label),
                    hint: "Disconnect one edge in the loop so each chain has a forward-only path."
                        .to_string(),
                    fixes: Vec::new(),
                }
            }
        }
    }

    fn non_linear_extra_edges(graph: &RuleGraph, node_id: u64) -> Vec<(u64, u64)> {
        let node_by_id = graph
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<HashMap<_, _>>();
        let mut targets = graph
            .edges
            .iter()
            .filter(|edge| edge.from == node_id)
            .map(|edge| edge.to)
            .collect::<Vec<_>>();
        targets.sort_by_key(|target| {
            node_by_id
                .get(target)
                .map(|node| (Self::graph_node_kind_rank(&node.kind), *target))
                .unwrap_or((usize::MAX, *target))
        });
        if targets.len() <= 1 {
            return Vec::new();
        }
        targets
            .into_iter()
            .skip(1)
            .map(|target| (node_id, target))
            .collect()
    }

    fn render_graph_validation_summary(
        ui: &mut egui::Ui,
        issues: &[GraphValidationIssue],
    ) -> Option<GraphValidationFixCommand> {
        let mut clicked_fix = None;
        if issues.is_empty() {
            ui.colored_label(
                egui::Color32::from_rgb(150, 210, 150),
                "Validation: Serializable (runtime/export ready)",
            );
            return None;
        }

        let error_count = issues
            .iter()
            .filter(|issue| issue.severity == GraphValidationSeverity::Error)
            .count();
        let warning_count = issues
            .iter()
            .filter(|issue| issue.severity == GraphValidationSeverity::Warning)
            .count();

        let header_color = if error_count > 0 {
            egui::Color32::from_rgb(255, 130, 130)
        } else {
            egui::Color32::from_rgb(255, 210, 120)
        };
        ui.group(|ui| {
            ui.colored_label(
                header_color,
                format!(
                    "Validation: {} error(s), {} warning(s)",
                    error_count, warning_count
                ),
            );
            for issue in issues {
                let (prefix, color) = match issue.severity {
                    GraphValidationSeverity::Error => {
                        ("Error", egui::Color32::from_rgb(255, 140, 140))
                    }
                    GraphValidationSeverity::Warning => {
                        ("Warning", egui::Color32::from_rgb(255, 210, 120))
                    }
                };
                ui.colored_label(color, format!("{prefix}: {}", issue.message));
                ui.label(format!("Hint: {}", issue.hint));
                if !issue.fixes.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for fix in &issue.fixes {
                            if ui.small_button(&fix.label).clicked() {
                                clicked_fix = Some(fix.command.clone());
                            }
                        }
                    });
                }
            }
        });
        clicked_fix
    }

    fn compute_auto_layout_positions(
        ui: &egui::Ui,
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
    ) -> HashMap<u64, [f32; 2]> {
        let mut node_sizes = HashMap::<u64, egui::Vec2>::new();
        for node in &graph.nodes {
            let badge = node_badges
                .get(&node.id)
                .cloned()
                .unwrap_or_else(|| "?".to_string());
            let label = format!(
                "{}: {}",
                badge,
                Self::rule_graph_node_kind_compact_label(&node.kind)
            );
            node_sizes.insert(node.id, Self::graph_node_size_for_label(ui, &label, 1.0));
        }
        Self::compute_auto_layout_positions_from_sizes(graph, &node_sizes)
    }

    fn compute_auto_layout_positions_from_sizes(
        graph: &RuleGraph,
        node_sizes: &HashMap<u64, egui::Vec2>,
    ) -> HashMap<u64, [f32; 2]> {
        let node_by_id = graph
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<HashMap<_, _>>();
        let mut node_ids = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        node_ids.sort_by_key(|node_id| {
            (
                Self::graph_node_kind_rank(&node_by_id[node_id].kind),
                *node_id,
            )
        });

        let mut incoming_count = HashMap::<u64, usize>::new();
        let mut outgoing = HashMap::<u64, Vec<u64>>::new();
        let mut incoming = HashMap::<u64, Vec<u64>>::new();
        for node_id in &node_ids {
            incoming_count.insert(*node_id, 0);
            outgoing.insert(*node_id, Vec::new());
            incoming.insert(*node_id, Vec::new());
        }
        for edge in &graph.edges {
            if !node_by_id.contains_key(&edge.from) || !node_by_id.contains_key(&edge.to) {
                continue;
            }
            outgoing.entry(edge.from).or_default().push(edge.to);
            incoming.entry(edge.to).or_default().push(edge.from);
            *incoming_count.entry(edge.to).or_default() += 1;
        }

        for targets in outgoing.values_mut() {
            targets.sort_by_key(|node_id| {
                (
                    Self::graph_node_kind_rank(&node_by_id[node_id].kind),
                    *node_id,
                )
            });
        }

        let mut ready = node_ids
            .iter()
            .copied()
            .filter(|node_id| incoming_count.get(node_id).copied().unwrap_or_default() == 0)
            .collect::<Vec<_>>();
        ready.sort_by_key(|node_id| {
            (
                Self::graph_node_kind_rank(&node_by_id[node_id].kind),
                *node_id,
            )
        });
        ready.reverse();

        let mut layer = node_ids
            .iter()
            .copied()
            .map(|node_id| (node_id, 0_usize))
            .collect::<HashMap<_, _>>();
        let mut processed = HashSet::<u64>::new();
        let mut topo_order = HashMap::<u64, usize>::new();
        let mut topo_index = 0_usize;

        while let Some(node_id) = ready.pop() {
            if !processed.insert(node_id) {
                continue;
            }
            topo_order.insert(node_id, topo_index);
            topo_index += 1;

            let current_layer = layer.get(&node_id).copied().unwrap_or_default();
            let targets = outgoing.get(&node_id).cloned().unwrap_or_default();
            for to in targets {
                let next_layer = current_layer + 1;
                let layer_entry = layer.entry(to).or_default();
                if *layer_entry < next_layer {
                    *layer_entry = next_layer;
                }
                if let Some(incoming) = incoming_count.get_mut(&to) {
                    *incoming = incoming.saturating_sub(1);
                    if *incoming == 0 {
                        ready.push(to);
                    }
                }
            }
            ready.sort_by_key(|candidate| {
                (
                    Self::graph_node_kind_rank(&node_by_id[candidate].kind),
                    *candidate,
                )
            });
            ready.reverse();
        }

        for node_id in node_ids.iter().copied() {
            if processed.contains(&node_id) {
                continue;
            }
            topo_order.insert(node_id, topo_index);
            topo_index += 1;
            processed.insert(node_id);
        }

        let mut layers = BTreeMap::<usize, Vec<u64>>::new();
        for node_id in node_ids {
            let node_layer = layer.get(&node_id).copied().unwrap_or_default();
            layers.entry(node_layer).or_default().push(node_id);
        }
        for layer_nodes in layers.values_mut() {
            layer_nodes.sort_by_key(|node_id| topo_order[node_id]);
        }

        let default_size = egui::vec2(
            RuleGraph::auto_layout_node_width(),
            RuleGraph::auto_layout_node_height(),
        );
        let mut positions = HashMap::<u64, [f32; 2]>::new();
        for (_layer_index, layer_nodes) in layers {
            let mut y_top = RuleGraph::auto_layout_start_y();
            for node_id in layer_nodes {
                let size = node_sizes.get(&node_id).copied().unwrap_or(default_size);
                let center_y = y_top + size.y * 0.5;
                positions.insert(node_id, [RuleGraph::auto_layout_start_x(), center_y]);
                y_top += size.y + RuleGraph::auto_layout_vertical_edge_spacing();
            }
        }

        let mut topo_nodes = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        topo_nodes.sort_by_key(|node_id| topo_order[node_id]);

        let horizontal_gap = RuleGraph::auto_layout_horizontal_edge_spacing();
        for node_id in &topo_nodes {
            let node_size = node_sizes.get(node_id).copied().unwrap_or(default_size);
            let predecessors = incoming.get(node_id).cloned().unwrap_or_default();
            let x = if predecessors.is_empty() {
                RuleGraph::auto_layout_start_x()
            } else {
                predecessors
                    .into_iter()
                    .filter_map(|from| {
                        let from_pos = positions.get(&from).copied()?;
                        let from_size = node_sizes.get(&from).copied().unwrap_or(default_size);
                        Some(from_pos[0] + from_size.x * 0.5 + horizontal_gap + node_size.x * 0.5)
                    })
                    .fold(RuleGraph::auto_layout_start_x(), f32::max)
            };
            if let Some(position) = positions.get_mut(node_id) {
                position[0] = x;
            }
        }

        // Enforce non-overlap strictly for node pairs that overlap vertically.
        let max_passes = topo_nodes.len().max(1).pow(2);
        for _ in 0..max_passes {
            let mut changed = false;
            for left_index in 0..topo_nodes.len() {
                for right_index in (left_index + 1)..topo_nodes.len() {
                    let left_id = topo_nodes[left_index];
                    let right_id = topo_nodes[right_index];
                    let Some(left_pos) = positions.get(&left_id).copied() else {
                        continue;
                    };
                    let Some(right_pos) = positions.get(&right_id).copied() else {
                        continue;
                    };
                    let left_size = node_sizes.get(&left_id).copied().unwrap_or(default_size);
                    let right_size = node_sizes.get(&right_id).copied().unwrap_or(default_size);

                    let required_dy = left_size.y * 0.5 + right_size.y * 0.5;
                    let actual_dy = (right_pos[1] - left_pos[1]).abs();
                    if actual_dy >= required_dy {
                        continue;
                    }

                    let (right_id, left_pos, right_pos, left_size, right_size) =
                        if left_pos[0] <= right_pos[0] {
                            (right_id, left_pos, right_pos, left_size, right_size)
                        } else {
                            (left_id, right_pos, left_pos, right_size, left_size)
                        };
                    let required_dx = left_size.x * 0.5 + right_size.x * 0.5 + horizontal_gap;
                    let actual_dx = right_pos[0] - left_pos[0];
                    if actual_dx >= required_dx {
                        continue;
                    }
                    if let Some(right_pos_mut) = positions.get_mut(&right_id) {
                        right_pos_mut[0] += required_dx - actual_dx;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }

        positions
    }

    fn graph_node_kind_rank(kind: &RuleGraphNodeKind) -> usize {
        match kind {
            RuleGraphNodeKind::Trigger(_) => 0,
            RuleGraphNodeKind::Condition(_) => 1,
            RuleGraphNodeKind::Action(_) => 2,
        }
    }

    fn render_graph_canvas(
        ui: &mut egui::Ui,
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        graph_zoom: f32,
        graph_pan: &mut [f32; 2],
    ) -> (Option<(u64, [f32; 2])>, Option<u64>) {
        let desired_size = egui::vec2(ui.available_width(), ui.available_height().max(220.0));
        let (rect, canvas_response) =
            ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, 6.0, egui::Color32::from_rgb(20, 24, 30));
        painter.rect_stroke(
            rect,
            6.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(70)),
            egui::StrokeKind::Inside,
        );

        if graph.nodes.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No Graph Nodes",
                egui::TextStyle::Body.resolve(ui.style()),
                egui::Color32::from_gray(170),
            );
            return (None, None);
        }

        let scale = graph_zoom.max(0.01);

        let to_canvas = |position: [f32; 2]| -> egui::Pos2 {
            egui::pos2(
                rect.left() + graph_pan[0] + position[0] * scale,
                rect.top() + graph_pan[1] + position[1] * scale,
            )
        };

        let mut node_positions = HashMap::<u64, egui::Pos2>::new();
        let mut node_labels = HashMap::<u64, String>::new();
        let mut node_rects = HashMap::<u64, egui::Rect>::new();
        for node in &graph.nodes {
            let center = to_canvas(node.position);
            let badge = node_badges
                .get(&node.id)
                .cloned()
                .unwrap_or_else(|| "?".to_string());
            let label = format!(
                "{}: {}",
                badge,
                Self::rule_graph_node_kind_compact_label(&node.kind)
            );
            let node_size = Self::graph_node_size_for_label(ui, &label, scale);
            let node_rect = egui::Rect::from_center_size(center, node_size);
            node_positions.insert(node.id, center);
            node_labels.insert(node.id, label);
            node_rects.insert(node.id, node_rect);
        }

        for edge in &graph.edges {
            let Some(from_rect) = node_rects.get(&edge.from).copied() else {
                continue;
            };
            let Some(to_rect) = node_rects.get(&edge.to).copied() else {
                continue;
            };
            let Some((start, end, direction)) = Self::graph_edge_points(from_rect, to_rect) else {
                continue;
            };
            let stroke = egui::Stroke::new(
                Self::graph_edge_stroke_width(scale),
                egui::Color32::from_rgb(130, 150, 185),
            );
            let arrow_length = (10.0 * scale).clamp(6.0, 18.0);
            let arrow_width = (5.0 * scale).clamp(3.0, 12.0);
            let body_end = end - direction * (arrow_length * 0.85);
            painter.line_segment([start, body_end], stroke);
            let perp = egui::vec2(-direction.y, direction.x);
            let arrow_left = end - direction * arrow_length + perp * arrow_width;
            let arrow_right = end - direction * arrow_length - perp * arrow_width;
            painter.line_segment([end, arrow_left], stroke);
            painter.line_segment([end, arrow_right], stroke);
        }

        let mut moved_node = None;
        let mut clicked_node = None;
        let mut any_node_dragged = false;
        let node_corner_radius = (6.0 * scale).clamp(2.0, 18.0);
        let node_stroke_width = (1.0 * scale).clamp(0.7, 2.5);
        let node_font_size = Self::graph_node_font_size(scale);
        for node in &graph.nodes {
            let Some(center) = node_positions.get(&node.id).copied() else {
                continue;
            };
            let Some(label) = node_labels.get(&node.id).cloned() else {
                continue;
            };
            let node_size = Self::graph_node_size_for_label(ui, &label, scale);
            let (fill, stroke) = match node.kind {
                RuleGraphNodeKind::Trigger(_) => (
                    egui::Color32::from_rgb(45, 122, 199),
                    egui::Color32::from_rgb(140, 190, 245),
                ),
                RuleGraphNodeKind::Condition(_) => (
                    egui::Color32::from_rgb(139, 92, 46),
                    egui::Color32::from_rgb(214, 158, 106),
                ),
                RuleGraphNodeKind::Action(_) => (
                    egui::Color32::from_rgb(58, 140, 82),
                    egui::Color32::from_rgb(133, 208, 154),
                ),
            };
            let node_rect = egui::Rect::from_center_size(center, node_size);
            let response = ui.interact(
                node_rect,
                ui.make_persistent_id(("graph_canvas_node", node.id)),
                egui::Sense::click_and_drag(),
            );
            if response.clicked() {
                clicked_node = Some(node.id);
            }
            if response.dragged() {
                any_node_dragged = true;
                let delta = ui.ctx().input(|input| input.pointer.delta());
                if delta != egui::Vec2::ZERO && scale > 0.0 {
                    moved_node = Some((
                        node.id,
                        [
                            node.position[0] + (delta.x / scale),
                            node.position[1] + (delta.y / scale),
                        ],
                    ));
                }
            }
            let draw_fill = if response.dragged() {
                fill.gamma_multiply(1.2)
            } else {
                fill
            };
            painter.rect_filled(node_rect, node_corner_radius, draw_fill);
            painter.rect_stroke(
                node_rect,
                node_corner_radius,
                egui::Stroke::new(node_stroke_width, stroke),
                egui::StrokeKind::Inside,
            );
            painter.text(
                node_rect.center(),
                egui::Align2::CENTER_CENTER,
                &label,
                egui::FontId::proportional(node_font_size),
                egui::Color32::WHITE,
            );
        }

        if !any_node_dragged && canvas_response.dragged() {
            let delta = ui.ctx().input(|input| input.pointer.delta());
            if delta != egui::Vec2::ZERO {
                graph_pan[0] += delta.x;
                graph_pan[1] += delta.y;
            }
        }

        (moved_node, clicked_node)
    }

    fn render_graph_selected_node_editor(
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
                let mut edited_condition = condition;
                let mut kind = Self::graph_condition_kind(condition);
                let kind_salt = format!("graph_canvas_condition_kind::{scene_name}::{node_id}");
                egui::ComboBox::from_id_salt(kind_salt)
                    .selected_text(Self::graph_condition_kind_label(kind))
                    .show_ui(ui, |ui| {
                        for candidate in [
                            GraphConditionKind::Always,
                            GraphConditionKind::TargetExists,
                            GraphConditionKind::KeyHeld,
                            GraphConditionKind::EntityActive,
                        ] {
                            ui.selectable_value(
                                &mut kind,
                                candidate,
                                Self::graph_condition_kind_label(candidate),
                            );
                        }
                    });
                if kind != Self::graph_condition_kind(condition) {
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

    fn graph_edge_points(
        from_rect: egui::Rect,
        to_rect: egui::Rect,
    ) -> Option<(egui::Pos2, egui::Pos2, egui::Vec2)> {
        let from_center = from_rect.center();
        let to_center = to_rect.center();
        let center_delta = to_center - from_center;
        if center_delta.length_sq() <= f32::EPSILON {
            return None;
        }
        let start = Self::rect_border_point_toward(from_rect, to_center);
        let end = Self::rect_border_point_toward(to_rect, from_center);
        let line_delta = end - start;
        let line_length = line_delta.length();
        if line_length <= f32::EPSILON {
            return None;
        }
        Some((start, end, line_delta / line_length))
    }

    fn rect_border_point_toward(rect: egui::Rect, toward: egui::Pos2) -> egui::Pos2 {
        let center = rect.center();
        let delta = toward - center;
        let half_size = rect.size() * 0.5;
        if half_size.x <= f32::EPSILON || half_size.y <= f32::EPSILON {
            return center;
        }
        let scale = (delta.x.abs() / half_size.x).max(delta.y.abs() / half_size.y);
        if scale <= f32::EPSILON {
            return center;
        }
        center + delta / scale
    }

    fn enforce_graph_border_gap(graph: &RuleGraph, graph_zoom: f32, graph_pan: &mut [f32; 2]) {
        let Some(min_x) = graph
            .nodes
            .iter()
            .map(|node| node.position[0])
            .min_by(|a, b| a.total_cmp(b))
        else {
            return;
        };
        let Some(min_y) = graph
            .nodes
            .iter()
            .map(|node| node.position[1])
            .min_by(|a, b| a.total_cmp(b))
        else {
            return;
        };

        let scale = graph_zoom.max(0.01);
        let node_size = Self::graph_node_max_size(scale);
        let border_gap = 10.0;
        let min_center_x = border_gap + node_size.x * 0.5;
        let min_center_y = border_gap + node_size.y * 0.5;
        let required_pan_x = min_center_x - (min_x * scale);
        let required_pan_y = min_center_y - (min_y * scale);

        if graph_pan[0] < required_pan_x {
            graph_pan[0] = required_pan_x;
        }
        if graph_pan[1] < required_pan_y {
            graph_pan[1] = required_pan_y;
        }
    }

    fn graph_node_max_size(scale: f32) -> egui::Vec2 {
        egui::vec2(
            (320.0 * scale).clamp(120.0, 860.0),
            (36.0 * scale).clamp(18.0, 96.0),
        )
    }

    fn graph_node_min_size(scale: f32) -> egui::Vec2 {
        egui::vec2(
            (120.0 * scale).clamp(80.0, 300.0),
            (20.0 * scale).clamp(14.0, 48.0),
        )
    }

    fn graph_node_size_for_label(ui: &egui::Ui, label: &str, scale: f32) -> egui::Vec2 {
        let font_size = Self::graph_node_font_size(scale);
        let font_id = egui::FontId::proportional(font_size);
        let text_size = ui
            .painter()
            .layout_no_wrap(label.to_string(), font_id, egui::Color32::WHITE)
            .size();
        let padding_x = (16.0 * scale).clamp(8.0, 36.0);
        let padding_y = (8.0 * scale).clamp(4.0, 24.0);
        let desired = egui::vec2(text_size.x + padding_x * 2.0, text_size.y + padding_y * 2.0);
        let min_size = Self::graph_node_min_size(scale);
        let max_size = Self::graph_node_max_size(scale);
        egui::vec2(
            desired.x.clamp(min_size.x, max_size.x),
            desired.y.clamp(min_size.y, max_size.y),
        )
    }

    fn graph_node_font_size(scale: f32) -> f32 {
        (11.0 * scale).clamp(7.0, 24.0)
    }

    fn graph_edge_stroke_width(scale: f32) -> f32 {
        (1.5 * scale).clamp(0.7, 4.0)
    }

    fn remember_graph_layout(graph: &RuleGraph) -> HashMap<String, [f32; 2]> {
        graph
            .nodes
            .iter()
            .filter_map(|node| {
                graph
                    .stable_node_key(node.id)
                    .map(|node_key| (node_key, node.position))
            })
            .collect()
    }

    fn restore_graph_layout(graph: &mut RuleGraph, layout: &HashMap<String, [f32; 2]>) {
        let node_ids = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        for node_id in node_ids {
            let Some(node_key) = graph.stable_node_key(node_id) else {
                continue;
            };
            let Some(position) = layout.get(&node_key).copied() else {
                continue;
            };
            let _ = graph.set_node_position(node_id, position);
        }
    }

    fn rule_graph_node_kind_compact_label(kind: &RuleGraphNodeKind) -> String {
        match kind {
            RuleGraphNodeKind::Trigger(trigger) => {
                format!("Trigger {}", Self::trigger_summary(*trigger))
            }
            RuleGraphNodeKind::Condition(condition) => {
                format!("Condition {}", Self::condition_summary(*condition))
            }
            RuleGraphNodeKind::Action(action) => {
                format!("Action {}", Self::action_summary(action))
            }
        }
    }

    fn trigger_summary(trigger: RuleTrigger) -> String {
        match trigger {
            RuleTrigger::OnStart => "OnStart".to_string(),
            RuleTrigger::OnUpdate => "OnUpdate".to_string(),
            RuleTrigger::OnPlayerMove => "OnPlayerMove".to_string(),
            RuleTrigger::OnKey { key } => format!("OnKey({})", Self::key_label(key)),
            RuleTrigger::OnCollision => "OnCollision".to_string(),
            RuleTrigger::OnTrigger => "OnTrigger".to_string(),
        }
    }

    fn condition_summary(condition: RuleCondition) -> String {
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

    fn action_summary(action: &RuleAction) -> String {
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

    fn key_label(key: RuleKey) -> &'static str {
        match key {
            RuleKey::Up => "Up",
            RuleKey::Down => "Down",
            RuleKey::Left => "Left",
            RuleKey::Right => "Right",
            RuleKey::DebugToggle => "DebugToggle",
        }
    }

    fn sound_channel_label(channel: RuleSoundChannel) -> &'static str {
        match channel {
            RuleSoundChannel::Movement => "Movement",
            RuleSoundChannel::Collision => "Collision",
        }
    }

    fn target_label(target: RuleTarget) -> String {
        match target {
            RuleTarget::Player => "Player".to_string(),
            RuleTarget::Entity(entity_id) => format!("Entity({})", entity_id),
        }
    }

    fn graph_trigger_kind(trigger: RuleTrigger) -> GraphTriggerKind {
        match trigger {
            RuleTrigger::OnStart => GraphTriggerKind::Start,
            RuleTrigger::OnUpdate => GraphTriggerKind::Update,
            RuleTrigger::OnPlayerMove => GraphTriggerKind::PlayerMove,
            RuleTrigger::OnKey { .. } => GraphTriggerKind::Key,
            RuleTrigger::OnCollision => GraphTriggerKind::Collision,
            RuleTrigger::OnTrigger => GraphTriggerKind::Trigger,
        }
    }

    fn graph_trigger_kind_label(kind: GraphTriggerKind) -> &'static str {
        match kind {
            GraphTriggerKind::Start => "OnStart",
            GraphTriggerKind::Update => "OnUpdate",
            GraphTriggerKind::PlayerMove => "OnPlayerMove",
            GraphTriggerKind::Key => "OnKey",
            GraphTriggerKind::Collision => "OnCollision",
            GraphTriggerKind::Trigger => "OnTrigger",
        }
    }

    fn graph_default_trigger(kind: GraphTriggerKind) -> RuleTrigger {
        match kind {
            GraphTriggerKind::Start => RuleTrigger::OnStart,
            GraphTriggerKind::Update => RuleTrigger::OnUpdate,
            GraphTriggerKind::PlayerMove => RuleTrigger::OnPlayerMove,
            GraphTriggerKind::Key => RuleTrigger::OnKey { key: RuleKey::Up },
            GraphTriggerKind::Collision => RuleTrigger::OnCollision,
            GraphTriggerKind::Trigger => RuleTrigger::OnTrigger,
        }
    }

    fn graph_condition_kind(condition: RuleCondition) -> GraphConditionKind {
        match condition {
            RuleCondition::Always => GraphConditionKind::Always,
            RuleCondition::TargetExists { .. } => GraphConditionKind::TargetExists,
            RuleCondition::KeyHeld { .. } => GraphConditionKind::KeyHeld,
            RuleCondition::EntityActive { .. } => GraphConditionKind::EntityActive,
        }
    }

    fn graph_condition_kind_label(kind: GraphConditionKind) -> &'static str {
        match kind {
            GraphConditionKind::Always => "Always",
            GraphConditionKind::TargetExists => "TargetExists",
            GraphConditionKind::KeyHeld => "KeyHeld",
            GraphConditionKind::EntityActive => "EntityActive",
        }
    }

    fn graph_default_condition(kind: GraphConditionKind) -> RuleCondition {
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

    fn graph_action_kind(action: &RuleAction) -> GraphActionKind {
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

    fn graph_action_kind_label(kind: GraphActionKind) -> &'static str {
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

    fn graph_default_action(kind: GraphActionKind) -> RuleAction {
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

    fn edit_graph_condition_payload(
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

    fn edit_graph_action_payload(
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

    fn edit_rule_target(ui: &mut egui::Ui, target: &mut RuleTarget, id_salt: &str) -> bool {
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

    fn edit_rule_key(ui: &mut egui::Ui, key: &mut RuleKey, id_salt: &str) -> bool {
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

    /// Renders the log/console panel at the bottom of the screen
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
mod tests {
    use super::{GraphValidationFixCommand, PanelSystem};
    use crate::ui::rule_graph::RuleGraph;
    use std::collections::HashMap;
    use toki_core::rules::{
        Rule, RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel, RuleTarget,
        RuleTrigger,
    };

    #[test]
    fn trigger_summary_is_semantic() {
        assert_eq!(
            PanelSystem::trigger_summary(RuleTrigger::OnStart),
            "OnStart"
        );
        assert_eq!(
            PanelSystem::trigger_summary(RuleTrigger::OnKey { key: RuleKey::Left }),
            "OnKey(Left)"
        );
    }

    #[test]
    fn condition_summary_is_semantic() {
        assert_eq!(
            PanelSystem::condition_summary(RuleCondition::Always),
            "Always"
        );
        assert_eq!(
            PanelSystem::condition_summary(RuleCondition::TargetExists {
                target: RuleTarget::Player
            }),
            "TargetExists(Player)"
        );
    }

    #[test]
    fn action_summary_is_semantic() {
        assert_eq!(
            PanelSystem::action_summary(&RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_step".to_string(),
            }),
            "PlaySound(Movement, sfx_step)"
        );
        assert_eq!(
            PanelSystem::action_summary(&RuleAction::PlayMusic {
                track_id: "bgm_forest".to_string(),
            }),
            "PlayMusic(bgm_forest)"
        );
    }

    #[test]
    fn sanitize_grid_size_axis_clamps_to_minimum_one() {
        assert_eq!(PanelSystem::sanitize_grid_size_axis(-32), 1);
        assert_eq!(PanelSystem::sanitize_grid_size_axis(0), 1);
        assert_eq!(PanelSystem::sanitize_grid_size_axis(24), 24);
    }

    #[test]
    fn first_grid_line_at_or_before_handles_negative_coordinates() {
        assert_eq!(PanelSystem::first_grid_line_at_or_before(0, 16), 0);
        assert_eq!(PanelSystem::first_grid_line_at_or_before(15, 16), 0);
        assert_eq!(PanelSystem::first_grid_line_at_or_before(16, 16), 16);
        assert_eq!(PanelSystem::first_grid_line_at_or_before(-1, 16), -16);
        assert_eq!(PanelSystem::first_grid_line_at_or_before(-17, 16), -32);
    }

    #[test]
    fn grid_world_lines_emits_step_aligned_lines_inside_range() {
        assert_eq!(PanelSystem::grid_world_lines(3, 40, 16), vec![16, 32]);
        assert_eq!(PanelSystem::grid_world_lines(-20, 20, 16), vec![-16, 0, 16]);
    }

    #[test]
    fn compute_viewport_display_rect_keeps_aspect_and_centers() {
        let outer = egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), egui::vec2(320.0, 144.0));
        let display = PanelSystem::compute_viewport_display_rect(outer, (160, 144), false);
        assert_eq!(display.width(), 160.0);
        assert_eq!(display.height(), 144.0);
        assert_eq!(display.left(), 80.0);
        assert_eq!(display.right(), 240.0);
    }

    #[test]
    fn compute_viewport_display_rect_uses_full_rect_for_responsive_viewports() {
        let outer =
            egui::Rect::from_min_size(egui::Pos2::new(10.0, 20.0), egui::vec2(640.0, 360.0));
        let display = PanelSystem::compute_viewport_display_rect(outer, (640, 360), true);
        assert_eq!(display, outer);
    }

    #[test]
    fn map_editor_tile_screen_rect_maps_tile_bounds_through_camera_scale() {
        let display =
            egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), egui::vec2(160.0, 144.0));
        let tile_rect = PanelSystem::map_editor_tile_screen_rect(
            display,
            (160, 144),
            glam::IVec2::ZERO,
            1.0,
            glam::UVec2::new(16, 16),
            glam::UVec2::new(1, 2),
        )
        .expect("tile screen rect should be computed");

        assert_eq!(tile_rect.min, egui::pos2(16.0, 32.0));
        assert_eq!(tile_rect.max, egui::pos2(32.0, 48.0));
    }

    #[test]
    fn restore_graph_layout_preserves_existing_nodes_after_add_chain() {
        let rules = RuleSet {
            rules: vec![Rule {
                id: "rule_1".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnStart,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::PlayMusic {
                    track_id: "bgm_1".to_string(),
                }],
            }],
        };
        let mut graph = RuleGraph::from_rule_set(&rules);
        let initial_nodes = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();

        for (index, node_id) in initial_nodes.iter().enumerate() {
            graph
                .set_node_position(*node_id, [400.0 + index as f32 * 37.0, 250.0])
                .expect("existing node should accept custom position");
        }

        let remembered_layout = PanelSystem::remember_graph_layout(&graph);
        graph
            .add_trigger_chain()
            .expect("adding trigger chain should succeed");
        PanelSystem::restore_graph_layout(&mut graph, &remembered_layout);

        for node_id in initial_nodes {
            let node = graph
                .nodes
                .iter()
                .find(|node| node.id == node_id)
                .expect("original node should still exist");
            let node_key = graph
                .stable_node_key(node_id)
                .expect("original node should still have stable key");
            assert_eq!(
                Some(node.position),
                remembered_layout.get(&node_key).copied(),
                "layout should be preserved for existing node key {node_key}"
            );
        }
    }

    #[test]
    fn graph_zoom_scales_node_visuals() {
        let zoomed_out = PanelSystem::graph_node_max_size(0.5);
        let zoomed_in = PanelSystem::graph_node_max_size(2.0);
        assert!(
            zoomed_out.x < zoomed_in.x && zoomed_out.y < zoomed_in.y,
            "node size should increase with zoom"
        );

        let font_out = PanelSystem::graph_node_font_size(0.5);
        let font_in = PanelSystem::graph_node_font_size(2.0);
        assert!(font_out < font_in, "font size should increase with zoom");

        let edge_out = PanelSystem::graph_edge_stroke_width(0.5);
        let edge_in = PanelSystem::graph_edge_stroke_width(2.0);
        assert!(edge_out < edge_in, "edge stroke should increase with zoom");
    }

    #[test]
    fn enforce_graph_border_gap_moves_pan_when_left_or_top_nodes_touch_border() {
        let mut graph = RuleGraph::from_rule_set(&RuleSet {
            rules: vec![Rule {
                id: "rule_1".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnStart,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::PlayMusic {
                    track_id: "bgm_1".to_string(),
                }],
            }],
        });
        for node in &mut graph.nodes {
            node.position[0] = 0.0;
            node.position[1] = 0.0;
        }

        let mut pan = [0.0, 0.0];
        PanelSystem::enforce_graph_border_gap(&graph, 1.0, &mut pan);

        assert!(
            pan[0] > 0.0,
            "x pan should be increased to preserve border gap"
        );
        assert!(
            pan[1] > 0.0,
            "y pan should be increased to preserve border gap"
        );
    }

    #[test]
    fn collect_graph_validation_issues_reports_non_linear_chain_error() {
        let rules = RuleSet {
            rules: vec![Rule {
                id: "rule_1".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnStart,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::PlayMusic {
                    track_id: "bgm_1".to_string(),
                }],
            }],
        };
        let mut graph = RuleGraph::from_rule_set(&rules);
        let trigger = graph.chains[0].trigger_node_id;
        let detached = graph
            .add_action_node(RuleAction::PlayMusic {
                track_id: "bgm_2".to_string(),
            })
            .expect("detached node should be added");
        graph
            .connect_nodes(trigger, detached)
            .expect("adding a second trigger outgoing edge should be allowed in free graph");

        let badges = PanelSystem::rule_graph_node_badges(&graph);
        let issues = PanelSystem::collect_graph_validation_issues(&graph, &badges);
        let issue = issues
            .iter()
            .find(|issue| {
                issue.severity == super::GraphValidationSeverity::Error
                    && issue.message.contains("multiple outgoing edges")
            })
            .expect("non-linear chain issue should be reported");
        assert!(issue.fixes.iter().any(|fix| matches!(
            &fix.command,
            GraphValidationFixCommand::DisconnectEdges(edges)
                if edges.iter().any(|(from, _)| *from == trigger)
        )));
    }

    #[test]
    fn collect_graph_validation_issues_warns_for_detached_action_nodes() {
        let rules = RuleSet {
            rules: vec![Rule {
                id: "rule_1".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnStart,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::PlayMusic {
                    track_id: "bgm_1".to_string(),
                }],
            }],
        };
        let mut graph = RuleGraph::from_rule_set(&rules);
        let detached = graph
            .add_action_node(RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_step".to_string(),
            })
            .expect("detached action should be added");

        let badges = PanelSystem::rule_graph_node_badges(&graph);
        let detached_label = PanelSystem::rule_graph_node_label(&graph, &badges, detached)
            .expect("detached node should have a display label");
        let issues = PanelSystem::collect_graph_validation_issues(&graph, &badges);
        let issue = issues
            .iter()
            .find(|issue| {
                issue.severity == super::GraphValidationSeverity::Warning
                    && issue.message.contains(&detached_label)
                    && issue.message.contains("detached")
            })
            .expect("detached node warning should be reported");
        assert!(issue.fixes.iter().any(|fix| {
            matches!(fix.command, GraphValidationFixCommand::RemoveNode(node_id) if node_id == detached)
        }));
    }

    #[test]
    fn auto_layout_uses_edge_direction_and_positions_all_nodes() {
        let rules = RuleSet {
            rules: vec![Rule {
                id: "rule_1".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnStart,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::PlayMusic {
                    track_id: "bgm_1".to_string(),
                }],
            }],
        };
        let mut graph = RuleGraph::from_rule_set(&rules);
        let detached_action = graph
            .add_action_node(RuleAction::PlayMusic {
                track_id: "bgm_2".to_string(),
            })
            .expect("detached node should be addable");
        let trigger = graph.chains[0].trigger_node_id;
        graph
            .connect_nodes(detached_action, trigger)
            .expect("detached action should be connectable to trigger");

        let node_sizes = graph
            .nodes
            .iter()
            .map(|node| (node.id, egui::vec2(180.0, 36.0)))
            .collect::<HashMap<_, _>>();
        let positions = PanelSystem::compute_auto_layout_positions_from_sizes(&graph, &node_sizes);
        assert_eq!(
            positions.len(),
            graph.nodes.len(),
            "auto-layout should position all nodes, including detached/editor-only nodes"
        );

        for edge in &graph.edges {
            let from = positions
                .get(&edge.from)
                .expect("edge source must have a position");
            let to = positions
                .get(&edge.to)
                .expect("edge target must have a position");
            assert!(
                from[0] < to[0],
                "edge direction should move left-to-right in auto-layout ({} -> {})",
                edge.from,
                edge.to
            );
        }
    }

    #[test]
    fn auto_layout_prevents_node_overlap() {
        let rules = RuleSet {
            rules: vec![Rule {
                id: "rule_1".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnStart,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::PlayMusic {
                    track_id: "bgm_1".to_string(),
                }],
            }],
        };
        let mut graph = RuleGraph::from_rule_set(&rules);
        for i in 0..6 {
            graph
                .add_action_node(RuleAction::PlaySound {
                    channel: RuleSoundChannel::Movement,
                    sound_id: format!("sfx_{i}"),
                })
                .expect("standalone action should be addable");
        }

        let mut node_sizes = HashMap::<u64, egui::Vec2>::new();
        for (index, node) in graph.nodes.iter().enumerate() {
            let width = 140.0 + (index as f32 * 11.0);
            let height = 28.0 + ((index % 3) as f32 * 6.0);
            node_sizes.insert(node.id, egui::vec2(width, height));
        }

        let positions = PanelSystem::compute_auto_layout_positions_from_sizes(&graph, &node_sizes);
        let node_ids = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        for left_index in 0..node_ids.len() {
            for right_index in (left_index + 1)..node_ids.len() {
                let left_id = node_ids[left_index];
                let right_id = node_ids[right_index];
                let left_pos = positions
                    .get(&left_id)
                    .expect("left node should have a computed position");
                let right_pos = positions
                    .get(&right_id)
                    .expect("right node should have a computed position");
                let left_size = node_sizes
                    .get(&left_id)
                    .expect("left node should have a known size");
                let right_size = node_sizes
                    .get(&right_id)
                    .expect("right node should have a known size");

                let overlaps_x =
                    (left_pos[0] - right_pos[0]).abs() < (left_size.x * 0.5 + right_size.x * 0.5);
                let overlaps_y =
                    (left_pos[1] - right_pos[1]).abs() < (left_size.y * 0.5 + right_size.y * 0.5);

                assert!(
                    !(overlaps_x && overlaps_y),
                    "auto-layout should never overlap nodes ({} and {})",
                    left_id,
                    right_id
                );
            }
        }
    }
}
