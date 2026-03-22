use super::*;
use crate::scene::viewport::viewport_math::compute_display_rect;

impl SceneViewport {
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
