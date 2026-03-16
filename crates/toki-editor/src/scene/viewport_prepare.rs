use super::*;
use crate::ui::editor_ui::PlacementPreviewVisual;

impl SceneViewport {
    pub(super) fn prepare_scene_data(
        &mut self,
        project_path: Option<&std::path::Path>,
        project_assets: &ProjectAssets,
        preview_data: Option<(glam::Vec2, PlacementPreviewVisual, bool)>,
        drag_preview_data: Option<&[DragPreviewSprite]>,
    ) -> SceneData {
        tracing::trace!("Preparing scene data for rendering...");

        let mut scene_data = SceneData::default();

        self.prepare_tilemap_data(&mut scene_data, project_path);
        self.prepare_tilemap_object_data(&mut scene_data, project_path, project_assets);
        self.prepare_sprite_data(&mut scene_data, project_path, project_assets);
        self.prepare_static_entity_sprite_data(&mut scene_data, project_assets);
        self.prepare_preview_sprite_data(&mut scene_data, preview_data);
        self.prepare_drag_preview_sprite_data(
            &mut scene_data,
            project_path,
            project_assets,
            drag_preview_data,
        );
        self.prepare_debug_shapes(&mut scene_data);

        tracing::trace!(
            "Scene data prepared: tilemap={}, atlas={}, sprites={}, debug_shapes={}",
            scene_data.tilemap.is_some(),
            scene_data.atlas.is_some(),
            scene_data.sprites.len(),
            scene_data.debug_shapes.len()
        );

        scene_data
    }

    pub(super) fn prepare_tilemap_object_data(
        &mut self,
        scene_data: &mut SceneData,
        _project_path: Option<&std::path::Path>,
        project_assets: &ProjectAssets,
    ) {
        let Some(tilemap) = self.scene_manager.tilemap().cloned() else {
            return;
        };
        if tilemap.objects.is_empty() {
            return;
        }

        for object in &tilemap.objects {
            if !object.visible {
                continue;
            }
            let Some(sprite_instance) =
                self.build_map_object_sprite_instance(project_assets, object)
            else {
                continue;
            };
            scene_data.sprites.push(sprite_instance);
        }
    }

    pub(super) fn prepare_tilemap_data(
        &mut self,
        scene_data: &mut SceneData,
        project_path: Option<&std::path::Path>,
    ) {
        let Some(tilemap) = self.scene_manager.tilemap().cloned() else {
            tracing::trace!("No tilemap found in scene manager");
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
        let renderable_entities = self.scene_manager.game_state().get_renderable_entities();
        tracing::trace!(
            "Starting sprite rendering for {} renderable entities",
            renderable_entities.len()
        );

        if renderable_entities.is_empty() {
            tracing::warn!("No renderable entities found - no sprites will be rendered");
            return;
        }

        for (entity_id, position, size) in renderable_entities {
            if self.suppressed_entity_ids.contains(&entity_id) {
                continue;
            }
            self.process_entity_sprite(
                scene_data,
                entity_id,
                position,
                size,
                project_path,
                project_assets,
            );
        }
    }

    pub(super) fn prepare_static_entity_sprite_data(
        &mut self,
        scene_data: &mut SceneData,
        project_assets: &ProjectAssets,
    ) {
        for entity in self
            .scene_manager
            .game_state()
            .get_static_entity_renderables()
        {
            if self.suppressed_entity_ids.contains(&entity.entity_id) {
                continue;
            }
            let Some(sprite_instance) = self.build_static_object_sprite_instance(
                project_assets,
                &entity.sheet,
                &entity.object_name,
                entity.position,
                entity.size,
            ) else {
                continue;
            };
            scene_data.sprites.push(sprite_instance);
        }
    }

    pub(super) fn process_entity_sprite(
        &mut self,
        scene_data: &mut SceneData,
        entity_id: u32,
        position: glam::IVec2,
        size: glam::UVec2,
        project_path: Option<&std::path::Path>,
        project_assets: &ProjectAssets,
    ) {
        tracing::trace!(
            "Processing entity {} at ({}, {}) with size {}x{}",
            entity_id,
            position.x,
            position.y,
            size.x,
            size.y
        );

        let Some(entity) = self
            .scene_manager
            .game_state()
            .entity_manager()
            .get_entity(entity_id)
        else {
            tracing::warn!("Entity {} not found in entity manager", entity_id);
            return;
        };

        tracing::trace!(
            "Found entity {} (type: {:?}, visible: {})",
            entity_id,
            entity.entity_kind,
            entity.attributes.visible
        );

        let Some(animation_controller) = &entity.attributes.animation_controller else {
            tracing::trace!(
                "Entity {} has no animation controller - skipping sprite rendering",
                entity_id
            );
            return;
        };

        tracing::trace!("Entity {} has animation controller", entity_id);

        let atlas_name = match animation_controller.current_atlas_name() {
            Ok(name) => name,
            Err(_) => {
                tracing::trace!(
                    "Entity {} animation controller failed to provide atlas name",
                    entity_id
                );
                return;
            }
        };

        tracing::trace!("Entity {} requesting atlas: '{}'", entity_id, atlas_name);

        self.load_and_create_sprite_instance(
            scene_data,
            entity_id,
            position,
            size,
            &atlas_name,
            (project_assets, project_path),
        );
    }

    pub(super) fn load_and_create_sprite_instance(
        &mut self,
        scene_data: &mut SceneData,
        entity_id: u32,
        position: glam::IVec2,
        size: glam::UVec2,
        atlas_name: &str,
        project_context: (&ProjectAssets, Option<&std::path::Path>),
    ) {
        let (project_assets, project_path) = project_context;
        let atlas_name_clean = atlas_name.strip_suffix(".json").unwrap_or(atlas_name);
        tracing::trace!(
            "Cleaned atlas name: '{}' -> '{}'",
            atlas_name,
            atlas_name_clean
        );
        tracing::trace!(
            "Available sprite atlases in ProjectAssets: {:?}",
            project_assets.sprite_atlases.keys().collect::<Vec<_>>()
        );

        let Some(atlas_asset) = project_assets.sprite_atlases.get(atlas_name_clean) else {
            tracing::error!(
                "Sprite atlas '{}' not found in ProjectAssets (cleaned name: '{}')",
                atlas_name,
                atlas_name_clean
            );
            return;
        };

        tracing::trace!(
            "Found atlas asset for '{}' at path: {}",
            atlas_name_clean,
            atlas_asset.path.display()
        );

        let sprite_atlas = match self.load_sprite_atlas_from_asset(atlas_asset, project_path) {
            Ok(atlas) => atlas,
            Err(error) => {
                tracing::error!("Failed to load sprite atlas '{}': {}", atlas_name, error);
                return;
            }
        };

        let sprite_texture_size = sprite_atlas
            .image_size()
            .unwrap_or(glam::UVec2::new(64, 16));
        tracing::trace!(
            "Using sprite atlas '{}' with texture size {}x{} (cache hit: {})",
            atlas_name,
            sprite_texture_size.x,
            sprite_texture_size.y,
            self.loaded_sprite_atlases
                .contains_key(&atlas_asset.path.to_string_lossy().to_string())
        );
        tracing::trace!("Atlas contains {} tiles", sprite_atlas.tiles.len());

        let Some(frame) = self.scene_manager.game_state().get_entity_sprite_frame(
            entity_id,
            &sprite_atlas,
            sprite_texture_size,
        ) else {
            tracing::warn!(
                "Failed to get sprite frame for entity {} - entity will not be rendered",
                entity_id
            );
            return;
        };

        let render_position_i32 = position;

        let sprite_instance = toki_render::SpriteInstance {
            frame,
            position: render_position_i32,
            size,
            texture_path: atlas_asset
                .path
                .parent()
                .map(|parent| parent.join(&sprite_atlas.image)),
            flip_x: self
                .scene_manager
                .game_state()
                .get_entity_sprite_flip_x(entity_id),
        };

        scene_data.sprites.push(sprite_instance);
        let effective_scale = self.effective_camera_scale();
        let viewport_x = (position.x - self.camera.position.x) as f32 / effective_scale;
        let viewport_y = (position.y - self.camera.position.y) as f32 / effective_scale;

        tracing::trace!("Added sprite instance for entity {} - entity world top-left: ({}, {}), viewport coords: ({:.1}, {:.1}), render position: ({}, {}), size: {}x{}",
            entity_id, position.x, position.y, viewport_x, viewport_y, render_position_i32.x, render_position_i32.y, size.x, size.y);
        tracing::trace!(
            "Sprite frame UVs: u0={:.3}, v0={:.3}, u1={:.3}, v1={:.3}",
            frame.u0,
            frame.v0,
            frame.u1,
            frame.v1
        );
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

        let outline_shape = toki_render::DebugShape {
            shape_type: toki_render::DebugShapeType::Rectangle,
            position: glam::Vec2::new(render_position_i32.x as f32, render_position_i32.y as f32),
            size: glam::Vec2::new(cached_visual.size.x as f32, cached_visual.size.y as f32),
            color: outline_color,
        };
        scene_data.debug_shapes.push(outline_shape);
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

        for preview in drag_preview_data {
            let Some(entity) = self
                .scene_manager
                .game_state()
                .entity_manager()
                .get_entity(preview.entity_id)
            else {
                continue;
            };

            let entity_size = entity.size;
            if let Some(animation_controller) = &entity.attributes.animation_controller {
                let Ok(atlas_name) = animation_controller.current_atlas_name() else {
                    continue;
                };
                let atlas_name = atlas_name.to_string();

                self.load_and_create_sprite_instance(
                    scene_data,
                    preview.entity_id,
                    preview.world_position,
                    entity_size,
                    &atlas_name,
                    (project_assets, project_path),
                );
            } else if let Some(static_object_render) =
                entity.attributes.static_object_render.clone()
            {
                let Some(sprite_instance) = self.build_static_object_sprite_instance(
                    project_assets,
                    &static_object_render.sheet,
                    &static_object_render.object_name,
                    preview.world_position,
                    entity_size,
                ) else {
                    continue;
                };
                scene_data.sprites.push(sprite_instance);
            } else {
                continue;
            }

            let outline_color = if preview.is_valid {
                [0.0, 1.0, 0.0, 1.0]
            } else {
                [1.0, 0.0, 0.0, 1.0]
            };
            scene_data.debug_shapes.push(toki_render::DebugShape {
                shape_type: toki_render::DebugShapeType::Rectangle,
                position: glam::Vec2::new(
                    preview.world_position.x as f32,
                    preview.world_position.y as f32,
                ),
                size: glam::Vec2::new(entity_size.x as f32, entity_size.y as f32),
                color: outline_color,
            });
        }
    }

    pub(super) fn prepare_debug_shapes(&mut self, scene_data: &mut SceneData) {
        if !self
            .scene_manager
            .game_state()
            .is_debug_collision_rendering_enabled()
        {
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

        let renderable_entities = self.scene_manager.game_state().get_renderable_entities();
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

        let entity_boxes = self.scene_manager.game_state().get_entity_collision_boxes();
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

        let solid_tiles = self
            .scene_manager
            .game_state()
            .get_solid_tile_positions(tilemap, atlas);
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

        let trigger_tiles = self
            .scene_manager
            .game_state()
            .get_trigger_tile_positions(tilemap, atlas);
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
