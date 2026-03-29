//! Canvas rendering and drawing operations.

use crate::ui::editor_ui::{
    CanvasSide, PixelColor, SelectionMask, SpriteCanvas, SpriteCanvasViewport,
};
use crate::ui::sprite_editor::{preview_indexed_color, FloatingOrigin, FloatingSelection};
use crate::ui::EditorUI;
use toki_core::assets::atlas::ColorMode;
use toki_core::palette::Palette4;

use super::shortcuts::handle_undo_redo_shortcuts;
use super::tools::{handle_tool_interaction, handle_tool_shortcuts};

pub fn render_canvas_viewport(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    target_side: Option<CanvasSide>,
) {
    let available_size = ui.available_size();

    let viewport_height = (available_size.y - 24.0).max(50.0);
    let viewport_width = available_size.x.max(50.0);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(viewport_width, viewport_height),
        egui::Sense::click_and_drag(),
    );

    let render_side = target_side.unwrap_or(crate::ui::editor_context::sprite_state_mut(ui_state).active_canvas);

    if let Some(side) = target_side {
        if response.clicked() || response.dragged() {
            crate::ui::editor_context::sprite_state_mut(ui_state).set_active_canvas(side);
        }
    }

    let is_interactive =
        target_side.is_none() || target_side == Some(crate::ui::editor_context::sprite_state_mut(ui_state).active_canvas);

    // Handle pan with right-click drag or middle-click drag
    if response.dragged_by(egui::PointerButton::Secondary)
        || response.dragged_by(egui::PointerButton::Middle)
    {
        let delta = response.drag_delta();
        ui_state
            .sprite_editor_context_mut()
            .sprite
            .canvas_state_mut(render_side)
            .viewport
            .pan_by(glam::Vec2::new(delta.x, delta.y));
    }

    // Handle scroll zoom
    if response.hovered() {
        let scroll_delta = ui.input(|input| input.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            if scroll_delta > 0.0 {
                ui_state
            .sprite_editor_context_mut()
            .sprite
                    .canvas_state_mut(render_side)
                    .viewport
                    .zoom_in();
            } else {
                ui_state
            .sprite_editor_context_mut()
            .sprite
                    .canvas_state_mut(render_side)
                    .viewport
                    .zoom_out();
            }
        }
    }

    // Handle keyboard shortcuts
    if is_interactive && !ui.ctx().wants_keyboard_input() {
        if ui.input(|input| {
            input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals)
        }) {
            crate::ui::editor_context::sprite_state_mut(ui_state).active_mut().viewport.zoom_in();
        }
        if ui.input(|input| input.key_pressed(egui::Key::Minus)) {
            crate::ui::editor_context::sprite_state_mut(ui_state).active_mut().viewport.zoom_out();
        }

        handle_floating_shortcuts(ui_state, ui);
        handle_tool_shortcuts(ui_state, ui);
        handle_undo_redo_shortcuts(ui_state, ui);
    }

    // Update cursor position
    if let Some(hover_pos) = response.hover_pos() {
        let canvas_pos = ui_state
            .sprite_editor_context_mut()
            .sprite
            .canvas_state(render_side)
            .viewport
            .screen_to_canvas(glam::Vec2::new(hover_pos.x, hover_pos.y), rect);
        ui_state
            .sprite_editor_context_mut()
            .sprite
            .canvas_state_mut(render_side)
            .cursor_canvas_pos = Some(glam::IVec2::new(
            canvas_pos.x.floor() as i32,
            canvas_pos.y.floor() as i32,
        ));
    } else {
        ui_state
            .sprite_editor_context_mut()
            .sprite
            .canvas_state_mut(render_side)
            .cursor_canvas_pos = None;
    }

    // Handle tool interactions
    if is_interactive {
        handle_tool_interaction(ui_state, &response, rect, ctx);
    }

    // Draw canvas background
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, egui::Color32::from_gray(40));

    // Ensure canvas texture is created
    let canvas_state = crate::ui::editor_context::sprite_state_mut(ui_state).canvas_state(render_side);
    if canvas_state.canvas.is_some() {
        ensure_canvas_texture_for_side(ui_state, ctx, render_side);
    }

    // Draw checkerboard and canvas
    let canvas_state = crate::ui::editor_context::sprite_state_mut(ui_state).canvas_state(render_side);
    if let Some(canvas) = &canvas_state.canvas {
        let viewport = canvas_state.viewport.clone();
        let texture = canvas_state.canvas_texture.as_ref();
        draw_canvas_with_checkerboard(&painter, rect, &viewport, canvas, texture);
    }

    // Draw tile preview (3x3 tiled copies)
    let canvas_state = crate::ui::editor_context::sprite_state_mut(ui_state).canvas_state(render_side);
    if canvas_state.tile_preview {
        if let Some(canvas) = &canvas_state.canvas {
            draw_tile_preview(&painter, rect, &canvas_state.viewport, canvas);
        }
    }

    // Draw pixel grid overlay
    let canvas_state = crate::ui::editor_context::sprite_state_mut(ui_state).canvas_state(render_side);
    if canvas_state.show_grid && canvas_state.viewport.zoom >= 4.0 {
        if let Some(canvas) = &canvas_state.canvas {
            draw_pixel_grid(&painter, rect, &canvas_state.viewport, canvas);
        }
    }

    // Draw cell grid overlay for sprite sheets
    let canvas_state = crate::ui::editor_context::sprite_state_mut(ui_state).canvas_state(render_side);
    if canvas_state.show_cell_grid {
        if let Some(canvas) = &canvas_state.canvas {
            draw_cell_grid(
                &painter,
                rect,
                &canvas_state.viewport,
                canvas,
                canvas_state.cell_size,
                canvas_state.selected_cell,
            );
        }
    }

    // Draw hovered pixel highlight
    let canvas_state = crate::ui::editor_context::sprite_state_mut(ui_state).canvas_state(render_side);
    if let Some(canvas) = &canvas_state.canvas {
        draw_hovered_pixel_highlight(
            &painter,
            rect,
            &canvas_state.viewport,
            canvas,
            canvas_state.cursor_canvas_pos,
        );
    }

    // Draw symmetry guide lines
    let symmetry_horizontal = crate::ui::editor_context::sprite_state(ui_state).symmetry_horizontal;
    let symmetry_vertical = crate::ui::editor_context::sprite_state(ui_state).symmetry_vertical;
    if symmetry_horizontal || symmetry_vertical {
        let canvas_state = crate::ui::editor_context::sprite_state_mut(ui_state).canvas_state(render_side);
        if let Some(canvas) = &canvas_state.canvas {
            draw_symmetry_guides(
                &painter,
                rect,
                &canvas_state.viewport,
                canvas,
                symmetry_horizontal,
                symmetry_vertical,
            );
        }
    }

    // Draw floating selection overlay OR static selection overlay
    let canvas_state = crate::ui::editor_context::sprite_state_mut(ui_state).canvas_state(render_side);
    if let Some(floating) = &canvas_state.floating {
        draw_floating_selection(&painter, rect, &canvas_state.viewport, floating);
    } else if let Some(selection) = &canvas_state.selection {
        draw_selection_mask(&painter, rect, &canvas_state.viewport, selection);
    }

    // Status bar
    if target_side.is_none() || target_side == Some(crate::ui::editor_context::sprite_state_mut(ui_state).active_canvas) {
        render_status_bar(ui, ui_state);
    }
}

/// Render an empty canvas slot with options to create/load
pub fn render_empty_canvas_slot(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    sprites_dir: Option<&std::path::Path>,
    side: CanvasSide,
) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.label("Empty");
            if ui.button("New").clicked() {
                crate::ui::editor_context::sprite_state_mut(ui_state).set_active_canvas(side);
                ui_state.begin_new_sprite_canvas_dialog();
            }
            let load_enabled = sprites_dir.is_some();
            if ui
                .add_enabled(load_enabled, egui::Button::new("Load"))
                .clicked()
            {
                if let Some(dir) = sprites_dir {
                    crate::ui::editor_context::sprite_state_mut(ui_state).set_active_canvas(side);
                    crate::ui::editor_context::sprite_state_mut(ui_state).begin_load_dialog(dir);
                }
            }
        });
    });
}

pub fn ensure_canvas_texture_for_side(
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    side: CanvasSide,
) {
    let cs = crate::ui::editor_context::sprite_state(ui_state).canvas_state(side);
    if cs.canvas_texture.is_some() && !cs.canvas_texture_dirty {
        return;
    }
    crate::ui::editor_context::sprite_state_mut(ui_state).canvas_state_mut(side).canvas_texture_dirty = false;

    let Some(canvas) = crate::ui::editor_context::sprite_state(ui_state).canvas_state(side).canvas.clone() else {
        return;
    };

    let color_mode = crate::ui::editor_context::sprite_state(ui_state).color_mode;
    let selected_palette = ui_state
        .sprite_editor_context()
        .sprite
        .selected_palette_id
        .as_ref()
        .and_then(|palette_id| ui_state.project.available_palettes.get(palette_id))
        .copied();

    let display_pixels = canvas_display_pixels(
        &canvas,
        color_mode,
        selected_palette,
    );
    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [canvas.width as usize, canvas.height as usize],
        &display_pixels,
    );

    let texture_name = match side {
        CanvasSide::Left => "sprite_editor_canvas_left",
        CanvasSide::Right => "sprite_editor_canvas_right",
    };

    let texture = ctx.load_texture(texture_name, color_image, egui::TextureOptions::NEAREST);
    crate::ui::editor_context::sprite_state_mut(ui_state).canvas_state_mut(side).canvas_texture = Some(texture);
}

pub fn invalidate_canvas_texture(ui_state: &mut EditorUI) {
    crate::ui::editor_context::sprite_state_mut(ui_state).active_mut().canvas_texture_dirty = true;
}

pub fn invalidate_canvas_texture_for_side(ui_state: &mut EditorUI, side: CanvasSide) {
    crate::ui::editor_context::sprite_state_mut(ui_state).canvas_state_mut(side).canvas_texture_dirty = true;
}

fn handle_floating_shortcuts(ui_state: &mut EditorUI, ui: &egui::Ui) {
    if crate::ui::editor_context::sprite_state_mut(ui_state).has_floating() {
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            crate::ui::editor_context::sprite_state_mut(ui_state).commit_floating();
            invalidate_canvas_texture(ui_state);
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            crate::ui::editor_context::sprite_state_mut(ui_state).cancel_floating();
            invalidate_canvas_texture(ui_state);
        }
    } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        crate::ui::editor_context::sprite_state_mut(ui_state).active_mut().selection = None;
    }

    // Arrow keys: lift selection into float (if needed) and nudge.
    // Works whether a float already exists or just a selection.
    let has_selection_or_float =
        crate::ui::editor_context::sprite_state_mut(ui_state).has_floating() || crate::ui::editor_context::sprite_state_mut(ui_state).active().selection.is_some();
    if has_selection_or_float {
        let arrow_keys = [
            (egui::Key::ArrowUp, glam::IVec2::new(0, -1)),
            (egui::Key::ArrowDown, glam::IVec2::new(0, 1)),
            (egui::Key::ArrowLeft, glam::IVec2::new(-1, 0)),
            (egui::Key::ArrowRight, glam::IVec2::new(1, 0)),
        ];
        for (key, delta) in arrow_keys {
            if ui.input(|i| i.key_pressed(key)) {
                crate::ui::editor_context::sprite_state_mut(ui_state).lift_and_nudge(delta);
                invalidate_canvas_texture(ui_state);
            }
        }
    }
}

fn canvas_display_pixels(
    canvas: &SpriteCanvas,
    color_mode: ColorMode,
    palette: Option<Palette4>,
) -> Vec<u8> {
    if color_mode != ColorMode::PaletteIndexed {
        return canvas.pixels().to_vec();
    }

    let Some(palette) = palette else {
        return canvas.pixels().to_vec();
    };

    let mut pixels = canvas.pixels().to_vec();
    for rgba in pixels.chunks_exact_mut(4) {
        let display = preview_indexed_color(
            crate::ui::editor_ui::PixelColor::from_rgba_array([rgba[0], rgba[1], rgba[2], rgba[3]]),
            palette,
        );
        rgba.copy_from_slice(&display.to_rgba_array());
    }
    pixels
}

fn draw_canvas_with_checkerboard(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    canvas: &SpriteCanvas,
    texture: Option<&egui::TextureHandle>,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;

    let canvas_screen_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));
    let canvas_screen_max = egui::pos2(
        canvas_screen_min.x + canvas.width as f32 * zoom,
        canvas_screen_min.y + canvas.height as f32 * zoom,
    );
    let canvas_screen_rect = egui::Rect::from_min_max(canvas_screen_min, canvas_screen_max);

    let visible_rect = canvas_screen_rect.intersect(rect);
    if visible_rect.is_positive() {
        draw_checkerboard(painter, rect, visible_rect, viewport, canvas);

        if let Some(tex) = texture {
            painter.image(
                tex.id(),
                canvas_screen_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
    }

    painter.rect_stroke(
        canvas_screen_rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
        egui::StrokeKind::Outside,
    );
}

fn draw_checkerboard(
    painter: &egui::Painter,
    viewport_rect: egui::Rect,
    visible_rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    canvas: &SpriteCanvas,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;

    let pixel_size = zoom;
    let color1 = egui::Color32::from_gray(180);
    let color2 = egui::Color32::from_gray(220);

    let canvas_screen_min = egui::pos2(
        viewport_rect.left() + (-pan.x * zoom),
        viewport_rect.top() + (-pan.y * zoom),
    );

    let first_visible_x = ((visible_rect.left() - canvas_screen_min.x) / pixel_size).floor() as i32;
    let first_visible_y = ((visible_rect.top() - canvas_screen_min.y) / pixel_size).floor() as i32;
    let last_visible_x = ((visible_rect.right() - canvas_screen_min.x) / pixel_size).ceil() as i32;
    let last_visible_y = ((visible_rect.bottom() - canvas_screen_min.y) / pixel_size).ceil() as i32;

    let start_x = first_visible_x.max(0) as u32;
    let start_y = first_visible_y.max(0) as u32;
    let end_x = (last_visible_x as u32).min(canvas.width);
    let end_y = (last_visible_y as u32).min(canvas.height);

    for py in start_y..end_y {
        for px in start_x..end_x {
            let color = if (px + py) % 2 == 0 { color1 } else { color2 };
            let screen_x = canvas_screen_min.x + px as f32 * pixel_size;
            let screen_y = canvas_screen_min.y + py as f32 * pixel_size;
            let check_rect = egui::Rect::from_min_size(
                egui::pos2(screen_x, screen_y),
                egui::vec2(pixel_size, pixel_size),
            );
            let clipped = check_rect.intersect(visible_rect);
            if clipped.width() > 0.0 && clipped.height() > 0.0 {
                painter.rect_filled(clipped, 0.0, color);
            }
        }
    }
}

fn draw_tile_preview(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    canvas: &SpriteCanvas,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;
    let canvas_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));
    let tile_w = canvas.width as f32 * zoom;
    let tile_h = canvas.height as f32 * zoom;

    // Render the canvas pixels directly for each of the 8 surrounding copies
    let tint = egui::Color32::from_white_alpha(90);
    let offsets: [(i32, i32); 8] = [
        (-1, -1), (0, -1), (1, -1),
        (-1, 0),           (1, 0),
        (-1, 1),  (0, 1),  (1, 1),
    ];

    for (dx, dy) in offsets {
        let copy_min = egui::pos2(
            canvas_min.x + dx as f32 * tile_w,
            canvas_min.y + dy as f32 * tile_h,
        );
        let copy_rect = egui::Rect::from_min_size(copy_min, egui::vec2(tile_w, tile_h));
        if !copy_rect.intersects(rect) {
            continue;
        }
        draw_tiled_copy(painter, rect, canvas, copy_min, zoom, tint);
    }
}

fn draw_tiled_copy(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    canvas: &SpriteCanvas,
    origin: egui::Pos2,
    zoom: f32,
    tint: egui::Color32,
) {
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let Some(color) = canvas.get_pixel(x, y) else {
                continue;
            };
            if color.a == 0 {
                continue;
            }
            let px = origin.x + x as f32 * zoom;
            let py = origin.y + y as f32 * zoom;
            let pixel_rect = egui::Rect::from_min_size(egui::pos2(px, py), egui::vec2(zoom, zoom));
            if !pixel_rect.intersects(clip_rect) {
                continue;
            }
            // Blend pixel color with tint alpha
            let blended = egui::Color32::from_rgba_unmultiplied(
                color.r, color.g, color.b,
                ((color.a as u16 * tint.a() as u16) / 255) as u8,
            );
            painter.rect_filled(pixel_rect, 0.0, blended);
        }
    }
}

fn draw_pixel_grid(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    canvas: &SpriteCanvas,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;

    let canvas_screen_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(80, 80, 80, 180));

    // Vertical lines
    for x in 0..=canvas.width {
        let screen_x = canvas_screen_min.x + x as f32 * zoom;
        if screen_x >= rect.left() && screen_x <= rect.right() {
            painter.line_segment(
                [
                    egui::pos2(screen_x, rect.top().max(canvas_screen_min.y)),
                    egui::pos2(
                        screen_x,
                        rect.bottom()
                            .min(canvas_screen_min.y + canvas.height as f32 * zoom),
                    ),
                ],
                stroke,
            );
        }
    }

    // Horizontal lines
    for y in 0..=canvas.height {
        let screen_y = canvas_screen_min.y + y as f32 * zoom;
        if screen_y >= rect.top() && screen_y <= rect.bottom() {
            painter.line_segment(
                [
                    egui::pos2(rect.left().max(canvas_screen_min.x), screen_y),
                    egui::pos2(
                        rect.right()
                            .min(canvas_screen_min.x + canvas.width as f32 * zoom),
                        screen_y,
                    ),
                ],
                stroke,
            );
        }
    }
}

fn draw_cell_grid(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    canvas: &SpriteCanvas,
    cell_size: glam::UVec2,
    selected_cell: Option<usize>,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;

    let canvas_screen_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));
    let stroke = egui::Stroke::new(
        2.0,
        egui::Color32::from_rgba_unmultiplied(255, 200, 50, 180),
    );

    let cols = canvas.width / cell_size.x.max(1);
    let rows = canvas.height / cell_size.y.max(1);

    // Vertical cell lines
    for x in 0..=cols {
        let pixel_x = x * cell_size.x;
        let screen_x = canvas_screen_min.x + pixel_x as f32 * zoom;
        if screen_x >= rect.left() && screen_x <= rect.right() {
            painter.line_segment(
                [
                    egui::pos2(screen_x, rect.top().max(canvas_screen_min.y)),
                    egui::pos2(
                        screen_x,
                        rect.bottom()
                            .min(canvas_screen_min.y + canvas.height as f32 * zoom),
                    ),
                ],
                stroke,
            );
        }
    }

    // Horizontal cell lines
    for y in 0..=rows {
        let pixel_y = y * cell_size.y;
        let screen_y = canvas_screen_min.y + pixel_y as f32 * zoom;
        if screen_y >= rect.top() && screen_y <= rect.bottom() {
            painter.line_segment(
                [
                    egui::pos2(rect.left().max(canvas_screen_min.x), screen_y),
                    egui::pos2(
                        rect.right()
                            .min(canvas_screen_min.x + canvas.width as f32 * zoom),
                        screen_y,
                    ),
                ],
                stroke,
            );
        }
    }

    // Highlight selected cell
    if let Some(cell_idx) = selected_cell {
        let col = cell_idx as u32 % cols;
        let row = cell_idx as u32 / cols;
        if row < rows {
            let cell_min = egui::pos2(
                canvas_screen_min.x + (col * cell_size.x) as f32 * zoom,
                canvas_screen_min.y + (row * cell_size.y) as f32 * zoom,
            );
            let cell_max = egui::pos2(
                cell_min.x + cell_size.x as f32 * zoom,
                cell_min.y + cell_size.y as f32 * zoom,
            );
            let cell_rect = egui::Rect::from_min_max(cell_min, cell_max);

            let fill = egui::Color32::from_rgba_unmultiplied(255, 200, 50, 40);
            let highlight_stroke = egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 200, 50));

            painter.rect_filled(cell_rect, 0.0, fill);
            painter.rect_stroke(cell_rect, 0.0, highlight_stroke, egui::StrokeKind::Inside);
        }
    }
}

fn draw_selection_mask(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    selection: &SelectionMask,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;

    let canvas_screen_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 150, 255));

    for y in 0..selection.height {
        for x in 0..selection.width {
            if !selection.is_selected(x, y) {
                continue;
            }

            let pixel_min = egui::pos2(
                canvas_screen_min.x + x as f32 * zoom,
                canvas_screen_min.y + y as f32 * zoom,
            );
            let pixel_max = egui::pos2(pixel_min.x + zoom, pixel_min.y + zoom);

            if y == 0 || !selection.is_selected(x, y - 1) {
                painter.line_segment(
                    [egui::pos2(pixel_min.x, pixel_min.y), egui::pos2(pixel_max.x, pixel_min.y)],
                    stroke,
                );
            }
            if y + 1 >= selection.height || !selection.is_selected(x, y + 1) {
                painter.line_segment(
                    [egui::pos2(pixel_min.x, pixel_max.y), egui::pos2(pixel_max.x, pixel_max.y)],
                    stroke,
                );
            }
            if x == 0 || !selection.is_selected(x - 1, y) {
                painter.line_segment(
                    [egui::pos2(pixel_min.x, pixel_min.y), egui::pos2(pixel_min.x, pixel_max.y)],
                    stroke,
                );
            }
            if x + 1 >= selection.width || !selection.is_selected(x + 1, y) {
                painter.line_segment(
                    [egui::pos2(pixel_max.x, pixel_min.y), egui::pos2(pixel_max.x, pixel_max.y)],
                    stroke,
                );
            }
        }
    }
}

fn draw_symmetry_guides(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    canvas: &SpriteCanvas,
    horizontal: bool,
    vertical: bool,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;
    let canvas_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 100, 100, 160));

    if horizontal {
        let mid_x = canvas_min.x + (canvas.width as f32 / 2.0) * zoom;
        let top = canvas_min.y.max(rect.top());
        let bottom = (canvas_min.y + canvas.height as f32 * zoom).min(rect.bottom());
        painter.line_segment([egui::pos2(mid_x, top), egui::pos2(mid_x, bottom)], stroke);
    }
    if vertical {
        let mid_y = canvas_min.y + (canvas.height as f32 / 2.0) * zoom;
        let left = canvas_min.x.max(rect.left());
        let right = (canvas_min.x + canvas.width as f32 * zoom).min(rect.right());
        painter.line_segment([egui::pos2(left, mid_y), egui::pos2(right, mid_y)], stroke);
    }
}

fn draw_floating_selection(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    floating: &FloatingSelection,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;
    let canvas_screen_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));

    // Draw floating pixels
    draw_floating_pixels(painter, canvas_screen_min, zoom, floating);

    // Draw marching-ants border around the floating mask
    let mask_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 150, 255));
    draw_offset_mask_border(painter, canvas_screen_min, zoom, floating, mask_stroke);
}

fn draw_floating_pixels(
    painter: &egui::Painter,
    canvas_screen_min: egui::Pos2,
    zoom: f32,
    floating: &FloatingSelection,
) {
    for y in 0..floating.pixels.height {
        for x in 0..floating.pixels.width {
            let Some(color) = floating.pixels.get_pixel(x, y) else {
                continue;
            };
            if color.a == 0 {
                continue;
            }
            let sx = canvas_screen_min.x + (floating.offset.x + x as i32) as f32 * zoom;
            let sy = canvas_screen_min.y + (floating.offset.y + y as i32) as f32 * zoom;
            let pixel_rect =
                egui::Rect::from_min_size(egui::pos2(sx, sy), egui::vec2(zoom, zoom));
            painter.rect_filled(pixel_rect, 0.0, floating_preview_color(color, &floating.origin));
        }
    }
}

fn floating_preview_color(color: PixelColor, origin: &FloatingOrigin) -> egui::Color32 {
    if !origin.is_paste_preview() {
        return color.to_color32();
    }

    let alpha = ((u16::from(color.a) * 160) / 255) as u8;
    egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, alpha)
}

fn draw_offset_mask_border(
    painter: &egui::Painter,
    canvas_screen_min: egui::Pos2,
    zoom: f32,
    floating: &FloatingSelection,
    stroke: egui::Stroke,
) {
    let mask = &floating.mask;
    for y in 0..mask.height {
        for x in 0..mask.width {
            if !mask.is_selected(x, y) {
                continue;
            }
            let sx = canvas_screen_min.x + (floating.offset.x + x as i32) as f32 * zoom;
            let sy = canvas_screen_min.y + (floating.offset.y + y as i32) as f32 * zoom;
            let p_min = egui::pos2(sx, sy);
            let p_max = egui::pos2(sx + zoom, sy + zoom);

            if y == 0 || !mask.is_selected(x, y - 1) {
                painter.line_segment([egui::pos2(p_min.x, p_min.y), egui::pos2(p_max.x, p_min.y)], stroke);
            }
            if y + 1 >= mask.height || !mask.is_selected(x, y + 1) {
                painter.line_segment([egui::pos2(p_min.x, p_max.y), egui::pos2(p_max.x, p_max.y)], stroke);
            }
            if x == 0 || !mask.is_selected(x - 1, y) {
                painter.line_segment([egui::pos2(p_min.x, p_min.y), egui::pos2(p_min.x, p_max.y)], stroke);
            }
            if x + 1 >= mask.width || !mask.is_selected(x + 1, y) {
                painter.line_segment([egui::pos2(p_max.x, p_min.y), egui::pos2(p_max.x, p_max.y)], stroke);
            }
        }
    }
}

fn hovered_pixel_screen_rect(
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    canvas: &SpriteCanvas,
    cursor_canvas_pos: Option<glam::IVec2>,
) -> Option<egui::Rect> {
    let pos = cursor_canvas_pos?;
    if pos.x < 0 || pos.y < 0 || pos.x >= canvas.width as i32 || pos.y >= canvas.height as i32 {
        return None;
    }

    let zoom = viewport.zoom;
    let pan = viewport.pan;
    let min = egui::pos2(
        rect.left() + (pos.x as f32 - pan.x) * zoom,
        rect.top() + (pos.y as f32 - pan.y) * zoom,
    );
    let pixel_rect = egui::Rect::from_min_size(min, egui::vec2(zoom, zoom));
    let clipped = pixel_rect.intersect(rect);
    if clipped.is_positive() {
        Some(clipped)
    } else {
        None
    }
}

fn draw_hovered_pixel_highlight(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    canvas: &SpriteCanvas,
    cursor_canvas_pos: Option<glam::IVec2>,
) {
    let Some(pixel_rect) = hovered_pixel_screen_rect(rect, viewport, canvas, cursor_canvas_pos)
    else {
        return;
    };

    let fill = egui::Color32::from_rgba_unmultiplied(90, 160, 255, 70);
    let stroke = egui::Stroke::new(
        1.0,
        egui::Color32::from_rgba_unmultiplied(120, 190, 255, 220),
    );
    painter.rect_filled(pixel_rect, 0.0, fill);
    painter.rect_stroke(pixel_rect, 0.0, stroke, egui::StrokeKind::Inside);
}

fn render_status_bar(ui: &mut egui::Ui, ui_state: &EditorUI) {
    ui.horizontal(|ui| {
        if let Some(pos) = crate::ui::editor_context::sprite_state(ui_state).active().cursor_canvas_pos {
            ui.label(format!("Cursor: {}, {}", pos.x, pos.y));
        } else {
            ui.label("Cursor: -, -");
        }

        ui.separator();

        if let Some((w, h)) = crate::ui::editor_context::sprite_state(ui_state).canvas_dimensions() {
            ui.label(format!("Canvas: {}x{}", w, h));
        }

        ui.separator();

        ui.label(format!(
            "Zoom: {}x",
            crate::ui::editor_context::sprite_state(ui_state).active().viewport.zoom as i32
        ));

        ui.separator();

        if crate::ui::editor_context::sprite_state(ui_state).active().dirty {
            ui.label("*Modified");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::editor_ui::PixelColor;

    #[test]
    fn hovered_pixel_screen_rect_returns_none_for_out_of_bounds_cursor() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 100.0));
        let viewport = SpriteCanvasViewport::default();
        let canvas = SpriteCanvas::new(8, 8);

        assert!(
            hovered_pixel_screen_rect(rect, &viewport, &canvas, Some(glam::IVec2::new(-1, 0)))
                .is_none()
        );
        assert!(
            hovered_pixel_screen_rect(rect, &viewport, &canvas, Some(glam::IVec2::new(8, 0)))
                .is_none()
        );
        assert!(hovered_pixel_screen_rect(rect, &viewport, &canvas, None).is_none());
    }

    #[test]
    fn hovered_pixel_screen_rect_maps_canvas_pixel_to_screen_rect() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(200.0, 200.0));
        let viewport = SpriteCanvasViewport {
            zoom: 4.0,
            pan: glam::Vec2::new(1.0, 2.0),
            ..Default::default()
        };
        let canvas = SpriteCanvas::new(8, 8);

        let pixel_rect =
            hovered_pixel_screen_rect(rect, &viewport, &canvas, Some(glam::IVec2::new(3, 5)))
                .unwrap();

        assert_eq!(pixel_rect.min, egui::pos2(18.0, 32.0));
        assert_eq!(pixel_rect.max, egui::pos2(22.0, 36.0));
    }

    #[test]
    fn canvas_display_pixels_recolors_canonical_indexed_shades_with_selected_palette() {
        let canvas =
            SpriteCanvas::from_rgba(2, 1, vec![0x00, 0x00, 0x00, 0xFF, 0xAA, 0xAA, 0xAA, 0x80])
                .unwrap();
        let palette = Palette4::new([
            [10, 20, 30, 255],
            [40, 50, 60, 255],
            [70, 80, 90, 255],
            [100, 110, 120, 255],
        ]);

        let pixels = canvas_display_pixels(&canvas, ColorMode::PaletteIndexed, Some(palette));

        assert_eq!(pixels, vec![10, 20, 30, 255, 70, 80, 90, 128]);
    }

    #[test]
    fn canvas_display_pixels_leaves_noncanonical_indexed_pixels_unchanged() {
        let canvas = SpriteCanvas::filled(1, 1, PixelColor::rgb(12, 34, 56));
        let palette = Palette4::new([[1, 2, 3, 255]; 4]);

        let pixels = canvas_display_pixels(&canvas, ColorMode::PaletteIndexed, Some(palette));

        assert_eq!(pixels, canvas.pixels());
    }

    #[test]
    fn floating_preview_color_keeps_full_alpha_for_lifted_selection() {
        let color = PixelColor::new(10, 20, 30, 200);
        let origin = FloatingOrigin::SelectionLift {
            selection_before_float: SelectionMask::new(1, 1),
        };

        assert_eq!(
            floating_preview_color(color, &origin),
            egui::Color32::from_rgba_unmultiplied(10, 20, 30, 200)
        );
    }

    #[test]
    fn floating_preview_color_reduces_alpha_for_paste_preview() {
        let color = PixelColor::new(10, 20, 30, 200);
        let origin = FloatingOrigin::PastePreview {
            selection_before_float: None,
        };

        assert_eq!(
            floating_preview_color(color, &origin),
            egui::Color32::from_rgba_unmultiplied(10, 20, 30, 125)
        );
    }
}
