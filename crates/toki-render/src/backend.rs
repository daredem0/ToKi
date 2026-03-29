use std::path::PathBuf;
use toki_core::graphics::image::DecodedImage;
use toki_core::graphics::vertex::QuadVertex;
use toki_core::project_runtime::ResolvedPostProcessSettings;
use toki_core::sprite::SpriteFrame;
use toki_core::text::TextItem;

use crate::RenderError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneClipRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Trait defining the rendering backend interface.
///
/// This trait abstracts GPU rendering operations, allowing for different implementations
/// (real GPU via wgpu, or mock for testing). It consolidates all rendering operations
/// into a single interface.
pub trait RenderBackend: std::fmt::Debug {
    /// Set the clip rect used for the scene pass. When present, all scene-pass rendering is
    /// scissored to this rectangle and the cleared background remains visible outside it.
    fn set_scene_clip_rect(&mut self, rect: Option<SceneClipRect>);

    /// Load a tilemap texture from file
    fn load_tilemap_texture(&mut self, texture_path: PathBuf) -> Result<(), RenderError>;

    /// Load a tilemap texture from raw RGBA8 image data
    fn load_tilemap_texture_rgba8(&mut self, image: &DecodedImage) -> Result<(), RenderError>;

    /// Load a sprite texture from file
    fn load_sprite_texture(&mut self, texture_path: PathBuf) -> Result<(), RenderError>;

    /// Load a sprite texture from raw RGBA8 image data
    fn load_sprite_texture_rgba8(&mut self, image: &DecodedImage) -> Result<(), RenderError>;

    /// Add a sprite using a cached RGBA8 texture identified by a synthetic texture key.
    fn add_sprite_with_texture_rgba8(
        &mut self,
        texture_key: PathBuf,
        image: &DecodedImage,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    );

    /// Load a font file for text rendering
    fn load_font_file(&mut self, font_path: PathBuf) -> Result<(), RenderError>;

    /// Update the projection/view matrix
    fn update_projection(&mut self, mvp: glam::Mat4);

    /// Update runtime post-process settings.
    fn set_post_process_settings(&mut self, settings: ResolvedPostProcessSettings);

    /// Enable or disable vsync by reconfiguring the surface present mode when supported.
    fn set_vsync(&mut self, enabled: bool);

    /// Enable or disable tilemap rendering
    fn set_tilemap_render_enabled(&mut self, enabled: bool);

    /// Resize the render surface
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>);

    /// Draw the current frame
    fn draw(&mut self);

    /// Update tilemap vertex data
    fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]);

    /// Clear all sprites
    fn clear_sprites(&mut self);

    /// Add a sprite to be rendered
    fn add_sprite(
        &mut self,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    );

    /// Add a sprite with a specific texture
    fn add_sprite_with_texture(
        &mut self,
        texture_path: PathBuf,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    );

    /// Clear all text items
    fn clear_text_items(&mut self);

    /// Add a text item to be rendered
    fn add_text_item(&mut self, text: TextItem);

    /// Clear all world underlay shapes rendered below sprites.
    fn clear_world_underlay_shapes(&mut self);

    /// Add an outline rectangle to the world underlay lane.
    fn add_world_underlay_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]);

    /// Add a filled rectangle to the world underlay lane.
    fn add_filled_world_underlay_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    );

    /// Finalize world underlay shapes for rendering.
    fn finalize_world_underlay_shapes(&mut self);

    /// Clear all debug shapes
    fn clear_debug_shapes(&mut self);

    /// Add a debug rectangle outline
    fn add_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]);

    /// Add a filled debug rectangle
    fn add_filled_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]);

    /// Finalize debug shapes for rendering
    fn finalize_debug_shapes(&mut self);

    /// Clear all UI shapes
    fn clear_ui_shapes(&mut self);

    /// Add a UI rectangle outline
    fn add_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]);

    /// Add a filled UI rectangle
    fn add_filled_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]);

    /// Finalize UI shapes for rendering
    fn finalize_ui_shapes(&mut self);
}

#[allow(dead_code)]
pub trait TextureLoader: RenderBackend {
    fn load_tilemap_texture(&mut self, texture_path: PathBuf) -> Result<(), RenderError> {
        RenderBackend::load_tilemap_texture(self, texture_path)
    }

    fn load_tilemap_texture_rgba8(&mut self, image: &DecodedImage) -> Result<(), RenderError> {
        RenderBackend::load_tilemap_texture_rgba8(self, image)
    }

    fn load_sprite_texture(&mut self, texture_path: PathBuf) -> Result<(), RenderError> {
        RenderBackend::load_sprite_texture(self, texture_path)
    }

    fn load_sprite_texture_rgba8(&mut self, image: &DecodedImage) -> Result<(), RenderError> {
        RenderBackend::load_sprite_texture_rgba8(self, image)
    }

    fn load_font_file(&mut self, font_path: PathBuf) -> Result<(), RenderError> {
        RenderBackend::load_font_file(self, font_path)
    }
}

impl<T: RenderBackend + ?Sized> TextureLoader for T {}

#[allow(dead_code)]
pub trait SpriteRenderer: RenderBackend {
    fn clear_sprites(&mut self) {
        RenderBackend::clear_sprites(self);
    }

    fn add_sprite(
        &mut self,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        RenderBackend::add_sprite(self, frame, position, size, flip_x);
    }

    fn add_sprite_with_texture(
        &mut self,
        texture_path: PathBuf,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        RenderBackend::add_sprite_with_texture(self, texture_path, frame, position, size, flip_x);
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
        RenderBackend::add_sprite_with_texture_rgba8(
            self,
            texture_key,
            image,
            frame,
            position,
            size,
            flip_x,
        );
    }
}

impl<T: RenderBackend + ?Sized> SpriteRenderer for T {}

#[allow(dead_code)]
pub trait ShapeRenderer: RenderBackend {
    fn clear_world_underlay_shapes(&mut self) {
        RenderBackend::clear_world_underlay_shapes(self);
    }

    fn add_world_underlay_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    ) {
        RenderBackend::add_world_underlay_rect(self, x, y, width, height, color);
    }

    fn add_filled_world_underlay_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    ) {
        RenderBackend::add_filled_world_underlay_rect(self, x, y, width, height, color);
    }

    fn finalize_world_underlay_shapes(&mut self) {
        RenderBackend::finalize_world_underlay_shapes(self);
    }

    fn clear_debug_shapes(&mut self) {
        RenderBackend::clear_debug_shapes(self);
    }

    fn add_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        RenderBackend::add_debug_rect(self, x, y, width, height, color);
    }

    fn add_filled_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        RenderBackend::add_filled_debug_rect(self, x, y, width, height, color);
    }

    fn finalize_debug_shapes(&mut self) {
        RenderBackend::finalize_debug_shapes(self);
    }

    fn clear_ui_shapes(&mut self) {
        RenderBackend::clear_ui_shapes(self);
    }

    fn add_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        RenderBackend::add_ui_rect(self, x, y, width, height, color);
    }

    fn add_filled_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        RenderBackend::add_filled_ui_rect(self, x, y, width, height, color);
    }

    fn finalize_ui_shapes(&mut self) {
        RenderBackend::finalize_ui_shapes(self);
    }
}

impl<T: RenderBackend + ?Sized> ShapeRenderer for T {}

#[allow(dead_code)]
pub trait TextRenderer: RenderBackend {
    fn clear_text_items(&mut self) {
        RenderBackend::clear_text_items(self);
    }

    fn add_text_item(&mut self, text: TextItem) {
        RenderBackend::add_text_item(self, text);
    }
}

impl<T: RenderBackend + ?Sized> TextRenderer for T {}

#[allow(dead_code)]
pub trait FrameLifecycle: RenderBackend {
    fn set_scene_clip_rect(&mut self, rect: Option<SceneClipRect>) {
        RenderBackend::set_scene_clip_rect(self, rect);
    }

    fn update_projection(&mut self, mvp: glam::Mat4) {
        RenderBackend::update_projection(self, mvp);
    }

    fn set_post_process_settings(&mut self, settings: ResolvedPostProcessSettings) {
        RenderBackend::set_post_process_settings(self, settings);
    }

    fn set_vsync(&mut self, enabled: bool) {
        RenderBackend::set_vsync(self, enabled);
    }

    fn set_tilemap_render_enabled(&mut self, enabled: bool) {
        RenderBackend::set_tilemap_render_enabled(self, enabled);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        RenderBackend::resize(self, new_size);
    }

    fn draw(&mut self) {
        RenderBackend::draw(self);
    }

    fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]) {
        RenderBackend::update_tilemap_vertices(self, vertices);
    }
}

impl<T: RenderBackend + ?Sized> FrameLifecycle for T {}
