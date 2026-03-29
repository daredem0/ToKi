use toki_core::math::projection::screen_space_projection;
use toki_core::text::TextItem;
use toki_core::project_runtime::RuntimeViewportMode;
use toki_core::ui::{UiComposition, UiRect};

use super::layout::{compute_layout_for_mode, ViewportLayout};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportPresentation {
    pub layout: ViewportLayout,
    pub projection: glam::Mat4,
}

impl ViewportPresentation {
    pub fn runtime_ui_scale_factor(&self) -> f32 {
        let surface_size = self.surface_viewport_size();
        let reference_size = glam::Vec2::new(
            self.layout.logical_viewport_size.x as f32 * 7.0,
            self.layout.logical_viewport_size.y as f32 * 7.0,
        );
        let size_ratio = (surface_size.x / reference_size.x)
            .min(surface_size.y / reference_size.y)
            .clamp(0.0, 1.0);

        0.06 + 0.94 * size_ratio.powf(1.15)
    }

    pub fn logical_viewport_size(&self) -> glam::Vec2 {
        glam::Vec2::new(
            self.layout.logical_viewport_size.x as f32,
            self.layout.logical_viewport_size.y as f32,
        )
    }

    pub fn surface_viewport_origin(&self) -> glam::Vec2 {
        glam::Vec2::new(self.layout.viewport_rect.x, self.layout.viewport_rect.y)
    }

    pub fn surface_viewport_size(&self) -> glam::Vec2 {
        glam::Vec2::new(
            self.layout.viewport_rect.width,
            self.layout.viewport_rect.height,
        )
    }

    pub fn logical_to_surface_position(&self, position: glam::Vec2) -> glam::Vec2 {
        glam::Vec2::new(
            self.layout.viewport_rect.x + position.x * self.layout.resolved_scale,
            self.layout.viewport_rect.y + position.y * self.layout.resolved_scale,
        )
    }

    pub fn surface_to_viewport_local_position(&self, position: glam::Vec2) -> Option<glam::Vec2> {
        let rect = self.layout.viewport_rect;
        let within_x = position.x >= rect.x && position.x <= rect.x + rect.width;
        let within_y = position.y >= rect.y && position.y <= rect.y + rect.height;
        if !(within_x && within_y) {
            return None;
        }

        Some(glam::Vec2::new(
            position.x - rect.x,
            position.y - rect.y,
        ))
    }

    pub fn offset_surface_rect(&self, rect: UiRect) -> UiRect {
        let origin = self.surface_viewport_origin();
        UiRect {
            x: origin.x + rect.x,
            y: origin.y + rect.y,
            width: rect.width,
            height: rect.height,
        }
    }

    pub fn offset_surface_text_item(&self, item: &TextItem) -> TextItem {
        let mut transformed = item.clone();
        let origin = self.surface_viewport_origin();
        transformed.position += origin;
        transformed
    }

    pub fn offset_surface_ui_composition(&self, composition: &UiComposition) -> UiComposition {
        let mut transformed = composition.clone();
        let origin = self.surface_viewport_origin();
        for block in &mut transformed.blocks {
            block.rect = self.offset_surface_rect(block.rect);
            if let Some(text) = block.text.as_mut() {
                text.position += origin;
            }
        }
        transformed
    }
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

#[cfg(test)]
mod tests {
    use super::resolve_fixed_viewport_presentation;
    use toki_core::project_runtime::{IntegerScaleFactor, RuntimeViewportMode};
    use toki_core::text::{TextAnchor, TextItem, TextStyle};
    use toki_core::ui::{UiBlock, UiComposition, UiRect, UiTextBlock};

    #[test]
    fn viewport_positions_are_scaled_and_centered_into_surface_space() {
        let presentation = resolve_fixed_viewport_presentation(
            glam::UVec2::new(800, 600),
            glam::UVec2::new(160, 144),
            RuntimeViewportMode::IntegerScale {
                factor: IntegerScaleFactor::Auto,
            },
        );

        let position = presentation.logical_to_surface_position(glam::Vec2::new(8.0, 8.0));
        assert_eq!(position, glam::Vec2::new(112.0, 44.0));
    }

    #[test]
    fn surface_positions_outside_viewport_are_rejected() {
        let presentation = resolve_fixed_viewport_presentation(
            glam::UVec2::new(800, 600),
            glam::UVec2::new(160, 144),
            RuntimeViewportMode::IntegerScale {
                factor: IntegerScaleFactor::Auto,
            },
        );

        assert!(presentation
            .surface_to_viewport_local_position(glam::Vec2::new(40.0, 40.0))
            .is_none());
        assert_eq!(
            presentation.surface_to_viewport_local_position(glam::Vec2::new(112.0, 44.0)),
            Some(glam::Vec2::new(32.0, 32.0))
        );
    }

    #[test]
    fn surface_text_and_ui_are_offset_into_the_letterboxed_viewport() {
        let presentation = resolve_fixed_viewport_presentation(
            glam::UVec2::new(800, 600),
            glam::UVec2::new(160, 144),
            RuntimeViewportMode::IntegerScale {
                factor: IntegerScaleFactor::Auto,
            },
        );

        let text = TextItem::new_screen(
            "HUD",
            glam::Vec2::new(8.0, 8.0),
            TextStyle {
                size_px: 10.0,
                ..TextStyle::default()
            },
        );
        let transformed_text = presentation.offset_surface_text_item(&text);
        assert_eq!(transformed_text.position, glam::Vec2::new(88.0, 20.0));
        assert!((transformed_text.style.size_px - 10.0).abs() < 0.01);

        let mut composition = UiComposition::default();
        composition.push(UiBlock {
            rect: UiRect {
                x: 10.0,
                y: 12.0,
                width: 20.0,
                height: 8.0,
            },
            fill_color: Some([0.0, 0.0, 0.0, 1.0]),
            border_color: Some([1.0, 1.0, 1.0, 1.0]),
            border_thickness: 1.0,
            text: Some(UiTextBlock {
                content: "Hello".to_string(),
                position: glam::Vec2::new(20.0, 20.0),
                anchor: TextAnchor::TopCenter,
                style: TextStyle {
                    size_px: 8.0,
                    ..TextStyle::default()
                },
                layer: 1,
            }),
        });
        let transformed = presentation.offset_surface_ui_composition(&composition);
        let block = &transformed.blocks[0];
        assert_eq!(block.rect.x, 90.0);
        assert_eq!(block.rect.y, 24.0);
        assert_eq!(block.rect.width, 20.0);
        assert_eq!(block.rect.height, 8.0);
        assert!((block.border_thickness - 1.0).abs() < 0.01);
        let text = block.text.as_ref().expect("text should exist");
        assert_eq!(text.position, glam::Vec2::new(100.0, 32.0));
        assert!((text.style.size_px - 8.0).abs() < 0.01);
    }

    #[test]
    fn runtime_ui_scale_factor_targets_scale_seven_as_full_size() {
        let scale_one = resolve_fixed_viewport_presentation(
            glam::UVec2::new(160, 144),
            glam::UVec2::new(160, 144),
            RuntimeViewportMode::IntegerScale {
                factor: IntegerScaleFactor::Fixed(1),
            },
        );
        let scale_three = resolve_fixed_viewport_presentation(
            glam::UVec2::new(480, 432),
            glam::UVec2::new(160, 144),
            RuntimeViewportMode::IntegerScale {
                factor: IntegerScaleFactor::Fixed(3),
            },
        );
        let scale_four = resolve_fixed_viewport_presentation(
            glam::UVec2::new(640, 576),
            glam::UVec2::new(160, 144),
            RuntimeViewportMode::IntegerScale {
                factor: IntegerScaleFactor::Fixed(4),
            },
        );
        let scale_seven = resolve_fixed_viewport_presentation(
            glam::UVec2::new(1120, 1008),
            glam::UVec2::new(160, 144),
            RuntimeViewportMode::IntegerScale {
                factor: IntegerScaleFactor::Fixed(7),
            },
        );

        assert!(scale_one.runtime_ui_scale_factor() > 0.13);
        assert!(scale_one.runtime_ui_scale_factor() < 0.19);
        assert!(scale_three.runtime_ui_scale_factor() > 0.38);
        assert!(scale_three.runtime_ui_scale_factor() < 0.45);
        assert!(scale_four.runtime_ui_scale_factor() > 0.5);
        assert!(scale_four.runtime_ui_scale_factor() < 0.59);
        assert!((scale_seven.runtime_ui_scale_factor() - 1.0).abs() < 0.01);
    }
}
