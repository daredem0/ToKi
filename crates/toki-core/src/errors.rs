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

    #[error("Map size mismatch: expected {expected} tiles, found {actual}")]
    InvalidMapSize { expected: usize, actual: usize },

    #[error("Tile coordinates ({x}, {y}) are out of bounds for map size {map_width}x{map_height}")]
    TileOutOfBounds {
        x: u32,
        y: u32,
        map_width: u32,
        map_height: u32,
    },

    #[error(
        "World position ({x}, {y}) has coordinates that are out of bounds, which are not supported"
    )]
    WorldPositionOutOfBounds { x: u32, y: u32 },

    #[error("Entity with ID {entity_id} not found")]
    EntityNotFound { entity_id: u32 },

    #[error("Entity with ID {entity_id} has no collision box")]
    NoCollisionBox { entity_id: u32 },

    #[error("Atlas missing tile '{tile_name}' referenced in tilemap")]
    MissingTileInAtlas { tile_name: String },

    #[error("Collision system not initialized")]
    CollisionSystemNotInitialized,

    #[error("Invalid entity position: ({x}, {y}) would place entity outside world bounds")]
    InvalidEntityPosition { x: i32, y: i32 },

    #[error("Animation clip '{clip_name}' not found")]
    AnimationClipNotFound { clip_name: String },

    #[error(
        "Animation frame index {frame_index} out of bounds for clip '{clip_name}' (max: 
  {max_frames})"
    )]
    AnimationFrameOutOfBounds {
        frame_index: usize,
        clip_name: String,
        max_frames: usize,
    },
}
