use std::collections::BTreeMap;
use std::sync::Arc;
use toki_core::cache_utils::clone_cached_or_load;
use toki_core::fonts::find_font_files;
use toki_core::graphics::image::DecodedImage;
use toki_core::graphics::vertex::QuadVertex;
use toki_core::math::projection::ProjectionParameter;
use toki_core::palette::recolor_indexed_image;
use toki_core::project_runtime::{ResolvedPostProcessSettings, RuntimeViewportMode};
use toki_core::sprite::SpriteFrame;
use toki_core::sprite_render::{ResolvedSpriteRenderInstance, SpriteRenderMaterial};
use toki_core::text::TextItem;
use toki_core::ui::UiComposition;
use toki_render::{
    FrameLifecycle, GpuState, RenderBackend, SceneClipRect, ShapeRenderer, SpriteRenderer,
    TextRenderer, TextureLoader,
};
use winit::window::Window;

use crate::viewport::presentation::ViewportPresentation;
use crate::viewport::runtime_state::{
    resolve_effective_runtime_viewport, EffectiveRuntimeViewport,
};

/// Rendering system that manages GPU state and projection calculations.
///
/// Centralizes all rendering-related state and provides clean APIs for
/// graphics operations while abstracting GPU implementation details.
#[derive(Debug)]
pub struct RenderingSystem {
    backend: Option<Box<dyn RenderBackend>>,
    projection_params: ProjectionParameter,
    viewport_mode: RuntimeViewportMode,
    loaded_tilemap_texture_path: Option<std::path::PathBuf>,
    loaded_sprite_texture_path: Option<std::path::PathBuf>,
    decoded_sprite_images: BTreeMap<std::path::PathBuf, DecodedImage>,
    recolored_sprite_images: BTreeMap<std::path::PathBuf, DecodedImage>,
}

impl Default for RenderingSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderingSystem {
    fn backend_mut(
        &mut self,
    ) -> Result<&mut (dyn RenderBackend + 'static), toki_render::RenderError> {
        self.backend
            .as_deref_mut()
            .ok_or_else(|| toki_render::RenderError::Other("GPU not initialized".to_string()))
    }

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
            viewport_mode: toki_core::project_runtime::default_runtime_viewport_mode(),
            loaded_tilemap_texture_path: None,
            loaded_sprite_texture_path: None,
            decoded_sprite_images: BTreeMap::new(),
            recolored_sprite_images: BTreeMap::new(),
        }
    }

    /// Create a new RenderingSystem with custom projection parameters (for editor)
    pub fn new_with_projection(projection_params: ProjectionParameter) -> Self {
        Self {
            backend: None,
            projection_params,
            viewport_mode: toki_core::project_runtime::default_runtime_viewport_mode(),
            loaded_tilemap_texture_path: None,
            loaded_sprite_texture_path: None,
            decoded_sprite_images: BTreeMap::new(),
            recolored_sprite_images: BTreeMap::new(),
        }
    }

    /// Create a new RenderingSystem with the specified desired resolution.
    /// The actual window size will be updated later via `update_window_size`.
    pub fn new_with_desired_resolution(desired_width: u32, desired_height: u32) -> Self {
        Self {
            backend: None,
            projection_params: ProjectionParameter {
                width: desired_width,
                height: desired_height,
                desired_width,
                desired_height,
            },
            viewport_mode: toki_core::project_runtime::default_runtime_viewport_mode(),
            loaded_tilemap_texture_path: None,
            loaded_sprite_texture_path: None,
            decoded_sprite_images: BTreeMap::new(),
            recolored_sprite_images: BTreeMap::new(),
        }
    }

    /// Set new projection parameters at runtime
    pub fn set_projection_params(&mut self, params: ProjectionParameter) {
        self.projection_params = params;
    }

    pub fn set_viewport_mode(&mut self, mode: RuntimeViewportMode) {
        self.viewport_mode = mode;
        self.update_scene_clip_rect();
    }

    /// Update desired resolution (useful for editor viewport scaling)
    pub fn set_desired_resolution(&mut self, width: u32, height: u32) {
        self.projection_params.desired_width = width;
        self.projection_params.desired_height = height;
    }

    /// Initialize GPU state with the given window (uses default textures)
    pub fn initialize_gpu(
        &mut self,
        window: Arc<Window>,
        vsync: bool,
    ) -> Result<(), toki_render::RenderError> {
        let gpu = GpuState::new(window, vsync)?;
        self.backend = Some(Box::new(gpu));
        Ok(())
    }

    #[cfg(test)]
    fn set_backend_for_tests(&mut self, backend: Box<dyn RenderBackend>) {
        self.backend = Some(backend);
    }

    /// Initialize GPU state with custom textures (for editor use)
    pub fn initialize_gpu_with_textures(
        &mut self,
        window: Arc<Window>,
        vsync: bool,
        tilemap_texture: Option<std::path::PathBuf>,
        sprite_texture: Option<std::path::PathBuf>,
    ) -> Result<(), toki_render::RenderError> {
        let gpu = GpuState::new_with_textures(window, vsync, tilemap_texture, sprite_texture)?;
        self.loaded_tilemap_texture_path = None;
        self.loaded_sprite_texture_path = None;
        self.backend = Some(Box::new(gpu));
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
        TextureLoader::load_tilemap_texture(self.backend_mut()?, texture_path.clone())?;
        self.loaded_tilemap_texture_path = Some(texture_path);
        Ok(())
    }

    pub fn load_tilemap_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), toki_render::RenderError> {
        TextureLoader::load_tilemap_texture_rgba8(self.backend_mut()?, image)?;
        self.loaded_tilemap_texture_path = None;
        Ok(())
    }

    /// Load new sprite texture at runtime
    pub fn load_sprite_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        if self.loaded_sprite_texture_path.as_ref() == Some(&texture_path) {
            return Ok(());
        }
        TextureLoader::load_sprite_texture(self.backend_mut()?, texture_path.clone())?;
        self.loaded_sprite_texture_path = Some(texture_path);
        Ok(())
    }

    pub fn load_sprite_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), toki_render::RenderError> {
        TextureLoader::load_sprite_texture_rgba8(self.backend_mut()?, image)?;
        self.loaded_sprite_texture_path = None;
        Ok(())
    }

    /// Load font from a specific file path.
    pub fn load_font_file(
        &mut self,
        font_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        TextureLoader::load_font_file(self.backend_mut()?, font_path)
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
        self.update_scene_clip_rect();
    }

    /// Calculate current projection matrix
    pub fn calculate_projection(&self) -> glam::Mat4 {
        self.effective_runtime_viewport().presentation.projection
    }

    pub fn viewport_presentation(&self) -> ViewportPresentation {
        self.effective_runtime_viewport().presentation
    }

    pub fn effective_runtime_viewport(&self) -> EffectiveRuntimeViewport {
        resolve_effective_runtime_viewport(
            glam::UVec2::new(
                self.projection_params.width.max(1),
                self.projection_params.height.max(1),
            ),
            glam::UVec2::new(
                self.projection_params.desired_width.max(1),
                self.projection_params.desired_height.max(1),
            ),
            self.viewport_mode,
        )
    }

    pub fn viewport_logical_size(&self) -> glam::Vec2 {
        self.viewport_presentation().logical_viewport_size()
    }

    pub fn viewport_surface_size(&self) -> glam::Vec2 {
        self.viewport_presentation().surface_viewport_size()
    }

    pub fn logical_to_surface_position(&self, position: glam::Vec2) -> glam::Vec2 {
        self.viewport_presentation().logical_to_surface_position(position)
    }

    pub fn surface_to_viewport_position(&self, position: glam::Vec2) -> Option<glam::Vec2> {
        self.viewport_presentation()
            .surface_to_viewport_local_position(position)
    }

    /// Update GPU projection matrix with view transform
    pub fn update_projection(&mut self, view_matrix: glam::Mat4) {
        let projection = self.calculate_projection();
        let clip_rect = self.scene_clip_rect();
        if let Some(backend) = &mut self.backend {
            FrameLifecycle::set_scene_clip_rect(backend.as_mut(), clip_rect);
            FrameLifecycle::update_projection(backend.as_mut(), projection * view_matrix);
        }
    }

    fn scene_clip_rect(&self) -> Option<SceneClipRect> {
        let rect = self.effective_runtime_viewport().presentation.layout.viewport_rect;
        Some(SceneClipRect {
            x: rect.x.round().max(0.0) as u32,
            y: rect.y.round().max(0.0) as u32,
            width: rect.width.round().max(1.0) as u32,
            height: rect.height.round().max(1.0) as u32,
        })
    }

    fn update_scene_clip_rect(&mut self) {
        let clip_rect = self.scene_clip_rect();
        if let Some(backend) = &mut self.backend {
            FrameLifecycle::set_scene_clip_rect(backend.as_mut(), clip_rect);
        }
    }

    pub fn set_post_process_settings(&mut self, settings: ResolvedPostProcessSettings) {
        if let Some(backend) = &mut self.backend {
            FrameLifecycle::set_post_process_settings(backend.as_mut(), settings);
        }
    }

    pub fn set_vsync(&mut self, enabled: bool) {
        if let Some(backend) = &mut self.backend {
            FrameLifecycle::set_vsync(backend.as_mut(), enabled);
        }
    }

    /// Resize GPU render targets
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if let Some(backend) = &mut self.backend {
            FrameLifecycle::resize(backend.as_mut(), new_size);
        }
        self.update_window_size(new_size);
    }

    /// Draw the current frame
    pub fn draw(&mut self) {
        if let Some(backend) = &mut self.backend {
            FrameLifecycle::draw(backend.as_mut());
        }
    }

    /// Check if GPU is initialized
    pub fn has_gpu(&self) -> bool {
        self.backend.is_some()
    }

    pub fn set_tilemap_render_enabled(&mut self, enabled: bool) {
        if let Some(backend) = &mut self.backend {
            FrameLifecycle::set_tilemap_render_enabled(backend.as_mut(), enabled);
        }
    }

    /// Get current projection parameters
    pub fn projection_params(&self) -> ProjectionParameter {
        self.projection_params
    }

    pub fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]) {
        if let Some(backend) = &mut self.backend {
            FrameLifecycle::update_tilemap_vertices(backend.as_mut(), vertices);
        }
    }

    pub fn clear_sprites(&mut self) {
        if let Some(backend) = &mut self.backend {
            SpriteRenderer::clear_sprites(backend.as_mut());
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
            SpriteRenderer::add_sprite(backend.as_mut(), frame, position, size, flip_x);
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
            SpriteRenderer::add_sprite_with_texture(
                backend.as_mut(),
                texture_path,
                frame,
                position,
                size,
                flip_x,
            );
        }
    }

    pub fn add_sprite_with_texture_rgba8(
        &mut self,
        texture_key: std::path::PathBuf,
        image: &DecodedImage,
        frame: SpriteFrame,
        position: glam::IVec2,
        size: glam::UVec2,
        flip_x: bool,
    ) {
        if let Some(backend) = &mut self.backend {
            SpriteRenderer::add_sprite_with_texture_rgba8(
                backend.as_mut(),
                texture_key,
                image,
                frame,
                position,
                size,
                flip_x,
            );
        }
    }

    pub fn add_resolved_sprite(&mut self, sprite: &ResolvedSpriteRenderInstance) {
        match (&sprite.texture_path, &sprite.material) {
            (Some(texture_path), SpriteRenderMaterial::TrueColor) => {
                self.add_sprite_with_texture(
                    texture_path.clone(),
                    sprite.frame,
                    sprite.position,
                    sprite.size,
                    sprite.flip_x,
                );
            }
            (
                Some(texture_path),
                SpriteRenderMaterial::PaletteIndexed {
                    palette_id,
                    palette,
                },
            ) => {
                let texture_key = palette_texture_key(texture_path.as_path(), palette_id.as_str());
                let image = match clone_cached_or_load(
                    self.recolored_sprite_images.get(&texture_key).cloned(),
                    || {
                        let decoded = clone_cached_or_load(
                            self.decoded_sprite_images.get(texture_path).cloned(),
                            || toki_core::graphics::image::load_image_rgba8(texture_path),
                            |image| {
                                self.decoded_sprite_images
                                    .insert(texture_path.clone(), image);
                            },
                        )
                        .map_err(|error| {
                            tracing::warn!(
                                "Failed to decode indexed sprite texture '{}': {}",
                                texture_path.display(),
                                error
                            );
                        })?;
                        recolor_indexed_image(&decoded, *palette).map_err(|error| {
                            tracing::warn!(
                                "Indexed sprite texture '{}' failed validation for palette '{}': {}",
                                texture_path.display(),
                                palette_id,
                                error
                            );
                        })
                    },
                    |image| {
                        self.recolored_sprite_images
                            .insert(texture_key.clone(), image);
                    },
                ) {
                    Ok(image) => image,
                    Err(()) => return,
                };

                self.add_sprite_with_texture_rgba8(
                    texture_key,
                    &image,
                    sprite.frame,
                    sprite.position,
                    sprite.size,
                    sprite.flip_x,
                );
            }
            (None, _) => {
                self.add_sprite(sprite.frame, sprite.position, sprite.size, sprite.flip_x);
            }
        }
    }

    pub fn clear_text_items(&mut self) {
        if let Some(backend) = &mut self.backend {
            TextRenderer::clear_text_items(backend.as_mut());
        }
    }

    pub fn add_text_item(&mut self, text: TextItem) {
        if let Some(backend) = &mut self.backend {
            TextRenderer::add_text_item(backend.as_mut(), text);
        }
    }

    pub fn add_viewport_text_item(&mut self, text: TextItem) {
        let transformed = self.viewport_presentation().offset_surface_text_item(&text);
        self.add_text_item(transformed);
    }

    pub fn clear_world_underlay_shapes(&mut self) {
        if let Some(backend) = &mut self.backend {
            ShapeRenderer::clear_world_underlay_shapes(backend.as_mut());
        }
    }

    pub fn add_world_underlay_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    ) {
        if let Some(backend) = &mut self.backend {
            ShapeRenderer::add_world_underlay_rect(backend.as_mut(), x, y, width, height, color);
        }
    }

    pub fn add_filled_world_underlay_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    ) {
        if let Some(backend) = &mut self.backend {
            ShapeRenderer::add_filled_world_underlay_rect(
                backend.as_mut(),
                x,
                y,
                width,
                height,
                color,
            );
        }
    }

    pub fn finalize_world_underlay_shapes(&mut self) {
        if let Some(backend) = &mut self.backend {
            ShapeRenderer::finalize_world_underlay_shapes(backend.as_mut());
        }
    }

    pub fn render_ui_composition(&mut self, composition: &UiComposition) {
        for block in &composition.blocks {
            if let Some(fill) = block.fill_color {
                self.add_filled_ui_rect(
                    block.rect.x,
                    block.rect.y,
                    block.rect.width,
                    block.rect.height,
                    fill,
                );
            }
            if let Some(border) = block.border_color {
                let passes = block.border_thickness.max(1.0).round() as u32;
                for pass in 0..passes {
                    let rect = block.rect.inset(pass as f32);
                    if rect.width <= 0.0 || rect.height <= 0.0 {
                        break;
                    }
                    self.add_ui_rect(rect.x, rect.y, rect.width, rect.height, border);
                }
            }
            if let Some(text) = &block.text {
                self.add_text_item(text.to_text_item());
            }
        }
    }

    pub fn render_viewport_ui_composition(&mut self, composition: &UiComposition) {
        let transformed = self
            .viewport_presentation()
            .offset_surface_ui_composition(composition);
        self.render_ui_composition(&transformed);
    }

    pub fn clear_debug_shapes(&mut self) {
        if let Some(backend) = &mut self.backend {
            ShapeRenderer::clear_debug_shapes(backend.as_mut());
        }
    }

    pub fn add_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        if let Some(backend) = &mut self.backend {
            ShapeRenderer::add_debug_rect(backend.as_mut(), x, y, width, height, color);
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
            ShapeRenderer::add_filled_debug_rect(backend.as_mut(), x, y, width, height, color);
        }
    }

    pub fn finalize_debug_shapes(&mut self) {
        if let Some(backend) = &mut self.backend {
            ShapeRenderer::finalize_debug_shapes(backend.as_mut());
        }
    }

    pub fn clear_ui_shapes(&mut self) {
        if let Some(backend) = &mut self.backend {
            ShapeRenderer::clear_ui_shapes(backend.as_mut());
        }
    }

    pub fn add_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        if let Some(backend) = &mut self.backend {
            ShapeRenderer::add_ui_rect(backend.as_mut(), x, y, width, height, color);
        }
    }

    pub fn add_filled_ui_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        if let Some(backend) = &mut self.backend {
            ShapeRenderer::add_filled_ui_rect(backend.as_mut(), x, y, width, height, color);
        }
    }

    pub fn finalize_ui_shapes(&mut self) {
        if let Some(backend) = &mut self.backend {
            ShapeRenderer::finalize_ui_shapes(backend.as_mut());
        }
    }
}

fn palette_texture_key(texture_path: &std::path::Path, palette_id: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(format!(
        "__palette__/{}::{}",
        palette_id,
        texture_path.display()
    ))
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
