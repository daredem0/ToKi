//! Sprite editor tool interaction handling.

use crate::ui::editor_ui::{SpriteEditorTool, SpriteSelection};
use crate::ui::interactions::SpritePaintInteraction;
use crate::ui::EditorUI;

use super::canvas::invalidate_canvas_texture;

pub fn handle_tool_interaction(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    rect: egui::Rect,
    ctx: &egui::Context,
) {
    let Some(canvas_pos) = ui_state.sprite.active().cursor_canvas_pos else {
        return;
    };

    match ui_state.sprite.tool {
        SpriteEditorTool::Drag => handle_drag_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Brush => handle_brush_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Eraser => handle_eraser_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Fill => handle_fill_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Eyedropper => handle_eyedropper_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Line => handle_line_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Select => handle_select_tool(ui_state, response, rect, ctx, canvas_pos),
        SpriteEditorTool::MagicWand => handle_magic_wand_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::MagicErase => handle_magic_erase_tool(ui_state, response, canvas_pos),
    }
}

fn handle_drag_tool(ui_state: &mut EditorUI, response: &egui::Response, canvas_pos: glam::IVec2) {
    // Click to select cell in sheet mode
    if response.clicked() && ui_state.sprite.is_sheet() && canvas_pos.x >= 0 && canvas_pos.y >= 0 {
        let cell = ui_state
            .sprite
            .cell_at_position(canvas_pos.x as u32, canvas_pos.y as u32);
        ui_state.sprite.active_mut().selected_cell = cell;
    }

    // Primary drag for panning
    if response.dragged_by(egui::PointerButton::Primary) {
        let delta = response.drag_delta();
        ui_state
            .sprite
            .active_mut()
            .viewport
            .pan_by(glam::Vec2::new(delta.x, delta.y));
    }
}

fn handle_brush_tool(ui_state: &mut EditorUI, response: &egui::Response, canvas_pos: glam::IVec2) {
    if response.drag_started_by(egui::PointerButton::Primary) {
        start_paint_stroke(ui_state);
    }

    if response.dragged_by(egui::PointerButton::Primary) || response.clicked() {
        let color = ui_state.sprite.foreground_color;
        let brush_size = ui_state.sprite.brush_size;
        if let Some(canvas) = &mut ui_state.sprite.active_mut().canvas {
            if SpritePaintInteraction::paint_brush(canvas, canvas_pos, color, brush_size) {
                ui_state.sprite.active_mut().dirty = true;
                invalidate_canvas_texture(ui_state);
            }
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        finish_paint_stroke(ui_state);
    }
}

fn handle_eraser_tool(ui_state: &mut EditorUI, response: &egui::Response, canvas_pos: glam::IVec2) {
    if response.drag_started_by(egui::PointerButton::Primary) {
        start_paint_stroke(ui_state);
    }

    if response.dragged_by(egui::PointerButton::Primary) || response.clicked() {
        let brush_size = ui_state.sprite.brush_size;
        if let Some(canvas) = &mut ui_state.sprite.active_mut().canvas {
            if SpritePaintInteraction::erase_brush(canvas, canvas_pos, brush_size) {
                ui_state.sprite.active_mut().dirty = true;
                invalidate_canvas_texture(ui_state);
            }
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        finish_paint_stroke(ui_state);
    }
}

fn handle_fill_tool(ui_state: &mut EditorUI, response: &egui::Response, canvas_pos: glam::IVec2) {
    if response.clicked() {
        start_paint_stroke(ui_state);
        let color = ui_state.sprite.foreground_color;
        if let Some(canvas) = &mut ui_state.sprite.active_mut().canvas {
            if SpritePaintInteraction::flood_fill(canvas, canvas_pos, color) {
                ui_state.sprite.active_mut().dirty = true;
                invalidate_canvas_texture(ui_state);
            }
        }
        finish_paint_stroke(ui_state);
    }
}

fn handle_eyedropper_tool(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    canvas_pos: glam::IVec2,
) {
    if response.clicked() {
        if let Some(canvas) = &ui_state.sprite.active().canvas {
            if let Some(color) = SpritePaintInteraction::pick_color(canvas, canvas_pos) {
                ui_state.sprite.foreground_color = color;
                ui_state.sprite.add_recent_color(color);
            }
        }
    }
}

fn handle_line_tool(ui_state: &mut EditorUI, response: &egui::Response, canvas_pos: glam::IVec2) {
    if response.drag_started_by(egui::PointerButton::Primary) {
        ui_state.sprite.active_mut().line_start_pos = Some(canvas_pos);
        start_paint_stroke(ui_state);
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        let color = ui_state.sprite.foreground_color;
        let brush_size = ui_state.sprite.brush_size;
        if let Some(start) = ui_state.sprite.active_mut().line_start_pos.take() {
            if let Some(canvas) = &mut ui_state.sprite.active_mut().canvas {
                if SpritePaintInteraction::draw_line(canvas, start, canvas_pos, color, brush_size) {
                    ui_state.sprite.active_mut().dirty = true;
                    invalidate_canvas_texture(ui_state);
                }
            }
        }
        finish_paint_stroke(ui_state);
    }
}

fn handle_select_tool(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    rect: egui::Rect,
    ctx: &egui::Context,
    canvas_pos: glam::IVec2,
) {
    let primary_pressed_in_rect = response.hovered()
        && ctx.input(|input| input.pointer.primary_pressed())
        && ctx
            .input(|input| input.pointer.interact_pos())
            .is_some_and(|pointer_pos| rect.contains(pointer_pos));

    if primary_pressed_in_rect {
        tracing::info!("Select tool: drag started at {:?}", canvas_pos);
        ui_state.sprite.active_mut().selection_start_pos = Some(canvas_pos);
        ui_state.sprite.active_mut().selection = None;
    }

    if response.dragged_by(egui::PointerButton::Primary) {
        if let Some(start) = ui_state.sprite.active().selection_start_pos {
            ui_state.sprite.active_mut().selection = Some(create_selection(start, canvas_pos));
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        if let Some(start) = ui_state.sprite.active_mut().selection_start_pos.take() {
            let selection = create_selection(start, canvas_pos);
            if selection.width > 0 && selection.height > 0 {
                tracing::info!(
                    "Select tool: created selection x={}, y={}, w={}, h={}",
                    selection.x,
                    selection.y,
                    selection.width,
                    selection.height
                );
                ui_state.sprite.active_mut().selection = Some(selection);
            } else {
                tracing::info!("Select tool: selection too small, discarded");
                ui_state.sprite.active_mut().selection = None;
            }
        }
    }

    // Clear selection with right-click
    if response.clicked_by(egui::PointerButton::Secondary) {
        tracing::info!("Select tool: selection cleared by right-click");
        ui_state.sprite.active_mut().selection = None;
    }
}

fn handle_magic_wand_tool(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    canvas_pos: glam::IVec2,
) {
    if response.clicked() && canvas_pos.x >= 0 && canvas_pos.y >= 0 {
        if let Some(canvas) = &ui_state.sprite.active().canvas {
            let x = canvas_pos.x as u32;
            let y = canvas_pos.y as u32;

            if let Some((sel_x, sel_y, sel_w, sel_h)) = canvas.find_connected_sprite(x, y) {
                tracing::info!(
                    "Magic wand: selected sprite at ({}, {}) with size {}x{}",
                    sel_x,
                    sel_y,
                    sel_w,
                    sel_h
                );
                ui_state.sprite.active_mut().selection =
                    Some(SpriteSelection::new(sel_x, sel_y, sel_w, sel_h));
            } else {
                tracing::info!("Magic wand: clicked on transparent pixel, clearing selection");
                ui_state.sprite.active_mut().selection = None;
            }
        }
    }

    // Clear selection with right-click
    if response.clicked_by(egui::PointerButton::Secondary) {
        ui_state.sprite.active_mut().selection = None;
    }
}

fn handle_magic_erase_tool(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    canvas_pos: glam::IVec2,
) {
    if !response.clicked() {
        return;
    }

    let Some(bounds) = magic_erase_bounds(ui_state, canvas_pos) else {
        return;
    };

    start_paint_stroke(ui_state);
    if let Some(canvas) = &mut ui_state.sprite.active_mut().canvas {
        if SpritePaintInteraction::erase_connected_color_in_bounds(canvas, canvas_pos, bounds) {
            ui_state.sprite.active_mut().dirty = true;
            invalidate_canvas_texture(ui_state);
        }
    }
    finish_paint_stroke(ui_state);
}

fn magic_erase_bounds(
    ui_state: &EditorUI,
    canvas_pos: glam::IVec2,
) -> Option<(glam::UVec2, glam::UVec2)> {
    if canvas_pos.x < 0 || canvas_pos.y < 0 {
        return None;
    }

    let x = canvas_pos.x as u32;
    let y = canvas_pos.y as u32;

    if ui_state.sprite.is_sheet() {
        let cell_idx = ui_state.sprite.cell_at_position(x, y)?;
        let (start_x, start_y, end_x, end_y) = ui_state.sprite.cell_bounds(cell_idx)?;
        return Some((
            glam::UVec2::new(start_x, start_y),
            glam::UVec2::new(end_x, end_y),
        ));
    }

    let (width, height) = ui_state.sprite.canvas_dimensions()?;
    Some((glam::UVec2::ZERO, glam::UVec2::new(width, height)))
}

fn create_selection(start: glam::IVec2, end: glam::IVec2) -> SpriteSelection {
    let x = start.x.min(end.x).max(0) as u32;
    let y = start.y.min(end.y).max(0) as u32;
    let w = (start.x - end.x).unsigned_abs() + 1;
    let h = (start.y - end.y).unsigned_abs() + 1;
    SpriteSelection::new(x, y, w, h)
}

fn start_paint_stroke(ui_state: &mut EditorUI) {
    if !ui_state.sprite.active().is_painting {
        ui_state.sprite.active_mut().is_painting = true;
        ui_state.sprite.active_mut().canvas_before_stroke = ui_state.sprite.active().canvas.clone();
    }
}

fn finish_paint_stroke(ui_state: &mut EditorUI) {
    if ui_state.sprite.active().is_painting {
        ui_state.sprite.active_mut().is_painting = false;
        if let Some(before) = ui_state.sprite.active_mut().canvas_before_stroke.take() {
            ui_state.sprite.push_undo_state(before);
        }
        ui_state
            .sprite
            .add_recent_color(ui_state.sprite.foreground_color);
    }
}

pub fn handle_tool_shortcuts(ui_state: &mut EditorUI, ui: &egui::Ui) {
    use SpriteEditorTool::*;

    if ui.input(|i| i.key_pressed(egui::Key::B)) {
        ui_state.sprite.tool = Brush;
    }
    if ui.input(|i| i.key_pressed(egui::Key::E)) {
        ui_state.sprite.tool = Eraser;
    }
    if ui.input(|i| i.key_pressed(egui::Key::G)) {
        ui_state.sprite.tool = Fill;
    }
    if ui.input(|i| i.key_pressed(egui::Key::I)) {
        ui_state.sprite.tool = Eyedropper;
    }
    if ui.input(|i| i.key_pressed(egui::Key::M)) {
        ui_state.sprite.tool = Select;
    }
    if ui.input(|i| i.key_pressed(egui::Key::D)) {
        ui_state.sprite.tool = Drag;
    }
    if ui.input(|i| i.key_pressed(egui::Key::L)) {
        ui_state.sprite.tool = Line;
    }
    if ui.input(|i| i.key_pressed(egui::Key::W)) {
        ui_state.sprite.tool = MagicWand;
    }
    if ui.input(|i| i.key_pressed(egui::Key::K)) {
        ui_state.sprite.tool = MagicErase;
    }

    // Brush size
    if ui.input(|i| i.key_pressed(egui::Key::OpenBracket)) {
        ui_state.sprite.brush_size = ui_state.sprite.brush_size.saturating_sub(1).max(1);
    }
    if ui.input(|i| i.key_pressed(egui::Key::CloseBracket)) {
        ui_state.sprite.brush_size = (ui_state.sprite.brush_size + 1).min(32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_selection_includes_both_start_and_end_pixels() {
        let selection = create_selection(glam::IVec2::new(2, 3), glam::IVec2::new(5, 7));

        assert_eq!(selection, SpriteSelection::new(2, 3, 4, 5));
    }

    #[test]
    fn create_selection_is_inclusive_when_dragging_backwards() {
        let selection = create_selection(glam::IVec2::new(5, 7), glam::IVec2::new(2, 3));

        assert_eq!(selection, SpriteSelection::new(2, 3, 4, 5));
    }

    #[test]
    fn create_selection_single_click_selects_one_pixel() {
        let selection = create_selection(glam::IVec2::new(4, 6), glam::IVec2::new(4, 6));

        assert_eq!(selection, SpriteSelection::new(4, 6, 1, 1));
    }
}
