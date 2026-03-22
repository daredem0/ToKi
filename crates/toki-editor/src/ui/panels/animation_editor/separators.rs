//! Draggable separator widgets.

/// Render a vertical draggable separator
pub fn render_vertical_separator(ui: &mut egui::Ui, height: f32) -> egui::Response {
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
pub fn render_horizontal_separator(ui: &mut egui::Ui, width: f32) -> egui::Response {
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
