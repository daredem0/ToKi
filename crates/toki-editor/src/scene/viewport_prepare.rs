use super::*;
use crate::ui::editor_ui::PlacementPreviewVisual;
use toki_core::sprite_render::{
    collect_map_object_sprite_render_requests, format_sprite_resolve_failure, SpriteRenderOrigin,
};

impl SceneViewport {
    pub(super) fn prepare_scene_data(
        &mut self,
        project_path: Option<&std::path::Path>,
        project_assets: &ProjectAssets,
        overlay_data: &ViewportOverlayData,
    ) -> SceneData {
        tracing::trace!("Preparing scene data for rendering...");

        let mut scene_data = SceneData::default();

        self.prepare_tilemap_data(&mut scene_data, project_path);
        self.prepare_sprite_data(&mut scene_data, project_path, project_assets);
        self.prepare_tilemap_object_data(&mut scene_data, project_path, project_assets);
        self.prepare_preview_sprite_data(&mut scene_data, overlay_data.placement_preview.clone());
        self.prepare_drag_preview_sprite_data(
            &mut scene_data,
            project_path,
            project_assets,
            Some(overlay_data.drag_preview_sprites.as_slice()),
        );
        self.prepare_overlay_sprite_data(
            &mut scene_data,
            Some(overlay_data.overlay_sprites.as_slice()),
        );
        self.prepare_overlay_rect_data(
            &mut scene_data,
            Some(overlay_data.overlay_rects.as_slice()),
        );
        self.prepare_overlay_line_data(
            &mut scene_data,
            Some(overlay_data.overlay_lines.as_slice()),
        );
        self.prepare_debug_shapes(&mut scene_data);

        tracing::trace!(
            "Scene data prepared: tilemap={}, atlas={}, sprites={}, debug_shapes={}, overlay_shapes={}",
            scene_data.tilemap.is_some(),
            scene_data.atlas.is_some(),
            scene_data.sprites.len(),
            scene_data.debug_shapes.len(),
            scene_data.overlay_shapes.len()
        );

        scene_data
    }

    pub(super) fn prepare_tilemap_object_data(
        &mut self,
        scene_data: &mut SceneData,
        project_path: Option<&std::path::Path>,
        project_assets: &ProjectAssets,
    ) {
        let Some(tilemap) = self.tilemap.as_ref().cloned() else {
            return;
        };
        if tilemap.objects.is_empty() {
            return;
        }

        let requests = collect_map_object_sprite_render_requests(&tilemap);
        self.resolve_sprite_requests_into_scene_data(
            scene_data,
            project_path,
            project_assets,
            &requests,
        );
    }

    pub(super) fn prepare_tilemap_data(
        &mut self,
        scene_data: &mut SceneData,
        project_path: Option<&std::path::Path>,
    ) {
        let Some(tilemap) = self.tilemap.as_ref().cloned() else {
            tracing::trace!("No tilemap found in scene viewport");
            return;
        };

        tracing::debug!(
            "Found tilemap: size={}x{}, atlas={}",
            tilemap.size.x,
            tilemap.size.y,
            tilemap.atlas.display()
        );
        scene_data.tilemap = Some(tilemap.clone());

        let Some(project_path) = project_path else {
            tracing::warn!("No project path provided for atlas loading");
            return;
        };

        tracing::debug!(
            "Loading atlas for tilemap from project path: {}",
            project_path.display()
        );
        match self.load_atlas_for_tilemap(&tilemap.atlas.to_string_lossy(), project_path) {
            Ok(atlas) => {
                tracing::trace!("Successfully loaded atlas with {} tiles", atlas.tiles.len());
                let texture_size = atlas.image_size().unwrap_or(glam::UVec2::new(64, 8));
                tracing::trace!(
                    "Calculated atlas texture size: {}x{}",
                    texture_size.x,
                    texture_size.y
                );
                scene_data.atlas = Some(atlas);
                scene_data.texture_size = texture_size;
            }
            Err(error) => {
                tracing::error!("Failed to load atlas: {}", error);
            }
        }
    }

    pub(super) fn prepare_sprite_data(
        &mut self,
        scene_data: &mut SceneData,
        project_path: Option<&std::path::Path>,
        project_assets: &ProjectAssets,
    ) {
        let requests = self
            .game_state
            .get_sprite_render_requests()
            .into_iter()
            .filter(|request| match request.origin {
                SpriteRenderOrigin::AnimatedEntity(entity_id)
                | SpriteRenderOrigin::StaticEntity(entity_id)
                | SpriteRenderOrigin::Projectile(entity_id) => {
                    !self.suppressed_entity_ids.contains(&entity_id)
                }
                SpriteRenderOrigin::MapObject { .. } => true,
            })
            .collect::<Vec<_>>();

        tracing::trace!(
            "Starting sprite rendering for {} logical sprite requests",
            requests.len()
        );

        if requests.is_empty() {
            tracing::warn!("No sprite render requests found - no sprites will be rendered");
            return;
        }

        self.resolve_sprite_requests_into_scene_data(
            scene_data,
            project_path,
            project_assets,
            &requests,
        );
    }

    fn resolve_sprite_requests_into_scene_data(
        &mut self,
        scene_data: &mut SceneData,
        project_path: Option<&std::path::Path>,
        project_assets: &ProjectAssets,
        requests: &[toki_core::sprite_render::SpriteRenderRequest],
    ) {
        let (sprites, failures) =
            self.resolve_sprite_requests_into_instances(project_assets, project_path, requests);

        for failure in failures {
            tracing::warn!(
                "Editor viewport: {}",
                format_sprite_resolve_failure(&failure.origin, &failure.error)
            );
        }

        scene_data.sprites.extend(sprites);
    }

    pub(super) fn prepare_preview_sprite_data(
        &mut self,
        scene_data: &mut SceneData,
        preview_data: Option<(glam::Vec2, PlacementPreviewVisual, bool)>,
    ) {
        let Some((preview_position, cached_visual, is_valid)) = preview_data else {
            return;
        };
        let render_position_i32 = world_to_i32_floor(preview_position);

        let preview_sprite = toki_render::SpriteInstance {
            frame: cached_visual.frame,
            position: render_position_i32,
            size: cached_visual.size,
            texture_path: cached_visual.texture_path,
            flip_x: false,
        };

        scene_data.sprites.push(preview_sprite);

        let outline_color = if is_valid {
            [0.0, 1.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0, 1.0]
        };

        let outline_shape = toki_render::OverlayShape {
            shape_type: toki_render::OverlayShapeType::Rectangle,
            position: glam::Vec2::new(render_position_i32.x as f32, render_position_i32.y as f32),
            size: glam::Vec2::new(cached_visual.size.x as f32, cached_visual.size.y as f32),
            color: outline_color,
        };
        scene_data.overlay_shapes.push(outline_shape);
    }

    pub(super) fn prepare_drag_preview_sprite_data(
        &mut self,
        scene_data: &mut SceneData,
        project_path: Option<&std::path::Path>,
        project_assets: &ProjectAssets,
        drag_preview_data: Option<&[DragPreviewSprite]>,
    ) {
        let Some(drag_preview_data) = drag_preview_data else {
            return;
        };
        let sprite_requests = self.game_state.get_sprite_render_requests();

        for preview in drag_preview_data {
            let Some(entity) = self
                .game_state
                .entity_manager()
                .get_entity(preview.entity_id)
            else {
                continue;
            };

            let entity_size = entity.size;
            let Some(mut request) = sprite_requests
                .iter()
                .find(|request| match request.origin {
                    SpriteRenderOrigin::AnimatedEntity(entity_id)
                    | SpriteRenderOrigin::StaticEntity(entity_id)
                    | SpriteRenderOrigin::Projectile(entity_id) => entity_id == preview.entity_id,
                    SpriteRenderOrigin::MapObject { .. } => false,
                })
                .cloned()
            else {
                continue;
            };
            request.position = preview.world_position;
            request.size = toki_core::sprite_render::SpriteRenderSize::Explicit(entity_size);
            self.resolve_sprite_requests_into_scene_data(
                scene_data,
                project_path,
                project_assets,
                &[request],
            );

            let outline_color = if preview.is_valid {
                [0.0, 1.0, 0.0, 1.0]
            } else {
                [1.0, 0.0, 0.0, 1.0]
            };
            scene_data.overlay_shapes.push(toki_render::OverlayShape {
                shape_type: toki_render::OverlayShapeType::Rectangle,
                position: glam::Vec2::new(
                    preview.world_position.x as f32,
                    preview.world_position.y as f32,
                ),
                size: glam::Vec2::new(entity_size.x as f32, entity_size.y as f32),
                color: outline_color,
            });
        }
    }

    pub(super) fn prepare_overlay_sprite_data(
        &mut self,
        scene_data: &mut SceneData,
        overlay_sprites: Option<&[OverlaySpriteInstance]>,
    ) {
        let Some(overlay_sprites) = overlay_sprites else {
            return;
        };

        for sprite in overlay_sprites {
            scene_data.sprites.push(toki_render::SpriteInstance {
                frame: sprite.visual.frame,
                position: sprite.world_position,
                size: sprite.visual.size,
                texture_path: sprite.visual.texture_path.clone(),
                flip_x: false,
            });
        }
    }

    pub(super) fn prepare_overlay_rect_data(
        &mut self,
        scene_data: &mut SceneData,
        overlay_rects: Option<&[OverlayRectInstance]>,
    ) {
        let Some(overlay_rects) = overlay_rects else {
            return;
        };

        for rect in overlay_rects {
            scene_data.overlay_shapes.push(toki_render::OverlayShape {
                shape_type: toki_render::OverlayShapeType::Rectangle,
                position: rect.position,
                size: rect.size,
                color: rect.color,
            });
        }
    }

    pub(super) fn prepare_overlay_line_data(
        &mut self,
        scene_data: &mut SceneData,
        overlay_lines: Option<&[OverlayLineInstance]>,
    ) {
        let Some(overlay_lines) = overlay_lines else {
            return;
        };

        for line in overlay_lines {
            scene_data.overlay_shapes.push(toki_render::OverlayShape {
                shape_type: toki_render::OverlayShapeType::Line {
                    end: line.end,
                    thickness: line.thickness,
                },
                position: line.start,
                size: glam::Vec2::ZERO,
                color: line.color,
            });
        }
    }

    pub(super) fn prepare_debug_shapes(&mut self, scene_data: &mut SceneData) {
        if !self.game_state.is_debug_collision_rendering_enabled() {
            return;
        }

        tracing::trace!("Debug collision rendering enabled - adding debug shapes");

        self.add_entity_debug_shapes(scene_data);
        self.add_tile_debug_shapes(scene_data);

        tracing::trace!("Added {} debug shapes total", scene_data.debug_shapes.len());
    }

    pub(super) fn add_entity_debug_shapes(&mut self, scene_data: &mut SceneData) {
        let entity_color = [1.0, 0.0, 0.0, 0.8];
        let trigger_tile_color = [1.0, 1.0, 0.0, 0.6];

        let renderable_entities = self.game_state.get_renderable_entities();
        for (entity_id, position, size) in renderable_entities {
            let debug_shape = toki_render::DebugShape {
                shape_type: toki_render::DebugShapeType::Rectangle,
                position: position.as_vec2(),
                size: size.as_vec2(),
                color: [0.0, 1.0, 0.0, 0.5],
            };
            scene_data.debug_shapes.push(debug_shape);
            tracing::trace!(
                "Added entity bounds for entity {} at ({}, {}) with size {}x{}",
                entity_id,
                position.x,
                position.y,
                size.x,
                size.y
            );
        }

        let entity_boxes = self.game_state.get_entity_collision_boxes();
        for (pos, size, is_trigger) in entity_boxes {
            let color = if is_trigger {
                trigger_tile_color
            } else {
                entity_color
            };

            let debug_shape = toki_render::DebugShape {
                shape_type: toki_render::DebugShapeType::Rectangle,
                position: pos.as_vec2(),
                size: size.as_vec2(),
                color,
            };
            scene_data.debug_shapes.push(debug_shape);
            tracing::trace!(
                "Added entity collision box at ({}, {}) with size {}x{}",
                pos.x,
                pos.y,
                size.x,
                size.y
            );
        }
    }

    pub(super) fn add_tile_debug_shapes(&mut self, scene_data: &mut SceneData) {
        let Some((tilemap, atlas)) = scene_data.tilemap.as_ref().zip(scene_data.atlas.as_ref())
        else {
            return;
        };

        let solid_tile_color = [0.0, 0.0, 1.0, 0.6];
        let trigger_tile_color = [1.0, 1.0, 0.0, 0.6];

        let solid_tiles = self.game_state.get_solid_tile_positions(tilemap, atlas);
        for (tile_x, tile_y) in solid_tiles {
            let world_pos = glam::Vec2::new(
                (tile_x * tilemap.tile_size.x) as f32,
                (tile_y * tilemap.tile_size.y) as f32,
            );

            let debug_shape = toki_render::DebugShape {
                shape_type: toki_render::DebugShapeType::Rectangle,
                position: world_pos,
                size: tilemap.tile_size.as_vec2(),
                color: solid_tile_color,
            };
            scene_data.debug_shapes.push(debug_shape);
        }

        let trigger_tiles = self.game_state.get_trigger_tile_positions(tilemap, atlas);
        for (tile_x, tile_y) in trigger_tiles {
            let world_pos = glam::Vec2::new(
                (tile_x * tilemap.tile_size.x) as f32,
                (tile_y * tilemap.tile_size.y) as f32,
            );

            let debug_shape = toki_render::DebugShape {
                shape_type: toki_render::DebugShapeType::Rectangle,
                position: world_pos,
                size: tilemap.tile_size.as_vec2(),
                color: trigger_tile_color,
            };
            scene_data.debug_shapes.push(debug_shape);
        }
    }
}
