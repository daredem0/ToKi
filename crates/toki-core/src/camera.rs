use crate::entity::{Entity, EntityId};
use crate::project_runtime::{default_resolution_height, default_resolution_width};
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
    /// Zoom factor (1.0 = native, 2.0 = 2x zoom in showing half the world, 0.5 = zoom out)
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

impl Camera {
    /// Creates a camera with default resolution from project preset.
    pub fn new() -> Self {
        Self::with_resolution(default_resolution_width(), default_resolution_height())
    }

    /// Creates a camera with the specified resolution.
    pub fn with_resolution(width: u32, height: u32) -> Self {
        Self {
            position: glam::IVec2::ZERO,
            viewport_size: glam::UVec2::new(width, height),
            zoom: 1.0,
        }
    }

    /// Creates a camera with the specified resolution and zoom level.
    pub fn with_resolution_and_zoom(width: u32, height: u32, zoom: f32) -> Self {
        Self {
            position: glam::IVec2::ZERO,
            viewport_size: glam::UVec2::new(width, height),
            zoom: zoom.max(0.1), // Prevent division by zero
        }
    }
    pub fn move_by(&mut self, delta: glam::IVec2) {
        self.position += delta;
    }

    pub fn center_on(&mut self, target: glam::IVec2) {
        // Center on target, accounting for zoom (visible area is viewport / zoom)
        let visible_w = (self.viewport_size.x as f32 / self.zoom) as i32;
        let visible_h = (self.viewport_size.y as f32 / self.zoom) as i32;
        self.position = target - glam::IVec2::new(visible_w / 2, visible_h / 2);
    }

    /// Returns the visible world size (viewport divided by zoom)
    pub fn visible_world_size(&self) -> glam::Vec2 {
        glam::Vec2::new(
            self.viewport_size.x as f32 / self.zoom,
            self.viewport_size.y as f32 / self.zoom,
        )
    }

    pub fn calculate_projection(&self) -> glam::Mat4 {
        // Convert integer position to f32 range
        // Visible world area = viewport_size / zoom
        let left = self.position.x as f32;
        let top = self.position.y as f32;
        let visible_size = self.visible_world_size();
        let right = left + visible_size.x;
        let bottom = top + visible_size.y;
        // Do the orthographic projection
        glam::Mat4::orthographic_rh_gl(left, right, bottom, top, -1.0, 1.0)
    }

    /// Clamp camera position to stay within world bounds
    pub fn clamp_to_world_bounds(&mut self, world_size: glam::UVec2) {
        let visible_size = self.visible_world_size();
        let view_w = visible_size.x as i32;
        let view_h = visible_size.y as i32;
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
    /// The camera's zoom determines how many world pixels each viewport pixel represents.
    ///
    /// # Arguments
    /// * `viewport_pos` - Position in viewport-local coordinates (0..viewport_size)
    ///
    /// # Returns
    /// The corresponding world position
    pub fn viewport_to_world(&self, viewport_pos: glam::Vec2) -> glam::Vec2 {
        // scale = 1/zoom (zoom in = fewer world pixels per viewport pixel)
        viewport_to_world(viewport_pos, self.position, 1.0 / self.zoom)
    }

    /// Converts a world position to viewport-local coordinates.
    ///
    /// The camera's world position maps to viewport (0, 0).
    /// The camera's zoom determines how many world pixels each viewport pixel represents.
    ///
    /// # Arguments
    /// * `world_pos` - Position in world coordinates
    ///
    /// # Returns
    /// The corresponding viewport-local position
    pub fn world_to_viewport(&self, world_pos: glam::Vec2) -> glam::Vec2 {
        // scale = 1/zoom (zoom in = fewer world pixels per viewport pixel)
        world_to_viewport(world_pos, self.position, 1.0 / self.zoom)
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
