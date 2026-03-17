use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;

use toki_core::graphics::image::DecodedImage;
use toki_core::graphics::vertex::QuadVertex;
use toki_core::math::projection::screen_space_projection;
use toki_core::sprite::SpriteFrame;
use toki_core::text::TextItem;

use crate::pipelines::sprite::SpriteInstance;
use crate::pipelines::RenderPipeline;
use crate::wgpu_utils::create_device_and_surface;
use crate::{
    DebugPipeline, GlyphonTextRenderer, SpritePipeline, TextBackgroundRect, TilemapPipeline,
};

#[allow(dead_code)]
pub struct GpuState {
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
    tilemap_pipeline: TilemapPipeline,
    sprite_pipeline: SpritePipeline,
    sprite_pipelines_by_texture: BTreeMap<PathBuf, SpritePipeline>,
    debug_pipeline: DebugPipeline,
    ui_rect_pipeline: DebugPipeline,
    ui_debug_pipeline: DebugPipeline,
    text_renderer: GlyphonTextRenderer,
    text_items: Vec<TextItem>,
    tilemap_render_enabled: bool,
    current_mvp: glam::Mat4,
}

impl std::fmt::Debug for GpuState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuState")
            .field("config", &self.config)
            .field("tilemap_render_enabled", &self.tilemap_render_enabled)
            .field("text_items_len", &self.text_items.len())
            .finish_non_exhaustive()
    }
}

fn default_texture_path() -> PathBuf {
    // Empty path activates the built-in 1x1 white texture fallback in GpuTexture::from_file.
    PathBuf::new()
}

impl GpuState {
    pub fn add_sprite(&mut self, frame: SpriteFrame, pos: glam::IVec2, size: glam::UVec2) {
        let instance = SpriteInstance {
            frame,
            position: pos.as_vec2(), // Convert to float for GPU
            size: size.as_vec2(),    // Convert to float for GPU
            flip_x: false,
        };
        self.sprite_pipeline.add_sprite(instance);
    }

    pub fn add_sprite_flipped(
        &mut self,
        frame: SpriteFrame,
        pos: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        let instance = SpriteInstance {
            frame,
            position: pos.as_vec2(),
            size: size.as_vec2(),
            flip_x,
        };
        self.sprite_pipeline.add_sprite(instance);
    }

    pub fn add_sprite_with_texture(
        &mut self,
        texture_path: PathBuf,
        frame: SpriteFrame,
        pos: glam::IVec2,
        size: glam::UVec2,
    ) {
        let instance = SpriteInstance {
            frame,
            position: pos.as_vec2(),
            size: size.as_vec2(),
            flip_x: false,
        };
        let pipeline = self
            .sprite_pipelines_by_texture
            .entry(texture_path.clone())
            .or_insert_with(|| {
                SpritePipeline::new(&self.device, &self.queue, self.config.format, texture_path)
            });
        pipeline.update_projection(&self.queue, self.current_mvp);
        pipeline.add_sprite(instance);
    }

    pub fn add_sprite_with_texture_flipped(
        &mut self,
        texture_path: PathBuf,
        frame: SpriteFrame,
        pos: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        let instance = SpriteInstance {
            frame,
            position: pos.as_vec2(),
            size: size.as_vec2(),
            flip_x,
        };
        let pipeline = self
            .sprite_pipelines_by_texture
            .entry(texture_path.clone())
            .or_insert_with(|| {
                SpritePipeline::new(&self.device, &self.queue, self.config.format, texture_path)
            });
        pipeline.update_projection(&self.queue, self.current_mvp);
        pipeline.add_sprite(instance);
    }

    pub fn clear_sprites(&mut self) {
        self.sprite_pipeline.clear_sprites();
        for pipeline in self.sprite_pipelines_by_texture.values_mut() {
            pipeline.clear_sprites();
        }
    }

    pub fn clear_text_items(&mut self) {
        self.text_items.clear();
    }

    pub fn add_text_item(&mut self, text: TextItem) {
        self.text_items.push(text);
    }

    pub fn load_font_file(&mut self, path: &Path) -> Result<(), crate::RenderError> {
        self.text_renderer.load_font_file(path)
    }

    /// Clear all debug shapes
    pub fn clear_debug_shapes(&mut self) {
        self.debug_pipeline.clear();
    }

    /// Add a debug rectangle
    pub fn add_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.debug_pipeline.add_rect(x, y, width, height, color);
    }

    /// Add a filled debug rectangle
    pub fn add_filled_debug_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    ) {
        self.debug_pipeline
            .add_filled_rect(x, y, width, height, color);
    }

    /// Finalize debug shapes for rendering (call after adding all shapes)
    pub fn finalize_debug_shapes(&mut self) {
        self.debug_pipeline.update_vertices(&self.device);
    }

    pub fn clear_ui_rects(&mut self) {
        self.ui_rect_pipeline.clear();
    }

    pub fn add_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.ui_rect_pipeline.add_rect(x, y, width, height, color);
    }

    pub fn add_filled_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.ui_rect_pipeline
            .add_filled_rect(x, y, width, height, color);
    }

    pub fn finalize_ui_rects(&mut self) {
        self.ui_rect_pipeline.update_camera(
            &self.queue,
            screen_space_projection(self.config.width as f32, self.config.height as f32),
        );
        self.ui_rect_pipeline.update_vertices(&self.device);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn new(window: Arc<Window>) -> Self {
        let (device, queue, surface, config) = create_device_and_surface(Arc::clone(&window));

        let tilemap_pipeline =
            TilemapPipeline::new(&device, &queue, config.format, default_texture_path());

        let sprite_pipeline =
            SpritePipeline::new(&device, &queue, config.format, default_texture_path());

        let debug_pipeline = DebugPipeline::new(&device, config.format);
        let ui_rect_pipeline = DebugPipeline::new(&device, config.format);
        let ui_debug_pipeline = DebugPipeline::new(&device, config.format);
        let text_renderer = GlyphonTextRenderer::new(&device, &queue, config.format);

        Self {
            surface,
            config,
            device,
            queue,
            tilemap_pipeline,
            sprite_pipeline,
            sprite_pipelines_by_texture: BTreeMap::new(),
            debug_pipeline,
            ui_rect_pipeline,
            ui_debug_pipeline,
            text_renderer,
            text_items: Vec::new(),
            tilemap_render_enabled: true,
            current_mvp: glam::Mat4::IDENTITY,
        }
    }

    /// Load a new tilemap texture at runtime
    pub fn load_tilemap_texture(
        &mut self,
        texture_path: PathBuf,
    ) -> Result<(), crate::RenderError> {
        // Create new tilemap pipeline with the specified texture
        let new_pipeline =
            TilemapPipeline::new(&self.device, &self.queue, self.config.format, texture_path);
        self.tilemap_pipeline = new_pipeline;
        Ok(())
    }

    /// Load a new sprite texture at runtime
    pub fn load_sprite_texture(&mut self, texture_path: PathBuf) -> Result<(), crate::RenderError> {
        // Create new sprite pipeline with the specified texture
        let new_pipeline =
            SpritePipeline::new(&self.device, &self.queue, self.config.format, texture_path);
        self.sprite_pipeline = new_pipeline;
        self.sprite_pipelines_by_texture.clear();
        Ok(())
    }

    pub fn load_sprite_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), crate::RenderError> {
        let new_pipeline =
            SpritePipeline::from_rgba8(&self.device, &self.queue, self.config.format, image);
        self.sprite_pipeline = new_pipeline;
        self.sprite_pipelines_by_texture.clear();
        Ok(())
    }

    /// Create GpuState and immediately load specific textures (for editor use)
    pub fn new_with_textures(
        window: Arc<Window>,
        tilemap_texture: Option<PathBuf>,
        sprite_texture: Option<PathBuf>,
    ) -> Result<Self, crate::RenderError> {
        let (device, queue, surface, config) = create_device_and_surface(Arc::clone(&window));

        // Use provided textures; otherwise fall back to a generated 1x1 white texture.
        let tilemap_path = tilemap_texture.unwrap_or_else(default_texture_path);
        let sprite_path = sprite_texture.unwrap_or_else(default_texture_path);

        let tilemap_pipeline = TilemapPipeline::new(&device, &queue, config.format, tilemap_path);

        let sprite_pipeline = SpritePipeline::new(&device, &queue, config.format, sprite_path);

        let debug_pipeline = DebugPipeline::new(&device, config.format);
        let ui_rect_pipeline = DebugPipeline::new(&device, config.format);
        let ui_debug_pipeline = DebugPipeline::new(&device, config.format);
        let text_renderer = GlyphonTextRenderer::new(&device, &queue, config.format);

        Ok(Self {
            surface,
            config,
            device,
            queue,
            tilemap_pipeline,
            sprite_pipeline,
            sprite_pipelines_by_texture: BTreeMap::new(),
            debug_pipeline,
            ui_rect_pipeline,
            ui_debug_pipeline,
            text_renderer,
            text_items: Vec::new(),
            tilemap_render_enabled: true,
            current_mvp: glam::Mat4::IDENTITY,
        })
    }

    pub fn set_tilemap_render_enabled(&mut self, enabled: bool) {
        self.tilemap_render_enabled = enabled;
    }

    pub fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]) {
        self.tilemap_pipeline
            .update_vertices(&self.device, vertices);
    }

    pub fn update_projection(&mut self, mvp: glam::Mat4) {
        self.current_mvp = mvp;
        self.tilemap_pipeline.update_projection(&self.queue, mvp);
        self.sprite_pipeline.update_projection(&self.queue, mvp);
        for pipeline in self.sprite_pipelines_by_texture.values_mut() {
            pipeline.update_projection(&self.queue, mvp);
        }
        self.debug_pipeline.update_camera(&self.queue, mvp);
    }

    pub fn draw(&mut self) {
        // Update pipelines before rendering
        self.tilemap_pipeline.update_with_queue(&self.queue);
        self.sprite_pipeline.update_with_queue(&self.queue);
        for pipeline in self.sprite_pipelines_by_texture.values_mut() {
            pipeline.update_with_queue(&self.queue);
        }

        let text_backgrounds = self
            .text_renderer
            .prepare(
                &self.device,
                &self.queue,
                self.config.width,
                self.config.height,
                &self.text_items,
                self.current_mvp,
            )
            .unwrap_or_else(|error| {
                tracing::warn!("Failed to prepare text renderer: {error}");
                Vec::new()
            });
        self.refresh_ui_text_backgrounds(&text_backgrounds);

        let output = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_viewport(
                0.0,
                0.0,
                self.config.width as f32,
                self.config.height as f32,
                0.0,
                1.0,
            );

            // Render tilemap first (background)
            if self.tilemap_render_enabled {
                self.tilemap_pipeline.render(&mut render_pass);
            }

            // Render sprites on top
            self.sprite_pipeline.render(&mut render_pass);
            for pipeline in self.sprite_pipelines_by_texture.values() {
                pipeline.render(&mut render_pass);
            }

            // Render debug shapes last (on top of everything)
            self.debug_pipeline.render(&mut render_pass);

            // Render generic runtime UI rectangles in screen-space above world debug overlays.
            self.ui_rect_pipeline.render(&mut render_pass);

            // Render UI background rectangles for text boxes in screen-space.
            self.ui_debug_pipeline.render(&mut render_pass);

            if let Err(error) = self.text_renderer.render(&mut render_pass) {
                tracing::warn!("Failed to render text layer: {error}");
            }
        }

        self.queue.submit(Some(encoder.finish()));
        output.present();
    }

    fn refresh_ui_text_backgrounds(&mut self, backgrounds: &[TextBackgroundRect]) {
        self.ui_debug_pipeline.clear();
        for rect in backgrounds {
            self.ui_debug_pipeline.add_filled_rect(
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                rect.background_color,
            );
            if let Some(border_color) = rect.border_color {
                self.ui_debug_pipeline.add_rect(
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    border_color,
                );
            }
        }
        self.ui_debug_pipeline.update_camera(
            &self.queue,
            screen_space_projection(self.config.width as f32, self.config.height as f32),
        );
        self.ui_debug_pipeline.update_vertices(&self.device);
    }
}

#[cfg(test)]
#[path = "gpu_tests.rs"]
mod tests;
