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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Frame-level render control and scene lifecycle operations.
pub trait RenderFrameControl {
    /// Set the clip rect used for the scene pass. When present, all scene-pass rendering is
    /// scissored to this rectangle and the cleared background remains visible outside it.
    fn set_scene_clip_rect(&mut self, rect: Option<SceneClipRect>);

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
    fn draw(&mut self) -> Result<(), RenderError>;

    /// Update tilemap vertex data
    fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]);
}

/// Texture and font asset loading operations used by rendering backends.
pub trait TextureBackend {
    /// Load a tilemap texture from file
    fn load_tilemap_texture(&mut self, texture_path: PathBuf) -> Result<(), RenderError>;

    /// Load a tilemap texture from raw RGBA8 image data
    fn load_tilemap_texture_rgba8(&mut self, image: &DecodedImage) -> Result<(), RenderError>;

    /// Load a sprite texture from file
    fn load_sprite_texture(&mut self, texture_path: PathBuf) -> Result<(), RenderError>;

    /// Load a sprite texture from raw RGBA8 image data
    fn load_sprite_texture_rgba8(&mut self, image: &DecodedImage) -> Result<(), RenderError>;

    /// Load a font file for text rendering
    fn load_font_file(&mut self, font_path: PathBuf) -> Result<(), RenderError>;
}

/// Sprite submission operations.
pub trait SpriteBackend {
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

    /// Add a shadow silhouette sprite — same texture as the entity, rendered as a black
    /// semi-transparent shape using texture alpha for the silhouette outline.
    fn add_shadow_sprite_with_texture(
        &mut self,
        texture_path: PathBuf,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    );
}

/// Text submission operations.
pub trait TextBackend {
    /// Clear all text items
    fn clear_text_items(&mut self);

    /// Add a text item to be rendered
    fn add_text_item(&mut self, text: TextItem);
}

/// Shape submission operations for world underlay, debug, and UI lanes.
pub trait ShapeBackend {
    /// Clear all world underlay shapes rendered below sprites.
    fn clear_world_underlay_shapes(&mut self);

    /// Add an outline rectangle to the world underlay lane.
    fn add_world_underlay_rect(&mut self, rect: Rect, color: [f32; 4]);

    /// Add a filled rectangle to the world underlay lane.
    fn add_filled_world_underlay_rect(&mut self, rect: Rect, color: [f32; 4]);

    /// Finalize world underlay shapes for rendering.
    fn finalize_world_underlay_shapes(&mut self);

    /// Clear all debug shapes
    fn clear_debug_shapes(&mut self);

    /// Add a debug rectangle outline
    fn add_debug_rect(&mut self, rect: Rect, color: [f32; 4]);

    /// Add a filled debug rectangle
    fn add_filled_debug_rect(&mut self, rect: Rect, color: [f32; 4]);

    /// Finalize debug shapes for rendering
    fn finalize_debug_shapes(&mut self);

    /// Clear all UI shapes
    fn clear_ui_shapes(&mut self);

    /// Add a UI shape outline
    fn add_ui_shape(&mut self, rect: Rect, color: [f32; 4]);

    /// Add a filled UI shape
    fn add_filled_ui_shape(&mut self, rect: Rect, color: [f32; 4]);

    /// Finalize UI shapes for rendering
    fn finalize_ui_shapes(&mut self);
}
