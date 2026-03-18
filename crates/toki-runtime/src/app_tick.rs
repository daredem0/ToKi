use std::time::Instant;

use toki_core::camera::RuntimeState;
use toki_core::sprite_render::{
    collect_map_object_sprite_render_requests, format_sprite_resolve_failure,
    resolve_sprite_render_requests, sort_sprite_render_requests,
};
use toki_core::text::{TextAnchor, TextItem, TextStyle, TextWeight};
use toki_core::{EventHandler, GameUpdateResult};

use super::App;

impl App {
    pub(super) fn tick(&mut self) {
        self.tick_internal(None)
    }

    pub(super) fn tick_with_delta(&mut self, delta_ms: f32) {
        self.tick_internal(Some(delta_ms))
    }

    fn tick_internal(&mut self, delta_ms: Option<f32>) {
        let tick_start = Instant::now();
        tracing::trace!("TICK @ {:?}", tick_start);

        let world_bounds = glam::UVec2::new(
            self.resources.tilemap_size().x * self.resources.tilemap_tile_size().x,
            self.resources.tilemap_size().y * self.resources.tilemap_tile_size().y,
        );
        let game_result = if self.should_gate_gameplay_for_menu() {
            GameUpdateResult::new()
        } else if let Some(delta) = delta_ms {
            self.game_system.update_with_delta(
                delta,
                world_bounds,
                self.resources.get_tilemap(),
                self.resources.get_terrain_atlas(),
            )
        } else {
            self.game_system.update(
                world_bounds,
                self.resources.get_tilemap(),
                self.resources.get_terrain_atlas(),
            )
        };

        let listener_position = self
            .game_system
            .player_id()
            .map(|_| self.game_system.player_position());
        self.audio_system.set_listener_position(listener_position);

        for event in &game_result.events {
            self.audio_system.handle(event);
        }

        let player_moved = game_result.player_moved;
        let entities = self.game_system.entities_for_camera();
        let runtime = RuntimeState {
            entities: &entities,
        };
        let cam_changed = self.camera_system.update(&runtime, world_bounds) || player_moved;

        if self.rendering.has_gpu() {
            if cam_changed {
                let view = self.camera_system.view_matrix();
                self.rendering.update_projection(view);

                if self
                    .camera_system
                    .update_chunk_cache(self.resources.get_tilemap())
                {
                    let atlas_size = self.resources.terrain_image_size().unwrap();
                    let verts = self.resources.get_tilemap().generate_vertices_for_chunks(
                        self.resources.get_terrain_atlas(),
                        atlas_size,
                        self.camera_system.cached_visible_chunks(),
                    );
                    self.rendering.update_tilemap_vertices(&verts);
                }
            }
            self.rendering.clear_sprites();
            self.rendering.clear_text_items();
            self.render_world_sprites();

            self.rendering.clear_debug_shapes();
            if self.launch_options.display.show_entity_health_bars {
                self.render_entity_health_bars();
            }
            if self.game_system.is_debug_collision_rendering_enabled() {
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

            self.render_runtime_menu_overlay();
            self.rendering.finalize_ui_shapes();
        }

        self.platform.request_redraw();
    }

    fn render_entity_health_bars(&mut self) {
        for health_bar in self.game_system.get_entity_health_bars() {
            let bar_width = health_bar.size.x.max(16) as f32;
            let bar_height = 3.0;
            let bar_x = health_bar.position.x as f32;
            let bar_y = health_bar.position.y as f32 - 6.0;
            let fill_ratio = (health_bar.current as f32 / health_bar.max as f32).clamp(0.0, 1.0);
            let fill_color = Self::health_bar_fill_color(fill_ratio);

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

    fn health_bar_fill_color(fill_ratio: f32) -> [f32; 4] {
        if fill_ratio > 0.6 {
            [0.2, 0.85, 0.25, 0.95]
        } else if fill_ratio > 0.3 {
            [0.95, 0.8, 0.2, 0.95]
        } else {
            [0.9, 0.2, 0.2, 0.95]
        }
    }

    pub(super) fn refresh_tilemap_vertices_for_current_camera(&mut self) {
        if !self.rendering.has_gpu() {
            return;
        }

        self.camera_system
            .update_chunk_cache(self.resources.get_tilemap());
        let atlas_size = self.resources.terrain_image_size().unwrap();
        let verts = self.resources.get_tilemap().generate_vertices_for_chunks(
            self.resources.get_terrain_atlas(),
            atlas_size,
            self.camera_system.cached_visible_chunks(),
        );
        self.rendering.update_tilemap_vertices(&verts);
    }

    fn render_world_sprites(&mut self) {
        let mut requests = self.game_system.get_sprite_render_requests();
        requests.extend(collect_map_object_sprite_render_requests(
            self.resources.get_tilemap(),
        ));
        sort_sprite_render_requests(&mut requests);

        let (resolved, failures) = resolve_sprite_render_requests(&mut self.resources, &requests);
        for failure in failures {
            tracing::warn!("{}", format_sprite_resolve_failure(&failure.origin, &failure.error));
        }
        for sprite in resolved {
            self.rendering.add_resolved_sprite(&sprite);
        }
    }
}
