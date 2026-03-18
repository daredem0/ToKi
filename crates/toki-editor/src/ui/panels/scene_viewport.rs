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

        let entity_count = viewport
            .game_state()
            .entity_manager()
            .active_entities()
            .len();
        let selected_entity = viewport.selected_entity();

        if let Err(e) = viewport.update() {
            tracing::error!("Scene viewport update error: {e}");
        }

        if let Some(cfg) = config.as_deref_mut() {
            if Self::render_grid_toolbar(ui, cfg) {
                viewport.mark_dirty();
            }
            ui.separator();
        }

        let available_size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(
            available_size,
            egui::Sense::click_and_drag().union(egui::Sense::hover()),
        );

        if !ui_state.is_entity_move_drag_active() {
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

        if response.drag_stopped() {
            if ui_state.is_marquee_selection_active() {
                SelectionInteraction::handle_marquee_drag_release(ui_state, viewport, rect, true);
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

        if !ui_state.is_entity_move_drag_active() && !ui_state.is_marquee_selection_active() {
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
                        rect,
                        config.as_deref(),
                    );
                } else {
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
    }
}
