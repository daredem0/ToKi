use crate::entity::{Entity, EntityId};
use glam;

/// Converts a viewport-local position to world coordinates.
///
/// This is a low-level utility function that accepts a float scale for use cases
/// requiring sub-integer zoom levels (e.g., editor zoom).
///
/// # Arguments
/// * `viewport_pos` - Position in viewport-local coordinates (0..viewport_size)
/// * `camera_position` - Camera's top-left corner in world space
/// * `scale` - Zoom scale (1.0 = native, 2.0 = zoomed out 2x, etc.)
pub fn viewport_to_world(
    viewport_pos: glam::Vec2,
    camera_position: glam::IVec2,
    scale: f32,
) -> glam::Vec2 {
    glam::Vec2::new(
        camera_position.x as f32 + viewport_pos.x * scale,
        camera_position.y as f32 + viewport_pos.y * scale,
    )
}

/// Converts a world position to viewport-local coordinates.
///
/// This is a low-level utility function that accepts a float scale for use cases
/// requiring sub-integer zoom levels (e.g., editor zoom).
///
/// # Arguments
/// * `world_pos` - Position in world coordinates
/// * `camera_position` - Camera's top-left corner in world space
/// * `scale` - Zoom scale (1.0 = native, 2.0 = zoomed out 2x, etc.)
pub fn world_to_viewport(
    world_pos: glam::Vec2,
    camera_position: glam::IVec2,
    scale: f32,
) -> glam::Vec2 {
    glam::Vec2::new(
        (world_pos.x - camera_position.x as f32) / scale,
        (world_pos.y - camera_position.y as f32) / scale,
    )
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// Top-left corner of the camera in world space (pixels)
    pub position: glam::IVec2,
    /// Viewport size in pixels (typically 160x144 for GB mode)
    pub viewport_size: glam::UVec2,
    /// Zoom scale (1 = native resolution, 2 = double, etc.)
    pub scale: u32,
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: glam::IVec2::ZERO,
            viewport_size: glam::UVec2::new(160, 144),
            scale: 1,
        }
    }
    pub fn move_by(&mut self, delta: glam::IVec2) {
        self.position += delta;
    }

    pub fn center_on(&mut self, target: glam::IVec2) {
        self.position = target - self.viewport_size.as_ivec2() / 2;
    }

    pub fn calculate_projection(&self) -> glam::Mat4 {
        // Convert integer position to f32 range
        let left = self.position.x as f32;
        let top = self.position.y as f32;
        let right = left + (self.viewport_size.x * self.scale) as f32;
        let bottom = top + (self.viewport_size.y * self.scale) as f32;
        // Do the orthographic projection
        glam::Mat4::orthographic_rh_gl(left, right, bottom, top, -1.0, 1.0)
    }

    /// Clamp camera position to stay within world bounds
    pub fn clamp_to_world_bounds(&mut self, world_size: glam::UVec2) {
        let view_w = (self.viewport_size.x * self.scale) as i32;
        let view_h = (self.viewport_size.y * self.scale) as i32;
        let world_w = world_size.x as i32;
        let world_h = world_size.y as i32;

        let max_x = (world_w - view_w).max(0);
        let max_y = (world_h - view_h).max(0);

        self.position.x = self.position.x.clamp(0, max_x);
        self.position.y = self.position.y.clamp(0, max_y);
    }

    /// Converts a viewport-local position to world coordinates.
    ///
    /// Viewport position (0, 0) maps to the camera's world position.
    /// The camera's scale determines how many world pixels each viewport pixel represents.
    ///
    /// # Arguments
    /// * `viewport_pos` - Position in viewport-local coordinates (0..viewport_size)
    ///
    /// # Returns
    /// The corresponding world position
    pub fn viewport_to_world(&self, viewport_pos: glam::Vec2) -> glam::Vec2 {
        viewport_to_world(viewport_pos, self.position, self.scale as f32)
    }

    /// Converts a world position to viewport-local coordinates.
    ///
    /// The camera's world position maps to viewport (0, 0).
    /// The camera's scale determines how many world pixels each viewport pixel represents.
    ///
    /// # Arguments
    /// * `world_pos` - Position in world coordinates
    ///
    /// # Returns
    /// The corresponding viewport-local position
    pub fn world_to_viewport(&self, world_pos: glam::Vec2) -> glam::Vec2 {
        world_to_viewport(world_pos, self.position, self.scale as f32)
    }
}

pub struct RuntimeState<'a> {
    pub entities: &'a [Entity],
}

#[derive(Debug, Clone, Copy)]
pub enum CameraMode {
    FollowEntity(EntityId), // You'll define EntityId type
    FreeScroll,
}

#[derive(Debug, Clone, Copy)]
pub struct CameraController {
    pub mode: CameraMode,
}

impl CameraController {
    pub fn update(&mut self, camera: &mut Camera, runtime: &RuntimeState) {
        match self.mode {
            CameraMode::FollowEntity(id) => {
                if let Some(entity) = runtime.entities.iter().find(|e| e.id == id) {
                    camera.center_on(entity.position);
                }
            }
            CameraMode::FreeScroll => {
                // Free-scroll is handled manually elsewhere
            }
        }
    }
}
