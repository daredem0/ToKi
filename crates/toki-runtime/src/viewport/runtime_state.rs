use toki_core::project_runtime::RuntimeViewportMode;

use super::presentation::{resolve_viewport_presentation, ViewportPresentation};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectiveRuntimeViewport {
    pub window_size: glam::UVec2,
    pub base_viewport_size: glam::UVec2,
    pub mode: RuntimeViewportMode,
    pub presentation: ViewportPresentation,
}

impl EffectiveRuntimeViewport {
    pub fn world_viewport_size(&self) -> glam::UVec2 {
        self.presentation.layout.logical_viewport_size
    }
}

pub fn resolve_effective_runtime_viewport(
    window_size: glam::UVec2,
    base_viewport_size: glam::UVec2,
    mode: RuntimeViewportMode,
) -> EffectiveRuntimeViewport {
    EffectiveRuntimeViewport {
        window_size,
        base_viewport_size,
        mode,
        presentation: resolve_viewport_presentation(window_size, base_viewport_size, mode),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_effective_runtime_viewport;
    use toki_core::project_runtime::{IntegerScaleFactor, RuntimeViewportMode};

    #[test]
    fn fixed_modes_preserve_base_world_viewport_size() {
        let viewport = resolve_effective_runtime_viewport(
            glam::UVec2::new(800, 600),
            glam::UVec2::new(160, 144),
            RuntimeViewportMode::IntegerScale {
                factor: IntegerScaleFactor::Auto,
            },
        );

        assert_eq!(viewport.world_viewport_size(), glam::UVec2::new(160, 144));
    }

    #[test]
    fn window_fill_uses_dynamic_world_viewport_size() {
        let viewport = resolve_effective_runtime_viewport(
            glam::UVec2::new(320, 144),
            glam::UVec2::new(160, 144),
            RuntimeViewportMode::WindowFill { zoom_percent: 100 },
        );

        assert_eq!(viewport.presentation.layout.viewport_rect.x, 0.0);
        assert_eq!(viewport.presentation.layout.viewport_rect.y, 0.0);
        assert_eq!(viewport.world_viewport_size(), glam::UVec2::new(320, 144));
    }
}
