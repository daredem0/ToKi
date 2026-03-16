use std::path::{Path, PathBuf};
use std::time::Duration;

use toki_core::graphics::image::load_image_rgba8_from_bytes;
use toki_core::math::projection::ProjectionParameter;
use toki_core::text::{TextAnchor, TextItem, TextStyle, TextWeight};

use super::{
    first_existing_path, App, RuntimeLaunchOptions, RuntimeSplashOptions,
    COMMUNITY_SPLASH_BRANDING_TEXT, COMMUNITY_SPLASH_LOGO_PNG, COMMUNITY_SPLASH_MAX_DURATION_MS,
    COMMUNITY_SPLASH_MIN_DURATION_MS, COMMUNITY_SPLASH_VERSION_TEXT,
    SPLASH_BRANDING_VERSION_GAP_PX, SPLASH_LOGO_HEIGHT, SPLASH_LOGO_WIDTH,
    SPLASH_TEXT_HORIZONTAL_PADDING_PX, SPLASH_TEXT_LINE_HEIGHT_MULTIPLIER,
    SPLASH_VERSION_DEFAULT_SIZE_PX, SPLASH_VERSION_MIN_SIZE_PX,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SplashPolicy {
    Community,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ResolvedSplashConfig {
    pub(super) duration: Duration,
    pub(super) show_branding: bool,
}

impl SplashPolicy {
    pub(super) fn resolve(self, requested: &RuntimeSplashOptions) -> ResolvedSplashConfig {
        match self {
            Self::Community => {
                if !requested.show_branding {
                    tracing::warn!(
                        "Splash branding cannot be disabled in Community bundle; forcing branding ON"
                    );
                }
                let clamped_duration = requested.duration_ms.clamp(
                    COMMUNITY_SPLASH_MIN_DURATION_MS,
                    COMMUNITY_SPLASH_MAX_DURATION_MS,
                );
                ResolvedSplashConfig {
                    duration: Duration::from_millis(clamped_duration),
                    show_branding: true,
                }
            }
        }
    }
}

impl App {
    pub(super) fn projection_view_size(parameters: ProjectionParameter) -> glam::Vec2 {
        let aspect = parameters.width as f32 / parameters.height as f32;
        let desired_aspect = parameters.desired_width as f32 / parameters.desired_height as f32;

        if aspect > desired_aspect {
            let height = parameters.desired_height as f32;
            let width = height * aspect;
            glam::Vec2::new(width, height)
        } else {
            let width = parameters.desired_width as f32;
            let height = width / aspect;
            glam::Vec2::new(width, height)
        }
    }

    pub(super) fn centered_logo_origin_for_view(
        view_size: glam::Vec2,
        logo_size: glam::UVec2,
    ) -> glam::IVec2 {
        let x = ((view_size.x - logo_size.x as f32) * 0.5).floor() as i32;
        let y = ((view_size.y - logo_size.y as f32) * 0.5).floor() as i32;
        glam::IVec2::new(x, y)
    }

    pub(super) fn splash_branding_positions(
        view_size: glam::Vec2,
        splash_logo_loaded: bool,
        logo_origin: glam::IVec2,
        logo_size: glam::UVec2,
        branding_style: &TextStyle,
        version_style: &TextStyle,
    ) -> (glam::Vec2, glam::Vec2) {
        let branding_height = branding_style.size_px * SPLASH_TEXT_LINE_HEIGHT_MULTIPLIER;
        let version_height = version_style.size_px * SPLASH_TEXT_LINE_HEIGHT_MULTIPLIER;
        let total_block_height = branding_height + SPLASH_BRANDING_VERSION_GAP_PX + version_height;
        let max_branding_top = (view_size.y - total_block_height - 4.0).max(0.0);
        let branding_position = if splash_logo_loaded {
            glam::Vec2::new(
                view_size.x * 0.5,
                (logo_origin.y as f32 + logo_size.y as f32 + 8.0).min(max_branding_top),
            )
        } else {
            glam::Vec2::new(view_size.x * 0.5, (view_size.y * 0.5).min(max_branding_top))
        };
        let version_position = glam::Vec2::new(
            branding_position.x,
            branding_position.y + branding_height + SPLASH_BRANDING_VERSION_GAP_PX,
        );
        (branding_position, version_position)
    }

    pub(super) fn fitted_splash_version_style(view_width: f32, content: &str) -> TextStyle {
        let available_width = (view_width - SPLASH_TEXT_HORIZONTAL_PADDING_PX).max(1.0);
        let char_count = content.chars().count().max(1) as f32;
        let max_size_for_width = available_width / (char_count * 0.55);
        let size_px =
            max_size_for_width.clamp(SPLASH_VERSION_MIN_SIZE_PX, SPLASH_VERSION_DEFAULT_SIZE_PX);
        TextStyle {
            font_family: "Sans".to_string(),
            size_px,
            weight: TextWeight::Normal,
            ..TextStyle::default()
        }
    }

    pub(super) fn render_startup_splash(&mut self) {
        let logo_size = glam::UVec2::new(SPLASH_LOGO_WIDTH, SPLASH_LOGO_HEIGHT);
        let view_size = Self::projection_view_size(self.rendering.projection_params());
        let logo_origin = Self::centered_logo_origin_for_view(view_size, logo_size);
        self.rendering.update_projection(glam::Mat4::IDENTITY);
        self.rendering.set_tilemap_render_enabled(false);
        self.rendering.clear_sprites();
        self.rendering.clear_text_items();
        self.rendering.clear_debug_shapes();
        self.rendering.finalize_debug_shapes();
        if self.splash_logo_loaded {
            self.rendering.add_sprite(
                toki_core::sprite::SpriteFrame {
                    u0: 0.0,
                    v0: 0.0,
                    u1: 1.0,
                    v1: 1.0,
                },
                logo_origin,
                logo_size,
                false,
            );
        }
        if self.splash_config.show_branding {
            let branding_style = TextStyle {
                font_family: "Sans".to_string(),
                size_px: 16.0,
                weight: TextWeight::Bold,
                ..TextStyle::default()
            };
            let version_style =
                Self::fitted_splash_version_style(view_size.x, COMMUNITY_SPLASH_VERSION_TEXT);
            let (branding_position, version_position) = Self::splash_branding_positions(
                view_size,
                self.splash_logo_loaded,
                logo_origin,
                logo_size,
                &branding_style,
                &version_style,
            );
            self.rendering.add_text_item(
                TextItem::new_screen(
                    COMMUNITY_SPLASH_BRANDING_TEXT,
                    branding_position,
                    branding_style,
                )
                .with_anchor(TextAnchor::TopCenter)
                .with_layer(10),
            );
            self.rendering.add_text_item(
                TextItem::new_screen(
                    COMMUNITY_SPLASH_VERSION_TEXT,
                    version_position,
                    version_style,
                )
                .with_max_width((view_size.x - SPLASH_TEXT_HORIZONTAL_PADDING_PX).max(1.0))
                .with_anchor(TextAnchor::TopCenter)
                .with_layer(10),
            );
        }
        self.rendering.draw();
        self.platform.request_redraw();
    }

    pub(super) fn restore_runtime_sprite_texture_after_splash(&mut self) {
        if let Some(path) = &self.post_splash_sprite_texture_path {
            if let Err(error) = self.rendering.load_sprite_texture(path.clone()) {
                tracing::warn!(
                    "Failed to restore sprite texture '{}' after splash: {}",
                    path.display(),
                    error
                );
            }
            return;
        }

        tracing::warn!(
            "No post-splash sprite texture path available; keeping current sprite texture"
        );
    }

    pub(super) fn resolve_post_splash_sprite_texture_path(
        launch_options: &RuntimeLaunchOptions,
        content_root: Option<&Path>,
    ) -> Option<PathBuf> {
        if let Some(root) = content_root {
            let (_, sprite_texture) = Self::project_texture_paths(root);
            if sprite_texture.is_some() {
                return sprite_texture;
            }
        }

        if let Some(project_path) = &launch_options.project_path {
            let (_, sprite_texture) = Self::project_texture_paths(project_path);
            if sprite_texture.is_some() {
                return sprite_texture;
            }
        }

        first_existing_path(&[
            PathBuf::from("assets/creatures.png"),
            PathBuf::from("assets/sprites/creatures.png"),
        ])
    }

    pub(super) fn initialize_splash_resources(&mut self) {
        self.splash_config = self.splash_policy.resolve(&self.launch_options.splash);
        if let Ok(decoded_logo) = load_image_rgba8_from_bytes(COMMUNITY_SPLASH_LOGO_PNG) {
            if let Err(error) = self.rendering.load_sprite_texture_rgba8(&decoded_logo) {
                tracing::warn!(
                    "Failed to load embedded startup logo texture (splash will render branding-only): {}",
                    error
                );
                self.splash_logo_loaded = false;
            } else {
                self.splash_logo_loaded = true;
            }
        } else {
            tracing::warn!(
                "Failed to decode embedded startup logo bytes; splash will render branding-only"
            );
            self.splash_logo_loaded = false;
        }
    }
}
