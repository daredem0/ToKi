use thiserror::Error;
use toki_core::CoreError;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("failed to create winit event loop: {0}")]
    WinitEventLoop(#[from] winit::error::EventLoopError),

    #[error("failed to create surface: {0}")]
    SurfaceCreation(#[from] wgpu::CreateSurfaceError),
    // Add other variants as needed
    #[error("Core error {0}")]
    Core(#[from] CoreError),

    #[error("Unknown render error")]
    Unknown,
}
