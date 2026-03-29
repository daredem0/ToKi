//! Sprite editor integration with EditorUI.
//!
//! This module provides the thin integration layer between EditorUI and
//! SpriteEditorState. The actual sprite editing state and logic lives in
//! the `sprite_editor` module.

use super::EditorUI;

// Re-export types from sprite_editor module for backward compatibility
#[allow(unused_imports)]
pub(crate) use crate::ui::sprite_editor::{
    canonical_indexed_color, indexed_slot_for_authored_color, nearest_palette_slot, CanvasSide,
    CanvasState, DiscoveredSpriteAsset, DualCanvasLayout, FloatingOrigin, FloatingSelection,
    PixelColor, ResizeAnchor, SelectionMask, SpriteAssetKind, SpriteCanvas, SpriteCanvasViewport,
    SpriteEditCommand, SpriteEditorHistory, SpriteEditorState, SpriteEditorTool, SpriteSelection,
};

impl EditorUI {
    /// Begin showing the new canvas dialog
    pub fn begin_new_sprite_canvas_dialog(&mut self) {
        crate::ui::editor_context::sprite_state_mut(self).new_canvas_source_image = None;
        crate::ui::editor_context::sprite_state_mut(self).new_canvas_source_image_size = None;
        crate::ui::editor_context::sprite_state_mut(self).new_canvas_error = None;
        crate::ui::editor_context::sprite_state_mut(self).show_new_canvas_dialog = true;
    }

    /// Cancel new canvas dialog
    pub fn cancel_new_sprite_canvas_dialog(&mut self) {
        crate::ui::editor_context::sprite_state_mut(self).new_canvas_source_image = None;
        crate::ui::editor_context::sprite_state_mut(self).new_canvas_source_image_size = None;
        crate::ui::editor_context::sprite_state_mut(self).new_canvas_error = None;
        crate::ui::editor_context::sprite_state_mut(self).show_new_canvas_dialog = false;
    }
}

#[cfg(test)]
#[path = "editor_ui_sprite_editor_tests.rs"]
mod tests;
