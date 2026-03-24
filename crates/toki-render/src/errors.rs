use thiserror::Error;
use toki_core::CoreError;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("failed to create winit event loop: {0}")]
    WinitEventLoop(#[from] winit::error::EventLoopError),

    #[error("failed to create surface: {0}")]
    SurfaceCreation(#[from] wgpu::CreateSurfaceError),

    #[error("failed to load texture from '{path}': {source}")]
    TextureLoad { path: String, source: CoreError },

    #[error("failed to create texture for {label}: {message}")]
    TextureCreation { label: String, message: String },

    #[error("no suitable GPU adapter found")]
    AdapterUnavailable,

    #[error("failed to create GPU device: {0}")]
    DeviceRequest(String),

    #[error("failed to acquire render surface: {0}")]
    SurfaceAcquire(#[from] wgpu::SurfaceError),

    #[error("Core error {0}")]
    Core(#[from] CoreError),

    #[error("Unknown render error")]
    Unknown,

    #[error("Other error: {0}")]
    Other(String),
}
