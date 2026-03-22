use crate::config::EditorConfig;
use crate::editor_grid::GridInteraction;
use glam::{IVec2, UVec2, Vec2};
use toki_core::assets::tilemap::TileMap;
use toki_core::camera::viewport_to_world;

#[derive(Debug, Clone, Copy)]
pub struct EditorViewportContext {
    viewport_size: (u32, u32),
    display_rect: egui::Rect,
    camera_position: IVec2,
    camera_scale: f32,
}

impl EditorViewportContext {
    pub fn new(
        outer_rect: egui::Rect,
        viewport_size: (u32, u32),
        responsive: bool,
        camera_position: IVec2,
        camera_scale: f32,
    ) -> Self {
        Self {
            viewport_size,
            display_rect: compute_display_rect(outer_rect, viewport_size, responsive),
            camera_position,
            camera_scale,
        }
    }

    pub fn display_rect(&self) -> egui::Rect {
        self.display_rect
    }

    pub fn screen_to_world(&self, screen_pos: egui::Pos2) -> Vec2 {
        screen_to_world_from_camera(
            screen_pos,
            self.display_rect,
            self.viewport_size,
            self.camera_position,
            self.camera_scale,
        )
    }

    pub fn contains_screen_pos(&self, screen_pos: egui::Pos2) -> bool {
        self.display_rect.contains(screen_pos)
    }

    pub fn hover_world_from_response(&self, response: &egui::Response) -> Option<Vec2> {
        response
            .hover_pos()
            .filter(|pos| self.contains_screen_pos(*pos))
            .map(|pos| self.screen_to_world(pos))
    }

    pub fn world_rect_to_screen_rect(
        &self,
        world_top_left: UVec2,
        world_size: UVec2,
    ) -> Option<egui::Rect> {
        if self.camera_scale <= 0.0 {
            return None;
        }

        let screen_min_x = self.display_rect.min.x
            + (world_top_left.x as f32 - self.camera_position.x as f32) / self.camera_scale;
        let screen_min_y = self.display_rect.min.y
            + (world_top_left.y as f32 - self.camera_position.y as f32) / self.camera_scale;
        let screen_size = egui::vec2(
            world_size.x as f32 / self.camera_scale,
            world_size.y as f32 / self.camera_scale,
        );
        Some(egui::Rect::from_min_size(
            egui::pos2(screen_min_x, screen_min_y),
            screen_size,
        ))
    }

    pub fn tile_screen_rect(&self, tile_size: UVec2, tile_pos: UVec2) -> Option<egui::Rect> {
        let world_top_left = UVec2::new(tile_pos.x * tile_size.x, tile_pos.y * tile_size.y);
        self.world_rect_to_screen_rect(world_top_left, tile_size)
    }

    pub fn effective_grid_size(tilemap: Option<&TileMap>, config: Option<&EditorConfig>) -> UVec2 {
        GridInteraction::effective_grid_size(tilemap, config)
            .unwrap_or(UVec2::ONE)
            .max(UVec2::ONE)
    }

    pub fn world_to_tile_coords(world_pos: IVec2, grid_size: UVec2) -> IVec2 {
        IVec2::new(
            world_pos.x.div_euclid(grid_size.x.max(1) as i32),
            world_pos.y.div_euclid(grid_size.y.max(1) as i32),
        )
    }
}

pub fn compute_display_rect(
    outer_rect: egui::Rect,
    viewport_size: (u32, u32),
    responsive: bool,
) -> egui::Rect {
    if responsive {
        return outer_rect;
    }

    let viewport_aspect = viewport_size.0 as f32 / viewport_size.1 as f32;
    let available_size = outer_rect.size();
    let available_aspect = available_size.x / available_size.y;

    let display_size = if available_aspect > viewport_aspect {
        egui::Vec2::new(available_size.y * viewport_aspect, available_size.y)
    } else {
        egui::Vec2::new(available_size.x, available_size.x / viewport_aspect)
    };
    let offset = (available_size - display_size) * 0.5;
    egui::Rect::from_min_size(outer_rect.min + offset, display_size)
}

pub fn screen_to_world_from_camera(
    screen_pos: egui::Pos2,
    display_rect: egui::Rect,
    viewport_size: (u32, u32),
    camera_position: IVec2,
    camera_scale: f32,
) -> Vec2 {
    let viewport_pos = screen_to_viewport(screen_pos, display_rect, viewport_size);
    viewport_to_world(viewport_pos, camera_position, camera_scale)
}

fn screen_to_viewport(
    screen_pos: egui::Pos2,
    display_rect: egui::Rect,
    viewport_size: (u32, u32),
) -> Vec2 {
    let normalized_x = (screen_pos.x - display_rect.min.x) / display_rect.width();
    let normalized_y = (screen_pos.y - display_rect.min.y) / display_rect.height();

    let display_aspect = display_rect.width() / display_rect.height();
    let viewport_aspect = viewport_size.0 as f32 / viewport_size.1 as f32;

    if display_aspect > viewport_aspect {
        let effective_width = display_rect.height() * viewport_aspect;
        let x_offset = (display_rect.width() - effective_width) * 0.5;
        let adjusted_x = (screen_pos.x - display_rect.min.x - x_offset) / effective_width;

        Vec2::new(
            adjusted_x.clamp(0.0, 1.0) * viewport_size.0 as f32,
            normalized_y * viewport_size.1 as f32,
        )
    } else {
        let effective_height = display_rect.width() / viewport_aspect;
        let y_offset = (display_rect.height() - effective_height) * 0.5;
        let adjusted_y = (screen_pos.y - display_rect.min.y - y_offset) / effective_height;

        Vec2::new(
            normalized_x * viewport_size.0 as f32,
            adjusted_y.clamp(0.0, 1.0) * viewport_size.1 as f32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_display_rect_centers_fixed_aspect_viewport() {
        let outer = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(320.0, 200.0));
        let display = compute_display_rect(outer, (160, 144), false);
        assert_eq!(display.size(), egui::vec2(222.22223, 200.0));
        assert!(display.min.x > 0.0);
    }

    #[test]
    fn screen_to_world_clamps_letterbox_sides() {
        let display = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(320.0, 144.0));
        let left = screen_to_world_from_camera(
            egui::Pos2::new(0.0, 72.0),
            display,
            (160, 144),
            IVec2::ZERO,
            1.0,
        );
        let right = screen_to_world_from_camera(
            egui::Pos2::new(320.0, 72.0),
            display,
            (160, 144),
            IVec2::ZERO,
            1.0,
        );
        assert_eq!(left.x, 0.0);
        assert_eq!(right.x, 160.0);
    }

    #[test]
    fn world_rect_to_screen_rect_respects_camera_scale() {
        let ctx = EditorViewportContext::new(
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(160.0, 144.0)),
            (160, 144),
            true,
            IVec2::new(0, 0),
            2.0,
        );
        let rect = ctx
            .world_rect_to_screen_rect(UVec2::new(16, 32), UVec2::new(32, 16))
            .expect("screen rect");
        assert_eq!(rect.min, egui::pos2(8.0, 16.0));
        assert_eq!(rect.size(), egui::vec2(16.0, 8.0));
    }

    #[test]
    fn world_to_tile_coords_uses_euclidean_division() {
        let tile = EditorViewportContext::world_to_tile_coords(IVec2::new(-1, 17), UVec2::new(16, 16));
        assert_eq!(tile, IVec2::new(-1, 1));
    }
}
