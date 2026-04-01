//! Floating selection type for the sprite editor.
//!
//! A floating selection holds pixels that have been "lifted" off the canvas
//! and can be repositioned before being committed back.

use super::canvas::SpriteCanvas;
use super::selection::SelectionMask;

/// Why the floating pixels exist, which drives cancel/render behavior.
#[derive(Debug, Clone)]
pub enum FloatingOrigin {
    /// Pixels were lifted from an existing selection.
    SelectionLift {
        selection_before_float: SelectionMask,
    },
    /// Pixels came from the clipboard and should restore the prior selection on cancel.
    PastePreview {
        selection_before_float: Option<SelectionMask>,
    },
}

impl FloatingOrigin {
    pub fn selection_before_float(&self) -> Option<&SelectionMask> {
        match self {
            Self::SelectionLift {
                selection_before_float,
            } => Some(selection_before_float),
            Self::PastePreview {
                selection_before_float,
            } => selection_before_float.as_ref(),
        }
    }

    pub fn is_paste_preview(&self) -> bool {
        matches!(self, Self::PastePreview { .. })
    }
}

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
    /// Source-specific behavior for cancel/render semantics.
    pub origin: FloatingOrigin,
    /// Target size for resize preview. `None` means original size.
    pub resize_size: Option<glam::UVec2>,
}

impl FloatingSelection {
    /// The size at which this floating selection should be displayed and committed.
    pub fn display_size(&self) -> glam::UVec2 {
        self.resize_size
            .unwrap_or(glam::UVec2::new(self.pixels.width, self.pixels.height))
    }
}
