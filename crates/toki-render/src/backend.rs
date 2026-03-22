use std::path::PathBuf;
use toki_core::graphics::image::DecodedImage;
use toki_core::graphics::vertex::QuadVertex;
use toki_core::sprite::SpriteFrame;
use toki_core::text::TextItem;

use crate::RenderError;

/// Trait defining the rendering backend interface.
///
/// This trait abstracts GPU rendering operations, allowing for different implementations
/// (real GPU via wgpu, or mock for testing). It consolidates all rendering operations
/// into a single interface.
pub trait RenderBackend: std::fmt::Debug {
    /// Load a tilemap texture from file
    fn load_tilemap_texture(&mut self, texture_path: PathBuf) -> Result<(), RenderError>;

    /// Load a sprite texture from file
    fn load_sprite_texture(&mut self, texture_path: PathBuf) -> Result<(), RenderError>;

    /// Load a sprite texture from raw RGBA8 image data
    fn load_sprite_texture_rgba8(&mut self, image: &DecodedImage) -> Result<(), RenderError>;

    /// Load a font file for text rendering
    fn load_font_file(&mut self, font_path: PathBuf) -> Result<(), RenderError>;

    /// Update the projection/view matrix
    fn update_projection(&mut self, mvp: glam::Mat4);

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
    fn add_world_underlay_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    );

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
