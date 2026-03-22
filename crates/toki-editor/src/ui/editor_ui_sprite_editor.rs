//! Sprite editor integration with EditorUI.
//!
//! This module provides the thin integration layer between EditorUI and
//! SpriteEditorState. The actual sprite editing state and logic lives in
//! the `sprite_editor` module.

use super::EditorUI;

// Re-export types from sprite_editor module for backward compatibility
#[allow(unused_imports)]
pub(crate) use crate::ui::sprite_editor::{
    CanvasSide, CanvasState, DiscoveredSpriteAsset, DualCanvasLayout, PixelColor, ResizeAnchor,
    SpriteAssetKind, SpriteCanvas, SpriteCanvasViewport, SpriteEditCommand, SpriteEditorHistory,
    SpriteEditorState, SpriteEditorTool, SpriteSelection,
};

impl EditorUI {
    /// Begin showing the new canvas dialog
    pub fn begin_new_sprite_canvas_dialog(&mut self) {
        self.sprite.show_new_canvas_dialog = true;
    }

    /// Cancel new canvas dialog
    pub fn cancel_new_sprite_canvas_dialog(&mut self) {
        self.sprite.show_new_canvas_dialog = false;
    }
}

#[cfg(test)]
#[path = "editor_ui_sprite_editor_tests.rs"]
mod tests;
