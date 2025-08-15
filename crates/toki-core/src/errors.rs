use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Failed to load file at {0}: {1}")]
    FileLoad(PathBuf, String),
    #[error("Image load failed: {0}")]
    ImageLoad(String),
}
