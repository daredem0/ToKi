//! Sprite editor layout and viewport arrangement.

use crate::ui::editor_ui::CanvasSide;
use crate::ui::EditorUI;

use super::canvas::{render_canvas_viewport, render_empty_canvas_slot};

pub fn render_no_canvas_message(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    sprites_dir: Option<&std::path::Path>,
) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.label("No canvas open");
            ui.add_space(10.0);
            if ui.button("Create New Canvas").clicked() {
                ui_state.begin_new_sprite_canvas_dialog();
            }
            ui.add_space(5.0);
            let load_enabled = sprites_dir.is_some();
            if ui
                .add_enabled(load_enabled, egui::Button::new("Load Existing Sprite"))
                .clicked()
            {
                if let Some(dir) = sprites_dir {
                    ui_state.sprite.begin_load_dialog(dir);
                }
            }
        });
    });
}

/// Render dual viewports side-by-side (horizontal layout)
pub fn render_dual_viewports_horizontal(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    sprites_dir: Option<&std::path::Path>,
) {
    let available = ui.available_size();
    let splitter_width = 8.0;
    let usable_width = (available.x - splitter_width).max(200.0);

    let split_ratio = ui_state.sprite.split_ratio.clamp(0.1, 0.9);
    let left_width = (usable_width * split_ratio).max(100.0);
    let right_width = (usable_width * (1.0 - split_ratio)).max(100.0);

    ui.horizontal(|ui| {
        // Left canvas
        ui.vertical(|ui| {
            ui.set_width(left_width);
            render_canvas_panel_header(ui, ui_state, CanvasSide::Left);
            if ui_state
                .sprite
                .canvas_state(CanvasSide::Left)
                .canvas
                .is_some()
            {
                render_canvas_viewport(ui, ui_state, ctx, Some(CanvasSide::Left));
            } else {
                render_empty_canvas_slot(ui, ui_state, sprites_dir, CanvasSide::Left);
            }
        });

        // Draggable splitter
        let splitter_response = render_vertical_splitter(ui, available.y);
        if splitter_response.dragged() {
            let delta = splitter_response.drag_delta().x;
            let new_ratio = ui_state.sprite.split_ratio + delta / usable_width;
            ui_state.sprite.split_ratio = new_ratio.clamp(0.1, 0.9);
        }

        // Right canvas
        ui.vertical(|ui| {
            ui.set_width(right_width);
            render_canvas_panel_header(ui, ui_state, CanvasSide::Right);
            if ui_state
                .sprite
                .canvas_state(CanvasSide::Right)
                .canvas
                .is_some()
            {
                render_canvas_viewport(ui, ui_state, ctx, Some(CanvasSide::Right));
            } else {
                render_empty_canvas_slot(ui, ui_state, sprites_dir, CanvasSide::Right);
            }
        });
    });
}

/// Render dual viewports stacked (vertical layout)
pub fn render_dual_viewports_vertical(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    sprites_dir: Option<&std::path::Path>,
) {
    let available = ui.available_size();
    let splitter_height = 8.0;
    let usable_height = (available.y - splitter_height - 48.0).max(200.0);

    let split_ratio = ui_state.sprite.split_ratio.clamp(0.1, 0.9);
    let top_height = (usable_height * split_ratio).max(100.0);
    let bottom_height = (usable_height * (1.0 - split_ratio)).max(100.0);

    // Top canvas
    ui.vertical(|ui| {
        ui.set_height(top_height);
        render_canvas_panel_header(ui, ui_state, CanvasSide::Left);
        if ui_state
            .sprite
            .canvas_state(CanvasSide::Left)
            .canvas
            .is_some()
        {
            render_canvas_viewport(ui, ui_state, ctx, Some(CanvasSide::Left));
        } else {
            render_empty_canvas_slot(ui, ui_state, sprites_dir, CanvasSide::Left);
        }
    });

    // Draggable splitter
    let splitter_response = render_horizontal_splitter(ui, available.x);
    if splitter_response.dragged() {
        let delta = splitter_response.drag_delta().y;
        let new_ratio = ui_state.sprite.split_ratio + delta / usable_height;
        ui_state.sprite.split_ratio = new_ratio.clamp(0.1, 0.9);
    }

    // Bottom canvas
    ui.vertical(|ui| {
        ui.set_height(bottom_height);
        render_canvas_panel_header(ui, ui_state, CanvasSide::Right);
        if ui_state
            .sprite
            .canvas_state(CanvasSide::Right)
            .canvas
            .is_some()
        {
            render_canvas_viewport(ui, ui_state, ctx, Some(CanvasSide::Right));
        } else {
            render_empty_canvas_slot(ui, ui_state, sprites_dir, CanvasSide::Right);
        }
    });
}

/// Render a vertical splitter (for horizontal layout - splits left/right)
fn render_vertical_splitter(ui: &mut egui::Ui, height: f32) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(8.0, height), egui::Sense::click_and_drag());

    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }

    let painter = ui.painter();
    let color = if response.hovered() || response.dragged() {
        egui::Color32::from_gray(120)
    } else {
        egui::Color32::from_gray(80)
    };

    let center_x = rect.center().x;
    painter.line_segment(
        [
            egui::pos2(center_x, rect.top()),
            egui::pos2(center_x, rect.bottom()),
        ],
        egui::Stroke::new(2.0, color),
    );

    response
}

/// Render a horizontal splitter (for vertical layout - splits top/bottom)
fn render_horizontal_splitter(ui: &mut egui::Ui, width: f32) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, 8.0), egui::Sense::click_and_drag());

    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }

    let painter = ui.painter();
    let color = if response.hovered() || response.dragged() {
        egui::Color32::from_gray(120)
    } else {
        egui::Color32::from_gray(80)
    };

    let center_y = rect.center().y;
    painter.line_segment(
        [
            egui::pos2(rect.left(), center_y),
            egui::pos2(rect.right(), center_y),
        ],
        egui::Stroke::new(2.0, color),
    );

    response
}

/// Render a header for a canvas panel showing its side and active state
fn render_canvas_panel_header(ui: &mut egui::Ui, ui_state: &mut EditorUI, side: CanvasSide) {
    let is_active = ui_state.sprite.active_canvas == side;
    let label = side.label();

    ui.horizontal(|ui| {
        if is_active {
            ui.label(egui::RichText::new(format!("● {}", label)).strong());
        } else if ui.button(label).clicked() {
            ui_state.sprite.set_active_canvas(side);
        }

        if ui_state.sprite.canvas_state(side).dirty {
            ui.label("*");
        }
    });
}
