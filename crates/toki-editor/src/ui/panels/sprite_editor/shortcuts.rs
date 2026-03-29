//! Sprite editor keyboard shortcuts handling.

use crate::ui::editor_ui::CanvasSide;
use crate::ui::EditorUI;

use super::canvas::{invalidate_canvas_texture, invalidate_canvas_texture_for_side};

pub fn handle_undo_redo_shortcuts(ui_state: &mut EditorUI, ui: &egui::Ui) {
    let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.mac_cmd);
    let shift = ui.input(|i| i.modifiers.shift);

    // Ctrl+Z for undo (without shift)
    if ctrl
        && !shift
        && ui.input(|i| i.key_pressed(egui::Key::Z))
        && crate::ui::editor_context::sprite_state_mut(ui_state).undo()
    {
        invalidate_canvas_texture(ui_state);
    }

    // Ctrl+Y or Ctrl+Shift+Z for redo
    let redo_pressed = ui.input(|i| i.key_pressed(egui::Key::Y))
        || (shift && ui.input(|i| i.key_pressed(egui::Key::Z)));
    if ctrl && redo_pressed && crate::ui::editor_context::sprite_state_mut(ui_state).redo() {
        invalidate_canvas_texture(ui_state);
    }
}

pub fn handle_copy_paste_shortcuts(ui_state: &mut EditorUI, ctx: &egui::Context) {
    let (ctrl, c_pressed, x_pressed, v_pressed, delete_pressed) = ctx.input(|i| {
        let ctrl = i.modifiers.ctrl || i.modifiers.mac_cmd;
        let mut c_pressed = false;
        let mut x_pressed = false;
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
                        egui::Key::X => x_pressed = true,
                        egui::Key::V => v_pressed = true,
                        _ => {}
                    }
                }
            }
        }

        (
            ctrl,
            c_pressed,
            x_pressed,
            v_pressed,
            i.key_pressed(egui::Key::Delete),
        )
    });

    // Ctrl+C for copy
    if ctrl && c_pressed {
        crate::ui::editor_context::sprite_state_mut(ui_state).copy_selection();
    }

    // Ctrl+X for cut
    if ctrl && x_pressed && crate::ui::editor_context::sprite_state_mut(ui_state).cut_selection() {
        invalidate_canvas_texture(ui_state);
    }

    // Delete for clearing selected pixels
    if delete_pressed && crate::ui::editor_context::sprite_state_mut(ui_state).delete_selection() {
        invalidate_canvas_texture(ui_state);
    }

    // Ctrl+V for paste
    if ctrl && v_pressed {
        let hovered = find_hovered_canvas(ui_state);
        let paste_side =
            hovered.unwrap_or(crate::ui::editor_context::sprite_state_mut(ui_state).active_canvas);
        let cursor_pos = crate::ui::editor_context::sprite_state_mut(ui_state)
            .canvas_state(paste_side)
            .cursor_canvas_pos;

        if cursor_pos.is_none() {
            ui_state
                .sprite_editor_context_mut()
                .sprite
                .canvas_state_mut(paste_side)
                .cursor_canvas_pos = Some(glam::IVec2::new(0, 0));
        }

        if crate::ui::editor_context::sprite_state_mut(ui_state).paste_at_cursor(paste_side) {
            invalidate_canvas_texture_for_side(ui_state, paste_side);
        }
    }
}

/// Find which canvas the cursor is currently hovering over
fn find_hovered_canvas(ui_state: &EditorUI) -> Option<CanvasSide> {
    if ui_state
        .sprite_editor_context()
        .sprite
        .canvas_state(CanvasSide::Left)
        .cursor_canvas_pos
        .is_some()
    {
        return Some(CanvasSide::Left);
    }
    if ui_state
        .sprite_editor_context()
        .sprite
        .canvas_state(CanvasSide::Right)
        .cursor_canvas_pos
        .is_some()
    {
        return Some(CanvasSide::Right);
    }
    None
}
