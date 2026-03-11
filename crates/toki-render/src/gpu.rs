use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;

use toki_core::graphics::vertex::QuadVertex;
use toki_core::sprite::SpriteFrame;

use crate::pipelines::sprite::SpriteInstance;
use crate::pipelines::RenderPipeline;
use crate::wgpu_utils::create_device_and_surface;
use crate::{DebugPipeline, SpritePipeline, TilemapPipeline};

#[allow(dead_code)]
#[derive(Debug)]
pub struct GpuState {
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
    tilemap_pipeline: TilemapPipeline,
    sprite_pipeline: SpritePipeline,
    debug_pipeline: DebugPipeline,
}

fn to_absolute_path<P: AsRef<Path>>(relative: P) -> std::io::Result<PathBuf> {
    fs::canonicalize(relative)
}

impl GpuState {
    pub fn add_sprite(&mut self, frame: SpriteFrame, pos: glam::IVec2, size: glam::UVec2) {
        let instance = SpriteInstance {
            frame,
            position: pos.as_vec2(), // Convert to float for GPU
            size: size.as_vec2(),    // Convert to float for GPU
        };
        self.sprite_pipeline.add_sprite(instance);
    }

    pub fn clear_sprites(&mut self) {
        self.sprite_pipeline.clear_sprites();
    }

    /// Clear all debug shapes
    pub fn clear_debug_shapes(&mut self) {
        self.debug_pipeline.clear();
    }

    /// Add a debug rectangle
    pub fn add_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.debug_pipeline.add_rect(x, y, width, height, color);
    }

    /// Finalize debug shapes for rendering (call after adding all shapes)
    pub fn finalize_debug_shapes(&mut self) {
        self.debug_pipeline.update_vertices(&self.device);
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

        let tilemap_pipeline = TilemapPipeline::new(
            &device,
            &queue,
            config.format,
            to_absolute_path("./assets/terrain.png").unwrap(),
        );

        let sprite_pipeline = SpritePipeline::new(
            &device,
            &queue,
            config.format,
            to_absolute_path("./assets/creatures.png").unwrap(),
        );

        let debug_pipeline = DebugPipeline::new(&device, config.format);

        Self {
            surface,
            config,
            device,
            queue,
            tilemap_pipeline,
            sprite_pipeline,
            debug_pipeline,
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
        Ok(())
    }

    /// Create GpuState and immediately load specific textures (for editor use)
    pub fn new_with_textures(
        window: Arc<Window>,
        tilemap_texture: Option<PathBuf>,
        sprite_texture: Option<PathBuf>,
    ) -> Result<Self, crate::RenderError> {
        let (device, queue, surface, config) = create_device_and_surface(Arc::clone(&window));

        // Use provided textures or fall back to defaults
        let tilemap_path = tilemap_texture.unwrap_or_else(|| {
            to_absolute_path("./assets/terrain.png")
                .unwrap_or_else(|_| PathBuf::from("./assets/terrain.png"))
        });

        let sprite_path = sprite_texture.unwrap_or_else(|| {
            to_absolute_path("./assets/creatures.png")
                .unwrap_or_else(|_| PathBuf::from("./assets/creatures.png"))
        });

        let tilemap_pipeline = TilemapPipeline::new(&device, &queue, config.format, tilemap_path);

        let sprite_pipeline = SpritePipeline::new(&device, &queue, config.format, sprite_path);

        let debug_pipeline = DebugPipeline::new(&device, config.format);

        Ok(Self {
            surface,
            config,
            device,
            queue,
            tilemap_pipeline,
            sprite_pipeline,
            debug_pipeline,
        })
    }

    pub fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]) {
        self.tilemap_pipeline
            .update_vertices(&self.device, vertices);
    }

    pub fn update_projection(&mut self, mvp: glam::Mat4) {
        self.tilemap_pipeline.update_projection(&self.queue, mvp);
        self.sprite_pipeline.update_projection(&self.queue, mvp);
        self.debug_pipeline.update_camera(&self.queue, mvp);
    }

    pub fn draw(&mut self) {
        // Update pipelines before rendering
        self.tilemap_pipeline.update_with_queue(&self.queue);
        self.sprite_pipeline.update_with_queue(&self.queue);

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
            self.tilemap_pipeline.render(&mut render_pass);

            // Render sprites on top
            self.sprite_pipeline.render(&mut render_pass);

            // Render debug shapes last (on top of everything)
            self.debug_pipeline.render(&mut render_pass);
        }

        self.queue.submit(Some(encoder.finish()));
        output.present();
    }
}

#[cfg(test)]
mod tests {
    use super::to_absolute_path;

    #[test]
    fn to_absolute_path_resolves_existing_path() {
        let absolute = to_absolute_path(".").expect("current dir should be canonicalizable");
        assert!(absolute.is_absolute());
    }

    #[test]
    fn to_absolute_path_returns_error_for_missing_path() {
        let missing = "this/path/should/not/exist/for/toki-render-tests";
        let err = to_absolute_path(missing).expect_err("missing path should fail");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
