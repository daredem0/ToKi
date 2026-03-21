// Animation editor panel
// Provides a dedicated tab for editing entity animations with preview
// Entities are loaded by selecting them in the hierarchy panel

use crate::project::Project;
use crate::ui::editor_ui::{AnimationAuthoringState, Selection};
use crate::ui::EditorUI;
use std::path::{Path, PathBuf};

/// Renders the animation editor panel
pub fn render_animation_editor(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    project: Option<&mut Project>,
) {
    let project_path = project.as_ref().map(|p| p.path.clone());

    // Check if an entity definition is selected and load it
    sync_with_selection(ui_state, project_path.as_deref());

    // Handle new clip dialog
    if ui_state.animation.show_new_clip_dialog {
        render_new_clip_dialog(ui_state, ctx);
    }

    // Toolbar
    render_toolbar(ui, ui_state);
    ui.separator();

    // Main content
    if ui_state.animation.has_entity() {
        render_editor_content(ui, ui_state, ctx);
    } else {
        render_no_entity_message(ui);
    }
}

/// Sync animation editor state with current selection
fn sync_with_selection(ui_state: &mut EditorUI, project_path: Option<&Path>) {
    let Some(project_path) = project_path else {
        return;
    };

    // Check if an EntityDefinition is selected
    let selected_entity = match &ui_state.selection {
        Some(Selection::EntityDefinition(name)) => Some(name.clone()),
        _ => None,
    };

    let Some(entity_name) = selected_entity else {
        return;
    };

    // Check if we already have this entity loaded
    if ui_state.animation.active_entity.as_ref() == Some(&entity_name) {
        return;
    }

    // Load the entity
    load_entity(ui_state, project_path, &entity_name);
}

fn render_toolbar(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.horizontal(|ui| {
        ui.heading("Animation Editor");

        if ui_state.animation.has_entity() {
            ui.separator();

            // Save button
            let is_dirty = ui_state.animation.authoring.dirty;
            if ui
                .add_enabled(is_dirty, egui::Button::new("Save"))
                .clicked()
            {
                save_current_entity(ui_state);
            }

            // Entity name label
            if let Some(name) = &ui_state.animation.active_entity {
                ui.separator();
                ui.label(format!("Entity: {}", name));
                if is_dirty {
                    ui.label("*");
                }
            }
        }
    });
}

fn render_no_entity_message(ui: &mut egui::Ui) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.label("No entity selected");
            ui.add_space(8.0);
            ui.label("Select an entity in the hierarchy panel to edit its animations.");
        });
    });
}

fn render_editor_content(ui: &mut egui::Ui, ui_state: &mut EditorUI, ctx: &egui::Context) {
    // Left: Clip list. Center: Atlas grid + Preview. Right: Frame sequence
    let available_width = ui.available_width();
    let available_height = ui.available_height();

    // Get panel widths from state
    let clip_list_width = ui_state.animation.clip_list_width;
    let frame_sequence_width = ui_state.animation.frame_sequence_width;
    let separator_width = 8.0; // Width of draggable separator
    let center_width =
        (available_width - clip_list_width - frame_sequence_width - separator_width * 2.0).max(200.0);

    ui.horizontal(|ui| {
        // Left panel: Clip list
        ui.allocate_ui_with_layout(
            egui::vec2(clip_list_width, available_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                render_clip_list(ui, ui_state);
            },
        );

        // Draggable separator between clip list and center
        let sep_response = render_vertical_separator(ui, available_height);
        if sep_response.dragged() {
            ui_state.animation.clip_list_width =
                (ui_state.animation.clip_list_width + sep_response.drag_delta().x)
                    .clamp(120.0, available_width * 0.4);
        }

        // Center panel: Atlas grid and preview
        ui.allocate_ui_with_layout(
            egui::vec2(center_width, available_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                render_center_panel(ui, ui_state, ctx);
            },
        );

        // Draggable separator between center and frame sequence
        let sep_response = render_vertical_separator(ui, available_height);
        if sep_response.dragged() {
            // Dragging right makes frame panel smaller
            ui_state.animation.frame_sequence_width =
                (ui_state.animation.frame_sequence_width - sep_response.drag_delta().x)
                    .clamp(150.0, available_width * 0.4);
        }

        // Right panel: Frame sequence
        ui.allocate_ui_with_layout(
            egui::vec2(frame_sequence_width, available_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                render_frame_sequence(ui, ui_state);
            },
        );
    });
}

/// Render a vertical draggable separator
fn render_vertical_separator(ui: &mut egui::Ui, height: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(8.0, height), egui::Sense::drag());

    let color = if response.dragged() {
        egui::Color32::from_gray(180)
    } else if response.hovered() {
        egui::Color32::from_gray(140)
    } else {
        egui::Color32::from_gray(80)
    };

    ui.painter().rect_filled(
        egui::Rect::from_center_size(rect.center(), egui::vec2(2.0, height)),
        0.0,
        color,
    );

    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }

    response
}

/// Render a horizontal draggable separator
fn render_horizontal_separator(ui: &mut egui::Ui, width: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 8.0), egui::Sense::drag());

    let color = if response.dragged() {
        egui::Color32::from_gray(180)
    } else if response.hovered() {
        egui::Color32::from_gray(140)
    } else {
        egui::Color32::from_gray(80)
    };

    ui.painter().rect_filled(
        egui::Rect::from_center_size(rect.center(), egui::vec2(width, 2.0)),
        0.0,
        color,
    );

    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }

    response
}

fn render_clip_list(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let available_height = ui.available_height();
    let available_width = ui.available_width();

    // Calculate heights based on ratio
    let header_height = 30.0; // Space for "Clips" heading
    let button_height = 30.0; // Space for "+ New Clip" button
    let bottom_section_min = 60.0; // Minimum height for default state section
    let separator_height = 8.0;

    let content_height = available_height - header_height - button_height - separator_height;
    let clip_list_height = (content_height * ui_state.animation.clip_list_ratio)
        .max(50.0)
        .min(content_height - bottom_section_min);
    let bottom_height = content_height - clip_list_height;

    ui.heading("Clips");

    // Add new clip button
    if ui.button("+ New Clip").clicked() {
        ui_state.animation.show_new_clip_dialog = true;
    }

    ui.add_space(4.0);

    // Collect clip info to avoid borrow issues
    let clip_info: Vec<_> = ui_state
        .animation
        .authoring
        .clips
        .iter()
        .enumerate()
        .map(|(idx, clip)| {
            let is_selected = ui_state.animation.authoring.selected_clip_index == Some(idx);
            let is_default = clip.state == ui_state.animation.authoring.default_state;
            (idx, clip.state.clone(), clip.frames.len(), is_selected, is_default)
        })
        .collect();

    let mut select_index: Option<usize> = None;
    let mut delete_index: Option<usize> = None;

    egui::ScrollArea::vertical()
        .id_salt("anim_clip_list")
        .auto_shrink([false, false])
        .max_height(clip_list_height)
        .show(ui, |ui| {
            for (idx, state, frame_count, is_selected, is_default) in &clip_info {
                ui.horizontal(|ui| {
                    let label = if *is_default {
                        format!("{} ({}) *", state, frame_count)
                    } else {
                        format!("{} ({})", state, frame_count)
                    };

                    if ui.selectable_label(*is_selected, &label).clicked() {
                        select_index = Some(*idx);
                    }

                    if ui.small_button("x").on_hover_text("Delete").clicked() {
                        delete_index = Some(*idx);
                    }
                });
            }
        });

    // Apply deferred actions
    if let Some(idx) = select_index {
        ui_state.animation.authoring.select_clip(idx);
        ui_state.animation.preview.stop();
    }

    if let Some(idx) = delete_index {
        ui_state.animation.authoring.delete_clip(idx);
    }

    // Draggable separator between clip list and default state
    let sep_response = render_horizontal_separator(ui, available_width);
    if sep_response.dragged() {
        let delta_ratio = sep_response.drag_delta().y / content_height;
        ui_state.animation.clip_list_ratio =
            (ui_state.animation.clip_list_ratio + delta_ratio).clamp(0.2, 0.9);
    }

    // Default state selector
    ui.allocate_ui_with_layout(
        egui::vec2(available_width, bottom_height),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            if !ui_state.animation.authoring.clips.is_empty() {
                ui.label("Default State:");
                let clip_states: Vec<String> = ui_state
                    .animation
                    .authoring
                    .clips
                    .iter()
                    .map(|c| c.state.clone())
                    .collect();

                let mut default_state = ui_state.animation.authoring.default_state.clone();
                egui::ComboBox::from_id_salt("anim_default_state")
                    .selected_text(&default_state)
                    .show_ui(ui, |ui| {
                        for state in &clip_states {
                            if ui
                                .selectable_value(&mut default_state, state.clone(), state)
                                .changed()
                            {
                                ui_state.animation.authoring.default_state = default_state.clone();
                                ui_state.animation.authoring.dirty = true;
                            }
                        }
                    });
            }
        },
    );
}

fn render_center_panel(ui: &mut egui::Ui, ui_state: &mut EditorUI, ctx: &egui::Context) {
    let available_width = ui.available_width();
    let available_height = ui.available_height();

    // Preview controls at top (fixed height)
    render_preview_controls(ui, ui_state, ctx);

    // Get preview height from state
    let preview_height = ui_state.animation.preview_height;

    // Preview area with stored height
    ui.group(|ui| {
        ui.set_min_height(preview_height);
        ui.set_max_height(preview_height);
        render_preview_area(ui, ui_state);
    });

    // Draggable separator between preview and atlas
    let sep_response = render_horizontal_separator(ui, available_width);
    if sep_response.dragged() {
        ui_state.animation.preview_height = (ui_state.animation.preview_height
            + sep_response.drag_delta().y)
            .clamp(100.0, available_height - 150.0);
    }

    // Atlas grid (for selecting frames)
    ui.heading("Atlas");

    // Use all remaining space for atlas grid
    render_atlas_grid(ui, ui_state, ctx);
}

fn render_preview_controls(ui: &mut egui::Ui, ui_state: &mut EditorUI, ctx: &egui::Context) {
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

fn render_preview_area(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
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

    let current_frame_idx = ui_state.animation.preview.current_frame().min(clip.frames.len() - 1);
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
            let progress_bar = egui::ProgressBar::new(frame_progress)
                .desired_width(60.0);
            ui.add(progress_bar);
        }
    });

    ui.add_space(4.0);

    // Calculate sprite size to fill available space while maintaining aspect ratio
    let remaining_height = (available.y - 30.0).max(32.0); // Reserve space for info line
    let remaining_width = available.x;

    // Try to render actual sprite frame from atlas texture
    let rendered = render_sprite_frame_scaled(ui, ui_state, frame.position, remaining_width, remaining_height);

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
            let cell_rect = egui::Rect::from_min_size(
                egui::pos2(x, y),
                egui::vec2(check_size, check_size),
            )
            .intersect(rect);
            painter.rect_filled(cell_rect, 0.0, color);
        }
    }
}

fn render_atlas_grid(ui: &mut egui::Ui, ui_state: &mut EditorUI, ctx: &egui::Context) {
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
    let grid_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 200, 50, 150));

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

    // Highlight cells that are in the current clip
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

    // Highlight hovered cell
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

    let texture = ctx.load_texture("animation_editor_atlas", color_image, egui::TextureOptions::NEAREST);
    ui_state.animation.atlas_texture = Some(texture);

    tracing::info!(
        "Loaded atlas texture: {}x{}",
        decoded.width,
        decoded.height
    );
}

fn render_frame_sequence(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.heading("Frames");

    let Some(clip_idx) = ui_state.animation.authoring.selected_clip_index else {
        ui.label("Select a clip");
        return;
    };

    if clip_idx >= ui_state.animation.authoring.clips.len() {
        return;
    };

    // Clip settings - get values first to avoid borrow issues
    let current_duration = ui_state.animation.authoring.clips[clip_idx].default_duration_ms;
    let current_loop_mode = ui_state.animation.authoring.clips[clip_idx].loop_mode.clone();

    ui.horizontal(|ui| {
        ui.label("Duration (ms):");
    });
    let mut duration = current_duration;
    if ui
        .add(
            egui::DragValue::new(&mut duration)
                .speed(5.0)
                .range(10.0..=5000.0),
        )
        .changed()
    {
        ui_state.animation.authoring.clips[clip_idx].default_duration_ms = duration;
        ui_state.animation.authoring.dirty = true;
    }

    ui.horizontal(|ui| {
        ui.label("Loop Mode:");
    });
    let loop_modes = ["loop", "once", "ping_pong"];
    let mut loop_mode = current_loop_mode;
    let mut loop_mode_changed = false;
    egui::ComboBox::from_id_salt("loop_mode")
        .selected_text(&loop_mode)
        .show_ui(ui, |ui| {
            for mode in &loop_modes {
                if ui
                    .selectable_value(&mut loop_mode, mode.to_string(), *mode)
                    .changed()
                {
                    loop_mode_changed = true;
                }
            }
        });

    if loop_mode_changed {
        ui_state.animation.authoring.clips[clip_idx].loop_mode = loop_mode;
        ui_state.animation.authoring.dirty = true;
    }

    ui.separator();

    // Frame list - collect info first to avoid borrow issues
    let frame_info: Vec<_> = ui_state
        .animation
        .authoring
        .clips
        .get(clip_idx)
        .map(|clip| {
            clip.frames
                .iter()
                .enumerate()
                .map(|(idx, frame)| {
                    let is_selected = ui_state.animation.authoring.selected_frame_index == Some(idx);
                    let is_preview = ui_state.animation.preview.current_frame() == idx;
                    (idx, frame.position, frame.duration_ms, is_selected, is_preview)
                })
                .collect()
        })
        .unwrap_or_default();

    let frame_count = frame_info.len();
    let mut select_frame: Option<usize> = None;
    let mut move_up: Option<usize> = None;
    let mut move_down: Option<usize> = None;
    let mut delete_frame: Option<usize> = None;

    egui::ScrollArea::vertical()
        .id_salt("anim_frame_sequence")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (idx, position, duration_override, is_selected, is_preview) in &frame_info {
                let label = format!(
                    "[{}, {}]{}{}",
                    position[0],
                    position[1],
                    duration_override
                        .map(|d| format!(" {:.0}ms", d))
                        .unwrap_or_default(),
                    if *is_preview { " <" } else { "" }
                );

                ui.horizontal(|ui| {
                    if ui.selectable_label(*is_selected, &label).clicked() {
                        select_frame = Some(*idx);
                    }

                    if *idx > 0 && ui.small_button("^").clicked() {
                        move_up = Some(*idx);
                    }
                    if *idx + 1 < frame_count && ui.small_button("v").clicked() {
                        move_down = Some(*idx);
                    }
                    if ui.small_button("x").clicked() {
                        delete_frame = Some(*idx);
                    }
                });
            }
        });

    // Apply deferred actions
    if let Some(idx) = select_frame {
        ui_state.animation.authoring.selected_frame_index = Some(idx);
        ui_state.animation.preview.go_to_frame(idx, frame_count);
    }

    if let Some(idx) = move_up {
        if let Some(clip) = ui_state.animation.authoring.clips.get_mut(clip_idx) {
            clip.move_frame(idx, idx - 1);
            if ui_state.animation.authoring.selected_frame_index == Some(idx) {
                ui_state.animation.authoring.selected_frame_index = Some(idx - 1);
            }
            ui_state.animation.authoring.dirty = true;
        }
    }

    if let Some(idx) = move_down {
        if let Some(clip) = ui_state.animation.authoring.clips.get_mut(clip_idx) {
            clip.move_frame(idx, idx + 1);
            if ui_state.animation.authoring.selected_frame_index == Some(idx) {
                ui_state.animation.authoring.selected_frame_index = Some(idx + 1);
            }
            ui_state.animation.authoring.dirty = true;
        }
    }

    if let Some(idx) = delete_frame {
        if let Some(clip) = ui_state.animation.authoring.clips.get_mut(clip_idx) {
            clip.remove_frame(idx);
            if clip.frames.is_empty() {
                ui_state.animation.authoring.selected_frame_index = None;
            } else if let Some(sel) = ui_state.animation.authoring.selected_frame_index {
                if sel >= clip.frames.len() {
                    ui_state.animation.authoring.selected_frame_index = Some(clip.frames.len() - 1);
                }
            }
            ui_state.animation.authoring.dirty = true;
        }
    }

    // Keyboard shortcuts
    let ctx = ui.ctx();
    if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
        ui_state.animation.authoring.remove_selected_frame();
    }
}

fn load_entity(ui_state: &mut EditorUI, project_path: &Path, entity_name: &str) {
    let file_path = project_path.join("entities").join(format!("{}.json", entity_name));

    let Ok(content) = std::fs::read_to_string(&file_path) else {
        tracing::error!("Failed to read entity file: {:?}", file_path);
        return;
    };

    let Ok(definition): Result<toki_core::entity::EntityDefinition, _> =
        serde_json::from_str(&content)
    else {
        tracing::error!("Failed to parse entity definition: {:?}", file_path);
        return;
    };

    // Load the atlas to get tile name to position mapping and metadata
    let atlas_name = &definition.animations.atlas_name;
    let tile_lookup = load_atlas_tile_lookup(project_path, atlas_name);
    let atlas_info = load_atlas_info(project_path, atlas_name);

    let authoring = AnimationAuthoringState::from_animations_def_with_tile_lookup(
        &definition.animations,
        tile_lookup.as_ref(),
    );

    ui_state
        .animation
        .load_entity(entity_name, file_path, authoring);

    // Store atlas metadata for canvas rendering
    if let Some((cell_size, png_path)) = atlas_info {
        ui_state.animation.atlas_cell_size = Some((cell_size.x, cell_size.y));
        ui_state.animation.atlas_texture_path = Some(png_path);
    }

    tracing::info!("Loaded entity for animation editing: {}", entity_name);
}

/// Load atlas metadata and return cell size and PNG path
fn load_atlas_info(project_path: &Path, atlas_name: &str) -> Option<(glam::UVec2, PathBuf)> {
    if atlas_name.is_empty() {
        return None;
    }

    // Normalize atlas name: ensure .json extension
    let atlas_filename = if atlas_name.ends_with(".json") {
        atlas_name.to_string()
    } else {
        format!("{}.json", atlas_name)
    };

    let atlas_path = project_path.join("assets").join("sprites").join(&atlas_filename);
    let atlas = toki_core::assets::atlas::AtlasMeta::load_from_file(&atlas_path).ok()?;

    // Get PNG path relative to atlas JSON
    let png_path = atlas_path.parent()?.join(&atlas.image);

    Some((atlas.tile_size, png_path))
}

/// Load an atlas file and extract the tile name to position mapping
fn load_atlas_tile_lookup(
    project_path: &Path,
    atlas_name: &str,
) -> Option<std::collections::HashMap<String, [u32; 2]>> {
    if atlas_name.is_empty() {
        return None;
    }

    // Normalize atlas name: ensure .json extension
    let atlas_filename = if atlas_name.ends_with(".json") {
        atlas_name.to_string()
    } else {
        format!("{}.json", atlas_name)
    };

    // Atlas files are in assets/sprites/
    let atlas_path = project_path.join("assets").join("sprites").join(&atlas_filename);

    // Use AtlasMeta from toki-core to load and parse the atlas
    let atlas = toki_core::assets::atlas::AtlasMeta::load_from_file(&atlas_path).ok()?;

    let lookup: std::collections::HashMap<String, [u32; 2]> = atlas
        .tiles
        .into_iter()
        .map(|(name, info)| (name, [info.position.x, info.position.y]))
        .collect();

    Some(lookup)
}

fn save_current_entity(ui_state: &mut EditorUI) {
    let Some(file_path) = ui_state.animation.entity_file_path.clone() else {
        tracing::error!("No entity file path set");
        return;
    };

    // Read the current definition
    let Ok(content) = std::fs::read_to_string(&file_path) else {
        tracing::error!("Failed to read entity file for saving: {:?}", file_path);
        return;
    };

    let Ok(mut definition): Result<toki_core::entity::EntityDefinition, _> =
        serde_json::from_str(&content)
    else {
        tracing::error!("Failed to parse entity definition for saving: {:?}", file_path);
        return;
    };

    // Update animations from authoring state
    definition.animations = ui_state.animation.authoring.to_animations_def();

    // Update atlas metadata with tile names for all frame positions
    if let Some(project_path) = file_path.parent().and_then(|p| p.parent()) {
        sync_atlas_tile_names(project_path, &definition.name, &mut definition.animations);
    }

    // Write back
    let Ok(json) = serde_json::to_string_pretty(&definition) else {
        tracing::error!("Failed to serialize entity definition");
        return;
    };

    if let Err(e) = std::fs::write(&file_path, json) {
        tracing::error!("Failed to write entity file: {}", e);
        return;
    }

    ui_state.animation.authoring.dirty = false;
    tracing::info!("Saved animation changes to {:?}", file_path);
}

/// Sync atlas metadata to have proper tile names for all frame positions used in animations.
/// Clears all existing tiles and writes fresh entries with proper naming convention.
/// Naming convention: `<entity_name>/<state>_<frame_letter>` (e.g., soldier/walk_down_a)
fn sync_atlas_tile_names(
    project_path: &Path,
    entity_name: &str,
    animations: &mut toki_core::entity::AnimationsDef,
) {
    use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};

    let atlas_name = &animations.atlas_name;
    if atlas_name.is_empty() {
        return;
    }

    let atlas_path = resolve_atlas_path(project_path, atlas_name);
    let Ok(mut atlas) = AtlasMeta::load_from_file(&atlas_path) else {
        tracing::warn!("Failed to load atlas for tile name sync: {:?}", atlas_path);
        return;
    };

    // Clear all existing tiles
    atlas.tiles.clear();

    // Process each animation clip
    for clip in &mut animations.clips {
        let Some(positions) = clip.frame_positions.take() else {
            continue;
        };

        // Generate proper tile names for this clip
        let tile_names = generate_tile_names(entity_name, &clip.state, positions.len());

        // Add tiles with proper names
        for (i, pos) in positions.iter().enumerate() {
            let tile_name = &tile_names[i];
            atlas.tiles.insert(
                tile_name.clone(),
                TileInfo {
                    position: glam::UVec2::new(pos[0], pos[1]),
                    properties: TileProperties::default(),
                },
            );
        }

        clip.frame_tiles = tile_names;
    }

    save_atlas(&atlas_path, &atlas);
}

/// Resolve atlas filename to full path
fn resolve_atlas_path(project_path: &Path, atlas_name: &str) -> PathBuf {
    let atlas_filename = if atlas_name.ends_with(".json") {
        atlas_name.to_string()
    } else {
        format!("{}.json", atlas_name)
    };
    project_path
        .join("assets")
        .join("sprites")
        .join(&atlas_filename)
}

/// Generate proper tile names for frame positions following the naming convention.
/// Always creates names in format `entity/state_letter` (e.g., soldier/walk_down_a).
fn generate_tile_names(entity_name: &str, state: &str, frame_count: usize) -> Vec<String> {
    const FRAME_LETTERS: &[char] = &[
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];

    (0..frame_count)
        .map(|i| {
            let letter = FRAME_LETTERS.get(i).unwrap_or(&'z');
            format!("{}/{}_{}", entity_name, state, letter)
        })
        .collect()
}

/// Save atlas metadata to file
fn save_atlas(atlas_path: &Path, atlas: &toki_core::assets::atlas::AtlasMeta) {
    if let Err(e) = atlas.save_to_file(atlas_path) {
        tracing::error!("Failed to save atlas with new tile names: {}", e);
    } else {
        tracing::info!("Updated atlas with tile names: {:?}", atlas_path);
    }
}

fn render_new_clip_dialog(ui_state: &mut EditorUI, ctx: &egui::Context) {
    let available_states = ui_state.animation.authoring.available_states();

    egui::Window::new("New Animation Clip")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("Select animation state:");

            // Quick add buttons for common states
            if !available_states.is_empty() {
                ui.label("Common states:");
                let mut created_state: Option<&str> = None;

                egui::ScrollArea::vertical()
                    .id_salt("anim_new_clip_states")
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for state in &available_states {
                            if ui.button(*state).clicked() {
                                created_state = Some(state);
                            }
                        }
                    });

                if let Some(state) = created_state {
                    ui_state.animation.authoring.create_clip(state);
                    ui_state.animation.show_new_clip_dialog = false;
                }

                ui.separator();
            }

            // Custom state input
            ui.label("Or enter custom state name:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut ui_state.animation.new_clip_state_input);
                let can_create = !ui_state.animation.new_clip_state_input.trim().is_empty()
                    && !ui_state
                        .animation
                        .authoring
                        .has_clip_for_state(ui_state.animation.new_clip_state_input.trim());

                if ui
                    .add_enabled(can_create, egui::Button::new("Create"))
                    .clicked()
                {
                    let state = ui_state.animation.new_clip_state_input.trim().to_string();
                    ui_state.animation.authoring.create_clip(&state);
                    ui_state.animation.new_clip_state_input.clear();
                    ui_state.animation.show_new_clip_dialog = false;
                }
            });

            ui.separator();
            if ui.button("Cancel").clicked() {
                ui_state.animation.new_clip_state_input.clear();
                ui_state.animation.show_new_clip_dialog = false;
            }
        });
}
