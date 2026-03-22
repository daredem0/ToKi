//! Sprite editor keyboard shortcuts handling.

use crate::ui::editor_ui::CanvasSide;
use crate::ui::EditorUI;

use super::canvas::{invalidate_canvas_texture, invalidate_canvas_texture_for_side};

pub fn handle_undo_redo_shortcuts(ui_state: &mut EditorUI, ui: &egui::Ui) {
    let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.mac_cmd);
    let shift = ui.input(|i| i.modifiers.shift);

    // Ctrl+Z for undo (without shift)
    if ctrl && !shift && ui.input(|i| i.key_pressed(egui::Key::Z)) && ui_state.sprite.undo() {
        invalidate_canvas_texture(ui_state);
    }

    // Ctrl+Y or Ctrl+Shift+Z for redo
    let redo_pressed = ui.input(|i| i.key_pressed(egui::Key::Y))
        || (shift && ui.input(|i| i.key_pressed(egui::Key::Z)));
    if ctrl && redo_pressed && ui_state.sprite.redo() {
        invalidate_canvas_texture(ui_state);
    }
}

pub fn handle_copy_paste_shortcuts(ui_state: &mut EditorUI, ctx: &egui::Context) {
    let (ctrl, c_pressed, v_pressed) = ctx.input(|i| {
        let ctrl = i.modifiers.ctrl || i.modifiers.mac_cmd;
        let mut c_pressed = false;
        let mut v_pressed = false;

        for event in &i.events {
            if let egui::Event::Key {
                key,
                pressed,
                modifiers,
                ..
            } = event
            {
                if *pressed && (modifiers.ctrl || modifiers.mac_cmd) {
                    match key {
                        egui::Key::C => c_pressed = true,
                        egui::Key::V => v_pressed = true,
                        _ => {}
                    }
                }
            }
        }

        (ctrl, c_pressed, v_pressed)
    });

    if c_pressed || v_pressed {
        tracing::info!(
            "Key pressed (via events): C={}, V={}, Ctrl={}",
            c_pressed,
            v_pressed,
            ctrl
        );
    }

    // Ctrl+C for copy
    if ctrl && c_pressed {
        let has_selection = ui_state.sprite.active().selection.is_some();
        let has_canvas = ui_state.sprite.active().canvas.is_some();
        tracing::info!(
            "Copy attempt: has_selection={}, has_canvas={}",
            has_selection,
            has_canvas
        );

        if let Some(sel) = &ui_state.sprite.active().selection {
            tracing::info!(
                "Selection: x={}, y={}, w={}, h={}",
                sel.x,
                sel.y,
                sel.width,
                sel.height
            );
        }

        if ui_state.sprite.copy_selection() {
            tracing::info!("Copy successful - clipboard has content");
        } else {
            tracing::warn!("Copy failed - no selection or no canvas");
        }
    }

    // Ctrl+V for paste
    if ctrl && v_pressed {
        let has_clipboard = ui_state.sprite.clipboard.is_some();
        let hovered = find_hovered_canvas(ui_state);
        let paste_side = hovered.unwrap_or(ui_state.sprite.active_canvas);
        let cursor_pos = ui_state.sprite.canvas_state(paste_side).cursor_canvas_pos;
        let has_canvas = ui_state.sprite.canvas_state(paste_side).canvas.is_some();

        tracing::info!(
            "Paste attempt: has_clipboard={}, hovered={:?}, paste_side={:?}, cursor_pos={:?}, has_canvas={}",
            has_clipboard,
            hovered,
            paste_side,
            cursor_pos,
            has_canvas
        );

        if cursor_pos.is_none() {
            tracing::info!("No cursor position, using (0, 0) fallback");
            ui_state
                .sprite
                .canvas_state_mut(paste_side)
                .cursor_canvas_pos = Some(glam::IVec2::new(0, 0));
        }

        if ui_state.sprite.paste_at_cursor(paste_side) {
            invalidate_canvas_texture_for_side(ui_state, paste_side);
            tracing::info!("Paste successful to {:?}", paste_side);
        } else {
            tracing::warn!("Paste failed - check clipboard and canvas state");
        }
    }
}

/// Find which canvas the cursor is currently hovering over
fn find_hovered_canvas(ui_state: &EditorUI) -> Option<CanvasSide> {
    if ui_state
        .sprite
        .canvas_state(CanvasSide::Left)
        .cursor_canvas_pos
        .is_some()
    {
        return Some(CanvasSide::Left);
    }
    if ui_state
        .sprite
        .canvas_state(CanvasSide::Right)
        .cursor_canvas_pos
        .is_some()
    {
        return Some(CanvasSide::Right);
    }
    None
}
