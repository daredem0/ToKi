use toki_core::math::projection::screen_space_projection;
use toki_core::project_runtime::RuntimeViewportMode;

use super::layout::{compute_layout_for_mode, ViewportLayout};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportPresentation {
    pub layout: ViewportLayout,
    pub projection: glam::Mat4,
}

pub fn resolve_fixed_viewport_presentation(
    window_size: glam::UVec2,
    base_viewport: glam::UVec2,
    mode: RuntimeViewportMode,
) -> ViewportPresentation {
    let layout = compute_layout_for_mode(window_size, base_viewport, mode);
    let projection = build_fixed_viewport_projection(layout);

    ViewportPresentation { layout, projection }
}

pub fn build_fixed_viewport_projection(layout: ViewportLayout) -> glam::Mat4 {
    let surface = screen_space_projection(
        layout.target_window_size.x as f32,
        layout.target_window_size.y as f32,
    );
    let translate = glam::Mat4::from_translation(glam::vec3(
        layout.viewport_rect.x,
        layout.viewport_rect.y,
        0.0,
    ));
    let scale = glam::Mat4::from_scale(glam::vec3(
        layout.viewport_rect.width / layout.logical_viewport_size.x as f32,
        layout.viewport_rect.height / layout.logical_viewport_size.y as f32,
        1.0,
    ));

    surface * translate * scale
}
