//! Sprite editor tool interaction handling.

use crate::ui::editor_ui::{SelectionMask, SpriteEditorTool, SpriteSelection};
use crate::ui::interactions::sprite_paint::{ShapeParams, SymmetryBounds, SymmetryConfig};
use crate::ui::interactions::SpritePaintInteraction;
use crate::ui::sprite_editor::{
    canonical_indexed_color, indexed_slot_for_authored_color, PixelColor, ResizeCorner,
    ResizeDrag,
};
use crate::ui::EditorUI;
use glam::UVec2;
use toki_core::assets::atlas::ColorMode;
use toki_core::palette::Palette4;

use super::canvas::invalidate_canvas_texture;

pub fn handle_tool_interaction(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    rect: egui::Rect,
    ctx: &egui::Context,
) {
    let Some(canvas_pos) = crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .cursor_canvas_pos
    else {
        return;
    };

    if crate::ui::editor_context::sprite_state_mut(ui_state).has_floating() {
        handle_floating_canvas_interaction(ui_state, response, ctx, rect);
        return;
    }

    match crate::ui::editor_context::sprite_state_mut(ui_state).tool {
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
        SpriteEditorTool::Rectangle => {
            handle_shape_tool(ui_state, response, canvas_pos, ShapeKind::Rectangle)
        }
        SpriteEditorTool::Ellipse => {
            handle_shape_tool(ui_state, response, canvas_pos, ShapeKind::Ellipse)
        }
    }
}

fn handle_drag_tool(ui_state: &mut EditorUI, response: &egui::Response, canvas_pos: glam::IVec2) {
    // Click to select cell in sheet mode
    if response.clicked()
        && crate::ui::editor_context::sprite_state_mut(ui_state).is_sheet()
        && canvas_pos.x >= 0
        && canvas_pos.y >= 0
    {
        let cell = ui_state
            .sprite_editor_context_mut()
            .sprite
            .cell_at_position(canvas_pos.x as u32, canvas_pos.y as u32);
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .selected_cell = cell;
    }

    // Primary drag for panning
    if response.dragged_by(egui::PointerButton::Primary) {
        let delta = response.drag_delta();
        ui_state
            .sprite_editor_context_mut()
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
            crate::ui::editor_context::sprite_state_mut(ui_state).color_mode,
            crate::ui::editor_context::sprite_state_mut(ui_state).foreground_color,
            selected_palette(ui_state),
        );
        let brush_size = crate::ui::editor_context::sprite_state_mut(ui_state).brush_size;
        let pattern = crate::ui::editor_context::sprite_state_mut(ui_state).dither_pattern;
        let sym = symmetry_config(ui_state);
        if let Some(canvas) = &mut crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .canvas
        {
            if SpritePaintInteraction::paint_brush_dithered_symmetric(
                canvas, canvas_pos, color, brush_size, pattern, &sym,
            ) {
                crate::ui::editor_context::sprite_state_mut(ui_state)
                    .active_mut()
                    .dirty = true;
                invalidate_canvas_texture(ui_state);
            }
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        finish_paint_stroke(ui_state);
    }

    // Right-click drag/click: erase with the same brush size and symmetry.
    if response.drag_started_by(egui::PointerButton::Secondary) {
        start_paint_stroke(ui_state);
    }

    if response.dragged_by(egui::PointerButton::Secondary)
        || response.clicked_by(egui::PointerButton::Secondary)
    {
        let brush_size = crate::ui::editor_context::sprite_state_mut(ui_state).brush_size;
        let sym = symmetry_config(ui_state);
        if let Some(canvas) = &mut crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .canvas
        {
            if SpritePaintInteraction::erase_brush_symmetric(canvas, canvas_pos, brush_size, &sym) {
                crate::ui::editor_context::sprite_state_mut(ui_state)
                    .active_mut()
                    .dirty = true;
                invalidate_canvas_texture(ui_state);
            }
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Secondary) {
        finish_paint_stroke(ui_state);
    }
}

fn handle_eraser_tool(ui_state: &mut EditorUI, response: &egui::Response, canvas_pos: glam::IVec2) {
    if response.drag_started_by(egui::PointerButton::Primary) {
        start_paint_stroke(ui_state);
    }

    if response.dragged_by(egui::PointerButton::Primary) || response.clicked() {
        let brush_size = crate::ui::editor_context::sprite_state_mut(ui_state).brush_size;
        let sym = symmetry_config(ui_state);
        if let Some(canvas) = &mut crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .canvas
        {
            if SpritePaintInteraction::erase_brush_symmetric(canvas, canvas_pos, brush_size, &sym) {
                crate::ui::editor_context::sprite_state_mut(ui_state)
                    .active_mut()
                    .dirty = true;
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
            crate::ui::editor_context::sprite_state_mut(ui_state).color_mode,
            crate::ui::editor_context::sprite_state_mut(ui_state).foreground_color,
            selected_palette(ui_state),
        );
        let sym = symmetry_config(ui_state);
        if let Some(canvas) = &mut crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .canvas
        {
            if SpritePaintInteraction::flood_fill_symmetric(canvas, canvas_pos, color, &sym) {
                crate::ui::editor_context::sprite_state_mut(ui_state)
                    .active_mut()
                    .dirty = true;
                invalidate_canvas_texture(ui_state);
            }
        }
        finish_paint_stroke(ui_state);
    }

    // Right-click: erase the filled region (flood fill with transparent).
    if response.clicked_by(egui::PointerButton::Secondary) {
        start_paint_stroke(ui_state);
        let sym = symmetry_config(ui_state);
        if let Some(canvas) = &mut crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .canvas
        {
            if SpritePaintInteraction::flood_fill_symmetric(
                canvas,
                canvas_pos,
                PixelColor::transparent(),
                &sym,
            ) {
                crate::ui::editor_context::sprite_state_mut(ui_state)
                    .active_mut()
                    .dirty = true;
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
        if let Some(canvas) = &crate::ui::editor_context::sprite_state_mut(ui_state)
            .active()
            .canvas
        {
            if let Some(color) = SpritePaintInteraction::pick_color(canvas, canvas_pos) {
                if crate::ui::editor_context::sprite_state_mut(ui_state).color_mode
                    == ColorMode::PaletteIndexed
                {
                    if let Some(slot) =
                        indexed_slot_for_authored_color(color, selected_palette(ui_state))
                    {
                        crate::ui::editor_context::sprite_state_mut(ui_state).foreground_color =
                            canonical_indexed_color(slot);
                    }
                } else {
                    crate::ui::editor_context::sprite_state_mut(ui_state).foreground_color = color;
                    crate::ui::editor_context::sprite_state_mut(ui_state).add_recent_color(color);
                }
            }
        }
    }
}

fn handle_line_tool(ui_state: &mut EditorUI, response: &egui::Response, canvas_pos: glam::IVec2) {
    handle_shape_tool(ui_state, response, canvas_pos, ShapeKind::Line);
}

#[derive(Clone, Copy)]
enum ShapeKind {
    Line,
    Rectangle,
    Ellipse,
}

fn handle_shape_tool(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    canvas_pos: glam::IVec2,
    kind: ShapeKind,
) {
    if response.drag_started_by(egui::PointerButton::Primary) {
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .stroke_erases = false;
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .line_start_pos = Some(canvas_pos);
        start_paint_stroke(ui_state);
    }

    // Right-click drag: erase version of the shape.
    if response.drag_started_by(egui::PointerButton::Secondary) {
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .stroke_erases = true;
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .line_start_pos = Some(canvas_pos);
        start_paint_stroke(ui_state);
    }

    // Live preview during drag (both buttons share the same preview path).
    if response.dragged_by(egui::PointerButton::Primary)
        || response.dragged_by(egui::PointerButton::Secondary)
    {
        preview_shape(ui_state, canvas_pos, kind);
    }

    if response.drag_stopped_by(egui::PointerButton::Primary)
        || response.drag_stopped_by(egui::PointerButton::Secondary)
    {
        // Restore and draw final shape
        preview_shape(ui_state, canvas_pos, kind);
        let cs = crate::ui::editor_context::sprite_state_mut(ui_state).active_mut();
        cs.line_start_pos = None;
        cs.stroke_erases = false;
        finish_paint_stroke(ui_state);
    }
}

fn preview_shape(ui_state: &mut EditorUI, canvas_pos: glam::IVec2, kind: ShapeKind) {
    let Some(start) = crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .line_start_pos
    else {
        return;
    };

    // Restore canvas to pre-stroke state before redrawing
    if let Some(before) = &crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .canvas_before_stroke
    {
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .canvas = Some(before.clone());
    }

    let params = build_shape_params(ui_state, start, canvas_pos);
    let sym = symmetry_config(ui_state);
    if let Some(canvas) = &mut crate::ui::editor_context::sprite_state_mut(ui_state)
        .active_mut()
        .canvas
    {
        let changed = match kind {
            ShapeKind::Line => SpritePaintInteraction::draw_line_symmetric(canvas, &params, &sym),
            ShapeKind::Rectangle => {
                SpritePaintInteraction::draw_rectangle_symmetric(canvas, &params, &sym)
            }
            ShapeKind::Ellipse => {
                SpritePaintInteraction::draw_ellipse_symmetric(canvas, &params, &sym)
            }
        };
        if changed {
            crate::ui::editor_context::sprite_state_mut(ui_state)
                .active_mut()
                .dirty = true;
            invalidate_canvas_texture(ui_state);
        }
    }
}

fn build_shape_params(ui_state: &EditorUI, start: glam::IVec2, end: glam::IVec2) -> ShapeParams {
    let color = if crate::ui::editor_context::sprite_state(ui_state)
        .active()
        .stroke_erases
    {
        PixelColor::transparent()
    } else {
        effective_paint_color(
            crate::ui::editor_context::sprite_state(ui_state).color_mode,
            crate::ui::editor_context::sprite_state(ui_state).foreground_color,
            selected_palette(ui_state),
        )
    };
    ShapeParams {
        start,
        end,
        color,
        brush_size: crate::ui::editor_context::sprite_state(ui_state).brush_size,
        filled: crate::ui::editor_context::sprite_state(ui_state).shape_filled,
    }
}

fn handle_select_tool(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    rect: egui::Rect,
    ctx: &egui::Context,
    canvas_pos: glam::IVec2,
) {
    handle_selection_drag(ui_state, response, rect, ctx, canvas_pos);
}

fn handle_floating_canvas_interaction(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    ctx: &egui::Context,
    rect: egui::Rect,
) -> bool {
    if !crate::ui::editor_context::sprite_state_mut(ui_state).has_floating() {
        return false;
    }

    if response.clicked_by(egui::PointerButton::Secondary) {
        crate::ui::editor_context::sprite_state_mut(ui_state).cancel_floating();
        invalidate_canvas_texture(ui_state);
        return true;
    }

    // --- Active resize drag (already started on a previous frame) ---
    if handle_active_resize_drag(ui_state, response, ctx, rect) {
        return true;
    }

    // --- Drag start: decide resize vs move based on where the drag originated ---
    if response.drag_started_by(egui::PointerButton::Primary) {
        try_start_resize_drag(ui_state, ctx, rect);
        // Whether resize or move, consume the event so selection tool doesn't interfere
        return true;
    }

    // --- Ongoing drag: resize if resize_drag is active, otherwise move ---
    if response.dragged_by(egui::PointerButton::Primary) {
        let is_resizing = crate::ui::editor_context::sprite_state_mut(ui_state)
            .active()
            .resize_drag
            .is_some();
        if is_resizing {
            apply_resize_drag(ui_state, ctx, rect);
        } else {
            apply_move_drag(ui_state, response);
        }
        return true;
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .resize_drag = None;
        return true;
    }

    if response.clicked_by(egui::PointerButton::Primary) {
        crate::ui::editor_context::sprite_state_mut(ui_state).commit_floating();
        invalidate_canvas_texture(ui_state);
        return true;
    }

    false
}

/// Screen-space tolerance (pixels) for hitting a corner handle.
const HANDLE_HIT_RADIUS: f32 = 12.0;

/// On drag start, check if the pointer is near a corner handle and begin resize if so.
fn try_start_resize_drag(ui_state: &mut EditorUI, ctx: &egui::Context, rect: egui::Rect) {
    let pointer_pos = match ctx.input(|i| i.pointer.interact_pos()) {
        Some(p) if rect.contains(p) => p,
        _ => return,
    };

    let cs = crate::ui::editor_context::sprite_state_mut(ui_state).active();
    let floating = match &cs.floating {
        Some(f) => f,
        None => return,
    };

    let zoom = cs.viewport.zoom;
    let pan = cs.viewport.pan;
    let offset = floating.offset;
    let size = floating.display_size();

    let corners = corner_screen_positions(rect, pan, zoom, offset, size);
    let hit = corners
        .iter()
        .find(|(_, pos)| pos.distance(pointer_pos) <= HANDLE_HIT_RADIUS);

    let (corner, _) = match hit {
        Some(c) => *c,
        None => return, // Not on a corner → will be a move drag
    };

    let anchor = anchor_for_corner(corner, offset, size);
    let aspect = size.x as f32 / size.y.max(1) as f32;

    let cs_mut = crate::ui::editor_context::sprite_state_mut(ui_state).active_mut();
    cs_mut.resize_drag = Some(ResizeDrag {
        corner,
        anchor_canvas: anchor,
        aspect_ratio: aspect,
    });
}

/// Continue an active resize drag that was already started.
fn handle_active_resize_drag(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    ctx: &egui::Context,
    rect: egui::Rect,
) -> bool {
    let is_dragging = crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .resize_drag
        .is_some();
    if !is_dragging {
        return false;
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .resize_drag = None;
        return true;
    }

    if response.dragged_by(egui::PointerButton::Primary) {
        apply_resize_drag(ui_state, ctx, rect);
        return true;
    }

    // Resize is active but no drag event this frame — consume to prevent fallthrough
    true
}

/// Apply one frame of resize based on current pointer position.
fn apply_resize_drag(ui_state: &mut EditorUI, ctx: &egui::Context, rect: egui::Rect) {
    let pointer_pos = match ctx.input(|i| i.pointer.latest_pos()) {
        Some(p) => p,
        None => return,
    };

    let cs = crate::ui::editor_context::sprite_state_mut(ui_state).active();
    let drag = cs.resize_drag.clone().unwrap();
    let zoom = cs.viewport.zoom;
    let pan = cs.viewport.pan;

    let canvas_screen_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));
    let mouse_canvas = glam::Vec2::new(
        (pointer_pos.x - canvas_screen_min.x) / zoom,
        (pointer_pos.y - canvas_screen_min.y) / zoom,
    );

    let (new_size, new_offset) =
        compute_resize(drag, mouse_canvas, ctx.input(|i| i.modifiers.shift));

    crate::ui::editor_context::sprite_state_mut(ui_state).resize_floating(new_size, new_offset);
}

/// Apply one frame of move-drag (nudge floating selection).
fn apply_move_drag(ui_state: &mut EditorUI, response: &egui::Response) {
    if crate::ui::editor_context::sprite_state_mut(ui_state).tool != SpriteEditorTool::Select {
        return;
    }
    let delta = response.drag_delta();
    let zoom = crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .viewport
        .zoom;
    let dx = (delta.x / zoom).round() as i32;
    let dy = (delta.y / zoom).round() as i32;
    if dx != 0 || dy != 0 {
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .nudge_floating(glam::IVec2::new(dx, dy));
    }
}

/// Compute new size and top-left offset from the resize drag and current mouse position.
fn compute_resize(
    drag: ResizeDrag,
    mouse_canvas: glam::Vec2,
    freeform: bool,
) -> (glam::UVec2, glam::IVec2) {
    let anchor = drag.anchor_canvas.as_vec2();

    let raw_w = (mouse_canvas.x - anchor.x).abs();
    let raw_h = (mouse_canvas.y - anchor.y).abs();

    let (w, h) = if freeform {
        (raw_w.max(1.0), raw_h.max(1.0))
    } else {
        constrain_aspect(raw_w, raw_h, drag.aspect_ratio)
    };

    let w = (w.round() as u32).max(1);
    let h = (h.round() as u32).max(1);

    // Top-left = min of anchor and the computed far corner
    let top_left = compute_top_left(drag.corner, drag.anchor_canvas, w, h);

    (glam::UVec2::new(w, h), top_left)
}

/// Constrain width/height to the given aspect ratio, fitting whichever axis is larger.
fn constrain_aspect(raw_w: f32, raw_h: f32, aspect: f32) -> (f32, f32) {
    let w_from_h = raw_h * aspect;
    let h_from_w = raw_w / aspect;
    if raw_w >= w_from_h {
        (raw_w.max(1.0), h_from_w.max(1.0))
    } else {
        (w_from_h.max(1.0), raw_h.max(1.0))
    }
}

/// Compute the top-left canvas offset given which corner is anchored.
fn compute_top_left(corner: ResizeCorner, anchor: glam::IVec2, w: u32, h: u32) -> glam::IVec2 {
    match corner {
        // User drags bottom-right → anchor is top-left
        ResizeCorner::BottomRight => anchor,
        // User drags bottom-left → anchor is top-right
        ResizeCorner::BottomLeft => glam::IVec2::new(anchor.x - w as i32, anchor.y),
        // User drags top-right → anchor is bottom-left
        ResizeCorner::TopRight => glam::IVec2::new(anchor.x, anchor.y - h as i32),
        // User drags top-left → anchor is bottom-right
        ResizeCorner::TopLeft => glam::IVec2::new(anchor.x - w as i32, anchor.y - h as i32),
    }
}

/// Get the fixed (opposite) corner in canvas coordinates for a given dragged corner.
fn anchor_for_corner(corner: ResizeCorner, offset: glam::IVec2, size: glam::UVec2) -> glam::IVec2 {
    match corner {
        ResizeCorner::BottomRight => offset,
        ResizeCorner::BottomLeft => glam::IVec2::new(offset.x + size.x as i32, offset.y),
        ResizeCorner::TopRight => glam::IVec2::new(offset.x, offset.y + size.y as i32),
        ResizeCorner::TopLeft => {
            glam::IVec2::new(offset.x + size.x as i32, offset.y + size.y as i32)
        }
    }
}

/// Compute screen positions for the four corner handles.
fn corner_screen_positions(
    rect: egui::Rect,
    pan: glam::Vec2,
    zoom: f32,
    offset: glam::IVec2,
    size: glam::UVec2,
) -> [(ResizeCorner, egui::Pos2); 4] {
    let canvas_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));
    let tl = egui::pos2(
        canvas_min.x + offset.x as f32 * zoom,
        canvas_min.y + offset.y as f32 * zoom,
    );
    let br = egui::pos2(tl.x + size.x as f32 * zoom, tl.y + size.y as f32 * zoom);
    [
        (ResizeCorner::TopLeft, tl),
        (ResizeCorner::TopRight, egui::pos2(br.x, tl.y)),
        (ResizeCorner::BottomLeft, egui::pos2(tl.x, br.y)),
        (ResizeCorner::BottomRight, br),
    ]
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
            crate::ui::editor_context::sprite_state_mut(ui_state).lift_selection();
            invalidate_canvas_texture(ui_state);
            return;
        }

        let existing_selection = crate::ui::editor_context::sprite_state_mut(ui_state)
            .active()
            .selection
            .clone();
        let active = crate::ui::editor_context::sprite_state_mut(ui_state).active_mut();
        active.selection_start_pos = Some(canvas_pos);
        active.selection_drag_base = existing_selection;
    }

    if response.dragged_by(egui::PointerButton::Primary) {
        if let Some(start) = crate::ui::editor_context::sprite_state_mut(ui_state)
            .active()
            .selection_start_pos
        {
            apply_drag_selection(ui_state, start, canvas_pos, selection_mode);
        }
    }

    let primary_released = ctx.input(|input| input.pointer.primary_released());
    if primary_released {
        if let Some(start) = crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .selection_start_pos
            .take()
        {
            apply_drag_selection(ui_state, start, canvas_pos, selection_mode);
            crate::ui::editor_context::sprite_state_mut(ui_state)
                .active_mut()
                .selection_drag_base = None;
        }
    }

    if response.clicked_by(egui::PointerButton::Secondary) {
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .selection = None;
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .selection_drag_base = None;
    }
}

fn is_click_inside_selection(ui_state: &EditorUI, pos: glam::IVec2) -> bool {
    if pos.x < 0 || pos.y < 0 {
        return false;
    }
    ui_state
        .sprite_editor_context()
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
        if let Some(canvas) = &crate::ui::editor_context::sprite_state_mut(ui_state)
            .active()
            .canvas
        {
            let x = canvas_pos.x as u32;
            let y = canvas_pos.y as u32;
            let selection_mode = current_selection_mode(&response.ctx);

            if let Some(mask) = canvas.find_connected_selection_mask(x, y) {
                let base = crate::ui::editor_context::sprite_state_mut(ui_state)
                    .active()
                    .selection
                    .clone();
                crate::ui::editor_context::sprite_state_mut(ui_state)
                    .active_mut()
                    .selection = merge_selection_masks(base.as_ref(), &mask, selection_mode);
            } else if selection_mode == SelectionModifyMode::Replace {
                crate::ui::editor_context::sprite_state_mut(ui_state)
                    .active_mut()
                    .selection = None;
            } else {
                // Keep the existing selection on transparent clicks when adding/subtracting.
            }
        }
    }

    // Clear selection with right-click
    if response.clicked_by(egui::PointerButton::Secondary) {
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .selection = None;
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
    if let Some(canvas) = &mut crate::ui::editor_context::sprite_state_mut(ui_state)
        .active_mut()
        .canvas
    {
        if SpritePaintInteraction::erase_connected_color_in_bounds(canvas, canvas_pos, bounds) {
            crate::ui::editor_context::sprite_state_mut(ui_state)
                .active_mut()
                .dirty = true;
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
        crate::ui::editor_context::sprite_state_mut(ui_state).color_mode,
        crate::ui::editor_context::sprite_state_mut(ui_state).foreground_color,
        selected_palette(ui_state),
    );
    if let Some(canvas) = &mut crate::ui::editor_context::sprite_state_mut(ui_state)
        .active_mut()
        .canvas
    {
        if SpritePaintInteraction::add_outline_in_bounds(canvas, canvas_pos, outline_color, bounds)
        {
            crate::ui::editor_context::sprite_state_mut(ui_state)
                .active_mut()
                .dirty = true;
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
        crate::ui::editor_context::sprite_state_mut(ui_state).color_mode,
        crate::ui::editor_context::sprite_state_mut(ui_state).foreground_color,
        selected_palette(ui_state),
    );
    if let Some(canvas) = &mut crate::ui::editor_context::sprite_state_mut(ui_state)
        .active_mut()
        .canvas
    {
        if SpritePaintInteraction::add_ground_shadow_in_bounds(
            canvas,
            canvas_pos,
            shadow_color,
            bounds,
        ) {
            crate::ui::editor_context::sprite_state_mut(ui_state)
                .active_mut()
                .dirty = true;
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

    if crate::ui::editor_context::sprite_state(ui_state).is_sheet() {
        let cell_idx = crate::ui::editor_context::sprite_state(ui_state).cell_at_position(x, y)?;
        let (start_x, start_y, end_x, end_y) =
            crate::ui::editor_context::sprite_state(ui_state).cell_bounds(cell_idx)?;
        return Some((
            glam::UVec2::new(start_x, start_y),
            glam::UVec2::new(end_x, end_y),
        ));
    }

    let (width, height) = crate::ui::editor_context::sprite_state(ui_state).canvas_dimensions()?;
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
    let Some((canvas_width, canvas_height)) =
        crate::ui::editor_context::sprite_state_mut(ui_state).canvas_dimensions()
    else {
        return;
    };
    let selection_rect = create_selection(start, end);
    let drag_mask = selection_mask_from_rect(canvas_width, canvas_height, selection_rect);
    let base = crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .selection_drag_base
        .clone();
    crate::ui::editor_context::sprite_state_mut(ui_state)
        .active_mut()
        .selection = merge_selection_masks(base.as_ref(), &drag_mask, mode);
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
    if !crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .is_painting
    {
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .is_painting = true;
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .canvas_before_stroke = crate::ui::editor_context::sprite_state_mut(ui_state)
            .active()
            .canvas
            .clone();
    }
}

fn finish_paint_stroke(ui_state: &mut EditorUI) {
    if crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .is_painting
    {
        crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .is_painting = false;
        if let Some(before) = crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .canvas_before_stroke
            .take()
        {
            crate::ui::editor_context::sprite_state_mut(ui_state).push_undo_state(before);
        }
        let foreground_color = crate::ui::editor_context::sprite_state(ui_state).foreground_color;
        ui_state
            .sprite_editor_context_mut()
            .sprite
            .add_recent_color(foreground_color);
    }
}

fn selected_palette(ui_state: &EditorUI) -> Option<Palette4> {
    ui_state
        .sprite_editor_context()
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

fn symmetry_config(ui_state: &EditorUI) -> SymmetryConfig {
    let (origin, size) = compute_symmetry_bounds(ui_state);
    SymmetryConfig {
        bounds: SymmetryBounds { origin, size },
        horizontal: crate::ui::editor_context::sprite_state(ui_state).symmetry_horizontal,
        vertical: crate::ui::editor_context::sprite_state(ui_state).symmetry_vertical,
    }
}

fn compute_symmetry_bounds(ui_state: &EditorUI) -> (UVec2, UVec2) {
    // In sheet mode with per-tile enabled, mirror within the tile under the cursor
    if crate::ui::editor_context::sprite_state(ui_state).symmetry_per_tile
        && crate::ui::editor_context::sprite_state(ui_state).is_sheet()
    {
        if let Some(pos) = crate::ui::editor_context::sprite_state(ui_state)
            .active()
            .cursor_canvas_pos
        {
            if pos.x >= 0 && pos.y >= 0 {
                if let Some(cell_idx) = crate::ui::editor_context::sprite_state(ui_state)
                    .cell_at_position(pos.x as u32, pos.y as u32)
                {
                    if let Some((sx, sy, ex, ey)) =
                        crate::ui::editor_context::sprite_state(ui_state).cell_bounds(cell_idx)
                    {
                        return (UVec2::new(sx, sy), UVec2::new(ex - sx, ey - sy));
                    }
                }
            }
        }
    }
    // Otherwise mirror within the full canvas
    let (w, h) = crate::ui::editor_context::sprite_state(ui_state)
        .canvas_dimensions()
        .unwrap_or((1, 1));
    (UVec2::ZERO, UVec2::new(w, h))
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
        (egui::Key::R, Rectangle),
        (egui::Key::C, Ellipse),
    ];

    for &(key, tool) in tool_keys {
        if ui.input(|i| i.key_pressed(key)) {
            crate::ui::editor_context::sprite_state_mut(ui_state).set_tool(tool);
        }
    }

    // Brush size
    if ui.input(|i| i.key_pressed(egui::Key::OpenBracket)) {
        crate::ui::editor_context::sprite_state_mut(ui_state).brush_size =
            crate::ui::editor_context::sprite_state_mut(ui_state)
                .brush_size
                .saturating_sub(1)
                .max(1);
    }
    if ui.input(|i| i.key_pressed(egui::Key::CloseBracket)) {
        crate::ui::editor_context::sprite_state_mut(ui_state).brush_size =
            (crate::ui::editor_context::sprite_state_mut(ui_state).brush_size + 1).min(32);
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
