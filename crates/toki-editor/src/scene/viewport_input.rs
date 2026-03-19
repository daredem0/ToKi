use super::*;
use crate::scene::viewport::viewport_math::compute_display_rect;

impl SceneViewport {
    #[allow(dead_code)]
    pub(super) fn handle_mouse_interaction(
        &mut self,
        response: &egui::Response,
        display_rect: egui::Rect,
    ) {
        if response.clicked() {
            tracing::info!(
                "Mouse clicked! Response: hovered={}, clicked={}, dragged={}",
                response.hovered(),
                response.clicked(),
                response.dragged()
            );
        }

        if response.drag_started() {
            tracing::info!("Drag started!");
        }

        if response.dragged() {
            tracing::info!("Dragging...");
        }

        if response.drag_stopped() {
            tracing::info!("Drag stopped!");
        }

        let mouse_pos_opt = response
            .interact_pointer_pos()
            .or_else(|| response.hover_pos())
            .or_else(|| response.ctx.pointer_hover_pos());

        if let Some(mouse_pos) = mouse_pos_opt {
            if display_rect.contains(mouse_pos) {
                tracing::trace!(
                    "Mouse at {:?} within display rect, handling interaction",
                    mouse_pos
                );
                self.handle_viewport_mouse_interaction(response, mouse_pos, display_rect);
            } else if self.is_dragging_camera {
                tracing::trace!(
                    "Mouse in letterbox area at {:?}, stopping camera drag",
                    mouse_pos
                );
                self.stop_camera_drag();
            }
        } else if self.is_dragging_camera {
            tracing::trace!("Mouse left viewport area while dragging, stopping camera drag");
            self.stop_camera_drag();
        }
    }

    pub(super) fn handle_viewport_mouse_interaction(
        &mut self,
        response: &egui::Response,
        mouse_pos: egui::Pos2,
        display_rect: egui::Rect,
    ) {
        let mouse_vec2 = glam::Vec2::new(mouse_pos.x, mouse_pos.y);

        if response.drag_started() {
            let _world_pos = self.screen_to_world_pos(mouse_pos, display_rect);
            self.start_camera_drag(mouse_vec2);
        } else if response.dragged() {
            if self.is_dragging_camera {
                // NOTE: This is now handled in editor_ui.rs
            }
        } else if response.drag_stopped() {
            self.stop_camera_drag();
        }

        if response.clicked() && !response.dragged() {
            let world_pos = self.screen_to_world_pos(mouse_pos, display_rect);
            tracing::trace!("Viewport clicked at world position: {:?}", world_pos);

            if let Some(entity_id) = self.get_entity_at_world_pos(world_pos) {
                tracing::info!(
                    "Entity {} clicked at world position ({:.1}, {:.1})",
                    entity_id,
                    world_pos.x,
                    world_pos.y
                );
            } else {
                tracing::trace!(
                    "No entity clicked at world position ({:.1}, {:.1})",
                    world_pos.x,
                    world_pos.y
                );
            }
        }
    }

    pub fn screen_to_world_pos_raw(
        &self,
        screen_pos: egui::Pos2,
        display_rect: egui::Rect,
    ) -> glam::Vec2 {
        screen_to_world_from_camera(
            screen_pos,
            display_rect,
            self.viewport_size,
            self.camera.position,
            self.effective_camera_scale(),
        )
    }

    pub fn screen_to_world_pos(
        &self,
        screen_pos: egui::Pos2,
        display_rect: egui::Rect,
    ) -> glam::Vec2 {
        self.screen_to_world_pos_raw(screen_pos, display_rect)
    }

    pub fn display_rect_in(&self, outer_rect: egui::Rect) -> egui::Rect {
        compute_display_rect(
            outer_rect,
            self.viewport_size,
            self.sizing_mode == ViewportSizingMode::Responsive,
        )
    }

    pub fn start_camera_drag(&mut self, mouse_pos: glam::Vec2) {
        self.is_dragging_camera = true;
        self.last_mouse_pos = Some(mouse_pos);
        tracing::info!("Started camera drag at: {:?}", mouse_pos);
    }

    pub fn update_camera_drag(&mut self, mouse_pos: glam::Vec2, pan_speed: f32) {
        if let Some(last_pos) = self.last_mouse_pos {
            let screen_delta = mouse_pos - last_pos;
            let effective_scale = self.effective_camera_scale();
            let world_delta_x = -screen_delta.x * effective_scale * pan_speed;
            let world_delta_y = -screen_delta.y * effective_scale * pan_speed;

            self.camera
                .move_by(glam::IVec2::new(world_delta_x as i32, world_delta_y as i32));

            self.mark_dirty();

            tracing::trace!(
                "Camera dragged by screen delta: {:?}, world delta: ({}, {}) [pan_speed: {}]",
                screen_delta,
                world_delta_x,
                world_delta_y,
                pan_speed
            );
        }

        self.last_mouse_pos = Some(mouse_pos);
    }

    pub fn stop_camera_drag(&mut self) {
        if self.is_dragging_camera {
            tracing::info!("Stopped camera drag");
            self.is_dragging_camera = false;
            self.last_mouse_pos = None;
        }
    }
}
