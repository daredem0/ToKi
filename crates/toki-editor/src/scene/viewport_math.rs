use super::ViewportSizingMode;

pub(super) fn screen_to_world_from_camera(
    screen_pos: egui::Pos2,
    display_rect: egui::Rect,
    viewport_size: (u32, u32),
    camera_position: glam::IVec2,
    camera_scale: f32,
) -> glam::Vec2 {
    let normalized_x = (screen_pos.x - display_rect.min.x) / display_rect.width();
    let normalized_y = (screen_pos.y - display_rect.min.y) / display_rect.height();

    let display_aspect = display_rect.width() / display_rect.height();
    let viewport_aspect = viewport_size.0 as f32 / viewport_size.1 as f32;

    let (viewport_x, viewport_y) = if display_aspect > viewport_aspect {
        let effective_width = display_rect.height() * viewport_aspect;
        let x_offset = (display_rect.width() - effective_width) * 0.5;
        let adjusted_x = (screen_pos.x - display_rect.min.x - x_offset) / effective_width;
        let adjusted_y = normalized_y;

        (
            adjusted_x.clamp(0.0, 1.0) * viewport_size.0 as f32,
            adjusted_y * viewport_size.1 as f32,
        )
    } else {
        let effective_height = display_rect.width() / viewport_aspect;
        let y_offset = (display_rect.height() - effective_height) * 0.5;
        let adjusted_x = normalized_x;
        let adjusted_y = (screen_pos.y - display_rect.min.y - y_offset) / effective_height;

        (
            adjusted_x * viewport_size.0 as f32,
            adjusted_y.clamp(0.0, 1.0) * viewport_size.1 as f32,
        )
    };

    let world_x = camera_position.x as f32 + viewport_x * camera_scale;
    let world_y = camera_position.y as f32 + viewport_y * camera_scale;
    glam::Vec2::new(world_x, world_y)
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
