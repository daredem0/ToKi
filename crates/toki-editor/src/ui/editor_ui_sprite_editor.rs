//! Sprite editor integration with EditorUI.
//!
//! This module provides the thin integration layer between EditorUI and
//! SpriteEditorState. The actual sprite editing state and logic lives in
//! the `sprite_editor` module.

use super::EditorUI;

// Re-export types from sprite_editor module for backward compatibility
#[allow(unused_imports)]
pub(crate) use crate::ui::sprite_editor::{
    canonical_indexed_color_for_size, indexed_slot_for_authored_color, nearest_palette_slot,
    CanvasSide, CanvasState, DiscoveredSpriteAsset, DualCanvasLayout, FloatingOrigin,
    FloatingSelection, GradientMode, GradientStyle, PixelColor, ResizeAnchor, SelectionMask,
    SpriteAssetKind, SpriteCanvas, SpriteCanvasViewport, SpriteEditCommand, SpriteEditorHistory,
    SpriteEditorState, SpriteEditorTool, SpriteSelection,
};

/// Begin showing the new canvas dialog
pub(crate) fn begin_new_sprite_canvas_dialog(ui_state: &mut EditorUI) {
    let default_group_name = {
        let state = crate::ui::editor_context::sprite_state_mut(ui_state);
        let candidate = state.active().save_asset_name.trim();
        if candidate.is_empty() {
            "autotile".to_string()
        } else {
            candidate.to_string()
        }
    };
    crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_source_image = None;
    crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_source_image_size = None;
    crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_error = None;
    crate::ui::editor_context::sprite_state_mut(ui_state).new_autotile_group_name =
        default_group_name;
    crate::ui::editor_context::sprite_state_mut(ui_state).show_new_canvas_dialog = true;
}

/// Cancel new canvas dialog
pub(crate) fn cancel_new_sprite_canvas_dialog(ui_state: &mut EditorUI) {
    crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_source_image = None;
    crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_source_image_size = None;
    crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_error = None;
    crate::ui::editor_context::sprite_state_mut(ui_state).show_new_canvas_dialog = false;
}

#[cfg(test)]
#[path = "editor_ui_sprite_editor_tests.rs"]
mod tests;
