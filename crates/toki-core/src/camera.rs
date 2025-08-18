use glam;

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
}

pub type EntityId = u32;

pub struct RuntimeState<'a> {
    pub entities: &'a [Entity],
}

#[derive(Debug)]
pub struct Entity {
    pub id: EntityId,
    pub position: glam::Vec2,
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
                    camera.center_on(entity.position.as_ivec2());
                }
            }
            CameraMode::FreeScroll => {
                // Free-scroll is handled manually elsewhere
            }
        }
    }
}
