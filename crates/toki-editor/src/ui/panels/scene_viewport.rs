use super::*;
use crate::ui::EditorUI;

impl PanelSystem {
    pub(super) fn render_scene_viewport_tab(
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
            ui.horizontal(|ui| {
                toolbar_changed = Self::render_grid_toolbar_contents(ui, cfg);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.checkbox(&mut ui_state.viewport_cursor_show_tiles, "P/T");
                    ui.label(Self::viewport_cursor_status_label(
                        ui_state.viewport_cursor_world_position,
                        ui_state.viewport_cursor_show_tiles,
                        grid_size,
                    ));
                });
            });
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
        let display_rect = viewport.display_rect_in(rect);

        if let Some(pointer_pos) = response
            .hover_pos()
            .filter(|pos| display_rect.contains(*pos))
        {
            let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, display_rect);
            ui_state.remember_viewport_cursor_world_position(world_pos);
        }

        if !ui_state.is_entity_move_drag_active() && !ui_state.is_scene_anchor_move_drag_active() {
            viewport.clear_suppressed_entity_rendering();
        }

        let ctrl_pressed = ui.input(|i| i.modifiers.ctrl);
        if response.drag_started() {
            if let Some(drag_start_pos) = response.interact_pointer_pos() {
                if ctrl_pressed && !ui_state.is_in_placement_mode() {
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

        if ui_state.is_marquee_selection_active() && response.dragged() {
            if let Some(drag_pos) = response
                .interact_pointer_pos()
                .or_else(|| response.hover_pos())
            {
                SelectionInteraction::handle_marquee_drag_update(ui_state, drag_pos);
            }
        }

        if response.drag_stopped() {
            if ui_state.is_marquee_selection_active() {
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

        if !ui_state.is_entity_move_drag_active()
            && !ui_state.is_scene_anchor_move_drag_active()
            && !ui_state.is_marquee_selection_active()
        {
            CameraInteraction::handle_drag(viewport, &response, config.as_deref());
        } else {
            viewport.stop_camera_drag();
        }

        PlacementInteraction::handle_hover(ui_state, viewport, &response, rect, config.as_deref());

        if response.clicked() {
            if let Some(click_pos) = response.hover_pos() {
                if ui_state.is_in_placement_mode() {
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
