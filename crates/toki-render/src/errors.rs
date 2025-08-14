use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("failed to initialize winit event loop: {0}")]
    EventLoopInit(#[from] winit::error::EventLoopError),
}
