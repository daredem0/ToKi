//! Sprite editor module - organized into focused submodules.
//!
//! This module provides all the types and state needed for pixel-level sprite editing,
//! including dual canvas support for side-by-side editing.

mod autotile_layout;
mod canvas;
mod dual_canvas;
mod floating;
mod history;
mod selection;
mod state;
mod state_canvas;
mod state_cells;
mod state_file_io;
mod state_floating;
mod state_history;
mod types;
mod viewport;

// Re-export all public types
pub(crate) use autotile_layout::{
    four_bit_mask_for_visual_slot, four_bit_visual_label_for_slot, FOUR_BIT_VISUAL_LAYOUT_TEXT,
};
pub use canvas::SpriteCanvas;
pub use dual_canvas::{CanvasSide, CanvasState, DualCanvasLayout};
pub use floating::{FloatingOrigin, FloatingSelection};
pub use history::{SpriteEditCommand, SpriteEditorHistory};
#[cfg(test)]
pub use selection::clear_masked_pixels;
pub use selection::extract_masked_selection;
pub use selection::{SelectionMask, SpriteSelection};
pub use state::{AutoTileSpriteInfo, SpriteEditorState};
#[cfg(test)]
pub use types::canonical_indexed_color;
pub use types::{
    canonical_indexed_color_for_size, indexed_slot_for_authored_color, nearest_palette_slot,
    preview_indexed_color, DiscoveredSpriteAsset, DitherPattern, GradientMode, GradientStyle,
    PixelColor, ProceduralBrushMode, ResizeAnchor, ResizeCorner, ResizeDrag, SpriteAssetKind,
    SpriteEditorTool,
};
pub use viewport::SpriteCanvasViewport;
