mod frame;
mod textures;

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
    per_frame_lru::PerFrameLruCache, DebugPipeline, GlyphonTextRenderer, PostProcessPipeline, Rect,
    RenderError, SceneClipRect, SpritePipeline, TextBackgroundRect, TilemapPipeline,
};

const GPU_TEXTURED_SPRITE_PIPELINE_CACHE_CAPACITY: usize = 64;

#[allow(dead_code)]
pub struct GpuState {
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    supported_present_modes: Vec<wgpu::PresentMode>,
    device: Device,
    queue: Queue,
    tilemap_pipeline: TilemapPipeline,
    overlay_tilemap_pipeline: TilemapPipeline,
    sprite_pipeline: SpritePipeline,
    sprite_pipelines_by_texture: PerFrameLruCache<PathBuf, SpritePipeline>,
    sprite_draw_batches: Vec<OrderedDrawBatch<GpuSpriteBatchKey>>,
    world_underlay_pipeline: DebugPipeline,
    debug_pipeline: DebugPipeline,
    ui_shape_pipeline: DebugPipeline,
    ui_debug_pipeline: DebugPipeline,
    post_process_pipeline: PostProcessPipeline,
    post_process_target: Option<OffscreenTarget>,
    post_process_settings: ResolvedPostProcessSettings,
    text_renderer: GlyphonTextRenderer,
    text_items: Vec<TextItem>,
    tilemap_render_enabled: bool,
    current_mvp: glam::Mat4,
    scene_clip_rect: Option<SceneClipRect>,
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
    fn new_internal(
        window: Arc<Window>,
        vsync: bool,
        tilemap_texture: TextureSource<'_>,
        sprite_texture: TextureSource<'_>,
    ) -> Result<Self, RenderError> {
        let (device, queue, surface, config, supported_present_modes) =
            create_device_and_surface(Arc::clone(&window), vsync)?;
        let tilemap_pipeline =
            TilemapPipeline::new(&device, &queue, config.format, tilemap_texture)?;
        let overlay_tilemap_pipeline =
            TilemapPipeline::new(&device, &queue, config.format, TextureSource::placeholder())?;
        let sprite_pipeline = SpritePipeline::new(&device, &queue, config.format, sprite_texture)?;
        let world_underlay_pipeline = DebugPipeline::new(&device, config.format);
        let debug_pipeline = DebugPipeline::new(&device, config.format);
        let ui_shape_pipeline = DebugPipeline::new(&device, config.format);
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
            overlay_tilemap_pipeline,
            sprite_pipeline,
            sprite_pipelines_by_texture: PerFrameLruCache::new(
                GPU_TEXTURED_SPRITE_PIPELINE_CACHE_CAPACITY,
            ),
            sprite_draw_batches: Vec::new(),
            world_underlay_pipeline,
            debug_pipeline,
            ui_shape_pipeline,
            ui_debug_pipeline,
            post_process_pipeline,
            post_process_target: None,
            post_process_settings: Self::default_post_process_settings(),
            text_renderer,
            text_items: Vec::new(),
            tilemap_render_enabled: true,
            current_mvp: glam::Mat4::IDENTITY,
            scene_clip_rect: None,
        })
    }

    fn build_sprite_instance(
        frame: SpriteFrame,
        pos: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
        tint_alpha: f32,
    ) -> SpriteInstance {
        SpriteInstance {
            frame,
            position: pos.as_vec2(),
            size: size.as_vec2(),
            flip_x,
            tint_alpha,
        }
    }

    fn add_default_sprite(
        &mut self,
        frame: SpriteFrame,
        pos: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        let instance = Self::build_sprite_instance(frame, pos, size, flip_x, 0.0);
        let instance_index = self.sprite_pipeline.instance_count();
        self.sprite_pipeline.add_sprite(instance);
        self.record_sprite_draw_batch(GpuSpriteBatchKey::Default, instance_index);
    }

    fn add_textured_sprite(
        &mut self,
        texture_key: &Path,
        texture_source: TextureSource<'_>,
        instance: SpriteInstance,
    ) {
        let instance_index = self
            .sprite_pipelines_by_texture
            .get(texture_key)
            .map(|pipeline| pipeline.instance_count())
            .unwrap_or(0);
        let texture_key_buf = texture_key.to_path_buf();
        let insert_result = self.sprite_pipelines_by_texture.get_or_try_insert_with(
            texture_key_buf.clone(),
            || {
                SpritePipeline::new(
                    &self.device,
                    &self.queue,
                    self.config.format,
                    texture_source,
                )
            },
        );
        let Ok(Some(pipeline)) = insert_result else {
            if let Err(error) = insert_result {
                tracing::warn!(
                    texture_key = %texture_key.display(),
                    "Skipping sprite with failed texture pipeline creation: {error}"
                );
            }
            return;
        };
        {
            pipeline.update_projection(&self.queue, self.current_mvp);
            pipeline.add_sprite(instance);
        }
        self.record_sprite_draw_batch(GpuSpriteBatchKey::Textured(texture_key_buf), instance_index);
    }

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

    fn record_sprite_draw_batch(&mut self, key: GpuSpriteBatchKey, start: usize) {
        append_ordered_draw_batch(&mut self.sprite_draw_batches, key, start);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            if let Some(target) = &mut self.post_process_target {
                if let Err(error) = target.resize(&self.device, (new_size.width, new_size.height)) {
                    tracing::warn!("Failed to resize post-process target: {error}");
                } else if let Ok(view) = target.get_render_view() {
                    self.post_process_pipeline
                        .update_source_texture(&self.device, view);
                }
            }
        }
    }

    pub fn new(window: Arc<Window>, vsync: bool) -> Result<Self, RenderError> {
        Self::new_internal(
            window,
            vsync,
            TextureSource::path(default_texture_path().as_path()),
            TextureSource::path(default_texture_path().as_path()),
        )
    }

    /// Create GpuState and immediately load specific textures (for editor use)
    pub fn new_with_textures(
        window: Arc<Window>,
        vsync: bool,
        tilemap_texture: Option<PathBuf>,
        sprite_texture: Option<PathBuf>,
    ) -> Result<Self, crate::RenderError> {
        // Use provided textures; otherwise fall back to a generated 1x1 white texture.
        let tilemap_path = tilemap_texture.unwrap_or_else(default_texture_path);
        let sprite_path = sprite_texture.unwrap_or_else(default_texture_path);
        Self::new_internal(
            window,
            vsync,
            TextureSource::path(tilemap_path.as_path()),
            TextureSource::path(sprite_path.as_path()),
        )
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
            .update_vertices(&self.device, &self.queue, vertices);
    }

    pub fn update_overlay_tilemap_vertices(&mut self, vertices: &[QuadVertex]) {
        self.overlay_tilemap_pipeline
            .update_vertices(&self.device, &self.queue, vertices);
    }

    pub fn update_projection(&mut self, mvp: glam::Mat4) {
        self.current_mvp = mvp;
        self.tilemap_pipeline.update_projection(&self.queue, mvp);
        self.overlay_tilemap_pipeline
            .update_projection(&self.queue, mvp);
        self.sprite_pipeline.update_projection(&self.queue, mvp);
        for pipeline in self.sprite_pipelines_by_texture.values_mut() {
            pipeline.update_projection(&self.queue, mvp);
        }
        self.world_underlay_pipeline.update_camera(&self.queue, mvp);
        self.debug_pipeline.update_camera(&self.queue, mvp);
    }
}

impl crate::RenderFrameControl for GpuState {
    fn set_scene_clip_rect(&mut self, rect: Option<SceneClipRect>) {
        self.scene_clip_rect = rect;
    }

    fn update_projection(&mut self, mvp: glam::Mat4) {
        GpuState::update_projection(self, mvp);
    }

    fn set_post_process_settings(&mut self, settings: ResolvedPostProcessSettings) {
        self.post_process_settings = settings;
    }

    fn set_vsync(&mut self, enabled: bool) {
        GpuState::set_vsync(self, enabled);
    }

    fn set_tilemap_render_enabled(&mut self, enabled: bool) {
        self.tilemap_render_enabled = enabled;
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        GpuState::resize(self, new_size);
    }

    fn draw(&mut self) -> Result<(), crate::RenderError> {
        GpuState::draw(self)
    }

    fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]) {
        GpuState::update_tilemap_vertices(self, vertices);
    }

    fn update_overlay_tilemap_vertices(&mut self, vertices: &[QuadVertex]) {
        GpuState::update_overlay_tilemap_vertices(self, vertices);
    }
}

impl crate::TextureBackend for GpuState {
    fn load_tilemap_texture(&mut self, texture_path: PathBuf) -> Result<(), crate::RenderError> {
        GpuState::load_tilemap_texture(self, texture_path)
    }

    fn load_tilemap_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), crate::RenderError> {
        GpuState::load_tilemap_texture_rgba8(self, image)
    }

    fn load_sprite_texture(&mut self, texture_path: PathBuf) -> Result<(), crate::RenderError> {
        GpuState::load_sprite_texture(self, texture_path)
    }

    fn load_sprite_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), crate::RenderError> {
        GpuState::load_sprite_texture_rgba8(self, image)
    }

    fn load_font_file(&mut self, font_path: PathBuf) -> Result<(), crate::RenderError> {
        self.text_renderer.load_font_file(&font_path)
    }
}

impl crate::SpriteBackend for GpuState {
    fn clear_sprites(&mut self) {
        self.sprite_pipelines_by_texture.begin_frame();
        self.sprite_pipeline.clear_sprites();
        for pipeline in self.sprite_pipelines_by_texture.values_mut() {
            pipeline.clear_sprites();
        }
        self.sprite_draw_batches.clear();
        self.sprite_pipelines_by_texture.evict_unused_lru();
    }

    fn add_sprite(
        &mut self,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        self.add_default_sprite(frame, position, size, flip_x);
    }

    fn add_sprite_with_texture(
        &mut self,
        texture_path: PathBuf,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        let instance = Self::build_sprite_instance(frame, position, size, flip_x, 0.0);
        self.add_textured_sprite(
            texture_path.as_path(),
            TextureSource::path(texture_path.as_path()),
            instance,
        );
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
        let instance = Self::build_sprite_instance(frame, position, size, flip_x, 0.0);
        self.add_textured_sprite(texture_key.as_path(), TextureSource::rgba8(image), instance);
    }

    fn add_shadow_sprite_with_texture(
        &mut self,
        texture_path: PathBuf,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        let instance = Self::build_sprite_instance(frame, position, size, flip_x, 0.35);
        self.add_textured_sprite(
            texture_path.as_path(),
            TextureSource::path(texture_path.as_path()),
            instance,
        );
    }
}

impl crate::TextBackend for GpuState {
    fn clear_text_items(&mut self) {
        self.text_items.clear();
    }

    fn add_text_item(&mut self, text: TextItem) {
        self.text_items.push(text);
    }
}

impl crate::ShapeBackend for GpuState {
    fn clear_world_underlay_shapes(&mut self) {
        self.world_underlay_pipeline.clear();
    }

    fn add_world_underlay_rect(&mut self, rect: Rect, color: [f32; 4]) {
        self.world_underlay_pipeline
            .add_rect(rect.x, rect.y, rect.width, rect.height, color);
    }

    fn add_filled_world_underlay_rect(&mut self, rect: Rect, color: [f32; 4]) {
        self.world_underlay_pipeline.add_filled_rect(
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            color,
        );
    }

    fn finalize_world_underlay_shapes(&mut self) {
        self.world_underlay_pipeline.update_vertices(&self.device);
    }

    fn clear_debug_shapes(&mut self) {
        self.debug_pipeline.clear();
    }

    fn add_debug_rect(&mut self, rect: Rect, color: [f32; 4]) {
        self.debug_pipeline
            .add_rect(rect.x, rect.y, rect.width, rect.height, color);
    }

    fn add_filled_debug_rect(&mut self, rect: Rect, color: [f32; 4]) {
        self.debug_pipeline
            .add_filled_rect(rect.x, rect.y, rect.width, rect.height, color);
    }

    fn finalize_debug_shapes(&mut self) {
        self.debug_pipeline.update_vertices(&self.device);
    }

    fn clear_ui_shapes(&mut self) {
        self.ui_shape_pipeline.clear();
    }

    fn add_ui_shape(&mut self, rect: Rect, color: [f32; 4]) {
        self.ui_shape_pipeline
            .add_rect(rect.x, rect.y, rect.width, rect.height, color);
    }

    fn add_filled_ui_shape(&mut self, rect: Rect, color: [f32; 4]) {
        self.ui_shape_pipeline
            .add_filled_rect(rect.x, rect.y, rect.width, rect.height, color);
    }

    fn finalize_ui_shapes(&mut self) {
        self.ui_shape_pipeline.update_camera(
            &self.queue,
            screen_space_projection(self.config.width as f32, self.config.height as f32),
        );
        self.ui_shape_pipeline.update_vertices(&self.device);
    }
}

#[cfg(test)]
#[path = "../gpu_tests.rs"]
mod tests;
