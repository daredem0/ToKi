use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;

use toki_core::graphics::image::DecodedImage;
use toki_core::graphics::vertex::QuadVertex;
use toki_core::math::projection::screen_space_projection;
use toki_core::project_runtime::{PostProcessMode, ResolvedPostProcessSettings};
use toki_core::sprite::SpriteFrame;
use toki_core::text::TextItem;

use crate::pipelines::sprite::SpriteInstance;
use crate::pipelines::{RenderPipeline, TextureSource};
use crate::sprite_batch_order::{append_ordered_draw_batch, OrderedDrawBatch};
use crate::targets::{OffscreenTarget, RenderTarget};
use crate::wgpu_utils::{choose_present_mode, create_device_and_surface};
use crate::{
    DebugPipeline, GlyphonTextRenderer, PostProcessPipeline, SpritePipeline, TextBackgroundRect,
    TilemapPipeline,
};

#[allow(dead_code)]
pub struct GpuState {
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    supported_present_modes: Vec<wgpu::PresentMode>,
    device: Device,
    queue: Queue,
    tilemap_pipeline: TilemapPipeline,
    sprite_pipeline: SpritePipeline,
    sprite_pipelines_by_texture: BTreeMap<PathBuf, SpritePipeline>,
    sprite_draw_batches: Vec<OrderedDrawBatch<GpuSpriteBatchKey>>,
    world_underlay_pipeline: DebugPipeline,
    debug_pipeline: DebugPipeline,
    ui_rect_pipeline: DebugPipeline,
    ui_debug_pipeline: DebugPipeline,
    post_process_pipeline: PostProcessPipeline,
    post_process_target: Option<OffscreenTarget>,
    post_process_settings: ResolvedPostProcessSettings,
    text_renderer: GlyphonTextRenderer,
    text_items: Vec<TextItem>,
    tilemap_render_enabled: bool,
    current_mvp: glam::Mat4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GpuSpriteBatchKey {
    Default,
    Textured(PathBuf),
}

impl std::fmt::Debug for GpuState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuState")
            .field("config", &self.config)
            .field("tilemap_render_enabled", &self.tilemap_render_enabled)
            .field("post_process_mode", &self.post_process_settings.mode)
            .field("text_items_len", &self.text_items.len())
            .finish_non_exhaustive()
    }
}

fn default_texture_path() -> PathBuf {
    // Empty path activates the built-in 1x1 white texture fallback in GpuTexture::from_file.
    PathBuf::new()
}

impl GpuState {
    fn default_post_process_settings() -> ResolvedPostProcessSettings {
        ResolvedPostProcessSettings {
            mode: PostProcessMode::None,
            quantize_strategy: toki_core::project_runtime::QuantizeStrategy::Luminance,
            tint_color: [0, 0, 0, 255],
            tint_strength_percent: 0,
            brightness_percent: 0,
            saturation_percent: 100,
            quantize_palette: toki_core::palette::Palette4::new([
                [0x11, 0x11, 0x11, 0xFF],
                [0x55, 0x55, 0x55, 0xFF],
                [0xAA, 0xAA, 0xAA, 0xFF],
                [0xF0, 0xF0, 0xF0, 0xFF],
            ]),
            gb_contrast_percent: 0,
            vignette_strength_percent: 60,
        }
    }

    fn ensure_post_process_target(&mut self) -> Result<(), crate::RenderError> {
        let size = (self.config.width.max(1), self.config.height.max(1));
        let target = self.post_process_target.get_or_insert(OffscreenTarget::new(
            self.device.clone(),
            size,
            self.config.format,
        )?);
        target.resize(size)?;
        self.post_process_pipeline
            .update_source_texture(&self.device, target.get_render_view()?);
        Ok(())
    }

    fn render_scene_to_view(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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

        if self.tilemap_render_enabled {
            self.tilemap_pipeline.render(&mut render_pass);
        }

        self.world_underlay_pipeline.render(&mut render_pass);
        let sprite_pipeline = &self.sprite_pipeline;
        let sprite_pipelines_by_texture = &self.sprite_pipelines_by_texture;
        let sprite_draw_batches = &self.sprite_draw_batches;
        for batch in sprite_draw_batches {
            match &batch.key {
                GpuSpriteBatchKey::Default => {
                    sprite_pipeline.render_range(&mut render_pass, batch.start, batch.count);
                }
                GpuSpriteBatchKey::Textured(texture_path) => {
                    if let Some(pipeline) = sprite_pipelines_by_texture.get(texture_path) {
                        pipeline.render_range(&mut render_pass, batch.start, batch.count);
                    }
                }
            }
        }
        self.debug_pipeline.render(&mut render_pass);
        self.ui_rect_pipeline.render(&mut render_pass);
        self.ui_debug_pipeline.render(&mut render_pass);

        if let Err(error) = self.text_renderer.render(&mut render_pass) {
            tracing::warn!("Failed to render text layer: {error}");
        }
    }

    fn record_sprite_draw_batch(&mut self, key: GpuSpriteBatchKey, start: usize) {
        append_ordered_draw_batch(&mut self.sprite_draw_batches, key, start);
    }

    pub fn add_sprite(&mut self, frame: SpriteFrame, pos: glam::IVec2, size: glam::UVec2) {
        let instance = SpriteInstance {
            frame,
            position: pos.as_vec2(), // Convert to float for GPU
            size: size.as_vec2(),    // Convert to float for GPU
            flip_x: false,
        };
        let instance_index = self.sprite_pipeline.instance_count();
        self.sprite_pipeline.add_sprite(instance);
        self.record_sprite_draw_batch(GpuSpriteBatchKey::Default, instance_index);
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
        let instance_index = self.sprite_pipeline.instance_count();
        self.sprite_pipeline.add_sprite(instance);
        self.record_sprite_draw_batch(GpuSpriteBatchKey::Default, instance_index);
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
        let instance_index = self
            .sprite_pipelines_by_texture
            .get(&texture_path)
            .map(|pipeline| pipeline.instance_count())
            .unwrap_or(0);
        let pipeline = self
            .sprite_pipelines_by_texture
            .entry(texture_path.clone())
            .or_insert_with(|| {
                SpritePipeline::new(
                    &self.device,
                    &self.queue,
                    self.config.format,
                    TextureSource::path(texture_path.clone()),
                )
            });
        pipeline.update_projection(&self.queue, self.current_mvp);
        pipeline.add_sprite(instance);
        self.record_sprite_draw_batch(GpuSpriteBatchKey::Textured(texture_path), instance_index);
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
        let instance_index = self
            .sprite_pipelines_by_texture
            .get(&texture_path)
            .map(|pipeline| pipeline.instance_count())
            .unwrap_or(0);
        let pipeline = self
            .sprite_pipelines_by_texture
            .entry(texture_path.clone())
            .or_insert_with(|| {
                SpritePipeline::new(
                    &self.device,
                    &self.queue,
                    self.config.format,
                    TextureSource::path(texture_path.clone()),
                )
            });
        pipeline.update_projection(&self.queue, self.current_mvp);
        pipeline.add_sprite(instance);
        self.record_sprite_draw_batch(GpuSpriteBatchKey::Textured(texture_path), instance_index);
    }

    pub fn add_sprite_with_texture_rgba8(
        &mut self,
        texture_key: PathBuf,
        image: &DecodedImage,
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
        let instance_index = self
            .sprite_pipelines_by_texture
            .get(&texture_key)
            .map(|pipeline| pipeline.instance_count())
            .unwrap_or(0);
        let pipeline = self
            .sprite_pipelines_by_texture
            .entry(texture_key.clone())
            .or_insert_with(|| {
                SpritePipeline::new(
                    &self.device,
                    &self.queue,
                    self.config.format,
                    TextureSource::rgba8(image),
                )
            });
        pipeline.update_projection(&self.queue, self.current_mvp);
        pipeline.add_sprite(instance);
        self.record_sprite_draw_batch(GpuSpriteBatchKey::Textured(texture_key), instance_index);
    }

    pub fn clear_sprites(&mut self) {
        self.sprite_pipeline.clear_sprites();
        for pipeline in self.sprite_pipelines_by_texture.values_mut() {
            pipeline.clear_sprites();
        }
        self.sprite_draw_batches.clear();
    }

    pub fn clear_text_items(&mut self) {
        self.text_items.clear();
    }

    pub fn add_text_item(&mut self, text: TextItem) {
        self.text_items.push(text);
    }

    pub fn clear_world_underlay_shapes(&mut self) {
        self.world_underlay_pipeline.clear();
    }

    pub fn add_world_underlay_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    ) {
        self.world_underlay_pipeline
            .add_rect(x, y, width, height, color);
    }

    pub fn add_filled_world_underlay_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    ) {
        self.world_underlay_pipeline
            .add_filled_rect(x, y, width, height, color);
    }

    pub fn finalize_world_underlay_shapes(&mut self) {
        self.world_underlay_pipeline.update_vertices(&self.device);
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
            if let Some(target) = &mut self.post_process_target {
                if let Err(error) = target.resize((new_size.width, new_size.height)) {
                    tracing::warn!("Failed to resize post-process target: {error}");
                } else if let Ok(view) = target.get_render_view() {
                    self.post_process_pipeline
                        .update_source_texture(&self.device, view);
                }
            }
        }
    }

    pub fn new(window: Arc<Window>, vsync: bool) -> Self {
        let (device, queue, surface, config, supported_present_modes) =
            create_device_and_surface(Arc::clone(&window), vsync);

        let tilemap_pipeline =
            TilemapPipeline::new(
                &device,
                &queue,
                config.format,
                TextureSource::path(default_texture_path()),
            );

        let sprite_pipeline =
            SpritePipeline::new(
                &device,
                &queue,
                config.format,
                TextureSource::path(default_texture_path()),
            );

        let world_underlay_pipeline = DebugPipeline::new(&device, config.format);
        let debug_pipeline = DebugPipeline::new(&device, config.format);
        let ui_rect_pipeline = DebugPipeline::new(&device, config.format);
        let ui_debug_pipeline = DebugPipeline::new(&device, config.format);
        let post_process_pipeline = PostProcessPipeline::new(&device, config.format);
        let text_renderer = GlyphonTextRenderer::new(&device, &queue, config.format);

        Self {
            surface,
            config,
            supported_present_modes,
            device,
            queue,
            tilemap_pipeline,
            sprite_pipeline,
            sprite_pipelines_by_texture: BTreeMap::new(),
            sprite_draw_batches: Vec::new(),
            world_underlay_pipeline,
            debug_pipeline,
            ui_rect_pipeline,
            ui_debug_pipeline,
            post_process_pipeline,
            post_process_target: None,
            post_process_settings: Self::default_post_process_settings(),
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
            TilemapPipeline::new(
                &self.device,
                &self.queue,
                self.config.format,
                TextureSource::path(texture_path),
            );
        self.tilemap_pipeline = new_pipeline;
        Ok(())
    }

    pub fn load_tilemap_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), crate::RenderError> {
        let new_pipeline =
            TilemapPipeline::new(
                &self.device,
                &self.queue,
                self.config.format,
                TextureSource::rgba8(image),
            );
        self.tilemap_pipeline = new_pipeline;
        Ok(())
    }

    /// Load a new sprite texture at runtime
    pub fn load_sprite_texture(&mut self, texture_path: PathBuf) -> Result<(), crate::RenderError> {
        // Create new sprite pipeline with the specified texture
        let new_pipeline =
            SpritePipeline::new(
                &self.device,
                &self.queue,
                self.config.format,
                TextureSource::path(texture_path),
            );
        self.sprite_pipeline = new_pipeline;
        self.sprite_pipelines_by_texture.clear();
        Ok(())
    }

    pub fn load_sprite_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), crate::RenderError> {
        let new_pipeline =
            SpritePipeline::new(
                &self.device,
                &self.queue,
                self.config.format,
                TextureSource::rgba8(image),
            );
        self.sprite_pipeline = new_pipeline;
        self.sprite_pipelines_by_texture.clear();
        Ok(())
    }

    /// Create GpuState and immediately load specific textures (for editor use)
    pub fn new_with_textures(
        window: Arc<Window>,
        vsync: bool,
        tilemap_texture: Option<PathBuf>,
        sprite_texture: Option<PathBuf>,
    ) -> Result<Self, crate::RenderError> {
        let (device, queue, surface, config, supported_present_modes) =
            create_device_and_surface(Arc::clone(&window), vsync);

        // Use provided textures; otherwise fall back to a generated 1x1 white texture.
        let tilemap_path = tilemap_texture.unwrap_or_else(default_texture_path);
        let sprite_path = sprite_texture.unwrap_or_else(default_texture_path);

        let tilemap_pipeline = TilemapPipeline::new(
            &device,
            &queue,
            config.format,
            TextureSource::path(tilemap_path),
        );

        let sprite_pipeline = SpritePipeline::new(
            &device,
            &queue,
            config.format,
            TextureSource::path(sprite_path),
        );

        let world_underlay_pipeline = DebugPipeline::new(&device, config.format);
        let debug_pipeline = DebugPipeline::new(&device, config.format);
        let ui_rect_pipeline = DebugPipeline::new(&device, config.format);
        let ui_debug_pipeline = DebugPipeline::new(&device, config.format);
        let post_process_pipeline = PostProcessPipeline::new(&device, config.format);
        let text_renderer = GlyphonTextRenderer::new(&device, &queue, config.format);

        Ok(Self {
            surface,
            config,
            supported_present_modes,
            device,
            queue,
            tilemap_pipeline,
            sprite_pipeline,
            sprite_pipelines_by_texture: BTreeMap::new(),
            sprite_draw_batches: Vec::new(),
            world_underlay_pipeline,
            debug_pipeline,
            ui_rect_pipeline,
            ui_debug_pipeline,
            post_process_pipeline,
            post_process_target: None,
            post_process_settings: Self::default_post_process_settings(),
            text_renderer,
            text_items: Vec::new(),
            tilemap_render_enabled: true,
            current_mvp: glam::Mat4::IDENTITY,
        })
    }

    pub fn set_tilemap_render_enabled(&mut self, enabled: bool) {
        self.tilemap_render_enabled = enabled;
    }

    pub fn set_post_process_settings(&mut self, settings: ResolvedPostProcessSettings) {
        self.post_process_settings = settings;
    }

    pub fn set_vsync(&mut self, enabled: bool) {
        let next_mode = choose_present_mode(&self.supported_present_modes, enabled);
        if self.config.present_mode != next_mode {
            self.config.present_mode = next_mode;
            self.surface.configure(&self.device, &self.config);
        }
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
        self.world_underlay_pipeline.update_camera(&self.queue, mvp);
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

        if self.post_process_settings.mode == PostProcessMode::None {
            self.render_scene_to_view(&mut encoder, &view);
        } else if let Err(error) = self.ensure_post_process_target() {
            tracing::warn!("Failed to prepare post-process target: {error}");
            self.render_scene_to_view(&mut encoder, &view);
        } else if let Some(target) = &mut self.post_process_target {
            self.post_process_pipeline
                .update_settings(&self.queue, self.post_process_settings);
            let target_view = target
                .get_render_view()
                .expect("post-process target render view must exist")
                .clone();
            self.render_scene_to_view(&mut encoder, &target_view);
            let mut post_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Post Process Pass"),
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
            self.post_process_pipeline.render(&mut post_pass);
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

impl crate::RenderBackend for GpuState {
    fn load_tilemap_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), crate::RenderError> {
        GpuState::load_tilemap_texture(self, texture_path)
    }

    fn load_tilemap_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), crate::RenderError> {
        GpuState::load_tilemap_texture_rgba8(self, image)
    }

    fn load_sprite_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), crate::RenderError> {
        GpuState::load_sprite_texture(self, texture_path)
    }

    fn load_sprite_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), crate::RenderError> {
        GpuState::load_sprite_texture_rgba8(self, image)
    }

    fn add_sprite_with_texture_rgba8(
        &mut self,
        texture_key: PathBuf,
        image: &DecodedImage,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        GpuState::add_sprite_with_texture_rgba8(
            self,
            texture_key,
            image,
            frame,
            position,
            size,
            flip_x,
        );
    }

    fn load_font_file(&mut self, font_path: std::path::PathBuf) -> Result<(), crate::RenderError> {
        GpuState::load_font_file(self, &font_path)
    }

    fn update_projection(&mut self, mvp: glam::Mat4) {
        GpuState::update_projection(self, mvp);
    }

    fn set_tilemap_render_enabled(&mut self, enabled: bool) {
        GpuState::set_tilemap_render_enabled(self, enabled);
    }

    fn set_post_process_settings(&mut self, settings: ResolvedPostProcessSettings) {
        GpuState::set_post_process_settings(self, settings);
    }

    fn set_vsync(&mut self, enabled: bool) {
        GpuState::set_vsync(self, enabled);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        GpuState::resize(self, new_size);
    }

    fn draw(&mut self) {
        GpuState::draw(self);
    }

    fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]) {
        GpuState::update_tilemap_vertices(self, vertices);
    }

    fn clear_sprites(&mut self) {
        GpuState::clear_sprites(self);
    }

    fn add_sprite(
        &mut self,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        GpuState::add_sprite_flipped(self, frame, position, size, flip_x);
    }

    fn add_sprite_with_texture(
        &mut self,
        texture_path: std::path::PathBuf,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        GpuState::add_sprite_with_texture_flipped(
            self,
            texture_path,
            frame,
            position,
            size,
            flip_x,
        );
    }

    fn clear_text_items(&mut self) {
        GpuState::clear_text_items(self);
    }

    fn add_text_item(&mut self, text: TextItem) {
        GpuState::add_text_item(self, text);
    }

    fn clear_world_underlay_shapes(&mut self) {
        GpuState::clear_world_underlay_shapes(self);
    }

    fn add_world_underlay_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    ) {
        GpuState::add_world_underlay_rect(self, x, y, width, height, color);
    }

    fn add_filled_world_underlay_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    ) {
        GpuState::add_filled_world_underlay_rect(self, x, y, width, height, color);
    }

    fn finalize_world_underlay_shapes(&mut self) {
        GpuState::finalize_world_underlay_shapes(self);
    }

    fn clear_debug_shapes(&mut self) {
        GpuState::clear_debug_shapes(self);
    }

    fn add_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        GpuState::add_debug_rect(self, x, y, width, height, color);
    }

    fn add_filled_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        GpuState::add_filled_debug_rect(self, x, y, width, height, color);
    }

    fn finalize_debug_shapes(&mut self) {
        GpuState::finalize_debug_shapes(self);
    }

    fn clear_ui_shapes(&mut self) {
        GpuState::clear_ui_rects(self);
    }

    fn add_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        GpuState::add_ui_rect(self, x, y, width, height, color);
    }

    fn add_filled_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        GpuState::add_filled_ui_rect(self, x, y, width, height, color);
    }

    fn finalize_ui_shapes(&mut self) {
        GpuState::finalize_ui_rects(self);
    }
}

#[cfg(test)]
#[path = "gpu_tests.rs"]
mod tests;
