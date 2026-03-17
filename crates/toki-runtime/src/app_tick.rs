use std::time::Instant;

use toki_core::camera::RuntimeState;
use toki_core::text::{TextAnchor, TextItem, TextStyle, TextWeight};
use toki_core::{EventHandler, GameUpdateResult};

use super::App;

impl App {
    pub(super) fn tick(&mut self) {
        let tick_start = Instant::now();
        tracing::trace!("TICK @ {:?}", tick_start);

        let world_bounds = glam::UVec2::new(
            self.resources.tilemap_size().x * self.resources.tilemap_tile_size().x,
            self.resources.tilemap_size().y * self.resources.tilemap_tile_size().y,
        );
        let game_result = if self.should_gate_gameplay_for_menu() {
            GameUpdateResult::new()
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

            let renderable_entities = self.game_system.get_renderable_entities();
            for (entity_id, position, size) in renderable_entities {
                let Some(atlas_name) = self.game_system.get_entity_current_atlas_name(entity_id)
                else {
                    continue;
                };
                let Some(sprite_atlas) = self.resources.get_sprite_atlas(&atlas_name) else {
                    tracing::warn!(
                        "Entity {} requested missing sprite atlas '{}'",
                        entity_id,
                        atlas_name
                    );
                    continue;
                };
                let texture_size = sprite_atlas
                    .image_size()
                    .unwrap_or(glam::UVec2::new(64, 16));
                if let Some(frame) =
                    self.game_system
                        .get_entity_sprite_frame(entity_id, sprite_atlas, texture_size)
                {
                    let flip_x = self.game_system.get_entity_sprite_flip_x(entity_id);
                    if let Some(texture_path) =
                        self.resources.get_sprite_texture_path(&atlas_name).cloned()
                    {
                        self.rendering.add_sprite_with_texture(
                            texture_path,
                            frame,
                            position,
                            size,
                            flip_x,
                        );
                    } else {
                        self.rendering.add_sprite(frame, position, size, flip_x);
                    }
                }
            }

            for entity in self.game_system.get_static_entity_renderables() {
                let Some(object_sheet) = self.resources.get_object_sheet(&entity.sheet) else {
                    tracing::warn!(
                        "Entity {} requested missing object sheet '{}'",
                        entity.entity_id,
                        entity.sheet
                    );
                    continue;
                };
                let texture_size = object_sheet
                    .image_size()
                    .unwrap_or(glam::UVec2::new(16, 16));
                let Some(uv_rect) = object_sheet.get_object_uvs(&entity.object_name, texture_size)
                else {
                    tracing::warn!(
                        "Entity {} requested missing object '{}' in sheet '{}'",
                        entity.entity_id,
                        entity.object_name,
                        entity.sheet
                    );
                    continue;
                };
                let frame = toki_core::sprite::SpriteFrame {
                    u0: uv_rect[0],
                    v0: uv_rect[1],
                    u1: uv_rect[2],
                    v1: uv_rect[3],
                };
                if let Some(texture_path) = self
                    .resources
                    .get_object_texture_path(&entity.sheet)
                    .cloned()
                {
                    self.rendering.add_sprite_with_texture(
                        texture_path,
                        frame,
                        entity.position,
                        entity.size,
                        false,
                    );
                } else {
                    self.rendering
                        .add_sprite(frame, entity.position, entity.size, false);
                }
            }

            for projectile in self.game_system.get_projectile_renderables() {
                let Some(object_sheet) = self.resources.get_object_sheet(&projectile.sheet) else {
                    tracing::warn!(
                        "Projectile {} requested missing object sheet '{}'",
                        projectile.entity_id,
                        projectile.sheet
                    );
                    continue;
                };
                let texture_size = object_sheet
                    .image_size()
                    .unwrap_or(glam::UVec2::new(16, 16));
                let Some(uv_rect) =
                    object_sheet.get_object_uvs(&projectile.object_name, texture_size)
                else {
                    tracing::warn!(
                        "Projectile {} requested missing object '{}' in sheet '{}'",
                        projectile.entity_id,
                        projectile.object_name,
                        projectile.sheet
                    );
                    continue;
                };
                let frame = toki_core::sprite::SpriteFrame {
                    u0: uv_rect[0],
                    v0: uv_rect[1],
                    u1: uv_rect[2],
                    v1: uv_rect[3],
                };
                if let Some(texture_path) = self
                    .resources
                    .get_object_texture_path(&projectile.sheet)
                    .cloned()
                {
                    self.rendering.add_sprite_with_texture(
                        texture_path,
                        frame,
                        projectile.position,
                        projectile.size,
                        false,
                    );
                } else {
                    self.rendering
                        .add_sprite(frame, projectile.position, projectile.size, false);
                }
            }

            for object in &self.resources.get_tilemap().objects {
                if !object.visible {
                    continue;
                }
                let sheet_name = object
                    .sheet
                    .file_name()
                    .and_then(|name| name.to_str())
                    .or_else(|| object.sheet.to_str());
                let Some(sheet_name) = sheet_name else {
                    continue;
                };
                let Some(object_sheet) = self.resources.get_object_sheet(sheet_name) else {
                    tracing::warn!("Map object requested missing object sheet '{}'", sheet_name);
                    continue;
                };
                let texture_size = object_sheet
                    .image_size()
                    .unwrap_or(glam::UVec2::new(16, 16));
                let Some(uv_rect) = object_sheet.get_object_uvs(&object.object_name, texture_size)
                else {
                    tracing::warn!(
                        "Map object '{}' missing from object sheet '{}'",
                        object.object_name,
                        sheet_name
                    );
                    continue;
                };
                let Some(rect) = object_sheet.get_object_rect(&object.object_name) else {
                    continue;
                };
                let frame = toki_core::sprite::SpriteFrame {
                    u0: uv_rect[0],
                    v0: uv_rect[1],
                    u1: uv_rect[2],
                    v1: uv_rect[3],
                };
                let size = glam::UVec2::new(rect[2], rect[3]);
                let position = object.position.as_ivec2();
                if let Some(texture_path) =
                    self.resources.get_object_texture_path(sheet_name).cloned()
                {
                    self.rendering.add_sprite_with_texture(
                        texture_path,
                        frame,
                        position,
                        size,
                        false,
                    );
                } else {
                    self.rendering.add_sprite(frame, position, size, false);
                }
            }

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
}
