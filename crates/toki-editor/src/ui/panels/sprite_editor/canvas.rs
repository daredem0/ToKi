//! Canvas rendering and drawing operations.

use crate::ui::editor_ui::{CanvasSide, SpriteCanvas, SpriteCanvasViewport, SpriteSelection};
use crate::ui::EditorUI;

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

    let render_side = target_side.unwrap_or(ui_state.sprite.active_canvas);

    if let Some(side) = target_side {
        if response.clicked() || response.dragged() {
            ui_state.sprite.set_active_canvas(side);
        }
    }

    let is_interactive =
        target_side.is_none() || target_side == Some(ui_state.sprite.active_canvas);

    // Handle pan with right-click drag or middle-click drag
    if response.dragged_by(egui::PointerButton::Secondary)
        || response.dragged_by(egui::PointerButton::Middle)
    {
        let delta = response.drag_delta();
        ui_state
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
                    .sprite
                    .canvas_state_mut(render_side)
                    .viewport
                    .zoom_in();
            } else {
                ui_state
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
            ui_state.sprite.active_mut().viewport.zoom_in();
        }
        if ui.input(|input| input.key_pressed(egui::Key::Minus)) {
            ui_state.sprite.active_mut().viewport.zoom_out();
        }

        handle_tool_shortcuts(ui_state, ui);
        handle_undo_redo_shortcuts(ui_state, ui);
    }

    // Update cursor position
    if let Some(hover_pos) = response.hover_pos() {
        let canvas_pos = ui_state
            .sprite
            .canvas_state(render_side)
            .viewport
            .screen_to_canvas(glam::Vec2::new(hover_pos.x, hover_pos.y), rect);
        ui_state
            .sprite
            .canvas_state_mut(render_side)
            .cursor_canvas_pos = Some(glam::IVec2::new(
            canvas_pos.x.floor() as i32,
            canvas_pos.y.floor() as i32,
        ));
    } else {
        ui_state
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
    let canvas_state = ui_state.sprite.canvas_state(render_side);
    if canvas_state.canvas.is_some() {
        ensure_canvas_texture_for_side(ui_state, ctx, render_side);
    }

    // Draw checkerboard and canvas
    let canvas_state = ui_state.sprite.canvas_state(render_side);
    if let Some(canvas) = &canvas_state.canvas {
        let viewport = canvas_state.viewport.clone();
        let texture = canvas_state.canvas_texture.as_ref();
        draw_canvas_with_checkerboard(&painter, rect, &viewport, canvas, texture);
    }

    // Draw pixel grid overlay
    let canvas_state = ui_state.sprite.canvas_state(render_side);
    if canvas_state.show_grid && canvas_state.viewport.zoom >= 4.0 {
        if let Some(canvas) = &canvas_state.canvas {
            draw_pixel_grid(&painter, rect, &canvas_state.viewport, canvas);
        }
    }

    // Draw cell grid overlay for sprite sheets
    let canvas_state = ui_state.sprite.canvas_state(render_side);
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
    let canvas_state = ui_state.sprite.canvas_state(render_side);
    if let Some(canvas) = &canvas_state.canvas {
        draw_hovered_pixel_highlight(
            &painter,
            rect,
            &canvas_state.viewport,
            canvas,
            canvas_state.cursor_canvas_pos,
        );
    }

    // Draw selection rectangle
    let canvas_state = ui_state.sprite.canvas_state(render_side);
    if let Some(selection) = &canvas_state.selection {
        draw_selection_rect(&painter, rect, &canvas_state.viewport, selection);
    }

    // Status bar
    if target_side.is_none() || target_side == Some(ui_state.sprite.active_canvas) {
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
                ui_state.sprite.set_active_canvas(side);
                ui_state.begin_new_sprite_canvas_dialog();
            }
            let load_enabled = sprites_dir.is_some();
            if ui
                .add_enabled(load_enabled, egui::Button::new("Load"))
                .clicked()
            {
                if let Some(dir) = sprites_dir {
                    ui_state.sprite.set_active_canvas(side);
                    ui_state.sprite.begin_load_dialog(dir);
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
    if ui_state.sprite.canvas_state(side).canvas_texture.is_some() {
        return;
    }

    let Some(canvas) = &ui_state.sprite.canvas_state(side).canvas else {
        return;
    };

    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [canvas.width as usize, canvas.height as usize],
        canvas.pixels(),
    );

    let texture_name = match side {
        CanvasSide::Left => "sprite_editor_canvas_left",
        CanvasSide::Right => "sprite_editor_canvas_right",
    };

    let texture = ctx.load_texture(texture_name, color_image, egui::TextureOptions::NEAREST);
    ui_state.sprite.canvas_state_mut(side).canvas_texture = Some(texture);
}

pub fn invalidate_canvas_texture(ui_state: &mut EditorUI) {
    ui_state.sprite.active_mut().canvas_texture = None;
}

pub fn invalidate_canvas_texture_for_side(ui_state: &mut EditorUI, side: CanvasSide) {
    ui_state.sprite.canvas_state_mut(side).canvas_texture = None;
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

fn draw_selection_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    selection: &SpriteSelection,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;

    let canvas_screen_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));

    let sel_min = egui::pos2(
        canvas_screen_min.x + selection.x as f32 * zoom,
        canvas_screen_min.y + selection.y as f32 * zoom,
    );
    let sel_max = egui::pos2(
        sel_min.x + selection.width as f32 * zoom,
        sel_min.y + selection.height as f32 * zoom,
    );
    let sel_rect = egui::Rect::from_min_max(sel_min, sel_max);

    let fill = egui::Color32::from_rgba_unmultiplied(100, 150, 255, 50);
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 150, 255));

    painter.rect_filled(sel_rect, 0.0, fill);
    painter.rect_stroke(sel_rect, 0.0, stroke, egui::StrokeKind::Outside);
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
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(120, 190, 255, 220));
    painter.rect_filled(pixel_rect, 0.0, fill);
    painter.rect_stroke(pixel_rect, 0.0, stroke, egui::StrokeKind::Inside);
}

fn render_status_bar(ui: &mut egui::Ui, ui_state: &EditorUI) {
    ui.horizontal(|ui| {
        if let Some(pos) = ui_state.sprite.active().cursor_canvas_pos {
            ui.label(format!("Cursor: {}, {}", pos.x, pos.y));
        } else {
            ui.label("Cursor: -, -");
        }

        ui.separator();

        if let Some((w, h)) = ui_state.sprite.canvas_dimensions() {
            ui.label(format!("Canvas: {}x{}", w, h));
        }

        ui.separator();

        ui.label(format!(
            "Zoom: {}x",
            ui_state.sprite.active().viewport.zoom as i32
        ));

        ui.separator();

        if ui_state.sprite.active().dirty {
            ui.label("*Modified");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hovered_pixel_screen_rect_returns_none_for_out_of_bounds_cursor() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 100.0));
        let viewport = SpriteCanvasViewport::default();
        let canvas = SpriteCanvas::new(8, 8);

        assert!(hovered_pixel_screen_rect(rect, &viewport, &canvas, Some(glam::IVec2::new(-1, 0))).is_none());
        assert!(hovered_pixel_screen_rect(rect, &viewport, &canvas, Some(glam::IVec2::new(8, 0))).is_none());
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
}
