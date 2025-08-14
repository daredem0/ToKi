use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("failed to create winit event loop: {0}")]
    WinitEventLoop(#[from] winit::error::EventLoopError),

    #[error("failed to create surface: {0}")]
    SurfaceCreation(#[from] wgpu::CreateSurfaceError),
    // Add other variants as needed
}
