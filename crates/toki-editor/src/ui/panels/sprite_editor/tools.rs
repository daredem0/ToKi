//! Sprite editor tool interaction handling.

use crate::ui::editor_ui::{SelectionMask, SpriteEditorTool, SpriteSelection};
use crate::ui::interactions::SpritePaintInteraction;
use crate::ui::sprite_editor::{canonical_indexed_color, indexed_slot_for_authored_color};
use crate::ui::EditorUI;
use toki_core::assets::atlas::ColorMode;
use toki_core::palette::Palette4;

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
        SpriteEditorTool::AddOutline => handle_add_outline_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::AddShadow => handle_add_shadow_tool(ui_state, response, canvas_pos),
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
        let color = effective_paint_color(
            ui_state.sprite.color_mode,
            ui_state.sprite.foreground_color,
            selected_palette(ui_state),
        );
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
        let color = effective_paint_color(
            ui_state.sprite.color_mode,
            ui_state.sprite.foreground_color,
            selected_palette(ui_state),
        );
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
                if ui_state.sprite.color_mode == ColorMode::PaletteIndexed {
                    if let Some(slot) =
                        indexed_slot_for_authored_color(color, selected_palette(ui_state))
                    {
                        ui_state.sprite.foreground_color = canonical_indexed_color(slot);
                    }
                } else {
                    ui_state.sprite.foreground_color = color;
                    ui_state.sprite.add_recent_color(color);
                }
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
        let color = effective_paint_color(
            ui_state.sprite.color_mode,
            ui_state.sprite.foreground_color,
            selected_palette(ui_state),
        );
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
    // If floating, handle move-drag
    if ui_state.sprite.has_floating() {
        handle_floating_drag(ui_state, response, ctx, canvas_pos);
        return;
    }

    handle_selection_drag(ui_state, response, rect, ctx, canvas_pos);
}

fn handle_floating_drag(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    ctx: &egui::Context,
    _canvas_pos: glam::IVec2,
) {
    if response.dragged_by(egui::PointerButton::Primary) {
        let delta = response.drag_delta();
        let zoom = ui_state.sprite.active().viewport.zoom;
        // Convert screen-space drag delta to canvas-space pixel delta
        let dx = (delta.x / zoom).round() as i32;
        let dy = (delta.y / zoom).round() as i32;
        if dx != 0 || dy != 0 {
            ui_state.sprite.nudge_floating(glam::IVec2::new(dx, dy));
        }
    }

    // Click outside floating → commit and clear
    let primary_released = ctx.input(|input| input.pointer.primary_released());
    if primary_released && !response.dragged() {
        ui_state.sprite.commit_floating();
        invalidate_canvas_texture(ui_state);
    }

    if response.clicked_by(egui::PointerButton::Secondary) {
        ui_state.sprite.cancel_floating();
        invalidate_canvas_texture(ui_state);
    }
}

fn handle_selection_drag(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    rect: egui::Rect,
    ctx: &egui::Context,
    canvas_pos: glam::IVec2,
) {
    let selection_mode = current_selection_mode(ctx);
    let primary_pressed_in_rect = response.hovered()
        && ctx.input(|input| input.pointer.primary_pressed())
        && ctx
            .input(|input| input.pointer.interact_pos())
            .is_some_and(|pointer_pos| rect.contains(pointer_pos));

    if primary_pressed_in_rect {
        // If clicking inside existing selection without modifiers, start a move
        if selection_mode == SelectionModifyMode::Replace
            && is_click_inside_selection(ui_state, canvas_pos)
        {
            ui_state.sprite.lift_selection();
            invalidate_canvas_texture(ui_state);
            return;
        }

        let existing_selection = ui_state.sprite.active().selection.clone();
        let active = ui_state.sprite.active_mut();
        active.selection_start_pos = Some(canvas_pos);
        active.selection_drag_base = existing_selection;
    }

    if response.dragged_by(egui::PointerButton::Primary) {
        if let Some(start) = ui_state.sprite.active().selection_start_pos {
            apply_drag_selection(ui_state, start, canvas_pos, selection_mode);
        }
    }

    let primary_released = ctx.input(|input| input.pointer.primary_released());
    if primary_released {
        if let Some(start) = ui_state.sprite.active_mut().selection_start_pos.take() {
            apply_drag_selection(ui_state, start, canvas_pos, selection_mode);
            ui_state.sprite.active_mut().selection_drag_base = None;
        }
    }

    if response.clicked_by(egui::PointerButton::Secondary) {
        ui_state.sprite.active_mut().selection = None;
        ui_state.sprite.active_mut().selection_drag_base = None;
    }
}

fn is_click_inside_selection(ui_state: &EditorUI, pos: glam::IVec2) -> bool {
    if pos.x < 0 || pos.y < 0 {
        return false;
    }
    ui_state
        .sprite
        .active()
        .selection
        .as_ref()
        .is_some_and(|sel| sel.is_selected(pos.x as u32, pos.y as u32))
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
            let selection_mode = current_selection_mode(&response.ctx);

            if let Some(mask) = canvas.find_connected_selection_mask(x, y) {
                let base = ui_state.sprite.active().selection.clone();
                ui_state.sprite.active_mut().selection =
                    merge_selection_masks(base.as_ref(), &mask, selection_mode);
            } else if selection_mode == SelectionModifyMode::Replace {
                ui_state.sprite.active_mut().selection = None;
            } else {
                // Keep the existing selection on transparent clicks when adding/subtracting.
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

    let Some(bounds) = clicked_tile_bounds(ui_state, canvas_pos) else {
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

fn handle_add_outline_tool(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    canvas_pos: glam::IVec2,
) {
    if !response.clicked() {
        return;
    }

    let Some(bounds) = clicked_tile_bounds(ui_state, canvas_pos) else {
        return;
    };

    start_paint_stroke(ui_state);
    let outline_color = effective_paint_color(
        ui_state.sprite.color_mode,
        ui_state.sprite.foreground_color,
        selected_palette(ui_state),
    );
    if let Some(canvas) = &mut ui_state.sprite.active_mut().canvas {
        if SpritePaintInteraction::add_outline_in_bounds(canvas, canvas_pos, outline_color, bounds)
        {
            ui_state.sprite.active_mut().dirty = true;
            invalidate_canvas_texture(ui_state);
        }
    }
    finish_paint_stroke(ui_state);
}

fn handle_add_shadow_tool(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    canvas_pos: glam::IVec2,
) {
    if !response.clicked() {
        return;
    }

    let Some(bounds) = clicked_tile_bounds(ui_state, canvas_pos) else {
        return;
    };

    start_paint_stroke(ui_state);
    let shadow_color = effective_paint_color(
        ui_state.sprite.color_mode,
        ui_state.sprite.foreground_color,
        selected_palette(ui_state),
    );
    if let Some(canvas) = &mut ui_state.sprite.active_mut().canvas {
        if SpritePaintInteraction::add_ground_shadow_in_bounds(
            canvas,
            canvas_pos,
            shadow_color,
            bounds,
        ) {
            ui_state.sprite.active_mut().dirty = true;
            invalidate_canvas_texture(ui_state);
        }
    }
    finish_paint_stroke(ui_state);
}

fn clicked_tile_bounds(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionModifyMode {
    Replace,
    Add,
    Subtract,
}

fn current_selection_mode(ctx: &egui::Context) -> SelectionModifyMode {
    ctx.input(|input| {
        if input.modifiers.ctrl || input.modifiers.mac_cmd {
            SelectionModifyMode::Subtract
        } else if input.modifiers.shift {
            SelectionModifyMode::Add
        } else {
            SelectionModifyMode::Replace
        }
    })
}

fn apply_drag_selection(
    ui_state: &mut EditorUI,
    start: glam::IVec2,
    end: glam::IVec2,
    mode: SelectionModifyMode,
) {
    let Some((canvas_width, canvas_height)) = ui_state.sprite.canvas_dimensions() else {
        return;
    };
    let selection_rect = create_selection(start, end);
    let drag_mask = selection_mask_from_rect(canvas_width, canvas_height, selection_rect);
    let base = ui_state.sprite.active().selection_drag_base.clone();
    ui_state.sprite.active_mut().selection = merge_selection_masks(base.as_ref(), &drag_mask, mode);
}

fn selection_mask_from_rect(width: u32, height: u32, rect: SpriteSelection) -> SelectionMask {
    let mut selection = SelectionMask::new(width, height);
    selection.select_rect(rect.x, rect.y, rect.width, rect.height);
    selection
}

fn merge_selection_masks(
    base: Option<&SelectionMask>,
    incoming: &SelectionMask,
    mode: SelectionModifyMode,
) -> Option<SelectionMask> {
    let mut merged = match mode {
        SelectionModifyMode::Replace => SelectionMask::new(incoming.width, incoming.height),
        SelectionModifyMode::Add | SelectionModifyMode::Subtract => base
            .cloned()
            .unwrap_or_else(|| SelectionMask::new(incoming.width, incoming.height)),
    };

    match mode {
        SelectionModifyMode::Replace | SelectionModifyMode::Add => merged.union_with(incoming),
        SelectionModifyMode::Subtract => merged.subtract(incoming),
    }

    (!merged.is_empty()).then_some(merged)
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

fn selected_palette(ui_state: &EditorUI) -> Option<Palette4> {
    ui_state
        .sprite
        .selected_palette_id
        .as_ref()
        .and_then(|palette_id| ui_state.project.available_palettes.get(palette_id))
        .copied()
}

fn effective_paint_color(
    color_mode: ColorMode,
    foreground_color: crate::ui::editor_ui::PixelColor,
    palette: Option<Palette4>,
) -> crate::ui::editor_ui::PixelColor {
    if color_mode != ColorMode::PaletteIndexed {
        return foreground_color;
    }

    indexed_slot_for_authored_color(foreground_color, palette)
        .map(canonical_indexed_color)
        .unwrap_or_else(|| canonical_indexed_color(3))
}

pub fn handle_tool_shortcuts(ui_state: &mut EditorUI, ui: &egui::Ui) {
    use SpriteEditorTool::*;

    let tool_keys: &[(egui::Key, SpriteEditorTool)] = &[
        (egui::Key::B, Brush),
        (egui::Key::E, Eraser),
        (egui::Key::G, Fill),
        (egui::Key::I, Eyedropper),
        (egui::Key::M, Select),
        (egui::Key::D, Drag),
        (egui::Key::L, Line),
        (egui::Key::W, MagicWand),
        (egui::Key::K, MagicErase),
        (egui::Key::O, AddOutline),
        (egui::Key::H, AddShadow),
    ];

    for &(key, tool) in tool_keys {
        if ui.input(|i| i.key_pressed(key)) {
            ui_state.sprite.set_tool(tool);
        }
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
    use crate::ui::editor_ui::PixelColor;
    use toki_core::palette::Palette4;

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

    #[test]
    fn effective_paint_color_keeps_truecolor_values() {
        let color = PixelColor::rgb(12, 34, 56);
        assert_eq!(
            effective_paint_color(ColorMode::TrueColor, color, None),
            color
        );
    }

    #[test]
    fn effective_paint_color_maps_palette_display_color_back_to_canonical_slot() {
        let palette = Palette4::new([
            [10, 20, 30, 255],
            [40, 50, 60, 255],
            [70, 80, 90, 255],
            [100, 110, 120, 255],
        ]);

        assert_eq!(
            effective_paint_color(
                ColorMode::PaletteIndexed,
                PixelColor::rgb(70, 80, 90),
                Some(palette),
            ),
            canonical_indexed_color(2)
        );
    }

    #[test]
    fn effective_paint_color_defaults_to_white_slot_for_invalid_indexed_color() {
        assert_eq!(
            effective_paint_color(ColorMode::PaletteIndexed, PixelColor::rgb(1, 2, 3), None),
            canonical_indexed_color(3)
        );
    }
}
