use super::ViewportSizingMode;
use crate::editor_viewport::{
    compute_display_rect as shared_compute_display_rect,
    screen_to_world_from_camera as shared_screen_to_world_from_camera,
};

pub(super) fn compute_display_rect(
    outer_rect: egui::Rect,
    viewport_size: (u32, u32),
    responsive: bool,
) -> egui::Rect {
    shared_compute_display_rect(outer_rect, viewport_size, responsive)
}

pub(super) fn screen_to_world_from_camera(
    screen_pos: egui::Pos2,
    display_rect: egui::Rect,
    viewport_size: (u32, u32),
    camera_position: glam::IVec2,
    camera_scale: f32,
) -> glam::Vec2 {
    shared_screen_to_world_from_camera(
        screen_pos,
        display_rect,
        viewport_size,
        camera_position,
        camera_scale,
    )
}

pub(super) fn world_to_i32_floor(world_pos: glam::Vec2) -> glam::IVec2 {
    glam::IVec2::new(world_pos.x.floor() as i32, world_pos.y.floor() as i32)
}

pub(super) fn point_in_entity_bounds(
    point_world: glam::IVec2,
    entity_top_left: glam::IVec2,
    entity_size: glam::UVec2,
) -> bool {
    let entity_max = entity_top_left + glam::IVec2::new(entity_size.x as i32, entity_size.y as i32);
    point_world.x >= entity_top_left.x
        && point_world.x < entity_max.x
        && point_world.y >= entity_top_left.y
        && point_world.y < entity_max.y
}

pub(super) fn request_viewport_size_state(
    sizing_mode: ViewportSizingMode,
    is_initialized: bool,
    current_size: (u32, u32),
    requested_size: Option<(u32, u32)>,
    new_size: (u32, u32),
) -> ((u32, u32), Option<(u32, u32)>, bool) {
    if sizing_mode != ViewportSizingMode::Responsive {
        return (current_size, requested_size, false);
    }

    let sanitized = (new_size.0.max(1), new_size.1.max(1));
    if sanitized == current_size && requested_size.is_none() {
        return (current_size, requested_size, false);
    }

    if !is_initialized {
        (sanitized, None, true)
    } else {
        (current_size, Some(sanitized), true)
    }
}

const EDITOR_ZOOM_LEVELS: &[f32] = &[
    0.1, 0.2, 0.4, 0.6, 0.8, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
];

pub(super) fn next_zoom_in_scale(current_scale: f32) -> f32 {
    EDITOR_ZOOM_LEVELS
        .iter()
        .rev()
        .copied()
        .find(|level| *level < current_scale - f32::EPSILON)
        .unwrap_or(current_scale)
}

pub(super) fn next_zoom_out_scale(current_scale: f32) -> f32 {
    EDITOR_ZOOM_LEVELS
        .iter()
        .copied()
        .find(|level| *level > current_scale + f32::EPSILON)
        .unwrap_or(current_scale)
}
