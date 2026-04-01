use super::*;
use crate::editor_viewport::EditorViewportContext;
use crate::ui::object_sheet_browser::build_decoration_placement_draft;
use crate::ui::EditorUI;

impl PanelSystem {
    pub(crate) fn render_scene_viewport_tab(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
        scene_viewport: Option<&mut SceneViewport>,
        mut config: Option<&mut EditorConfig>,
        renderer: Option<&mut egui_wgpu::Renderer>,
    ) {
        let Some(viewport) = scene_viewport else {
            let available_size = ui.available_size();
            ui.allocate_response(available_size, egui::Sense::click())
                .on_hover_text("Scene viewport not initialized");
            return;
        };

        if let Err(e) = viewport.update() {
            tracing::error!("Scene viewport update error: {e}");
        }

        if let Some(cfg) = config.as_deref_mut() {
            let mut toolbar_changed = false;
            let grid_size = Self::effective_grid_size(viewport, cfg);
            let current_mode_label = ui_state
                .scene_viewport_context()
                .placement
                .mode_label()
                .unwrap_or_else(|| "Select".to_string());
            let object_tool_ready = match (
                cfg.current_project_path(),
                ui_state
                    .scene_viewport_context()
                    .toolbox
                    .selected_object_sheet
                    .as_deref(),
                ui_state
                    .scene_viewport_context()
                    .toolbox
                    .selected_object_name
                    .as_deref(),
            ) {
                (Some(project_path), Some(sheet), Some(object_name)) => {
                    build_decoration_placement_draft(project_path, sheet, object_name).is_some()
                }
                _ => false,
            };
            let mut show_tiles = ui_state.scene_viewport_context().viewport_cursor.show_tiles;
            let cursor_world_position = ui_state.scene_viewport_context().viewport_cursor.world_position;
            ui.horizontal(|ui| {
                toolbar_changed = Self::render_grid_toolbar_contents(ui, cfg);
                ui.separator();
                ui.label(format!("Mode: {current_mode_label}"));
                if ui
                    .add_enabled(object_tool_ready, egui::Button::new("Object Tool"))
                    .clicked()
                {
                    let project_path = cfg
                        .current_project_path()
                        .expect("object tool requires current project path");
                    let toolbox = &ui_state.scene_viewport_context().toolbox;
                    if let (Some(sheet), Some(object_name)) = (
                        toolbox.selected_object_sheet.as_deref(),
                        toolbox.selected_object_name.as_deref(),
                    ) {
                        if let Some(draft) =
                            build_decoration_placement_draft(project_path, sheet, object_name)
                        {
                            ui_state
                                .scene_viewport_context_mut()
                                .placement
                                .enter_decoration_placement_mode(draft);
                            toolbar_changed = true;
                        }
                    }
                }
                if ui_state
                    .scene_viewport_context()
                    .placement
                    .is_in_placement_mode()
                    && ui.button("Cancel").clicked()
                {
                    ui_state
                        .scene_viewport_context_mut()
                        .placement
                        .exit_placement_mode();
                    toolbar_changed = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.checkbox(&mut show_tiles, "P/T");
                    ui.label(Self::viewport_cursor_status_label(
                        cursor_world_position,
                        show_tiles,
                        grid_size,
                    ));
                });
            });
            if show_tiles != ui_state.scene_viewport_context().viewport_cursor.show_tiles {
                ui_state.scene_viewport_context_mut().viewport_cursor.show_tiles = show_tiles;
                toolbar_changed = true;
            }
            if toolbar_changed {
                viewport.mark_dirty();
            }
            ui.separator();
        }

        let available_size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(
            available_size,
            egui::Sense::click_and_drag().union(egui::Sense::hover()),
        );
        let (camera_position, camera_scale) = viewport.camera_state();
        let viewport_ctx = EditorViewportContext::new(
            rect,
            viewport.viewport_size(),
            viewport.sizing_mode() == crate::scene::viewport::ViewportSizingMode::Responsive,
            camera_position,
            camera_scale,
        );
        let display_rect = viewport_ctx.display_rect();

        if let Some(world_pos) = viewport_ctx.hover_world_from_response(&response) {
            ui_state
                .scene_viewport_context_mut()
                .viewport_cursor
                .world_position = Some(glam::IVec2::new(
                world_pos.x.floor() as i32,
                world_pos.y.floor() as i32,
            ));
        }

        if !ui_state
            .scene_viewport_context()
            .placement
            .is_entity_move_drag_active()
            && !ui_state
                .scene_viewport_context()
                .placement
                .is_scene_anchor_move_drag_active()
        {
            viewport.clear_suppressed_entity_rendering();
        }

        let ctrl_pressed = ui.input(|i| i.modifiers.ctrl);
        if response.drag_started() {
            if let Some(drag_start_pos) = response.interact_pointer_pos() {
                if !viewport_ctx.contains_screen_pos(drag_start_pos) {
                    viewport.stop_camera_drag();
                } else if ctrl_pressed
                    && !ui_state
                        .scene_viewport_context()
                        .placement
                        .is_in_placement_mode()
                {
                    SelectionInteraction::handle_marquee_drag_start(ui_state, drag_start_pos);
                    viewport.stop_camera_drag();
                } else {
                    SelectionInteraction::handle_drag_start(
                        ui_state,
                        viewport,
                        drag_start_pos,
                        display_rect,
                        config.as_deref(),
                        ctrl_pressed,
                    );
                }
            }
        }

        if ui_state
            .scene_viewport_context()
            .placement
            .is_marquee_selection_active()
            && response.dragged()
        {
            if let Some(drag_pos) = response
                .interact_pointer_pos()
                .or_else(|| response.hover_pos())
            {
                SelectionInteraction::handle_marquee_drag_update(ui_state, drag_pos);
            }
        }

        if response.drag_stopped() {
            if ui_state
                .scene_viewport_context()
                .placement
                .is_marquee_selection_active()
            {
                SelectionInteraction::handle_marquee_drag_release(
                    ui_state,
                    viewport,
                    display_rect,
                    true,
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
                    display_rect,
                    config.as_deref(),
                );
            }
        }

        if !ui_state
            .scene_viewport_context()
            .placement
            .is_entity_move_drag_active()
            && !ui_state
                .scene_viewport_context()
                .placement
                .is_scene_anchor_move_drag_active()
            && !ui_state
                .scene_viewport_context()
                .placement
                .is_marquee_selection_active()
        {
            CameraInteraction::handle_drag(viewport, &response, config.as_deref());
        } else {
            viewport.stop_camera_drag();
        }

        PlacementInteraction::handle_hover(ui_state, viewport, &response, rect, config.as_deref());

        if response.clicked() {
            if let Some(click_pos) = response.hover_pos() {
                if !viewport_ctx.contains_screen_pos(click_pos) {
                    // Ignore clicks in the letterboxed area.
                } else if ui_state
                    .scene_viewport_context()
                    .placement
                    .is_in_placement_mode()
                {
                    PlacementInteraction::handle_click(
                        ui_state,
                        viewport,
                        click_pos,
                        display_rect,
                        config.as_deref(),
                    );
                } else {
                    SelectionInteraction::handle_click(
                        ui_state,
                        viewport,
                        click_pos,
                        display_rect,
                        config.as_deref(),
                        ctrl_pressed,
                    );
                }
            }
        }

        if response.hovered() {
            let scroll_delta = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                let sensitivity = config
                    .as_deref()
                    .map(|c| c.editor_settings.camera.scroll_zoom_sensitivity)
                    .unwrap_or(0.02);
                // Scale the scroll delta by sensitivity; require threshold to trigger discrete zoom
                let scaled = scroll_delta.abs() * sensitivity;
                if scaled > 0.3 {
                    if scroll_delta > 0.0 {
                        viewport.zoom_in();
                    } else {
                        viewport.zoom_out();
                    }
                }
            }
        }

        let project_path = config.as_deref().and_then(|c| c.current_project_path());
        viewport.render(ui, rect, project_path.map(|p| p.as_path()), renderer);
        if let Some(cfg) = config.as_deref() {
            Self::paint_viewport_grid_overlay(ui, rect, viewport, cfg);
        }
        Self::paint_marquee_selection_overlay(ui, ui_state);
    }
}
