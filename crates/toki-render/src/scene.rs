use crate::pipelines::sprite::SpriteInstance as SpriteRenderInstance;
use crate::targets::RenderTarget;
use crate::{DebugPipeline, RenderError, RenderPipeline, SpritePipeline, TilemapPipeline};
use std::collections::BTreeMap;
use toki_core::assets::atlas::AtlasMeta;
use toki_core::assets::tilemap::TileMap;
use toki_core::sprite::SpriteFrame;

/// Data needed to render a scene
#[derive(Debug)]
pub struct SceneData {
    pub tilemap: Option<TileMap>,
    pub atlas: Option<AtlasMeta>,
    pub texture_size: glam::UVec2,
    pub visible_chunks: Vec<(u32, u32)>,
    pub sprites: Vec<SpriteInstance>,
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
            debug_shapes: Vec::new(),
            overlay_shapes: Vec::new(),
        }
    }
}

/// Unified scene renderer that works with any render target
pub struct SceneRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    tilemap_pipeline: TilemapPipeline,
    sprite_pipeline: SpritePipeline,
    sprite_pipelines_by_texture: BTreeMap<std::path::PathBuf, SpritePipeline>,
    debug_pipeline: DebugPipeline,
    current_sprite_texture_path: Option<std::path::PathBuf>, // Cache current sprite texture
    current_projection: glam::Mat4,
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
        tracing::info!("Surface format: {:?}", surface_format);
        tracing::info!("Tilemap texture: {:?}", tilemap_texture);
        tracing::info!("Sprite texture: {:?}", sprite_texture);
        let tilemap_pipeline = if let Some(texture_path) = tilemap_texture {
            TilemapPipeline::new(&device, &queue, surface_format, texture_path)
        } else {
            // Create with default/placeholder texture
            TilemapPipeline::new(
                &device,
                &queue,
                surface_format,
                std::path::PathBuf::from(""),
            )
        };

        // Clone sprite_texture for caching before moving it
        let sprite_texture_cache = sprite_texture.clone();
        let sprite_pipeline = if let Some(texture_path) = sprite_texture {
            SpritePipeline::new(&device, &queue, surface_format, texture_path)
        } else {
            // Create with default/placeholder texture
            SpritePipeline::new(
                &device,
                &queue,
                surface_format,
                std::path::PathBuf::from(""),
            )
        };

        let debug_pipeline = DebugPipeline::new(&device, surface_format);

        tracing::info!("SceneRenderer created successfully");

        Ok(Self {
            device,
            queue,
            tilemap_pipeline,
            sprite_pipeline,
            sprite_pipelines_by_texture: BTreeMap::new(),
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
            wgpu::TextureFormat::Bgra8UnormSrgb, // TODO: Get from render target
            texture_path.clone(),
        );
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
            wgpu::TextureFormat::Bgra8UnormSrgb, // TODO: Get from render target
            texture_path.clone(),
        );
        self.sprite_pipelines_by_texture.clear();
        self.current_sprite_texture_path = Some(texture_path);
        tracing::info!("Sprite texture loaded successfully");
        Ok(())
    }

    fn update_sprite_projection(&mut self, projection: glam::Mat4) {
        self.sprite_pipeline
            .update_projection(&self.queue, projection);
        for pipeline in self.sprite_pipelines_by_texture.values_mut() {
            pipeline.update_projection(&self.queue, projection);
        }
    }

    fn clear_sprite_batches(&mut self) {
        self.sprite_pipeline.clear_sprites();
        for pipeline in self.sprite_pipelines_by_texture.values_mut() {
            pipeline.clear_sprites();
        }
    }

    fn add_sprite_instance(&mut self, sprite: &SpriteInstance) {
        let render_instance = SpriteRenderInstance {
            frame: sprite.frame,
            position: sprite.position.as_vec2(),
            size: sprite.size.as_vec2(),
            flip_x: sprite.flip_x,
        };

        if let Some(texture_path) = &sprite.texture_path {
            let pipeline = self
                .sprite_pipelines_by_texture
                .entry(texture_path.clone())
                .or_insert_with(|| {
                    SpritePipeline::new(
                        &self.device,
                        &self.queue,
                        wgpu::TextureFormat::Bgra8UnormSrgb,
                        texture_path.clone(),
                    )
                });
            pipeline.update_projection(&self.queue, self.current_projection);
            pipeline.add_sprite(render_instance);
        } else {
            self.sprite_pipeline.add_sprite(render_instance);
        }
    }

    fn update_sprite_batches(&mut self) {
        self.sprite_pipeline.update_with_queue(&self.queue);
        for pipeline in self.sprite_pipelines_by_texture.values_mut() {
            pipeline.update_with_queue(&self.queue);
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

    fn add_overlay_shape_batch(&mut self, shapes: &[OverlayShape]) {
        for shape in shapes {
            match shape.shape_type {
                OverlayShapeType::Rectangle => {
                    self.debug_pipeline.add_rect(
                        shape.position.x,
                        shape.position.y,
                        shape.size.x,
                        shape.size.y,
                        shape.color,
                    );
                }
                OverlayShapeType::Circle => {}
                OverlayShapeType::Line { end, thickness } => {
                    self.debug_pipeline
                        .add_line(shape.position, end, thickness, shape.color);
                }
            }
        }
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
            "Scene data - tilemap: {}, sprites: {}, debug_shapes: {}, overlay_shapes: {}",
            scene_data.tilemap.is_some(),
            scene_data.sprites.len(),
            scene_data.debug_shapes.len(),
            scene_data.overlay_shapes.len()
        );

        target.begin_frame()?;

        // Use provided projection matrix
        self.update_projection(projection);

        // Generate and upload tilemap vertices (same logic as runtime)
        if let (Some(tilemap), Some(atlas)) = (&scene_data.tilemap, &scene_data.atlas) {
            let vertices = if scene_data.visible_chunks.is_empty() {
                // Render all tiles (for editor or small maps)
                tracing::trace!(
                    "Generating vertices for all tiles ({}x{})",
                    tilemap.size.x,
                    tilemap.size.y
                );
                tilemap.generate_vertices(atlas, scene_data.texture_size)
            } else {
                // Render only visible chunks (for runtime performance)
                tracing::trace!(
                    "Generating vertices for {} visible chunks",
                    scene_data.visible_chunks.len()
                );
                tilemap.generate_vertices_for_chunks(
                    atlas,
                    scene_data.texture_size,
                    &scene_data.visible_chunks,
                )
            };
            tracing::trace!("Updating tilemap pipeline with {} vertices", vertices.len());
            self.tilemap_pipeline
                .update_vertices(&self.device, &vertices);
        } else {
            tracing::trace!("No tilemap or atlas to render");
        }

        // Add sprites (same logic as runtime)
        tracing::trace!("Adding {} sprites to pipeline", scene_data.sprites.len());
        self.clear_sprite_batches();
        for sprite in &scene_data.sprites {
            self.add_sprite_instance(sprite);
        }

        self.update_sprite_batches();
        tracing::trace!("Updated sprite vertex buffer on GPU");

        // Add debug shapes
        tracing::trace!(
            "Adding {} debug shapes and {} overlay shapes to pipeline",
            scene_data.debug_shapes.len()
            ,
            scene_data.overlay_shapes.len()
        );
        self.debug_pipeline.clear();
        self.add_debug_shape_batch(&scene_data.debug_shapes);
        self.add_overlay_shape_batch(&scene_data.overlay_shapes);

        // Finalize debug shapes
        tracing::trace!("Finalizing debug shapes");
        self.debug_pipeline.update_vertices(&self.device);

        // Render using WGPU pipelines
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

            // Same pipeline calls for both runtime and editor!
            tracing::trace!("Rendering tilemap pipeline");
            self.tilemap_pipeline.render(&mut render_pass);
            tracing::trace!("Rendering sprite pipeline");
            self.sprite_pipeline.render(&mut render_pass);
            for pipeline in self.sprite_pipelines_by_texture.values() {
                pipeline.render(&mut render_pass);
            }
            tracing::trace!("Rendering debug pipeline");
            self.debug_pipeline.render(&mut render_pass);
        }

        tracing::trace!("Submitting render commands to GPU");
        self.queue.submit(std::iter::once(encoder.finish()));
        target.end_frame()?;

        tracing::trace!("Scene render complete");
        Ok(())
    }

    /// Render scene to any render target
    pub fn render_scene<T: RenderTarget>(
        &mut self,
        target: &mut T,
        scene_data: &SceneData,
    ) -> Result<(), RenderError> {
        tracing::trace!("Starting scene render");
        tracing::trace!(
            "Scene data - tilemap: {}, sprites: {}, debug_shapes: {}, overlay_shapes: {}",
            scene_data.tilemap.is_some(),
            scene_data.sprites.len(),
            scene_data.debug_shapes.len(),
            scene_data.overlay_shapes.len()
        );

        target.begin_frame()?;

        // Update projection matrix based on target size
        let (width, height) = target.size();
        tracing::trace!("Render target size: {}x{}", width, height);
        let projection = self.calculate_projection_for_size(width, height);
        self.update_projection(projection);

        // Generate and upload tilemap vertices (same logic as runtime)
        if let (Some(tilemap), Some(atlas)) = (&scene_data.tilemap, &scene_data.atlas) {
            let vertices = if scene_data.visible_chunks.is_empty() {
                // Render all tiles (for editor or small maps)
                tracing::trace!(
                    "Generating vertices for all tiles ({}x{})",
                    tilemap.size.x,
                    tilemap.size.y
                );
                tilemap.generate_vertices(atlas, scene_data.texture_size)
            } else {
                // Render only visible chunks (for runtime performance)
                tracing::trace!(
                    "Generating vertices for {} visible chunks",
                    scene_data.visible_chunks.len()
                );
                tilemap.generate_vertices_for_chunks(
                    atlas,
                    scene_data.texture_size,
                    &scene_data.visible_chunks,
                )
            };
            tracing::trace!("Updating tilemap pipeline with {} vertices", vertices.len());
            self.tilemap_pipeline
                .update_vertices(&self.device, &vertices);
        } else {
            tracing::trace!("No tilemap or atlas to render");
        }

        // Add sprites (same logic as runtime)
        tracing::trace!("Adding {} sprites to pipeline", scene_data.sprites.len());
        self.clear_sprite_batches();
        for sprite in &scene_data.sprites {
            self.add_sprite_instance(sprite);
        }

        self.update_sprite_batches();
        tracing::trace!("Updated sprite vertex buffer on GPU");

        // Add debug shapes
        tracing::trace!(
            "Adding {} debug shapes and {} overlay shapes to pipeline",
            scene_data.debug_shapes.len()
            ,
            scene_data.overlay_shapes.len()
        );
        self.debug_pipeline.clear();
        self.add_debug_shape_batch(&scene_data.debug_shapes);
        self.add_overlay_shape_batch(&scene_data.overlay_shapes);

        // Finalize debug shapes
        tracing::trace!("Finalizing debug shapes");
        self.debug_pipeline.update_vertices(&self.device);

        // Render using WGPU pipelines
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

            // Same pipeline calls for both runtime and editor!
            tracing::trace!("Rendering tilemap pipeline");
            self.tilemap_pipeline.render(&mut render_pass);
            tracing::trace!("Rendering sprite pipeline");
            self.sprite_pipeline.render(&mut render_pass);
            for pipeline in self.sprite_pipelines_by_texture.values() {
                pipeline.render(&mut render_pass);
            }
            tracing::trace!("Rendering debug pipeline");
            self.debug_pipeline.render(&mut render_pass);
        }

        tracing::trace!("Submitting render commands to GPU");
        self.queue.submit(std::iter::once(encoder.finish()));
        target.end_frame()?;

        tracing::trace!("Scene render complete");
        Ok(())
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
        self.update_sprite_projection(projection);
        self.debug_pipeline.update_camera(&self.queue, projection);
    }
}
