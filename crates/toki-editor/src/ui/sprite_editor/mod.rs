//! Sprite editor module - organized into focused submodules.
//!
//! This module provides all the types and state needed for pixel-level sprite editing,
//! including dual canvas support for side-by-side editing.

#![allow(dead_code)]

mod canvas;
mod dual_canvas;
mod history;
mod selection;
mod state;
mod state_canvas;
mod state_cells;
mod state_file_io;
mod state_history;
mod types;
mod viewport;

// Re-export all public types
pub use canvas::SpriteCanvas;
pub use dual_canvas::{CanvasSide, CanvasState, DualCanvasLayout};
pub use history::{SpriteEditCommand, SpriteEditorHistory};
pub use selection::SpriteSelection;
pub use state::SpriteEditorState;
pub use types::{
    DiscoveredSpriteAsset, PixelColor, ResizeAnchor, SpriteAssetKind, SpriteEditorTool,
    WarningAction,
};
pub use viewport::SpriteCanvasViewport;
