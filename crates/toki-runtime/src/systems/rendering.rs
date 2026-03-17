use std::sync::Arc;
use toki_core::fonts::find_font_files;
use toki_core::graphics::image::DecodedImage;
use toki_core::graphics::vertex::QuadVertex;
use toki_core::math::projection::{calculate_projection, ProjectionParameter};
use toki_core::sprite::SpriteFrame;
use toki_core::text::TextItem;
use toki_render::GpuState;
use winit::window::Window;

trait RuntimeRenderBackend: std::fmt::Debug {
    fn load_tilemap_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError>;
    fn load_sprite_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError>;
    fn load_sprite_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), toki_render::RenderError>;
    fn load_font_file(
        &mut self,
        font_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError>;
    fn update_projection(&mut self, mvp: glam::Mat4);
    fn set_tilemap_render_enabled(&mut self, enabled: bool);
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>);
    fn draw(&mut self);
    fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]);
    fn clear_sprites(&mut self);
    fn add_sprite(
        &mut self,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    );
    fn add_sprite_with_texture(
        &mut self,
        texture_path: std::path::PathBuf,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    );
    fn clear_text_items(&mut self);
    fn add_text_item(&mut self, text: TextItem);
    fn clear_debug_shapes(&mut self);
    fn add_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]);
    fn add_filled_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]);
    fn finalize_debug_shapes(&mut self);
    fn clear_ui_shapes(&mut self);
    fn add_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]);
    fn add_filled_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]);
    fn finalize_ui_shapes(&mut self);
}

#[derive(Debug)]
struct WgpuRenderBackend {
    gpu: GpuState,
}

impl WgpuRenderBackend {
    fn new(window: Arc<Window>) -> Self {
        Self {
            gpu: GpuState::new(window),
        }
    }

    fn new_with_textures(
        window: Arc<Window>,
        tilemap_texture: Option<std::path::PathBuf>,
        sprite_texture: Option<std::path::PathBuf>,
    ) -> Result<Self, toki_render::RenderError> {
        Ok(Self {
            gpu: GpuState::new_with_textures(window, tilemap_texture, sprite_texture)?,
        })
    }
}

impl RuntimeRenderBackend for WgpuRenderBackend {
    fn load_tilemap_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        self.gpu.load_tilemap_texture(texture_path)
    }

    fn load_sprite_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        self.gpu.load_sprite_texture(texture_path)
    }

    fn load_sprite_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), toki_render::RenderError> {
        self.gpu.load_sprite_texture_rgba8(image)
    }

    fn load_font_file(
        &mut self,
        font_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        self.gpu.load_font_file(&font_path)
    }

    fn update_projection(&mut self, mvp: glam::Mat4) {
        self.gpu.update_projection(mvp);
    }

    fn set_tilemap_render_enabled(&mut self, enabled: bool) {
        self.gpu.set_tilemap_render_enabled(enabled);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.gpu.resize(new_size);
    }

    fn draw(&mut self) {
        self.gpu.draw();
    }

    fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]) {
        self.gpu.update_tilemap_vertices(vertices);
    }

    fn clear_sprites(&mut self) {
        self.gpu.clear_sprites();
    }

    fn add_sprite(
        &mut self,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        self.gpu.add_sprite_flipped(frame, position, size, flip_x);
    }

    fn add_sprite_with_texture(
        &mut self,
        texture_path: std::path::PathBuf,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        self.gpu
            .add_sprite_with_texture_flipped(texture_path, frame, position, size, flip_x);
    }

    fn clear_text_items(&mut self) {
        self.gpu.clear_text_items();
    }

    fn add_text_item(&mut self, text: TextItem) {
        self.gpu.add_text_item(text);
    }

    fn clear_debug_shapes(&mut self) {
        self.gpu.clear_debug_shapes();
    }

    fn add_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.gpu.add_debug_rect(x, y, width, height, color);
    }

    fn add_filled_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.gpu.add_filled_debug_rect(x, y, width, height, color);
    }

    fn finalize_debug_shapes(&mut self) {
        self.gpu.finalize_debug_shapes();
    }

    fn clear_ui_shapes(&mut self) {
        self.gpu.clear_ui_rects();
    }

    fn add_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.gpu.add_ui_rect(x, y, width, height, color);
    }

    fn add_filled_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.gpu.add_filled_ui_rect(x, y, width, height, color);
    }

    fn finalize_ui_shapes(&mut self) {
        self.gpu.finalize_ui_rects();
    }
}

/// Rendering system that manages GPU state and projection calculations.
///
/// Centralizes all rendering-related state and provides clean APIs for
/// graphics operations while abstracting GPU implementation details.
#[derive(Debug)]
pub struct RenderingSystem {
    backend: Option<Box<dyn RuntimeRenderBackend>>,
    projection_params: ProjectionParameter,
    loaded_tilemap_texture_path: Option<std::path::PathBuf>,
    loaded_sprite_texture_path: Option<std::path::PathBuf>,
}

impl Default for RenderingSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderingSystem {
    /// Create a new RenderingSystem with default projection parameters
    pub fn new() -> Self {
        Self {
            backend: None,
            projection_params: ProjectionParameter {
                width: 160,
                height: 144,
                desired_width: 160,
                desired_height: 144,
            },
            loaded_tilemap_texture_path: None,
            loaded_sprite_texture_path: None,
        }
    }

    /// Create a new RenderingSystem with custom projection parameters (for editor)
    pub fn new_with_projection(projection_params: ProjectionParameter) -> Self {
        Self {
            backend: None,
            projection_params,
            loaded_tilemap_texture_path: None,
            loaded_sprite_texture_path: None,
        }
    }

    /// Set new projection parameters at runtime
    pub fn set_projection_params(&mut self, params: ProjectionParameter) {
        self.projection_params = params;
    }

    /// Update desired resolution (useful for editor viewport scaling)
    pub fn set_desired_resolution(&mut self, width: u32, height: u32) {
        self.projection_params.desired_width = width;
        self.projection_params.desired_height = height;
    }

    /// Initialize GPU state with the given window (uses default textures)
    pub fn initialize_gpu(&mut self, window: Arc<Window>) {
        let backend = WgpuRenderBackend::new(window);
        self.backend = Some(Box::new(backend));
    }

    /// Initialize GPU state with custom textures (for editor use)
    pub fn initialize_gpu_with_textures(
        &mut self,
        window: Arc<Window>,
        tilemap_texture: Option<std::path::PathBuf>,
        sprite_texture: Option<std::path::PathBuf>,
    ) -> Result<(), toki_render::RenderError> {
        let backend =
            WgpuRenderBackend::new_with_textures(window, tilemap_texture, sprite_texture)?;
        self.loaded_tilemap_texture_path = None;
        self.loaded_sprite_texture_path = None;
        self.backend = Some(Box::new(backend));
        Ok(())
    }

    /// Load new tilemap texture at runtime
    pub fn load_tilemap_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        if self.loaded_tilemap_texture_path.as_ref() == Some(&texture_path) {
            return Ok(());
        }
        if let Some(backend) = &mut self.backend {
            backend.load_tilemap_texture(texture_path.clone())?;
            self.loaded_tilemap_texture_path = Some(texture_path);
            Ok(())
        } else {
            Err(toki_render::RenderError::Other(
                "GPU not initialized".to_string(),
            ))
        }
    }

    /// Load new sprite texture at runtime
    pub fn load_sprite_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        if self.loaded_sprite_texture_path.as_ref() == Some(&texture_path) {
            return Ok(());
        }
        if let Some(backend) = &mut self.backend {
            backend.load_sprite_texture(texture_path.clone())?;
            self.loaded_sprite_texture_path = Some(texture_path);
            Ok(())
        } else {
            Err(toki_render::RenderError::Other(
                "GPU not initialized".to_string(),
            ))
        }
    }

    pub fn load_sprite_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), toki_render::RenderError> {
        if let Some(backend) = &mut self.backend {
            backend.load_sprite_texture_rgba8(image)?;
            self.loaded_sprite_texture_path = None;
            Ok(())
        } else {
            Err(toki_render::RenderError::Other(
                "GPU not initialized".to_string(),
            ))
        }
    }

    /// Load font from a specific file path.
    pub fn load_font_file(
        &mut self,
        font_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        if let Some(backend) = &mut self.backend {
            backend.load_font_file(font_path)
        } else {
            Err(toki_render::RenderError::Other(
                "GPU not initialized".to_string(),
            ))
        }
    }

    /// Helper to load textures from a project assets directory
    pub fn load_project_textures(
        &mut self,
        project_path: &std::path::Path,
    ) -> Result<(), toki_render::RenderError> {
        let assets_path = project_path.join("assets");
        let sprites_path = assets_path.join("sprites");
        let tilemaps_path = assets_path.join("tilemaps");

        // Look for common sprite atlas files (only .json format supported)
        if let Some(creatures_atlas) = find_atlas_file(&sprites_path, "creatures") {
            if let Some(creatures_image) = find_image_for_atlas(&creatures_atlas) {
                self.load_sprite_texture(creatures_image)?;
            }
        }

        // Look for common tilemap atlas files (only .json format supported)
        if let Some(terrain_atlas) = find_atlas_file(&tilemaps_path, "terrain")
            .or_else(|| find_atlas_file(&sprites_path, "terrain"))
        {
            if let Some(terrain_image) = find_image_for_atlas(&terrain_atlas) {
                self.load_tilemap_texture(terrain_image)?;
            }
        }

        let fonts_path = assets_path.join("fonts");
        for font_file in find_font_files(&fonts_path) {
            self.load_font_file(font_file)?;
        }

        Ok(())
    }

    /// Update projection parameters with new window size
    pub fn update_window_size(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.projection_params.width = size.width;
        self.projection_params.height = size.height;
    }

    /// Calculate current projection matrix
    pub fn calculate_projection(&self) -> glam::Mat4 {
        calculate_projection(self.projection_params)
    }

    /// Update GPU projection matrix with view transform
    pub fn update_projection(&mut self, view_matrix: glam::Mat4) {
        let projection = self.calculate_projection();
        if let Some(backend) = &mut self.backend {
            backend.update_projection(projection * view_matrix);
        }
    }

    /// Resize GPU render targets
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if let Some(backend) = &mut self.backend {
            backend.resize(new_size);
        }
        self.update_window_size(new_size);
    }

    /// Draw the current frame
    pub fn draw(&mut self) {
        if let Some(backend) = &mut self.backend {
            backend.draw();
        }
    }

    /// Check if GPU is initialized
    pub fn has_gpu(&self) -> bool {
        self.backend.is_some()
    }

    pub fn set_tilemap_render_enabled(&mut self, enabled: bool) {
        if let Some(backend) = &mut self.backend {
            backend.set_tilemap_render_enabled(enabled);
        }
    }

    /// Get current projection parameters
    pub fn projection_params(&self) -> ProjectionParameter {
        self.projection_params
    }

    pub fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]) {
        if let Some(backend) = &mut self.backend {
            backend.update_tilemap_vertices(vertices);
        }
    }

    pub fn clear_sprites(&mut self) {
        if let Some(backend) = &mut self.backend {
            backend.clear_sprites();
        }
    }

    pub fn add_sprite(
        &mut self,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        if let Some(backend) = &mut self.backend {
            backend.add_sprite(frame, position, size, flip_x);
        }
    }

    pub fn add_sprite_with_texture(
        &mut self,
        texture_path: std::path::PathBuf,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        if let Some(backend) = &mut self.backend {
            backend.add_sprite_with_texture(texture_path, frame, position, size, flip_x);
        }
    }

    pub fn clear_text_items(&mut self) {
        if let Some(backend) = &mut self.backend {
            backend.clear_text_items();
        }
    }

    pub fn add_text_item(&mut self, text: TextItem) {
        if let Some(backend) = &mut self.backend {
            backend.add_text_item(text);
        }
    }

    pub fn clear_debug_shapes(&mut self) {
        if let Some(backend) = &mut self.backend {
            backend.clear_debug_shapes();
        }
    }

    pub fn add_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        if let Some(backend) = &mut self.backend {
            backend.add_debug_rect(x, y, width, height, color);
        }
    }

    pub fn add_filled_debug_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    ) {
        if let Some(backend) = &mut self.backend {
            backend.add_filled_debug_rect(x, y, width, height, color);
        }
    }

    pub fn finalize_debug_shapes(&mut self) {
        if let Some(backend) = &mut self.backend {
            backend.finalize_debug_shapes();
        }
    }

    pub fn clear_ui_shapes(&mut self) {
        if let Some(backend) = &mut self.backend {
            backend.clear_ui_shapes();
        }
    }

    pub fn add_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        if let Some(backend) = &mut self.backend {
            backend.add_ui_rect(x, y, width, height, color);
        }
    }

    pub fn add_filled_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        if let Some(backend) = &mut self.backend {
            backend.add_filled_ui_rect(x, y, width, height, color);
        }
    }

    pub fn finalize_ui_shapes(&mut self) {
        if let Some(backend) = &mut self.backend {
            backend.finalize_ui_shapes();
        }
    }
}

/// Helper function to find atlas files by name in a directory (only .json supported)
fn find_atlas_file(dir: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    if !dir.exists() {
        return None;
    }

    // Look for .json atlas files (only supported format)
    let json_path = dir.join(format!("{}.json", name));
    if json_path.exists() {
        return Some(json_path);
    }

    None
}

/// Helper function to find the image file corresponding to a .json atlas
fn find_image_for_atlas(atlas_path: &std::path::Path) -> Option<std::path::PathBuf> {
    if let Some(dir) = atlas_path.parent() {
        if let Some(stem) = atlas_path.file_stem() {
            if let Some(name) = stem.to_str() {
                // Common image formats
                for ext in &["png", "jpg", "jpeg"] {
                    let image_path = dir.join(format!("{}.{}", name, ext));
                    if image_path.exists() {
                        return Some(image_path);
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "rendering_tests.rs"]
mod tests;
