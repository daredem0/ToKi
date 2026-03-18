use super::*;

impl SceneViewport {
    pub(super) fn render_placeholder(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        ui.painter()
            .rect_filled(rect, 4.0, egui::Color32::from_rgb(32, 32, 40));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Initializing Scene Viewport...",
            egui::FontId::default(),
            egui::Color32::WHITE,
        );
    }

    pub(super) fn render_error(&self, ui: &mut egui::Ui, rect: egui::Rect, error_msg: &str) {
        ui.painter()
            .rect_filled(rect, 4.0, egui::Color32::from_rgb(60, 32, 32));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            error_msg,
            egui::FontId::default(),
            egui::Color32::WHITE,
        );
    }

    pub(super) fn render_debug_status(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        status_msg: &str,
    ) {
        ui.painter()
            .rect_filled(rect, 4.0, egui::Color32::from_rgb(40, 40, 50));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            status_msg,
            egui::FontId::default(),
            egui::Color32::LIGHT_BLUE,
        );

        let debug_y = rect.min.y + 30.0;
        let debug_text = format!(
            "✓ SceneRenderer: {}\n✓ OffscreenTarget: {}\n✓ Tilemap: {}",
            if self.scene_renderer.is_some() {
                "Ready"
            } else {
                "Not Ready"
            },
            if self.offscreen_target.is_some() {
                "Ready"
            } else {
                "Not Ready"
            },
            if self.tilemap.is_some() {
                "Loaded"
            } else {
                "None"
            }
        );

        ui.painter().text(
            egui::pos2(rect.min.x + 10.0, debug_y),
            egui::Align2::LEFT_TOP,
            debug_text,
            egui::FontId::monospace(10.0),
            egui::Color32::LIGHT_GRAY,
        );
    }
}
