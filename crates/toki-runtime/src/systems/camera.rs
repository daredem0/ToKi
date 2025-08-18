use toki_core::camera::{Camera, CameraController, RuntimeState};
use toki_core::assets::tilemap::TileMap;

/// Camera system that manages camera state, controller logic, and chunk caching optimization.
/// 
/// Handles the coordination between camera movement, world bounds, and visible chunk caching
/// for optimal rendering performance.
#[derive(Debug)]
pub struct CameraSystem {
    camera: Camera,
    controller: CameraController,
    cached_visible_chunks: Vec<(u32, u32)>,
}

impl CameraSystem {
    /// Create a new CameraSystem with the given camera and controller
    pub fn new(camera: Camera, controller: CameraController) -> Self {
        Self {
            camera,
            controller,
            cached_visible_chunks: Vec::new(),
        }
    }

    /// Get a reference to the camera
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Get a mutable reference to the camera
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    /// Get a reference to the controller
    pub fn controller(&self) -> &CameraController {
        &self.controller
    }

    /// Get a mutable reference to the controller
    pub fn controller_mut(&mut self) -> &mut CameraController {
        &mut self.controller
    }

    /// Get the cached visible chunks
    pub fn cached_visible_chunks(&self) -> &[(u32, u32)] {
        &self.cached_visible_chunks
    }

    /// Update camera position based on controller and clamp to world bounds.
    /// Returns true if the camera position changed.
    pub fn update(&mut self, runtime: &RuntimeState, world_size: glam::UVec2) -> bool {
        let prev_pos = self.camera.position;
        
        // Update camera based on controller
        self.controller.update(&mut self.camera, runtime);
        
        // Clamp to world bounds  
        self.camera.clamp_to_world_bounds(world_size);
        
        // Return whether camera moved
        prev_pos != self.camera.position
    }

    /// Update the visible chunks cache based on current camera position.
    /// Returns true if the visible chunks changed (indicating rendering updates are needed).
    pub fn update_chunk_cache(&mut self, tilemap: &TileMap) -> bool {
        let current_chunks = tilemap.visible_chunks(
            glam::UVec2::new(self.camera.position.x as u32, self.camera.position.y as u32),
            self.camera.viewport_size,
        );
        
        if current_chunks != self.cached_visible_chunks {
            self.cached_visible_chunks = current_chunks;
            true
        } else {
            false
        }
    }

    /// Convenience method to get the camera position for rendering calculations
    pub fn position(&self) -> glam::IVec2 {
        self.camera.position
    }

    /// Convenience method to get the viewport size
    pub fn viewport_size(&self) -> glam::UVec2 {
        self.camera.viewport_size
    }

    /// Create view matrix for rendering
    pub fn view_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_translation(glam::vec3(
            -(self.camera.position.x as f32),
            -(self.camera.position.y as f32),
            0.0,
        ))
    }

    /// Get camera projection matrix
    pub fn projection_matrix(&self) -> glam::Mat4 {
        self.camera.calculate_projection()
    }
}