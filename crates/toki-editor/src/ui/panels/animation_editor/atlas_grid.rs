//! Atlas grid for frame selection.

use crate::ui::EditorUI;

pub fn render_atlas_grid(ui: &mut egui::Ui, ui_state: &mut EditorUI, ctx: &egui::Context) {
    let atlas_name = ui_state.animation.authoring.atlas_name.clone();
    if atlas_name.is_empty() {
        ui.label("No atlas selected. Select an atlas first.");

        // Atlas selector
        ui.horizontal(|ui| {
            ui.label("Atlas:");
            let mut atlas = ui_state.animation.authoring.atlas_name.clone();
            if ui.text_edit_singleline(&mut atlas).changed() {
                ui_state.animation.authoring.atlas_name = atlas;
                ui_state.animation.authoring.dirty = true;
            }
        });
        return;
    }

    // Ensure atlas texture is loaded
    ensure_atlas_texture(ui_state, ctx);

    let Some(texture) = &ui_state.animation.atlas_texture else {
        ui.label("Loading atlas...");
        return;
    };

    let Some((cell_w, cell_h)) = ui_state.animation.atlas_cell_size else {
        ui.label("Atlas metadata missing");
        return;
    };

    let Some((img_w, img_h)) = ui_state.animation.atlas_image_size else {
        ui.label("Atlas image not loaded");
        return;
    };

    // Calculate grid dimensions
    let cols = img_w / cell_w;
    let rows = img_h / cell_h;
    ui_state.animation.atlas_grid_size = Some((cols, rows));

    let has_selected_clip = ui_state.animation.authoring.selected_clip_index.is_some();

    // Zoom controls
    ui.horizontal(|ui| {
        ui.label("Zoom:");
        if ui.button("-").clicked() {
            ui_state.animation.atlas_viewport.zoom_out();
        }
        ui.label(format!("{:.1}x", ui_state.animation.atlas_viewport.zoom));
        if ui.button("+").clicked() {
            ui_state.animation.atlas_viewport.zoom_in();
        }
        ui.separator();
        ui.label("Click cells to add frames to clip");
    });

    // Allocate viewport area - use all remaining space
    let available_size = ui.available_size();
    let viewport_height = available_size.y.max(100.0);
    let viewport_width = available_size.x.max(100.0);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(viewport_width, viewport_height),
        egui::Sense::click_and_drag(),
    );

    // Handle pan with drag
    if response.dragged_by(egui::PointerButton::Primary)
        || response.dragged_by(egui::PointerButton::Middle)
    {
        let delta = response.drag_delta();
        ui_state
            .animation
            .atlas_viewport
            .pan_by(glam::Vec2::new(delta.x, delta.y));
    }

    // Handle scroll zoom
    if response.hovered() {
        let scroll_delta = ui.input(|input| input.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            if scroll_delta > 0.0 {
                ui_state.animation.atlas_viewport.zoom_in();
            } else {
                ui_state.animation.atlas_viewport.zoom_out();
            }
        }

        // Handle +/- keys for zoom
        if ui.input(|i| i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)) {
            ui_state.animation.atlas_viewport.zoom_in();
        }
        if ui.input(|i| i.key_pressed(egui::Key::Minus)) {
            ui_state.animation.atlas_viewport.zoom_out();
        }
    }

    // Update cursor position
    let cursor_canvas_pos = response.hover_pos().map(|hover_pos| {
        let canvas_pos = ui_state
            .animation
            .atlas_viewport
            .screen_to_canvas(glam::Vec2::new(hover_pos.x, hover_pos.y), rect);
        glam::IVec2::new(canvas_pos.x.floor() as i32, canvas_pos.y.floor() as i32)
    });
    ui_state.animation.atlas_viewport.cursor_canvas_pos = cursor_canvas_pos;

    // Draw background
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, egui::Color32::from_gray(40));

    // Draw atlas texture
    let zoom = ui_state.animation.atlas_viewport.zoom;
    let pan = ui_state.animation.atlas_viewport.pan;

    let canvas_screen_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));
    let canvas_screen_max = egui::pos2(
        canvas_screen_min.x + img_w as f32 * zoom,
        canvas_screen_min.y + img_h as f32 * zoom,
    );
    let canvas_screen_rect = egui::Rect::from_min_max(canvas_screen_min, canvas_screen_max);

    // Draw the atlas image
    painter.image(
        texture.id(),
        canvas_screen_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );

    // Draw cell grid overlay
    draw_cell_grid(
        &painter,
        rect,
        canvas_screen_min,
        canvas_screen_max,
        cell_w,
        cell_h,
        cols,
        rows,
        zoom,
    );

    // Highlight cells that are in the current clip
    highlight_clip_frames(
        ui_state,
        &painter,
        canvas_screen_min,
        cell_w,
        cell_h,
        cols,
        rows,
        zoom,
    );

    // Highlight hovered cell and handle clicks
    let add_frame = handle_cell_interaction(
        ui_state,
        &painter,
        &response,
        cursor_canvas_pos,
        canvas_screen_min,
        cell_w,
        cell_h,
        cols,
        rows,
        zoom,
        has_selected_clip,
    );

    // Apply deferred add
    if let Some((col, row)) = add_frame {
        ui_state.animation.authoring.add_frame_to_selected(col, row);
    }

    // Draw canvas border
    painter.rect_stroke(
        canvas_screen_rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
        egui::StrokeKind::Outside,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_cell_grid(
    painter: &egui::Painter,
    rect: egui::Rect,
    canvas_screen_min: egui::Pos2,
    canvas_screen_max: egui::Pos2,
    cell_w: u32,
    cell_h: u32,
    cols: u32,
    rows: u32,
    zoom: f32,
) {
    let grid_stroke = egui::Stroke::new(
        1.0,
        egui::Color32::from_rgba_unmultiplied(255, 200, 50, 150),
    );

    // Vertical lines
    for col in 0..=cols {
        let x = canvas_screen_min.x + (col * cell_w) as f32 * zoom;
        if x >= rect.left() && x <= rect.right() {
            painter.line_segment(
                [
                    egui::pos2(x, rect.top().max(canvas_screen_min.y)),
                    egui::pos2(x, rect.bottom().min(canvas_screen_max.y)),
                ],
                grid_stroke,
            );
        }
    }

    // Horizontal lines
    for row in 0..=rows {
        let y = canvas_screen_min.y + (row * cell_h) as f32 * zoom;
        if y >= rect.top() && y <= rect.bottom() {
            painter.line_segment(
                [
                    egui::pos2(rect.left().max(canvas_screen_min.x), y),
                    egui::pos2(rect.right().min(canvas_screen_max.x), y),
                ],
                grid_stroke,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn highlight_clip_frames(
    ui_state: &EditorUI,
    painter: &egui::Painter,
    canvas_screen_min: egui::Pos2,
    cell_w: u32,
    cell_h: u32,
    cols: u32,
    rows: u32,
    zoom: f32,
) {
    if let Some(clip) = ui_state.animation.authoring.selected_clip() {
        for frame in &clip.frames {
            let col = frame.position[0];
            let row = frame.position[1];
            if col < cols && row < rows {
                let cell_min = egui::pos2(
                    canvas_screen_min.x + (col * cell_w) as f32 * zoom,
                    canvas_screen_min.y + (row * cell_h) as f32 * zoom,
                );
                let cell_max = egui::pos2(
                    cell_min.x + cell_w as f32 * zoom,
                    cell_min.y + cell_h as f32 * zoom,
                );
                let cell_rect = egui::Rect::from_min_max(cell_min, cell_max);
                painter.rect_filled(
                    cell_rect,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(100, 200, 100, 80),
                );
            }
        }
    }
}

#[expect(clippy::too_many_arguments)]
fn handle_cell_interaction(
    _ui_state: &EditorUI,
    painter: &egui::Painter,
    response: &egui::Response,
    cursor_canvas_pos: Option<glam::IVec2>,
    canvas_screen_min: egui::Pos2,
    cell_w: u32,
    cell_h: u32,
    cols: u32,
    rows: u32,
    zoom: f32,
    has_selected_clip: bool,
) -> Option<(u32, u32)> {
    let mut add_frame: Option<(u32, u32)> = None;

    if let Some(canvas_pos) = cursor_canvas_pos {
        if canvas_pos.x >= 0 && canvas_pos.y >= 0 {
            let col = (canvas_pos.x as u32) / cell_w;
            let row = (canvas_pos.y as u32) / cell_h;
            if col < cols && row < rows {
                let cell_min = egui::pos2(
                    canvas_screen_min.x + (col * cell_w) as f32 * zoom,
                    canvas_screen_min.y + (row * cell_h) as f32 * zoom,
                );
                let cell_max = egui::pos2(
                    cell_min.x + cell_w as f32 * zoom,
                    cell_min.y + cell_h as f32 * zoom,
                );
                let cell_rect = egui::Rect::from_min_max(cell_min, cell_max);

                // Highlight on hover
                if has_selected_clip {
                    painter.rect_stroke(
                        cell_rect,
                        0.0,
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                        egui::StrokeKind::Inside,
                    );
                }

                // Handle click to add frame
                if response.clicked() && has_selected_clip {
                    add_frame = Some((col, row));
                }
            }
        }
    }

    add_frame
}

/// Ensure the atlas texture is loaded
fn ensure_atlas_texture(ui_state: &mut EditorUI, ctx: &egui::Context) {
    // Check if already loaded
    if ui_state.animation.atlas_texture.is_some() {
        return;
    }

    let Some(png_path) = &ui_state.animation.atlas_texture_path else {
        return;
    };

    // Load the PNG
    let Ok(decoded) = toki_core::graphics::image::load_image_rgba8(png_path) else {
        tracing::error!("Failed to load atlas PNG: {:?}", png_path);
        return;
    };

    // Store image dimensions
    ui_state.animation.atlas_image_size = Some((decoded.width, decoded.height));

    // Create texture
    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [decoded.width as usize, decoded.height as usize],
        &decoded.data,
    );

    let texture = ctx.load_texture(
        "animation_editor_atlas",
        color_image,
        egui::TextureOptions::NEAREST,
    );
    ui_state.animation.atlas_texture = Some(texture);

    tracing::info!("Loaded atlas texture: {}x{}", decoded.width, decoded.height);
}
