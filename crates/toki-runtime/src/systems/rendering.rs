use std::sync::Arc;
use winit::window::Window;
use toki_core::math::projection::{calculate_projection, ProjectionParameter};
use toki_render::GpuState;

/// Rendering system that manages GPU state and projection calculations.
/// 
/// Centralizes all rendering-related state and provides clean APIs for
/// graphics operations while abstracting GPU implementation details.
#[derive(Debug)]
pub struct RenderingSystem {
    gpu: Option<GpuState>,
    projection_params: ProjectionParameter,
}

impl RenderingSystem {
    /// Create a new RenderingSystem with default projection parameters
    pub fn new() -> Self {
        Self {
            gpu: None,
            projection_params: ProjectionParameter {
                width: 160,
                height: 144,
                desired_width: 160,
                desired_height: 144,
            },
        }
    }
    
    /// Initialize GPU state with the given window
    pub fn initialize_gpu(&mut self, window: Arc<Window>) {
        let gpu = GpuState::new(window);
        self.gpu = Some(gpu);
    }
    
    /// Get mutable reference to GPU state
    pub fn gpu_mut(&mut self) -> Option<&mut GpuState> {
        self.gpu.as_mut()
    }
    
    /// Get reference to GPU state
    pub fn gpu(&self) -> Option<&GpuState> {
        self.gpu.as_ref()
    }
    
    /// Update projection parameters with new window size
    pub fn update_window_size(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.projection_params.width = size.width;
        self.projection_params.height = size.height;
    }
    
    /// Calculate current projection matrix
    pub fn calculate_projection(&self) -> glam::Mat4 {
        calculate_projection(self.projection_params)
    }
    
    /// Update GPU projection matrix with view transform
    pub fn update_projection(&mut self, view_matrix: glam::Mat4) {
        let projection = self.calculate_projection();
        if let Some(gpu) = &mut self.gpu {
            gpu.update_projection(projection * view_matrix);
        }
    }
    
    /// Resize GPU render targets
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if let Some(gpu) = &mut self.gpu {
            gpu.resize(new_size);
        }
        self.update_window_size(new_size);
    }
    
    /// Draw the current frame
    pub fn draw(&mut self) {
        if let Some(gpu) = &mut self.gpu {
            gpu.draw();
        }
    }
    
    /// Check if GPU is initialized
    pub fn has_gpu(&self) -> bool {
        self.gpu.is_some()
    }
    
    /// Get current projection parameters
    pub fn projection_params(&self) -> ProjectionParameter {
        self.projection_params
    }
}