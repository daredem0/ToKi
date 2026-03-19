use super::ViewportSizingMode;
use toki_core::camera::viewport_to_world;

pub(super) fn compute_display_rect(
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

pub(super) fn screen_to_world_from_camera(
    screen_pos: egui::Pos2,
    display_rect: egui::Rect,
    viewport_size: (u32, u32),
    camera_position: glam::IVec2,
    camera_scale: f32,
) -> glam::Vec2 {
    // Convert screen coordinates to viewport-local coordinates,
    // handling letterboxing/pillarboxing for aspect ratio mismatches
    let viewport_pos = screen_to_viewport(screen_pos, display_rect, viewport_size);

    // Use shared utility for viewport-to-world conversion
    viewport_to_world(viewport_pos, camera_position, camera_scale)
}

/// Converts screen coordinates to viewport-local coordinates.
///
/// Handles letterboxing (horizontal bars) when display is wider than viewport,
/// and pillarboxing (vertical bars) when display is taller than viewport.
fn screen_to_viewport(
    screen_pos: egui::Pos2,
    display_rect: egui::Rect,
    viewport_size: (u32, u32),
) -> glam::Vec2 {
    let normalized_x = (screen_pos.x - display_rect.min.x) / display_rect.width();
    let normalized_y = (screen_pos.y - display_rect.min.y) / display_rect.height();

    let display_aspect = display_rect.width() / display_rect.height();
    let viewport_aspect = viewport_size.0 as f32 / viewport_size.1 as f32;

    if display_aspect > viewport_aspect {
        // Display is wider - pillarboxing (bars on sides)
        let effective_width = display_rect.height() * viewport_aspect;
        let x_offset = (display_rect.width() - effective_width) * 0.5;
        let adjusted_x = (screen_pos.x - display_rect.min.x - x_offset) / effective_width;

        glam::Vec2::new(
            adjusted_x.clamp(0.0, 1.0) * viewport_size.0 as f32,
            normalized_y * viewport_size.1 as f32,
        )
    } else {
        // Display is taller - letterboxing (bars on top/bottom)
        let effective_height = display_rect.width() / viewport_aspect;
        let y_offset = (display_rect.height() - effective_height) * 0.5;
        let adjusted_y = (screen_pos.y - display_rect.min.y - y_offset) / effective_height;

        glam::Vec2::new(
            normalized_x * viewport_size.0 as f32,
            adjusted_y.clamp(0.0, 1.0) * viewport_size.1 as f32,
        )
    }
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
