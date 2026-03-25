//! Floating selection type for the sprite editor.
//!
//! A floating selection holds pixels that have been "lifted" off the canvas
//! and can be repositioned before being committed back.

use super::canvas::SpriteCanvas;
use super::selection::SelectionMask;

/// Pixels lifted off the canvas into a movable overlay.
/// Created by move-dragging a selection or by pasting from the clipboard.
#[derive(Debug, Clone)]
pub struct FloatingSelection {
    /// The lifted pixel data (bounding-rect-sized canvas, transparent where unselected).
    pub pixels: SpriteCanvas,
    /// Per-pixel mask within `pixels` (same dimensions as pixels).
    pub mask: SelectionMask,
    /// Current top-left position in canvas coordinates (can be negative).
    pub offset: glam::IVec2,
    /// Canvas state before the lift, used for cancel and undo.
    pub canvas_before_lift: SpriteCanvas,
    /// Offset at the time of lift, used by cancel to restore the original position.
    pub original_offset: glam::IVec2,
}
