use crate::config::EditorConfig;
use crate::scene::SceneViewport;

/// Handles camera interaction logic for the viewport
pub struct CameraInteraction;

impl CameraInteraction {
    /// Handle camera drag interactions
    pub fn handle_drag(
        viewport: &mut SceneViewport,
        response: &egui::Response,
        config: Option<&EditorConfig>,
    ) {
        if response.drag_started() {
            if let Some(start_pos) = response.interact_pointer_pos() {
                tracing::info!("Camera drag started at {:?}", start_pos);
                let start_vec = glam::Vec2::new(start_pos.x, start_pos.y);
                viewport.start_camera_drag(start_vec);
            }
        } else if response.dragged() {
            if let Some(drag_pos) = response.interact_pointer_pos() {
                tracing::debug!("Camera dragging to {:?}", drag_pos);
                let drag_vec = glam::Vec2::new(drag_pos.x, drag_pos.y);
                let pan_speed = config
                    .map(|c| c.editor_settings.camera.pan_speed)
                    .unwrap_or(1.0);
                viewport.update_camera_drag(drag_vec, pan_speed);
            }
        } else if response.drag_stopped() {
            tracing::info!("Camera drag stopped");
            viewport.stop_camera_drag();
        }
    }
}
