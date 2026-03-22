//! Animation preview area and controls.

use crate::ui::EditorUI;

pub fn render_preview_controls(ui: &mut egui::Ui, ui_state: &mut EditorUI, ctx: &egui::Context) {
    ui.horizontal(|ui| {
        ui.heading("Preview");

        ui.separator();

        let has_clip = ui_state.animation.selected_clip().is_some();
        let has_frames = ui_state.animation.frame_count() > 0;
        let can_play = has_clip && has_frames;

        // Play/Pause button
        let play_label = if ui_state.animation.is_playing() {
            "Pause"
        } else {
            "Play"
        };
        if ui
            .add_enabled(can_play, egui::Button::new(play_label))
            .clicked()
        {
            ui_state.animation.preview.toggle_playback();
        }

        // Stop button
        if ui
            .add_enabled(can_play, egui::Button::new("Stop"))
            .clicked()
        {
            ui_state.animation.preview.stop();
        }

        // Step buttons
        if ui
            .add_enabled(can_play, egui::Button::new("|<"))
            .on_hover_text("Previous frame")
            .clicked()
        {
            ui_state
                .animation
                .preview
                .step_backward(ui_state.animation.frame_count());
        }

        if ui
            .add_enabled(can_play, egui::Button::new(">|"))
            .on_hover_text("Next frame")
            .clicked()
        {
            ui_state
                .animation
                .preview
                .step_forward(ui_state.animation.frame_count());
        }

        ui.separator();

        // Speed control with preset buttons
        ui.label("Speed:");
        let current_speed = ui_state.animation.preview.speed();
        for (label, speed_val) in [("0.5x", 0.5), ("1x", 1.0), ("2x", 2.0)] {
            let selected = (current_speed - speed_val).abs() < 0.01;
            if ui.selectable_label(selected, label).clicked() {
                ui_state.animation.preview.set_speed(speed_val);
            }
        }

        // Frame counter
        if has_frames {
            ui.separator();
            ui.label(format!(
                "Frame: {} / {}",
                ui_state.animation.preview.current_frame() + 1,
                ui_state.animation.frame_count()
            ));
        }
    });

    // Frame scrubbing slider
    let frame_count = ui_state.animation.frame_count();
    if frame_count > 1 {
        ui.horizontal(|ui| {
            ui.label("Scrub:");
            let mut current = ui_state.animation.preview.current_frame();
            let label = format!("{}/{}", current + 1, frame_count);
            let slider = egui::Slider::new(&mut current, 0..=(frame_count - 1))
                .show_value(false)
                .text(label);
            if ui.add(slider).changed() {
                ui_state.animation.preview.go_to_frame(current, frame_count);
            }
        });
    }

    // Update playback if playing
    if ui_state.animation.is_playing() {
        if let Some(clip) = ui_state.animation.selected_clip().cloned() {
            let delta = ctx.input(|i| i.stable_dt);
            ui_state.animation.preview.update(delta, &clip);
            ctx.request_repaint();
        }
    }
}

pub fn render_preview_area(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(clip) = ui_state.animation.selected_clip().cloned() else {
        ui.centered_and_justified(|ui| {
            ui.label("Select a clip to preview");
        });
        return;
    };

    if clip.frames.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("No frames in clip");
        });
        return;
    }

    let current_frame_idx = ui_state
        .animation
        .preview
        .current_frame()
        .min(clip.frames.len() - 1);
    let frame = &clip.frames[current_frame_idx];
    let frame_duration = clip.frame_duration_at(current_frame_idx);
    let frame_progress = ui_state.animation.preview.frame_progress(&clip);

    // Get available space for the preview
    let available = ui.available_size();

    // Display frame info and timing on same line
    ui.horizontal(|ui| {
        ui.label(format!(
            "Frame {} [{},{}]",
            current_frame_idx + 1,
            frame.position[0],
            frame.position[1]
        ));
        ui.separator();
        ui.label(format!("{}ms", frame_duration as u32));
        if frame.duration_ms.is_some() {
            ui.label("(custom)");
        }

        // Frame progress bar (compact, only when playing)
        if ui_state.animation.is_playing() {
            let progress_bar = egui::ProgressBar::new(frame_progress).desired_width(60.0);
            ui.add(progress_bar);
        }
    });

    ui.add_space(4.0);

    // Calculate sprite size to fill available space while maintaining aspect ratio
    let remaining_height = (available.y - 30.0).max(32.0); // Reserve space for info line
    let remaining_width = available.x;

    // Try to render actual sprite frame from atlas texture
    let rendered = render_sprite_frame_scaled(
        ui,
        ui_state,
        frame.position,
        remaining_width,
        remaining_height,
    );

    if !rendered {
        // Fallback to placeholder if texture not available
        let size = egui::vec2(remaining_width.min(128.0), remaining_height.min(128.0));
        ui.centered_and_justified(|ui| {
            let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());

            ui.painter()
                .rect_filled(rect, 4.0, egui::Color32::from_rgb(60, 60, 80));
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("[{},{}]", frame.position[0], frame.position[1]),
                egui::FontId::default(),
                egui::Color32::WHITE,
            );
        });
    }
}

/// Render a sprite frame scaled to fit within given dimensions. Returns true if rendered successfully.
fn render_sprite_frame_scaled(
    ui: &mut egui::Ui,
    ui_state: &EditorUI,
    position: [u32; 2],
    max_width: f32,
    max_height: f32,
) -> bool {
    let Some(texture) = &ui_state.animation.atlas_texture else {
        return false;
    };
    let Some((cell_w, cell_h)) = ui_state.animation.atlas_cell_size else {
        return false;
    };
    let Some((img_w, img_h)) = ui_state.animation.atlas_image_size else {
        return false;
    };

    // Calculate UV coordinates for this cell
    let col = position[0];
    let row = position[1];

    let u_min = (col * cell_w) as f32 / img_w as f32;
    let v_min = (row * cell_h) as f32 / img_h as f32;
    let u_max = ((col + 1) * cell_w) as f32 / img_w as f32;
    let v_max = ((row + 1) * cell_h) as f32 / img_h as f32;

    let uv_rect = egui::Rect::from_min_max(egui::pos2(u_min, v_min), egui::pos2(u_max, v_max));

    // Calculate scale to fit within available space while maintaining aspect ratio
    let cell_aspect = cell_w as f32 / cell_h as f32;
    let available_aspect = max_width / max_height;

    let scale = if cell_aspect > available_aspect {
        // Cell is wider, constrain by width
        max_width / cell_w as f32
    } else {
        // Cell is taller, constrain by height
        max_height / cell_h as f32
    };

    let display_size = egui::vec2(cell_w as f32 * scale, cell_h as f32 * scale);

    // Center the sprite in the available space
    ui.vertical_centered(|ui| {
        let (rect, _response) = ui.allocate_exact_size(display_size, egui::Sense::hover());

        // Draw checkered background for transparency
        draw_checkered_background(ui.painter(), rect);

        // Draw the sprite
        ui.painter()
            .image(texture.id(), rect, uv_rect, egui::Color32::WHITE);
    });

    true
}

/// Draw a checkered background pattern for transparency visualization
fn draw_checkered_background(painter: &egui::Painter, rect: egui::Rect) {
    let check_size = 8.0;
    let light = egui::Color32::from_gray(200);
    let dark = egui::Color32::from_gray(150);

    let cols = ((rect.width() / check_size).ceil() as usize).max(1);
    let rows = ((rect.height() / check_size).ceil() as usize).max(1);

    for row in 0..rows {
        for col in 0..cols {
            let color = if (row + col) % 2 == 0 { light } else { dark };
            let x = rect.left() + col as f32 * check_size;
            let y = rect.top() + row as f32 * check_size;
            let cell_rect =
                egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(check_size, check_size))
                    .intersect(rect);
            painter.rect_filled(cell_rect, 0.0, color);
        }
    }
}
