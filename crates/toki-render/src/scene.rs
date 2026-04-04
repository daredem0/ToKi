use crate::per_frame_lru::PerFrameLruCache;
use crate::pipelines::sprite::SpriteInstance as SpriteRenderInstance;
use crate::pipelines::TextureSource;
use crate::sprite_batch_order::{append_ordered_draw_batch, OrderedDrawBatch};
use crate::targets::RenderTarget;
use crate::{DebugPipeline, RenderError, RenderPipeline, SpritePipeline, TilemapPipeline};
use toki_core::assets::atlas::AtlasMeta;
use toki_core::assets::tilemap::TileMap;
use toki_core::graphics::image::DecodedImage;
use toki_core::sprite::SpriteFrame;

const SCENE_TEXTURED_SPRITE_PIPELINE_CACHE_CAPACITY: usize = 64;

/// Data needed to render a scene
#[derive(Debug)]
pub struct SceneData {
    pub tilemap: Option<TileMap>,
    pub atlas: Option<AtlasMeta>,
    pub texture_size: glam::UVec2,
    pub visible_chunks: Vec<(u32, u32)>,
    pub sprites: Vec<SpriteInstance>,
    pub underlay_shapes: Vec<OverlayShape>,
    pub debug_shapes: Vec<DebugShape>,
    pub overlay_shapes: Vec<OverlayShape>,
}

/// Sprite instance for rendering
#[derive(Debug, Clone)]
pub struct SpriteInstance {
    pub frame: SpriteFrame,
    pub position: glam::IVec2,
    pub size: glam::UVec2,
    pub texture_path: Option<std::path::PathBuf>,
    pub texture_image: Option<DecodedImage>,
    pub texture_cache_key: Option<String>,
    pub flip_x: bool,
}

/// Debug shape for rendering
#[derive(Debug, Clone)]
pub struct DebugShape {
    pub shape_type: DebugShapeType,
    pub position: glam::Vec2,
    pub size: glam::Vec2,
    pub color: [f32; 4],
}

#[derive(Debug, Clone)]
pub enum DebugShapeType {
    Rectangle,
    Circle,
    Line { end: glam::Vec2, thickness: f32 },
}

/// Non-debug overlay shape for editor/runtime annotations rendered in the scene pass.
#[derive(Debug, Clone)]
pub struct OverlayShape {
    pub shape_type: OverlayShapeType,
    pub position: glam::Vec2,
    pub size: glam::Vec2,
    pub color: [f32; 4],
}

#[derive(Debug, Clone)]
pub enum OverlayShapeType {
    Rectangle,
    Circle,
    Line { end: glam::Vec2, thickness: f32 },
}

impl Default for SceneData {
    fn default() -> Self {
        Self {
            tilemap: None,
            atlas: None,
            texture_size: glam::UVec2::new(256, 256),
            visible_chunks: Vec::new(),
            sprites: Vec::new(),
            underlay_shapes: Vec::new(),
            debug_shapes: Vec::new(),
            overlay_shapes: Vec::new(),
        }
    }
}

/// Unified scene renderer that works with any render target
pub struct SceneRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    tilemap_pipeline: TilemapPipeline,
    overlay_tilemap_pipeline: TilemapPipeline,
    sprite_pipeline: SpritePipeline,
    sprite_pipelines_by_texture: PerFrameLruCache<std::path::PathBuf, SpritePipeline>,
    sprite_draw_batches: Vec<OrderedDrawBatch<SceneSpriteBatchKey>>,
    underlay_pipeline: DebugPipeline,
    debug_pipeline: DebugPipeline,
    current_sprite_texture_path: Option<std::path::PathBuf>,
    current_projection: glam::Mat4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SceneSpriteBatchKey {
    Default,
    Textured(std::path::PathBuf),
}

enum SceneSpriteTextureSource<'a> {
    Default,
    File {
        key: std::path::PathBuf,
        path: &'a std::path::Path,
    },
    Rgba8 {
        key: std::path::PathBuf,
        image: &'a DecodedImage,
    },
}

impl SceneRenderer {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        tilemap_texture: Option<std::path::PathBuf>,
        sprite_texture: Option<std::path::PathBuf>,
    ) -> Result<Self, RenderError> {
        tracing::info!("Creating new SceneRenderer");
        tracing::debug!("Surface format: {:?}", surface_format);
        tracing::debug!("Tilemap texture: {:?}", tilemap_texture);
        tracing::debug!("Sprite texture: {:?}", sprite_texture);
        let make_tilemap_source = || {
            tilemap_texture
                .as_deref()
                .map(TextureSource::path)
                .unwrap_or_else(TextureSource::placeholder)
        };
        let tilemap_pipeline =
            TilemapPipeline::new(&device, &queue, surface_format, make_tilemap_source())?;
        let overlay_tilemap_pipeline =
            TilemapPipeline::new(&device, &queue, surface_format, make_tilemap_source())?;

        // Clone sprite_texture for caching before moving it
        let sprite_texture_cache = sprite_texture.clone();
        let sprite_source = sprite_texture
            .as_deref()
            .map(TextureSource::path)
            .unwrap_or_else(TextureSource::placeholder);
        let sprite_pipeline = SpritePipeline::new(&device, &queue, surface_format, sprite_source)?;

        let underlay_pipeline = DebugPipeline::new(&device, surface_format);
        let debug_pipeline = DebugPipeline::new(&device, surface_format);

        tracing::info!("SceneRenderer created successfully");

        Ok(Self {
            device,
            queue,
            format: surface_format,
            tilemap_pipeline,
            overlay_tilemap_pipeline,
            sprite_pipeline,
            sprite_pipelines_by_texture: PerFrameLruCache::new(
                SCENE_TEXTURED_SPRITE_PIPELINE_CACHE_CAPACITY,
            ),
            sprite_draw_batches: Vec::new(),
            underlay_pipeline,
            debug_pipeline,
            current_sprite_texture_path: sprite_texture_cache,
            current_projection: glam::Mat4::IDENTITY,
        })
    }

    /// Load new tilemap texture
    pub fn load_tilemap_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), RenderError> {
        tracing::info!("Loading tilemap texture: {:?}", texture_path);
        self.tilemap_pipeline = TilemapPipeline::new(
            &self.device,
            &self.queue,
            self.format,
            TextureSource::path(texture_path.as_path()),
        )?;
        self.overlay_tilemap_pipeline = TilemapPipeline::new(
            &self.device,
            &self.queue,
            self.format,
            TextureSource::path(texture_path.as_path()),
        )?;
        tracing::info!("Tilemap texture loaded successfully");
        Ok(())
    }

    /// Load new sprite texture (with caching to avoid redundant loads)
    pub fn load_sprite_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), RenderError> {
        // Check if this texture is already loaded
        if let Some(current_path) = &self.current_sprite_texture_path {
            if current_path == &texture_path {
                tracing::trace!("Sprite texture already loaded: {:?}", texture_path);
                return Ok(());
            }
        }

        tracing::info!("Loading sprite texture: {:?}", texture_path);
        self.sprite_pipeline = SpritePipeline::new(
            &self.device,
            &self.queue,
            self.format,
            TextureSource::path(texture_path.as_path()),
        )?;
        self.sprite_pipelines_by_texture.clear();
        self.current_sprite_texture_path = Some(texture_path);
        tracing::info!("Sprite texture loaded successfully");
        Ok(())
    }

    pub fn clear_sprite_texture_cache(&mut self) {
        self.sprite_pipelines_by_texture.clear();
    }

    #[doc(hidden)]
    pub fn debug_textured_sprite_pipeline_cache_len(&self) -> usize {
        self.sprite_pipelines_by_texture.len()
    }

    #[doc(hidden)]
    pub fn debug_has_textured_sprite_pipeline(&self, texture_path: &std::path::Path) -> bool {
        self.sprite_pipelines_by_texture.get(texture_path).is_some()
    }

    fn update_sprite_projection(&mut self, projection: glam::Mat4) {
        self.sprite_pipeline
            .update_projection(&self.queue, projection);
        for pipeline in self.sprite_pipelines_by_texture.values_mut() {
            pipeline.update_projection(&self.queue, projection);
        }
    }

    fn clear_sprite_batches(&mut self) {
        self.sprite_pipelines_by_texture.begin_frame();
        self.sprite_pipeline.clear_sprites();
        for pipeline in self.sprite_pipelines_by_texture.values_mut() {
            pipeline.clear_sprites();
        }
        self.sprite_draw_batches.clear();
        self.sprite_pipelines_by_texture.evict_unused_lru();
    }

    fn record_sprite_draw_batch(&mut self, key: SceneSpriteBatchKey, start: usize) {
        append_ordered_draw_batch(&mut self.sprite_draw_batches, key, start);
    }

    fn resolve_sprite_texture_source<'a>(
        &self,
        sprite: &'a SpriteInstance,
    ) -> SceneSpriteTextureSource<'a> {
        if let Some(image) = &sprite.texture_image {
            let key = sprite
                .texture_cache_key
                .as_deref()
                .map(std::path::PathBuf::from)
                .or_else(|| sprite.texture_path.clone())
                .unwrap_or_else(|| {
                    std::path::PathBuf::from(format!(
                        "__inline_rgba8_{}x{}_{}",
                        image.width,
                        image.height,
                        self.sprite_pipelines_by_texture.len()
                    ))
                });
            SceneSpriteTextureSource::Rgba8 { key, image }
        } else if let Some(texture_path) = &sprite.texture_path {
            SceneSpriteTextureSource::File {
                key: texture_path.clone(),
                path: texture_path.as_path(),
            }
        } else {
            SceneSpriteTextureSource::Default
        }
    }

    fn add_textured_sprite_instance(
        &mut self,
        texture_key: &std::path::Path,
        texture_source: TextureSource<'_>,
        render_instance: SpriteRenderInstance,
    ) {
        let instance_index = self
            .sprite_pipelines_by_texture
            .get(texture_key)
            .map(|pipeline| pipeline.instance_count())
            .unwrap_or(0);
        let texture_key_buf = texture_key.to_path_buf();
        let insert_result = self
            .sprite_pipelines_by_texture
            .get_or_try_insert_with(texture_key_buf.clone(), || {
                SpritePipeline::new(&self.device, &self.queue, self.format, texture_source)
            });
        let Ok(Some(pipeline)) = insert_result else {
            if let Err(error) = insert_result {
                tracing::warn!(
                    texture_key = ?texture_key,
                    "Skipping sprite with failed texture pipeline creation: {error}"
                );
            }
            return;
        };
        {
            pipeline.update_projection(&self.queue, self.current_projection);
            pipeline.add_sprite(render_instance);
        }
        self.record_sprite_draw_batch(
            SceneSpriteBatchKey::Textured(texture_key_buf),
            instance_index,
        );
    }

    fn add_sprite_instance(&mut self, sprite: &SpriteInstance) {
        let render_instance = SpriteRenderInstance {
            frame: sprite.frame,
            position: sprite.position.as_vec2(),
            size: sprite.size.as_vec2(),
            flip_x: sprite.flip_x,
            tint_alpha: 0.0,
        };

        match self.resolve_sprite_texture_source(sprite) {
            SceneSpriteTextureSource::Default => {
                let instance_index = self.sprite_pipeline.instance_count();
                self.sprite_pipeline.add_sprite(render_instance);
                self.record_sprite_draw_batch(SceneSpriteBatchKey::Default, instance_index);
            }
            SceneSpriteTextureSource::File { key, path } => {
                self.add_textured_sprite_instance(&key, TextureSource::path(path), render_instance);
            }
            SceneSpriteTextureSource::Rgba8 { key, image } => {
                self.add_textured_sprite_instance(
                    &key,
                    TextureSource::rgba8(image),
                    render_instance,
                );
            }
        }
    }

    fn update_sprite_batches(&mut self) {
        self.sprite_pipeline
            .update_with_queue(&self.device, &self.queue);
        for pipeline in self.sprite_pipelines_by_texture.values_mut() {
            pipeline.update_with_queue(&self.device, &self.queue);
        }
    }

    fn add_debug_shape_batch(&mut self, shapes: &[DebugShape]) {
        for shape in shapes {
            match shape.shape_type {
                DebugShapeType::Rectangle => {
                    self.debug_pipeline.add_rect(
                        shape.position.x,
                        shape.position.y,
                        shape.size.x,
                        shape.size.y,
                        shape.color,
                    );
                }
                DebugShapeType::Circle => {}
                DebugShapeType::Line { end, thickness } => {
                    self.debug_pipeline
                        .add_line(shape.position, end, thickness, shape.color);
                }
            }
        }
    }

    fn add_overlay_shape_batch_to(pipeline: &mut DebugPipeline, shapes: &[OverlayShape]) {
        for shape in shapes {
            match shape.shape_type {
                OverlayShapeType::Rectangle => {
                    pipeline.add_rect(
                        shape.position.x,
                        shape.position.y,
                        shape.size.x,
                        shape.size.y,
                        shape.color,
                    );
                }
                OverlayShapeType::Circle => {}
                OverlayShapeType::Line { end, thickness } => {
                    pipeline.add_line(shape.position, end, thickness, shape.color);
                }
            }
        }
    }

    fn render_sprite_batches<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        for batch in &self.sprite_draw_batches {
            match &batch.key {
                SceneSpriteBatchKey::Default => {
                    self.sprite_pipeline
                        .render_range(render_pass, batch.start, batch.count);
                }
                SceneSpriteBatchKey::Textured(ref texture_key) => {
                    if let Some(pipeline) =
                        self.sprite_pipelines_by_texture.get(texture_key.as_path())
                    {
                        pipeline.render_range(render_pass, batch.start, batch.count);
                    }
                }
            }
        }
    }

    fn prepare_scene_pipelines(&mut self, scene_data: &SceneData) {
        if let (Some(tilemap), Some(atlas)) = (&scene_data.tilemap, &scene_data.atlas) {
            let split = if scene_data.visible_chunks.is_empty() {
                tracing::trace!(
                    "Generating split vertices for all tiles ({}x{})",
                    tilemap.size.x,
                    tilemap.size.y
                );
                tilemap.generate_split_vertices(atlas, scene_data.texture_size)
            } else {
                tracing::trace!(
                    "Generating split vertices for {} visible chunks",
                    scene_data.visible_chunks.len()
                );
                tilemap.generate_split_vertices_for_chunks(
                    atlas,
                    scene_data.texture_size,
                    &scene_data.visible_chunks,
                )
            };
            tracing::trace!(
                "Updating tilemap pipelines: {} below, {} above vertices",
                split.below.len(),
                split.above.len()
            );
            self.tilemap_pipeline
                .update_vertices(&self.device, &self.queue, &split.below);
            self.overlay_tilemap_pipeline
                .update_vertices(&self.device, &self.queue, &split.above);
        } else {
            tracing::trace!("No tilemap or atlas to render");
        }

        tracing::trace!("Adding {} sprites to pipeline", scene_data.sprites.len());
        self.clear_sprite_batches();
        for sprite in &scene_data.sprites {
            self.add_sprite_instance(sprite);
        }
        self.update_sprite_batches();
        tracing::trace!("Updated sprite vertex buffer on GPU");

        tracing::trace!(
            "Adding {} underlay shapes, {} debug shapes and {} overlay shapes to pipeline",
            scene_data.underlay_shapes.len(),
            scene_data.debug_shapes.len(),
            scene_data.overlay_shapes.len()
        );
        self.underlay_pipeline.clear();
        Self::add_overlay_shape_batch_to(&mut self.underlay_pipeline, &scene_data.underlay_shapes);
        self.underlay_pipeline.update_vertices(&self.device);
        self.debug_pipeline.clear();
        self.add_debug_shape_batch(&scene_data.debug_shapes);
        Self::add_overlay_shape_batch_to(&mut self.debug_pipeline, &scene_data.overlay_shapes);
        tracing::trace!("Finalizing debug shapes");
        self.debug_pipeline.update_vertices(&self.device);
    }

    fn execute_scene_render_pass<T: RenderTarget>(
        &mut self,
        target: &mut T,
    ) -> Result<(), RenderError> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Scene Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Scene Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target.get_render_view()?,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.12,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            tracing::trace!("Rendering tilemap pipeline (below entities)");
            self.tilemap_pipeline.render(&mut render_pass);
            tracing::trace!("Rendering underlay pipeline");
            self.underlay_pipeline.render(&mut render_pass);
            tracing::trace!("Rendering sprite pipeline");
            self.render_sprite_batches(&mut render_pass);
            tracing::trace!("Rendering overlay tilemap pipeline (above entities)");
            self.overlay_tilemap_pipeline.render(&mut render_pass);
            tracing::trace!("Rendering debug pipeline");
            self.debug_pipeline.render(&mut render_pass);
        }

        tracing::trace!("Submitting render commands to GPU");
        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn render_scene_internal<T: RenderTarget>(
        &mut self,
        target: &mut T,
        scene_data: &SceneData,
        projection: glam::Mat4,
    ) -> Result<(), RenderError> {
        target.begin_frame()?;
        self.update_projection(projection);
        self.prepare_scene_pipelines(scene_data);
        self.execute_scene_render_pass(target)?;
        target.end_frame()?;
        tracing::trace!("Scene render complete");
        Ok(())
    }

    /// Render scene to any render target with custom projection matrix
    pub fn render_scene_with_projection<T: RenderTarget>(
        &mut self,
        target: &mut T,
        scene_data: &SceneData,
        projection: glam::Mat4,
    ) -> Result<(), RenderError> {
        tracing::trace!("Starting scene render with custom projection");
        tracing::trace!(
            "Scene data - tilemap: {}, sprites: {}, underlay_shapes: {}, debug_shapes: {}, overlay_shapes: {}",
            scene_data.tilemap.is_some(),
            scene_data.sprites.len(),
            scene_data.underlay_shapes.len(),
            scene_data.debug_shapes.len(),
            scene_data.overlay_shapes.len()
        );
        self.render_scene_internal(target, scene_data, projection)
    }

    /// Render scene to any render target
    pub fn render_scene<T: RenderTarget>(
        &mut self,
        target: &mut T,
        scene_data: &SceneData,
    ) -> Result<(), RenderError> {
        tracing::trace!("Starting scene render");
        tracing::trace!(
            "Scene data - tilemap: {}, sprites: {}, underlay_shapes: {}, debug_shapes: {}, overlay_shapes: {}",
            scene_data.tilemap.is_some(),
            scene_data.sprites.len(),
            scene_data.underlay_shapes.len(),
            scene_data.debug_shapes.len(),
            scene_data.overlay_shapes.len()
        );
        let (width, height) = target.size();
        tracing::trace!("Render target size: {}x{}", width, height);
        let projection = self.calculate_projection_for_size(width, height);
        self.render_scene_internal(target, scene_data, projection)
    }

    fn calculate_projection_for_size(&self, width: u32, height: u32) -> glam::Mat4 {
        // Use toki-core's projection calculation
        toki_core::math::projection::calculate_projection(
            toki_core::math::projection::ProjectionParameter {
                width,
                height,
                desired_width: width,
                desired_height: height,
            },
        )
    }

    fn update_projection(&mut self, projection: glam::Mat4) {
        self.current_projection = projection;
        self.tilemap_pipeline
            .update_projection(&self.queue, projection);
        self.overlay_tilemap_pipeline
            .update_projection(&self.queue, projection);
        self.update_sprite_projection(projection);
        self.underlay_pipeline
            .update_camera(&self.queue, projection);
        self.debug_pipeline.update_camera(&self.queue, projection);
    }
}

#[cfg(test)]
mod tests {
    use super::SceneSpriteBatchKey;
    use std::path::PathBuf;

    #[test]
    fn textured_batch_key_stores_pathbuf() {
        let key = SceneSpriteBatchKey::Textured(PathBuf::from("sprites/hero.png"));
        match &key {
            SceneSpriteBatchKey::Textured(path) => {
                assert_eq!(path.extension().and_then(|e| e.to_str()), Some("png"));
            }
            _ => panic!("expected Textured variant"),
        }
    }
}
