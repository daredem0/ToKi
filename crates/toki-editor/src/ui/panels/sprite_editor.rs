use crate::ui::editor_ui::{SpriteCanvas, SpriteCanvasViewport, SpriteEditorTool};
use crate::ui::EditorUI;

/// Renders the sprite editor panel
pub fn render_sprite_editor(ui: &mut egui::Ui, ui_state: &mut EditorUI, ctx: &egui::Context) {
    // Handle new canvas dialog
    if ui_state.sprite.show_new_canvas_dialog {
        render_new_canvas_dialog(ui_state, ctx);
    }

    // Toolbar (simplified - tools are in inspector panel)
    render_toolbar(ui, ui_state);
    ui.separator();

    // Main content area
    if ui_state.sprite.has_canvas() {
        render_canvas_viewport(ui, ui_state, ctx);
    } else {
        render_no_canvas_message(ui, ui_state);
    }
}

fn render_toolbar(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.horizontal(|ui| {
        ui.heading("Sprite Editor");
        ui.separator();

        if ui.button("New Canvas").clicked() {
            ui_state.begin_new_sprite_canvas_dialog();
        }

        if ui_state.sprite.has_canvas() && ui_state.sprite.dirty {
            ui.label("Unsaved changes");
        }
    });

    // Show current tool (like map editor)
    if ui_state.sprite.has_canvas() {
        ui.horizontal(|ui| {
            ui.label("Tool:");
            ui.label(tool_label(ui_state.sprite.tool));
        });
    }
}

fn tool_label(tool: SpriteEditorTool) -> &'static str {
    match tool {
        SpriteEditorTool::Drag => "Drag",
        SpriteEditorTool::Brush => "Brush",
        SpriteEditorTool::Eraser => "Eraser",
        SpriteEditorTool::Fill => "Fill",
        SpriteEditorTool::Eyedropper => "Eyedropper",
        SpriteEditorTool::Select => "Select",
        SpriteEditorTool::Line => "Line",
    }
}

fn render_new_canvas_dialog(ui_state: &mut EditorUI, ctx: &egui::Context) {
    egui::Window::new("New Canvas")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Width:");
                ui.add(
                    egui::DragValue::new(&mut ui_state.sprite.new_canvas_width)
                        .range(1..=2048)
                        .speed(1),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Height:");
                ui.add(
                    egui::DragValue::new(&mut ui_state.sprite.new_canvas_height)
                        .range(1..=2048)
                        .speed(1),
                );
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Create").clicked() {
                    ui_state.submit_new_sprite_canvas();
                }
                if ui.button("Cancel").clicked() {
                    ui_state.cancel_new_sprite_canvas_dialog();
                }
            });
        });
}

fn render_no_canvas_message(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.label("No canvas open");
            ui.add_space(10.0);
            if ui.button("Create New Canvas").clicked() {
                ui_state.begin_new_sprite_canvas_dialog();
            }
        });
    });
}

fn render_canvas_viewport(ui: &mut egui::Ui, ui_state: &mut EditorUI, ctx: &egui::Context) {
    let available_size = ui.available_size();

    // Allocate the viewport area
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(available_size.x, available_size.y - 24.0), // Reserve space for status bar
        egui::Sense::click_and_drag(),
    );

    // Handle pan with right-click drag or middle-click drag
    if response.dragged_by(egui::PointerButton::Secondary)
        || response.dragged_by(egui::PointerButton::Middle)
    {
        let delta = response.drag_delta();
        ui_state
            .sprite
            .viewport
            .pan_by(glam::Vec2::new(delta.x, delta.y));
    }

    // Handle scroll zoom
    if response.hovered() {
        let scroll_delta = ui.input(|input| input.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            if scroll_delta > 0.0 {
                ui_state.sprite.viewport.zoom_in();
            } else {
                ui_state.sprite.viewport.zoom_out();
            }
        }
    }

    // Handle keyboard zoom (+/- keys)
    if !ui.ctx().wants_keyboard_input() {
        if ui.input(|input| {
            input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals)
        }) {
            ui_state.sprite.viewport.zoom_in();
        }
        if ui.input(|input| input.key_pressed(egui::Key::Minus)) {
            ui_state.sprite.viewport.zoom_out();
        }
    }

    // Update cursor position
    if let Some(hover_pos) = response.hover_pos() {
        let canvas_pos = ui_state
            .sprite
            .viewport
            .screen_to_canvas(glam::Vec2::new(hover_pos.x, hover_pos.y), rect);
        ui_state.sprite.cursor_canvas_pos = Some(glam::IVec2::new(
            canvas_pos.x.floor() as i32,
            canvas_pos.y.floor() as i32,
        ));
    } else {
        ui_state.sprite.cursor_canvas_pos = None;
    }

    // Handle tool interactions
    handle_tool_interaction(ui_state, &response, rect, ctx);

    // Draw canvas background
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, egui::Color32::from_gray(40));

    // Ensure canvas texture is created before drawing
    if ui_state.sprite.canvas.is_some() {
        ensure_canvas_texture(ui_state, ctx);
    }

    // Draw checkerboard transparency pattern and canvas
    if ui_state.sprite.has_canvas() {
        let viewport = ui_state.sprite.viewport.clone();
        let canvas = ui_state.sprite.canvas.as_ref().unwrap();
        let texture = ui_state.sprite.canvas_texture.as_ref();
        draw_canvas_with_checkerboard(&painter, rect, &viewport, canvas, texture);
    }

    // Draw pixel grid overlay
    if ui_state.sprite.show_grid && ui_state.sprite.viewport.zoom >= 4.0 {
        if let Some(canvas) = &ui_state.sprite.canvas {
            draw_pixel_grid(&painter, rect, &ui_state.sprite.viewport, canvas);
        }
    }

    // Draw selection rectangle
    if let Some(selection) = &ui_state.sprite.selection {
        draw_selection_rect(&painter, rect, &ui_state.sprite.viewport, selection);
    }

    // Status bar
    render_status_bar(ui, ui_state);
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

    // Calculate canvas screen rect
    let canvas_screen_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));
    let canvas_screen_max = egui::pos2(
        canvas_screen_min.x + canvas.width as f32 * zoom,
        canvas_screen_min.y + canvas.height as f32 * zoom,
    );
    let canvas_screen_rect = egui::Rect::from_min_max(canvas_screen_min, canvas_screen_max);

    // Clip to viewport
    let visible_rect = canvas_screen_rect.intersect(rect);
    if visible_rect.is_positive() {
        // Draw checkerboard pattern for transparency
        draw_checkerboard(painter, visible_rect, zoom);

        // Draw canvas texture
        if let Some(tex) = texture {
            painter.image(
                tex.id(),
                canvas_screen_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
    }

    // Draw canvas border
    painter.rect_stroke(
        canvas_screen_rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
        egui::StrokeKind::Outside,
    );
}

fn draw_checkerboard(painter: &egui::Painter, rect: egui::Rect, zoom: f32) {
    // Draw a simple checkerboard pattern
    let check_size = (8.0 * (zoom / 8.0).max(1.0)).min(16.0);
    let color1 = egui::Color32::from_gray(180);
    let color2 = egui::Color32::from_gray(220);

    let start_x = rect.left();
    let start_y = rect.top();
    let end_x = rect.right();
    let end_y = rect.bottom();

    let mut y = start_y;
    let mut row = 0;
    while y < end_y {
        let mut x = start_x;
        let mut col = 0;
        while x < end_x {
            let color = if (row + col) % 2 == 0 { color1 } else { color2 };
            let check_rect = egui::Rect::from_min_size(
                egui::pos2(x, y),
                egui::vec2(check_size.min(end_x - x), check_size.min(end_y - y)),
            );
            painter.rect_filled(check_rect, 0.0, color);
            x += check_size;
            col += 1;
        }
        y += check_size;
        row += 1;
    }
}

fn ensure_canvas_texture(ui_state: &mut EditorUI, ctx: &egui::Context) {
    // Check if we already have a valid texture
    if ui_state.sprite.canvas_texture.is_some() {
        return;
    }

    let Some(canvas) = &ui_state.sprite.canvas else {
        return;
    };

    // Create texture from canvas pixels
    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [canvas.width as usize, canvas.height as usize],
        canvas.pixels(),
    );

    let texture = ctx.load_texture(
        "sprite_editor_canvas",
        color_image,
        egui::TextureOptions::NEAREST,
    );

    ui_state.sprite.canvas_texture = Some(texture);
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

    let stroke = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(40));

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

fn draw_selection_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    selection: &crate::ui::editor_ui::SpriteSelection,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;

    let canvas_screen_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));

    // Calculate selection screen rect
    let sel_min = egui::pos2(
        canvas_screen_min.x + selection.x as f32 * zoom,
        canvas_screen_min.y + selection.y as f32 * zoom,
    );
    let sel_max = egui::pos2(
        sel_min.x + selection.width as f32 * zoom,
        sel_min.y + selection.height as f32 * zoom,
    );
    let sel_rect = egui::Rect::from_min_max(sel_min, sel_max);

    // Draw selection with dashed border and semi-transparent fill
    let fill = egui::Color32::from_rgba_unmultiplied(100, 150, 255, 50);
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 150, 255));

    painter.rect_filled(sel_rect, 0.0, fill);
    painter.rect_stroke(sel_rect, 0.0, stroke, egui::StrokeKind::Outside);
}

fn render_status_bar(ui: &mut egui::Ui, ui_state: &EditorUI) {
    ui.horizontal(|ui| {
        // Cursor position
        if let Some(pos) = ui_state.sprite.cursor_canvas_pos {
            ui.label(format!("Cursor: {}, {}", pos.x, pos.y));
        } else {
            ui.label("Cursor: -, -");
        }

        ui.separator();

        // Canvas dimensions
        if let Some((w, h)) = ui_state.sprite.canvas_dimensions() {
            ui.label(format!("Canvas: {}x{}", w, h));
        }

        ui.separator();

        // Zoom level
        ui.label(format!("Zoom: {}x", ui_state.sprite.viewport.zoom as i32));

        ui.separator();

        // Dirty indicator
        if ui_state.sprite.dirty {
            ui.label("*Modified");
        }
    });
}

fn handle_tool_interaction(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    _rect: egui::Rect,
    _ctx: &egui::Context,
) {
    use crate::ui::editor_ui::SpriteEditorTool;

    let Some(canvas_pos) = ui_state.sprite.cursor_canvas_pos else {
        return;
    };

    match ui_state.sprite.tool {
        SpriteEditorTool::Drag => handle_drag_tool(ui_state, response),
        SpriteEditorTool::Brush => handle_brush_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Eraser => handle_eraser_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Fill => handle_fill_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Eyedropper => handle_eyedropper_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Line => handle_line_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Select => handle_select_tool(ui_state, response, canvas_pos),
    }
}

fn handle_drag_tool(ui_state: &mut EditorUI, response: &egui::Response) {
    // Primary drag for panning (same as secondary/middle)
    if response.dragged_by(egui::PointerButton::Primary) {
        let delta = response.drag_delta();
        ui_state
            .sprite
            .viewport
            .pan_by(glam::Vec2::new(delta.x, delta.y));
    }
}

fn handle_brush_tool(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    canvas_pos: glam::IVec2,
) {
    use crate::ui::interactions::SpritePaintInteraction;

    if response.drag_started_by(egui::PointerButton::Primary) {
        start_paint_stroke(ui_state);
    }

    if response.dragged_by(egui::PointerButton::Primary) || response.clicked() {
        if let Some(canvas) = &mut ui_state.sprite.canvas {
            let color = ui_state.sprite.foreground_color;
            let brush_size = ui_state.sprite.brush_size;
            if SpritePaintInteraction::paint_brush(canvas, canvas_pos, color, brush_size) {
                ui_state.sprite.dirty = true;
                invalidate_canvas_texture(ui_state);
            }
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        finish_paint_stroke(ui_state);
    }
}

fn handle_eraser_tool(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    canvas_pos: glam::IVec2,
) {
    use crate::ui::interactions::SpritePaintInteraction;

    if response.drag_started_by(egui::PointerButton::Primary) {
        start_paint_stroke(ui_state);
    }

    if response.dragged_by(egui::PointerButton::Primary) || response.clicked() {
        if let Some(canvas) = &mut ui_state.sprite.canvas {
            let brush_size = ui_state.sprite.brush_size;
            if SpritePaintInteraction::erase_brush(canvas, canvas_pos, brush_size) {
                ui_state.sprite.dirty = true;
                invalidate_canvas_texture(ui_state);
            }
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        finish_paint_stroke(ui_state);
    }
}

fn handle_fill_tool(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    canvas_pos: glam::IVec2,
) {
    use crate::ui::interactions::SpritePaintInteraction;

    if response.clicked() {
        start_paint_stroke(ui_state);
        if let Some(canvas) = &mut ui_state.sprite.canvas {
            let color = ui_state.sprite.foreground_color;
            if SpritePaintInteraction::flood_fill(canvas, canvas_pos, color) {
                ui_state.sprite.dirty = true;
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
    use crate::ui::interactions::SpritePaintInteraction;

    if response.clicked() {
        if let Some(canvas) = &ui_state.sprite.canvas {
            if let Some(color) = SpritePaintInteraction::pick_color(canvas, canvas_pos) {
                ui_state.sprite.foreground_color = color;
                ui_state.sprite.add_recent_color(color);
            }
        }
    }
}

fn handle_line_tool(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    canvas_pos: glam::IVec2,
) {
    use crate::ui::interactions::SpritePaintInteraction;

    if response.drag_started_by(egui::PointerButton::Primary) {
        ui_state.sprite.line_start_pos = Some(canvas_pos);
        start_paint_stroke(ui_state);
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        if let Some(start) = ui_state.sprite.line_start_pos.take() {
            if let Some(canvas) = &mut ui_state.sprite.canvas {
                let color = ui_state.sprite.foreground_color;
                let brush_size = ui_state.sprite.brush_size;
                if SpritePaintInteraction::draw_line(canvas, start, canvas_pos, color, brush_size) {
                    ui_state.sprite.dirty = true;
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
    canvas_pos: glam::IVec2,
) {

    if response.drag_started_by(egui::PointerButton::Primary) {
        ui_state.sprite.selection_start_pos = Some(canvas_pos);
        ui_state.sprite.selection = None;
    }

    if response.dragged_by(egui::PointerButton::Primary) {
        if let Some(start) = ui_state.sprite.selection_start_pos {
            ui_state.sprite.selection = Some(create_selection(start, canvas_pos));
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        if let Some(start) = ui_state.sprite.selection_start_pos.take() {
            let selection = create_selection(start, canvas_pos);
            // Only keep selection if it has non-zero size
            if selection.width > 0 && selection.height > 0 {
                ui_state.sprite.selection = Some(selection);
            } else {
                ui_state.sprite.selection = None;
            }
        }
    }

    // Clear selection with right-click
    if response.clicked_by(egui::PointerButton::Secondary) {
        ui_state.sprite.selection = None;
    }
}

fn create_selection(start: glam::IVec2, end: glam::IVec2) -> crate::ui::editor_ui::SpriteSelection {
    let x = start.x.min(end.x).max(0) as u32;
    let y = start.y.min(end.y).max(0) as u32;
    let w = (start.x - end.x).unsigned_abs();
    let h = (start.y - end.y).unsigned_abs();
    crate::ui::editor_ui::SpriteSelection::new(x, y, w, h)
}

fn start_paint_stroke(ui_state: &mut EditorUI) {
    if !ui_state.sprite.is_painting {
        ui_state.sprite.is_painting = true;
        ui_state.sprite.canvas_before_stroke = ui_state.sprite.canvas.clone();
    }
}

fn finish_paint_stroke(ui_state: &mut EditorUI) {
    if ui_state.sprite.is_painting {
        ui_state.sprite.is_painting = false;
        if let Some(before) = ui_state.sprite.canvas_before_stroke.take() {
            ui_state.sprite.push_undo_state(before);
        }
        // Add the used color to recent colors
        ui_state.sprite.add_recent_color(ui_state.sprite.foreground_color);
    }
}

fn invalidate_canvas_texture(ui_state: &mut EditorUI) {
    ui_state.sprite.canvas_texture = None;
}
