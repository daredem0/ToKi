//! Viewport state for the sprite canvas (pan/zoom).

/// Viewport state for the sprite canvas
#[derive(Debug, Clone)]
pub struct SpriteCanvasViewport {
    /// Camera offset in canvas pixels (top-left corner of view)
    pub pan: glam::Vec2,
    /// Zoom level (1.0 = 1 canvas pixel = 1 screen pixel)
    pub zoom: f32,
    /// Minimum zoom level
    pub zoom_min: f32,
    /// Maximum zoom level
    pub zoom_max: f32,
}

impl Default for SpriteCanvasViewport {
    fn default() -> Self {
        Self {
            pan: glam::Vec2::ZERO,
            zoom: 8.0, // Start zoomed in for pixel editing
            zoom_min: 1.0,
            zoom_max: 64.0,
        }
    }
}

impl SpriteCanvasViewport {
    /// Zoom in by one step (doubling)
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.2).min(self.zoom_max);
    }

    /// Zoom out by one step
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.2).max(self.zoom_min);
    }

    /// Pan by delta in screen pixels
    pub fn pan_by(&mut self, delta: glam::Vec2) {
        // Convert screen delta to canvas delta
        self.pan -= delta / self.zoom;
    }

    /// Convert screen position to canvas position
    pub fn screen_to_canvas(
        &self,
        screen_pos: glam::Vec2,
        viewport_rect: egui::Rect,
    ) -> glam::Vec2 {
        let viewport_pos = screen_pos - glam::Vec2::new(viewport_rect.left(), viewport_rect.top());
        viewport_pos / self.zoom + self.pan
    }

    /// Convert canvas position to screen position
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn canvas_to_screen(
        &self,
        canvas_pos: glam::Vec2,
        viewport_rect: egui::Rect,
    ) -> glam::Vec2 {
        let viewport_pos = (canvas_pos - self.pan) * self.zoom;
        viewport_pos + glam::Vec2::new(viewport_rect.left(), viewport_rect.top())
    }

}
