use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Failed to load file at {0}: {1}")]
    FileLoad(PathBuf, String),
    #[error("Image load failed: {0}")]
    ImageLoad(String),

    #[error("I/O error while reading file: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid atlas JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Atlas file not found: {0}")]
    NotFound(PathBuf),
}
