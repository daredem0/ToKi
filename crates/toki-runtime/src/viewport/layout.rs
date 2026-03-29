use toki_core::project_runtime::{IntegerScaleFactor, RuntimeViewportMode};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportLayout {
    pub target_window_size: glam::UVec2,
    pub viewport_rect: ViewportRect,
    pub resolved_scale: f32,
    pub logical_viewport_size: glam::UVec2,
}

pub fn compute_aspect_fit_layout(
    window_size: glam::UVec2,
    base_viewport: glam::UVec2,
    fit_percent: u16,
) -> ViewportLayout {
    let window_size = sanitize_size(window_size);
    let logical_viewport_size = sanitize_size(base_viewport);
    let fit_scale = (window_size.x as f32 / logical_viewport_size.x as f32)
        .min(window_size.y as f32 / logical_viewport_size.y as f32);
    let percent_scale = (fit_percent.max(1) as f32 / 100.0).min(1.0);
    let resolved_scale = (fit_scale * percent_scale).max(f32::EPSILON);

    build_layout(window_size, logical_viewport_size, resolved_scale)
}

pub fn compute_integer_scale_layout(
    window_size: glam::UVec2,
    base_viewport: glam::UVec2,
    factor: IntegerScaleFactor,
) -> ViewportLayout {
    let window_size = sanitize_size(window_size);
    let logical_viewport_size = sanitize_size(base_viewport);
    let max_factor_x = window_size.x / logical_viewport_size.x;
    let max_factor_y = window_size.y / logical_viewport_size.y;
    let max_factor = max_factor_x.min(max_factor_y);

    let resolved_scale = match factor {
        IntegerScaleFactor::Auto => {
            if max_factor > 0 {
                max_factor as f32
            } else {
                fractional_fit_scale(window_size, logical_viewport_size)
            }
        }
        IntegerScaleFactor::Fixed(requested) => {
            let requested = requested.max(1) as u32;
            let effective = requested.min(max_factor);
            if effective > 0 {
                effective as f32
            } else {
                fractional_fit_scale(window_size, logical_viewport_size)
            }
        }
    };

    build_layout(window_size, logical_viewport_size, resolved_scale)
}

pub fn compute_layout_for_mode(
    window_size: glam::UVec2,
    base_viewport: glam::UVec2,
    mode: RuntimeViewportMode,
) -> ViewportLayout {
    match mode {
        RuntimeViewportMode::AspectFit { fit_percent } => {
            compute_aspect_fit_layout(window_size, base_viewport, fit_percent)
        }
        RuntimeViewportMode::IntegerScale { factor } => {
            compute_integer_scale_layout(window_size, base_viewport, factor)
        }
    }
}

fn sanitize_size(size: glam::UVec2) -> glam::UVec2 {
    glam::UVec2::new(size.x.max(1), size.y.max(1))
}

fn fractional_fit_scale(window_size: glam::UVec2, logical_viewport_size: glam::UVec2) -> f32 {
    (window_size.x as f32 / logical_viewport_size.x as f32)
        .min(window_size.y as f32 / logical_viewport_size.y as f32)
        .max(f32::EPSILON)
}

fn build_layout(
    window_size: glam::UVec2,
    logical_viewport_size: glam::UVec2,
    resolved_scale: f32,
) -> ViewportLayout {
    let width = logical_viewport_size.x as f32 * resolved_scale;
    let height = logical_viewport_size.y as f32 * resolved_scale;

    ViewportLayout {
        target_window_size: window_size,
        viewport_rect: ViewportRect {
            x: (window_size.x as f32 - width) * 0.5,
            y: (window_size.y as f32 - height) * 0.5,
            width,
            height,
        },
        resolved_scale,
        logical_viewport_size,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        compute_aspect_fit_layout, compute_integer_scale_layout, compute_layout_for_mode,
    };
    use toki_core::project_runtime::{IntegerScaleFactor, RuntimeViewportMode};

    #[test]
    fn aspect_fit_centers_in_wide_windows() {
        let layout =
            compute_aspect_fit_layout(glam::UVec2::new(320, 144), glam::UVec2::new(160, 144), 100);

        assert!((layout.viewport_rect.x - 80.0).abs() < 0.01);
        assert!((layout.viewport_rect.y - 0.0).abs() < 0.01);
        assert!((layout.viewport_rect.width - 160.0).abs() < 0.01);
        assert!((layout.viewport_rect.height - 144.0).abs() < 0.01);
    }

    #[test]
    fn aspect_fit_centers_in_tall_windows() {
        let layout =
            compute_aspect_fit_layout(glam::UVec2::new(160, 320), glam::UVec2::new(160, 144), 100);

        assert!((layout.viewport_rect.x - 0.0).abs() < 0.01);
        assert!((layout.viewport_rect.y - 88.0).abs() < 0.01);
        assert!((layout.viewport_rect.width - 160.0).abs() < 0.01);
        assert!((layout.viewport_rect.height - 144.0).abs() < 0.01);
    }

    #[test]
    fn aspect_fit_respects_fit_percent() {
        let layout =
            compute_aspect_fit_layout(glam::UVec2::new(320, 288), glam::UVec2::new(160, 144), 50);

        assert!((layout.resolved_scale - 1.0).abs() < 0.01);
        assert!((layout.viewport_rect.x - 80.0).abs() < 0.01);
        assert!((layout.viewport_rect.y - 72.0).abs() < 0.01);
        assert!((layout.viewport_rect.width - 160.0).abs() < 0.01);
        assert!((layout.viewport_rect.height - 144.0).abs() < 0.01);
    }

    #[test]
    fn integer_scale_auto_picks_largest_factor_that_fits() {
        let layout = compute_integer_scale_layout(
            glam::UVec2::new(800, 600),
            glam::UVec2::new(160, 144),
            IntegerScaleFactor::Auto,
        );

        assert!((layout.resolved_scale - 4.0).abs() < 0.01);
        assert!((layout.viewport_rect.x - 80.0).abs() < 0.01);
        assert!((layout.viewport_rect.y - 12.0).abs() < 0.01);
        assert!((layout.viewport_rect.width - 640.0).abs() < 0.01);
        assert!((layout.viewport_rect.height - 576.0).abs() < 0.01);
    }

    #[test]
    fn integer_scale_fixed_uses_requested_factor_when_it_fits() {
        let layout = compute_integer_scale_layout(
            glam::UVec2::new(800, 600),
            glam::UVec2::new(160, 144),
            IntegerScaleFactor::Fixed(3),
        );

        assert!((layout.resolved_scale - 3.0).abs() < 0.01);
        assert!((layout.viewport_rect.x - 160.0).abs() < 0.01);
        assert!((layout.viewport_rect.y - 84.0).abs() < 0.01);
    }

    #[test]
    fn integer_scale_fixed_clamps_down_when_requested_factor_does_not_fit() {
        let layout = compute_integer_scale_layout(
            glam::UVec2::new(640, 576),
            glam::UVec2::new(160, 144),
            IntegerScaleFactor::Fixed(8),
        );

        assert!((layout.resolved_scale - 4.0).abs() < 0.01);
        assert!((layout.viewport_rect.x - 0.0).abs() < 0.01);
        assert!((layout.viewport_rect.y - 0.0).abs() < 0.01);
    }

    #[test]
    fn integer_scale_falls_back_to_fractional_fit_when_window_is_smaller_than_base() {
        let layout = compute_integer_scale_layout(
            glam::UVec2::new(80, 72),
            glam::UVec2::new(160, 144),
            IntegerScaleFactor::Auto,
        );

        assert!((layout.resolved_scale - 0.5).abs() < 0.01);
        assert!((layout.viewport_rect.x - 0.0).abs() < 0.01);
        assert!((layout.viewport_rect.y - 0.0).abs() < 0.01);
    }

    #[test]
    fn tiny_window_sizes_are_sanitized() {
        let layout = compute_layout_for_mode(
            glam::UVec2::ZERO,
            glam::UVec2::ZERO,
            RuntimeViewportMode::IntegerScale {
                factor: IntegerScaleFactor::Auto,
            },
        );

        assert_eq!(layout.target_window_size, glam::UVec2::ONE);
        assert_eq!(layout.logical_viewport_size, glam::UVec2::ONE);
        assert!(layout.viewport_rect.width > 0.0);
        assert!(layout.viewport_rect.height > 0.0);
    }
}
