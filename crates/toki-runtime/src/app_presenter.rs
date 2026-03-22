use toki_core::sprite_render::{
    collect_map_object_sprite_render_requests, format_sprite_resolve_failure,
    resolve_sprite_render_requests, sort_sprite_render_requests,
};
use toki_core::text::{TextAnchor, TextItem, TextStyle, TextWeight};
use toki_core::game::GroundShadow;

use crate::systems::{GameManager, PerformanceMonitor, RenderingSystem, ResourceManager};

use super::{RuntimeDisplayOptions, SceneTransitionController};

#[derive(Debug, Clone, PartialEq)]
struct GroundShadowBand {
    position: glam::Vec2,
    size: glam::Vec2,
    color: [f32; 4],
}

pub(super) struct WorldFramePresenter<'a> {
    game_system: &'a GameManager,
    resources: &'a mut ResourceManager,
    rendering: &'a mut RenderingSystem,
    display: &'a RuntimeDisplayOptions,
    performance: &'a PerformanceMonitor,
}

impl<'a> WorldFramePresenter<'a> {
    pub(super) fn new(
        game_system: &'a GameManager,
        resources: &'a mut ResourceManager,
        rendering: &'a mut RenderingSystem,
        display: &'a RuntimeDisplayOptions,
        performance: &'a PerformanceMonitor,
    ) -> Self {
        Self {
            game_system,
            resources,
            rendering,
            display,
            performance,
        }
    }

    pub(super) fn render_world_frame(&mut self) {
        self.rendering.clear_sprites();
        self.rendering.clear_text_items();
        self.rendering.clear_world_underlay_shapes();
        if self.display.show_ground_shadows {
            self.render_ground_shadows();
        }
        self.rendering.finalize_world_underlay_shapes();
        self.render_world_sprites();

        self.rendering.clear_debug_shapes();
        if self.display.show_entity_health_bars {
            self.render_entity_health_bars();
        }
        if self.game_system.is_debug_collision_rendering_enabled() {
            self.render_debug_collision_overlay();
        }
        self.rendering.finalize_debug_shapes();
        self.rendering.clear_ui_shapes();

        if let Some(stats_line) = self.performance.stats_line() {
            let hud_style = TextStyle {
                font_family: "Sans".to_string(),
                size_px: 14.0,
                weight: TextWeight::Bold,
                ..TextStyle::default()
            };
            let hud_text =
                TextItem::new_screen(stats_line, glam::Vec2::new(8.0, 8.0), hud_style)
                    .with_anchor(TextAnchor::TopLeft)
                    .with_layer(1);
            self.rendering.add_text_item(hud_text);
        }
    }

    fn render_world_sprites(&mut self) {
        let mut requests = self.game_system.get_sprite_render_requests();
        requests.extend(collect_map_object_sprite_render_requests(
            self.resources.get_tilemap(),
        ));
        sort_sprite_render_requests(&mut requests);

        let (resolved, failures) = resolve_sprite_render_requests(self.resources, &requests);
        for failure in failures {
            tracing::warn!(
                "{}",
                format_sprite_resolve_failure(&failure.origin, &failure.error)
            );
        }
        for sprite in resolved {
            self.rendering.add_resolved_sprite(&sprite);
        }
    }

    fn render_ground_shadows(&mut self) {
        for shadow in self.game_system.get_entity_ground_shadows() {
            for band in ground_shadow_bands(&shadow) {
                self.rendering.add_filled_world_underlay_rect(
                    band.position.x,
                    band.position.y,
                    band.size.x,
                    band.size.y,
                    band.color,
                );
            }
        }
    }

    fn render_entity_health_bars(&mut self) {
        for health_bar in self.game_system.get_entity_health_bars() {
            let bar_width = health_bar.size.x.max(16) as f32;
            let bar_height = 3.0;
            let bar_x = health_bar.position.x as f32;
            let bar_y = health_bar.position.y as f32 - 6.0;
            let fill_ratio = (health_bar.current as f32 / health_bar.max as f32).clamp(0.0, 1.0);
            let fill_color = health_bar_fill_color(fill_ratio);

            self.rendering.add_filled_debug_rect(
                bar_x,
                bar_y,
                bar_width,
                bar_height,
                [0.1, 0.1, 0.1, 0.8],
            );
            if fill_ratio > 0.0 {
                self.rendering.add_filled_debug_rect(
                    bar_x,
                    bar_y,
                    (bar_width * fill_ratio).max(1.0),
                    bar_height,
                    fill_color,
                );
            }
            self.rendering.add_debug_rect(
                bar_x,
                bar_y,
                bar_width,
                bar_height,
                [0.0, 0.0, 0.0, 1.0],
            );
        }
    }

    fn render_debug_collision_overlay(&mut self) {
        let entity_boxes = self.game_system.get_entity_collision_boxes();
        let solid_tiles = self.game_system.get_solid_tile_positions(
            self.resources.get_tilemap(),
            self.resources.get_terrain_atlas(),
        );
        let trigger_tiles = self.game_system.get_trigger_tile_positions(
            self.resources.get_tilemap(),
            self.resources.get_terrain_atlas(),
        );

        let entity_color = [1.0, 0.0, 0.0, 0.8];
        let solid_tile_color = [0.0, 0.0, 1.0, 0.6];
        let trigger_tile_color = [1.0, 1.0, 0.0, 0.6];

        for (pos, size, is_trigger) in entity_boxes {
            let color = if is_trigger {
                trigger_tile_color
            } else {
                entity_color
            };
            self.rendering.add_debug_rect(
                pos.x as f32,
                pos.y as f32,
                size.x as f32,
                size.y as f32,
                color,
            );
        }

        let tilemap = self.resources.get_tilemap();
        for (tile_x, tile_y) in solid_tiles {
            let world_x = tile_x * tilemap.tile_size.x;
            let world_y = tile_y * tilemap.tile_size.y;
            self.rendering.add_debug_rect(
                world_x as f32,
                world_y as f32,
                tilemap.tile_size.x as f32,
                tilemap.tile_size.y as f32,
                solid_tile_color,
            );
        }

        for (tile_x, tile_y) in trigger_tiles {
            let world_x = tile_x * tilemap.tile_size.x;
            let world_y = tile_y * tilemap.tile_size.y;
            self.rendering.add_debug_rect(
                world_x as f32,
                world_y as f32,
                tilemap.tile_size.x as f32,
                tilemap.tile_size.y as f32,
                trigger_tile_color,
            );
        }
    }

}

fn ground_shadow_bands(shadow: &GroundShadow) -> Vec<GroundShadowBand> {
    let band_count = if shadow.size.y >= 5.0 { 7 } else { 5 };
    let mut bands = Vec::with_capacity(band_count);
    let band_height = (shadow.size.y / band_count as f32).max(0.5);
    let center_x = shadow.position.x + shadow.size.x * 0.5;
    let mut current_y = shadow.position.y + 1.0;

    for index in 0..band_count {
        let normalized = if band_count == 1 {
            0.0
        } else {
            (index as f32 / (band_count - 1) as f32) * 2.0 - 1.0
        };
        let distance = normalized.abs();
        let width_scale = (1.0 - 0.34 * distance.powf(1.55)).clamp(0.60, 1.0);
        let alpha_scale = (1.0 - 0.18 * distance.powf(1.35)).clamp(0.76, 1.0);
        let band_width = (shadow.size.x * width_scale).round().max(2.0);
        let band_x = center_x - band_width * 0.5;
        let mut color = shadow.color;
        color[3] *= alpha_scale;

        bands.push(GroundShadowBand {
            position: glam::Vec2::new(band_x, current_y),
            size: glam::Vec2::new(band_width, band_height),
            color,
        });

        current_y += band_height;
    }

    bands
}

pub(super) fn health_bar_fill_color(fill_ratio: f32) -> [f32; 4] {
    if fill_ratio > 0.6 {
        [0.2, 0.85, 0.25, 0.95]
    } else if fill_ratio > 0.3 {
        [0.95, 0.8, 0.2, 0.95]
    } else {
        [0.9, 0.2, 0.2, 0.95]
    }
}

pub(super) fn render_scene_transition_overlay(
    rendering: &mut RenderingSystem,
    scene_transition: &SceneTransitionController,
) {
    let alpha = scene_transition.fade_alpha();
    if alpha <= f32::EPSILON {
        return;
    }

    let projection = rendering.projection_params();
    rendering.add_filled_ui_rect(
        0.0,
        0.0,
        projection.width as f32,
        projection.height as f32,
        [0.0, 0.0, 0.0, alpha.clamp(0.0, 1.0)],
    );
}

#[cfg(test)]
mod tests {
    use super::{ground_shadow_bands, health_bar_fill_color};
    use toki_core::game::GroundShadow;

    #[test]
    fn health_bar_fill_color_uses_expected_thresholds() {
        assert_eq!(health_bar_fill_color(0.8), [0.2, 0.85, 0.25, 0.95]);
        assert_eq!(health_bar_fill_color(0.5), [0.95, 0.8, 0.2, 0.95]);
        assert_eq!(health_bar_fill_color(0.1), [0.9, 0.2, 0.2, 0.95]);
    }

    #[test]
    fn ground_shadow_bands_create_center_widest_oval_shape() {
        let bands = ground_shadow_bands(&GroundShadow {
            position: glam::Vec2::new(12.0, 33.0),
            size: glam::Vec2::new(12.0, 3.0),
            color: [0.0, 0.0, 0.0, 0.28],
        });

        assert_eq!(bands.len(), 5);
        let center_band = &bands[2];
        assert_eq!(center_band.size.x, 12.0);
        assert!((bands[0].position.y - 34.0).abs() < 0.001);
        assert!(bands[0].size.x < bands[1].size.x);
        assert!(bands[1].size.x < center_band.size.x);
        assert_eq!(bands[0].size.x, bands[4].size.x);
        assert_eq!(bands[1].size.x, bands[3].size.x);
        assert!(bands[0].color[3] < bands[1].color[3]);
        assert!(bands[1].color[3] < center_band.color[3]);
        let total_height: f32 = bands.iter().map(|band| band.size.y).sum();
        assert!((total_height - 3.0).abs() < 0.001);
    }

    #[test]
    fn ground_shadow_bands_use_more_rows_for_taller_shadows() {
        let bands = ground_shadow_bands(&GroundShadow {
            position: glam::Vec2::new(20.0, 40.0),
            size: glam::Vec2::new(20.0, 5.0),
            color: [0.0, 0.0, 0.0, 0.3],
        });

        assert_eq!(bands.len(), 7);
        assert!((bands[0].position.y - 41.0).abs() < 0.001);
        let total_height: f32 = bands.iter().map(|band| band.size.y).sum();
        assert!((total_height - 5.0).abs() < 0.001);
        let center_band = &bands[3];
        assert_eq!(center_band.size.x, 20.0);
        assert!(bands[0].size.x < center_band.size.x);
        assert!(bands[6].size.x < center_band.size.x);
        assert!(bands[1].size.x <= bands[2].size.x);
        assert!(bands[4].size.x >= bands[5].size.x);
        assert_eq!(bands[0].size.x, bands[6].size.x);
        assert_eq!(bands[1].size.x, bands[5].size.x);
        assert_eq!(bands[2].size.x, bands[4].size.x);
    }
}
