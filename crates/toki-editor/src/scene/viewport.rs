use crate::editor_types::PlacementPreviewVisual;
use crate::project::assets::{ObjectSheetAsset, SpriteAtlasAsset};
use crate::project::ProjectAssets;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;
use toki_core::assets::tile_animation::TileAnimationClock;
use toki_core::assets::tilemap::TileMap;
use toki_core::assets::{
    atlas::AtlasMeta,
    object_sheet::ObjectSheetMeta,
    tileset::{TileSetAtlasSource, TileSetMeta, TileSetResolver},
};
use toki_core::graphics::image::DecodedImage;
use toki_core::indexed_presentation::IndexedPresentationSettings;
use toki_core::palette::{builtin_palettes, Palette};
use toki_core::project_runtime::{default_resolution_height, default_resolution_width};
use toki_core::{Camera, GameState, ResourceManager};
use toki_render::{SceneData, SceneRenderer, SceneTilemapBatch};

#[path = "viewport_assets.rs"]
mod viewport_assets;
#[path = "viewport_input.rs"]
mod viewport_input;
#[path = "viewport_math.rs"]
mod viewport_math;
#[path = "viewport_prepare.rs"]
mod viewport_prepare;
#[path = "viewport_ui.rs"]
mod viewport_ui;

use viewport_math::{
    next_zoom_in_scale, next_zoom_out_scale, point_in_entity_bounds, request_viewport_size_state,
    screen_to_world_from_camera, world_to_i32_floor,
};

#[derive(Debug, Clone, Copy)]
pub struct DragPreviewSprite {
    pub entity_id: toki_core::entity::EntityId,
    pub world_position: glam::IVec2,
    pub is_valid: bool,
}

#[derive(Debug, Clone)]
pub struct OverlaySpriteInstance {
    pub world_position: glam::IVec2,
    pub visual: PlacementPreviewVisual,
}

#[derive(Debug, Clone, Copy)]
pub struct OverlayRectInstance {
    pub position: glam::Vec2,
    pub size: glam::Vec2,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Copy)]
pub struct OverlayLineInstance {
    pub start: glam::Vec2,
    pub end: glam::Vec2,
    pub thickness: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Default, Clone)]
pub struct ViewportOverlayData {
    pub placement_preview: Option<(glam::Vec2, PlacementPreviewVisual, bool)>,
    pub drag_preview_sprites: Vec<DragPreviewSprite>,
    pub overlay_sprites: Vec<OverlaySpriteInstance>,
    pub overlay_rects: Vec<OverlayRectInstance>,
    pub overlay_lines: Vec<OverlayLineInstance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportSizingMode {
    Fixed,
    Responsive,
}

/// Handles the scene viewport - integration between scene data and rendering
pub struct SceneViewport {
    // Inlined from SceneManager (removed middle-man)
    game_state: GameState,
    #[allow(dead_code)]
    resources: ResourceManager,
    tilemap: Option<TileMap>,
    tilemap_path: Option<std::path::PathBuf>,
    // Rendering infrastructure
    scene_renderer: Option<SceneRenderer>,
    presentation_target: Option<crate::rendering::PresentedOffscreenTexture>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    is_initialized: bool,
    sizing_mode: ViewportSizingMode,
    viewport_size: (u32, u32),
    requested_viewport_size: Option<(u32, u32)>,
    scene_clear_color: wgpu::Color,
    ui_background_fill: Option<egui::Color32>,
    tileset_cache: Option<TileSetMeta>,
    tileset_cache_path: Option<std::path::PathBuf>,
    tileset_atlas_cache: std::collections::HashMap<String, TileSetAtlasSource>,
    tile_animation_clock: TileAnimationClock,
    needs_render: bool, // Track if scene needs re-rendering
    tilemap_render_cache_dirty: bool,
    cached_tilemap_batches: Vec<SceneTilemapBatch>,
    tilemap_revision: u64,
    camera: Camera, // Camera for zoom and pan
    editor_zoom_scale: f32,
    // Mouse interaction state
    last_mouse_pos: Option<glam::Vec2>, // For camera panning
    is_dragging_camera: bool,
    // Hide entities while they are being interactively dragged in editor UI.
    suppressed_entity_ids: std::collections::HashSet<toki_core::entity::EntityId>,
    // Sprite atlas caching to prevent redundant loads
    loaded_sprite_atlases: std::collections::HashMap<String, toki_core::assets::atlas::AtlasMeta>,
    loaded_object_sheets:
        std::collections::HashMap<String, toki_core::assets::object_sheet::ObjectSheetMeta>,
    decoded_sprite_images: std::collections::HashMap<std::path::PathBuf, DecodedImage>,
    recolored_sprite_images: std::collections::HashMap<String, DecodedImage>,
    available_palettes: BTreeMap<String, Palette>,
    indexed_presentation_settings: IndexedPresentationSettings,
    tilemap_texture_cache_key: Option<String>,
}

impl SceneViewport {
    /// Create viewport with existing game state using default resolution
    pub fn with_game_state(game_state: GameState) -> Result<Self> {
        Self::with_game_state_and_resolution(
            game_state,
            default_resolution_width(),
            default_resolution_height(),
        )
    }

    /// Create viewport with existing game state and custom resolution
    pub fn with_game_state_and_resolution(
        game_state: GameState,
        resolution_width: u32,
        resolution_height: u32,
    ) -> Result<Self> {
        Self::with_options(
            game_state,
            ViewportSizingMode::Fixed,
            resolution_width,
            resolution_height,
        )
    }

    /// Create responsive viewport with existing game state using default resolution
    pub fn with_game_state_responsive(game_state: GameState) -> Result<Self> {
        Self::with_options(
            game_state,
            ViewportSizingMode::Responsive,
            default_resolution_width(),
            default_resolution_height(),
        )
    }

    fn with_options(
        game_state: GameState,
        sizing_mode: ViewportSizingMode,
        resolution_width: u32,
        resolution_height: u32,
    ) -> Result<Self> {
        let resources = ResourceManager::load_all()
            .map_err(|e| anyhow::anyhow!("Failed to load resources: {e}"))?;
        Self::with_resources_and_options(
            game_state,
            resources,
            sizing_mode,
            resolution_width,
            resolution_height,
        )
    }

    fn with_resources_and_options(
        game_state: GameState,
        resources: ResourceManager,
        sizing_mode: ViewportSizingMode,
        resolution_width: u32,
        resolution_height: u32,
    ) -> Result<Self> {
        let mut camera = Camera::with_resolution(resolution_width, resolution_height);
        camera.zoom = 1.0;
        let center_x = (resolution_width / 2) as i32;
        let center_y = (resolution_height / 2) as i32;
        camera.center_on(glam::IVec2::new(center_x, center_y));

        tracing::info!(
            "Scene viewport created with resolution {}x{}",
            resolution_width,
            resolution_height
        );

        Ok(Self {
            game_state,
            resources,
            tilemap: None,
            tilemap_path: None,
            scene_renderer: None,
            presentation_target: None,
            device: None,
            queue: None,
            is_initialized: false,
            sizing_mode,
            viewport_size: (resolution_width, resolution_height),
            requested_viewport_size: None,
            scene_clear_color: wgpu::Color {
                r: 0.1,
                g: 0.1,
                b: 0.12,
                a: 1.0,
            },
            ui_background_fill: Some(egui::Color32::from_rgb(20, 20, 25)),
            tileset_cache: None,
            tileset_cache_path: None,
            tileset_atlas_cache: std::collections::HashMap::new(),
            tile_animation_clock: TileAnimationClock::new(),
            needs_render: true,
            tilemap_render_cache_dirty: true,
            cached_tilemap_batches: Vec::new(),
            tilemap_revision: 0,
            camera,
            editor_zoom_scale: 1.0,
            last_mouse_pos: None,
            is_dragging_camera: false,
            suppressed_entity_ids: std::collections::HashSet::new(),
            loaded_sprite_atlases: std::collections::HashMap::new(),
            loaded_object_sheets: std::collections::HashMap::new(),
            decoded_sprite_images: std::collections::HashMap::new(),
            recolored_sprite_images: std::collections::HashMap::new(),
            available_palettes: builtin_palettes(),
            indexed_presentation_settings: IndexedPresentationSettings::default(),
            tilemap_texture_cache_key: None,
        })
    }

    #[cfg(test)]
    pub(crate) fn with_game_state_and_resources_for_tests(
        game_state: GameState,
        resources: ResourceManager,
    ) -> Result<Self> {
        Self::with_resources_and_options(
            game_state,
            resources,
            ViewportSizingMode::Fixed,
            default_resolution_width(),
            default_resolution_height(),
        )
    }

    /// Initialize the viewport with WGPU context
    pub async fn initialize(&mut self, device: wgpu::Device, queue: wgpu::Queue) -> Result<()> {
        // Create scene renderer
        let scene_renderer = SceneRenderer::new(
            device.clone(),
            queue.clone(),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            None, // No default tilemap texture
            None, // No default sprite texture
        )
        .map_err(|e| anyhow::anyhow!("Failed to create scene renderer: {}", e))?;

        let presentation_target =
            crate::rendering::PresentedOffscreenTexture::new(&device, self.viewport_size)
                .map_err(|e| anyhow::anyhow!("Failed to create viewport target: {}", e))?;

        let mut scene_renderer = scene_renderer;
        scene_renderer.set_clear_color(self.scene_clear_color);
        self.scene_renderer = Some(scene_renderer);
        self.presentation_target = Some(presentation_target);
        self.device = Some(device);
        self.queue = Some(queue);
        self.is_initialized = true;

        tracing::info!("Scene viewport initialized with unified rendering");
        Ok(())
    }

    fn set_viewport_size_immediate(&mut self, new_size: (u32, u32)) {
        self.viewport_size = new_size;
        self.camera.viewport_size = glam::UVec2::new(new_size.0, new_size.1);
    }

    fn effective_camera_scale(&self) -> f32 {
        // In editor mode, we use the editor's zoom scale for viewing
        // The camera.zoom is for game runtime zoom-in effect
        (1.0 / self.camera.zoom) * self.editor_zoom_scale
    }

    fn calculate_editor_projection(&self) -> glam::Mat4 {
        let left = self.camera.position.x as f32;
        let top = self.camera.position.y as f32;
        let effective_scale = self.effective_camera_scale();
        let right = left + self.viewport_size.0 as f32 * effective_scale;
        let bottom = top + self.viewport_size.1 as f32 * effective_scale;
        glam::Mat4::orthographic_rh_gl(left, right, bottom, top, -1.0, 1.0)
    }

    fn apply_requested_viewport_size(&mut self) -> Result<()> {
        let Some(new_size) = self.requested_viewport_size.take() else {
            return Ok(());
        };

        if new_size == self.viewport_size {
            return Ok(());
        }

        self.set_viewport_size_immediate(new_size);
        if let Some(target) = &mut self.presentation_target {
            let device = self
                .device
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing device for viewport resize"))?;
            target
                .resize(device, new_size)
                .map_err(|e| anyhow::anyhow!("Failed to resize viewport target: {}", e))?;
        }
        self.request_render();
        Ok(())
    }

    pub fn request_viewport_size(&mut self, new_size: (u32, u32)) -> bool {
        let (current_size, requested_size, changed) = request_viewport_size_state(
            self.sizing_mode,
            self.is_initialized,
            self.viewport_size,
            self.requested_viewport_size,
            new_size,
        );
        if !changed {
            return false;
        }

        self.requested_viewport_size = requested_size;
        if !self.is_initialized {
            self.set_viewport_size_immediate(current_size);
        }
        self.request_render();
        true
    }

    /// Update the viewport (called every frame if needed)
    pub fn update(&mut self) -> Result<()> {
        if !self.is_initialized {
            return Ok(());
        }

        self.tick_tile_animations();
        Ok(())
    }

    /// Advance tile animation playback and mark dirty when a frame changes.
    fn tick_tile_animations(&mut self) {
        if self.tileset_atlas_cache.is_empty() {
            return;
        }
        // Fixed timestep matching ~60 fps; editor has no high-resolution delta.
        const EDITOR_FRAME_DELTA_MS: f32 = 16.67;
        if self.tile_animation_clock.update_from_iter(
            EDITOR_FRAME_DELTA_MS,
            self.tileset_atlas_cache.values().map(|source| &source.meta),
        ) {
            self.invalidate_tilemap_render_cache();
        }
    }

    /// Render scene to offscreen texture (called before egui UI construction)
    pub fn render_to_texture(
        &mut self,
        project_path: &std::path::Path,
        project_assets: &ProjectAssets,
        renderer: &mut crate::rendering::WindowRenderer,
        overlay_data: &ViewportOverlayData,
    ) -> Result<()> {
        if !self.is_initialized {
            return Ok(()); // Skip if not initialized
        }

        self.apply_requested_viewport_size()?;

        // Only render if scene needs updating
        if !self.needs_render {
            return Ok(()); // Skip silently - no need to log this every frame
        }

        tracing::trace!("Scene needs re-rendering, proceeding with render");

        // Prepare scene data
        let scene_data = self.prepare_scene_data(Some(project_path), project_assets, overlay_data);

        // Render to offscreen target
        let projection = self.calculate_editor_projection();

        if let (Some(scene_renderer), Some(target)) =
            (&mut self.scene_renderer, &mut self.presentation_target)
        {
            scene_renderer.set_post_process_settings(
                self.indexed_presentation_settings
                    .resolve_post_process(&self.available_palettes),
            );
            tracing::trace!("About to render scene with data: tilemap={}, tilemap_batches={}, sprites={}, debug_shapes={}",
                           scene_data.tilemap.is_some(),
                           scene_data.tilemap_batches.len(),
                           scene_data.sprites.len(),
                           scene_data.debug_shapes.len());

            // Render scene to texture with camera projection
            scene_renderer.render_scene_with_projection(
                target.scene_target_mut(),
                &scene_data,
                projection,
            )?;

            let device = self
                .device
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing device for egui registration"))?;
            let queue = self
                .queue
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing queue for egui registration"))?;
            let texture_id = target.present_to_egui(device, queue, renderer.egui_renderer_mut())?;
            tracing::trace!("Registered texture with egui, texture_id: {:?}", texture_id);

            tracing::trace!("Scene rendered to texture successfully");

            // Clear dirty flag after successful render
            self.needs_render = false;
        } else {
            tracing::warn!(
                "Scene renderer or offscreen target not available: renderer={}, target={}",
                self.scene_renderer.is_some(),
                self.presentation_target.is_some()
            );
        }

        Ok(())
    }

    /// Display the pre-rendered texture in egui UI
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        _project_path: Option<&std::path::Path>,
        _renderer: Option<&mut crate::rendering::WindowRenderer>,
    ) {
        if !self.is_initialized {
            self.render_placeholder(ui, rect);
            return;
        }

        // Keep native resolution - don't resize offscreen target based on UI size
        // The texture will be stretched by egui to fit the UI rect

        // Display the pre-rendered texture or show fallback message
        if let Some(target) = &self.presentation_target {
            if let Some(texture_id) = target.texture_id() {
                // Calculate aspect ratio preserving viewport size
                let display_rect = if self.sizing_mode == ViewportSizingMode::Responsive {
                    rect
                } else {
                    let viewport_aspect = self.viewport_size.0 as f32 / self.viewport_size.1 as f32;
                    let available_size = rect.size();
                    let available_aspect = available_size.x / available_size.y;

                    let display_size = if available_aspect > viewport_aspect {
                        egui::Vec2::new(available_size.y * viewport_aspect, available_size.y)
                    } else {
                        egui::Vec2::new(available_size.x, available_size.x / viewport_aspect)
                    };

                    let offset = (available_size - display_size) * 0.5;
                    egui::Rect::from_min_size(rect.min + offset, display_size)
                };

                // Handle mouse interaction for camera panning and future entity selection
                let response = ui.allocate_response(rect.size(), egui::Sense::click_and_drag());

                // Log once when UI response is created (only if mouse is interacting)
                if response.hovered() || response.clicked() || response.dragged() {
                    tracing::trace!(
                        "UI response - rect size: {:?}, hovered: {}, clicked: {}, dragged: {}",
                        rect.size(),
                        response.hovered(),
                        response.clicked(),
                        response.dragged()
                    );
                }

                // Mouse interaction now handled in editor_ui.rs

                if let Some(fill) = self.ui_background_fill {
                    ui.painter().rect_filled(rect, 0.0, fill);
                }

                // Draw the viewport texture with preserved aspect ratio
                ui.painter().image(
                    texture_id,
                    display_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                // Show status instead of error - this is normal during initialization
                self.render_debug_status(ui, rect, "Texture rendering in progress...");
            }
        } else {
            self.render_error(ui, rect, "Viewport target not initialized");
        }
    }

    /// Get reference to game state
    pub fn game_state(&self) -> &GameState {
        &self.game_state
    }

    /// Get mutable reference to game state
    pub fn game_state_mut(&mut self) -> &mut GameState {
        &mut self.game_state
    }

    /// Get reference to resources
    #[allow(dead_code)]
    pub fn resources(&self) -> &ResourceManager {
        &self.resources
    }

    /// Get reference to current tilemap
    pub fn tilemap(&self) -> Option<&TileMap> {
        self.tilemap.as_ref()
    }

    pub fn tileset_resolver(&self) -> Option<TileSetResolver<'_>> {
        Some(TileSetResolver::new(
            self.tileset_cache.as_ref()?,
            &self.tileset_atlas_cache,
        ))
    }

    /// Get mutable reference to current tilemap
    pub fn tilemap_mut(&mut self) -> Option<&mut TileMap> {
        self.tilemap.as_mut()
    }

    /// Load a tilemap from file
    pub fn load_tilemap<P: AsRef<Path>>(&mut self, map_path: P) -> Result<()> {
        let tilemap = TileMap::load_from_file(&map_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tilemap: {}", e))?;

        tilemap
            .validate()
            .map_err(|e| anyhow::anyhow!("Invalid tilemap: {}", e))?;

        self.tilemap = Some(tilemap);
        self.tilemap_path = Some(map_path.as_ref().to_path_buf());
        self.mark_dirty();
        tracing::info!("Loaded tilemap from: {}", map_path.as_ref().display());
        Ok(())
    }

    /// Set the current tilemap directly without loading from disk.
    pub fn set_tilemap(&mut self, tilemap: TileMap) -> Result<()> {
        tilemap
            .validate()
            .map_err(|e| anyhow::anyhow!("Invalid tilemap: {}", e))?;
        self.tilemap = Some(tilemap);
        self.tilemap_path = None;
        self.mark_dirty();
        tracing::info!("Set in-memory tilemap on scene viewport");
        Ok(())
    }

    /// Clear the current tilemap
    pub fn clear_tilemap(&mut self) {
        self.tilemap = None;
        self.tilemap_path = None;
        self.mark_dirty();
        tracing::info!("Cleared tilemap from scene viewport");
    }

    pub fn camera_state(&self) -> (glam::IVec2, f32) {
        (self.camera.position, self.effective_camera_scale())
    }

    pub fn viewport_size(&self) -> (u32, u32) {
        self.viewport_size
    }

    pub fn sizing_mode(&self) -> ViewportSizingMode {
        self.sizing_mode
    }

    /// Find entity at world position for hit detection
    pub fn get_entity_at_world_pos(
        &self,
        world_pos: glam::Vec2,
    ) -> Option<toki_core::entity::EntityId> {
        // Convert world position to integer coordinates for comparison
        let world_pos_i32 = world_to_i32_floor(world_pos);

        let mut entities = self
            .game_state
            .world()
            .entity_manager()
            .visible_entities()
            .into_iter()
            .filter_map(|entity_id| {
                self.game_state
                    .world()
                    .entity_manager()
                    .get_entity(entity_id)
            })
            .collect::<Vec<_>>();
        entities.sort_by_key(|entity| {
            (
                entity.ground_contact_y(),
                entity.rendering.render_layer,
                entity.id,
            )
        });

        for entity in entities.into_iter().rev() {
            if point_in_entity_bounds(world_pos_i32, entity.position, entity.size) {
                tracing::debug!(
                    "Entity hit detected: ID={}, position=({}, {}), size={}x{}, click=({}, {})",
                    entity.id,
                    entity.position.x,
                    entity.position.y,
                    entity.size.x,
                    entity.size.y,
                    world_pos_i32.x,
                    world_pos_i32.y
                );
                return Some(entity.id);
            }
        }

        tracing::trace!(
            "No entity hit at world position ({}, {})",
            world_pos_i32.x,
            world_pos_i32.y
        );
        None
    }

    /// Mark the scene as needing a re-render
    pub fn mark_dirty(&mut self) {
        tracing::trace!("Scene viewport marked dirty - will re-render on next frame");
        self.invalidate_tilemap_render_cache();
    }

    pub fn request_render(&mut self) {
        tracing::trace!("Scene viewport redraw requested");
        self.needs_render = true;
    }

    pub fn invalidate_tilemap_render_cache(&mut self) {
        self.cached_tilemap_batches.clear();
        self.tilemap_render_cache_dirty = true;
        self.tilemap_revision = self.tilemap_revision.wrapping_add(1);
        self.request_render();
    }

    pub fn tilemap_revision(&self) -> u64 {
        self.tilemap_revision
    }

    pub fn clear_asset_caches(&mut self) {
        self.tileset_cache = None;
        self.tileset_cache_path = None;
        self.tileset_atlas_cache.clear();
        self.loaded_sprite_atlases.clear();
        self.loaded_object_sheets.clear();
        self.decoded_sprite_images.clear();
        self.recolored_sprite_images.clear();
        self.tilemap_texture_cache_key = None;
        if let Some(scene_renderer) = &mut self.scene_renderer {
            scene_renderer.clear_sprite_texture_cache();
        }
        self.invalidate_tilemap_render_cache();
    }

    pub fn set_available_palettes(&mut self, palettes: &BTreeMap<String, Palette>) {
        self.available_palettes = palettes.clone();
        self.recolored_sprite_images.clear();
        self.tilemap_texture_cache_key = None;
        self.invalidate_tilemap_render_cache();
    }

    pub fn set_indexed_presentation_settings(&mut self, settings: IndexedPresentationSettings) {
        if self.indexed_presentation_settings != settings {
            self.indexed_presentation_settings = settings;
            self.invalidate_tilemap_render_cache();
        }
    }

    pub fn set_clear_color(&mut self, clear_color: wgpu::Color) {
        self.scene_clear_color = clear_color;
        if let Some(scene_renderer) = &mut self.scene_renderer {
            scene_renderer.set_clear_color(clear_color);
        }
        self.request_render();
    }

    pub fn set_ui_background_fill(&mut self, fill: Option<egui::Color32>) {
        self.ui_background_fill = fill;
        self.request_render();
    }

    #[allow(dead_code)]
    pub fn set_indexed_palette_override(&mut self, palette_id: Option<String>) {
        let mut settings = self.indexed_presentation_settings.clone();
        settings.indexed_palette_override = palette_id;
        self.set_indexed_presentation_settings(settings);
    }

    pub fn indexed_presentation_settings(&self) -> &IndexedPresentationSettings {
        &self.indexed_presentation_settings
    }

    pub fn needs_render(&self) -> bool {
        self.needs_render
    }

    /// Returns `true` when the viewport has active tile animations that require
    /// continuous repainting.
    pub fn has_active_tile_animations(&self) -> bool {
        self.tileset_atlas_cache
            .values()
            .any(|atlas| !atlas.meta.animated_tiles.is_empty())
    }

    /// Temporarily suppress rendering for multiple entities.
    pub fn suppress_entity_rendering_many(
        &mut self,
        entity_ids: impl IntoIterator<Item = toki_core::entity::EntityId>,
    ) {
        let mut changed = false;
        for entity_id in entity_ids {
            if self.suppressed_entity_ids.insert(entity_id) {
                changed = true;
            }
        }
        if changed {
            self.request_render();
        }
    }

    /// Clear temporary entity render suppression.
    pub fn clear_suppressed_entity_rendering(&mut self) {
        if !self.suppressed_entity_ids.is_empty() {
            self.suppressed_entity_ids.clear();
            self.request_render();
        }
    }

    /// Zoom in (increase scale)
    pub fn zoom_in(&mut self) {
        let next_scale = next_zoom_in_scale(self.editor_zoom_scale);
        if (next_scale - self.editor_zoom_scale).abs() > f32::EPSILON {
            self.editor_zoom_scale = next_scale;
            self.request_render();
            tracing::debug!("Zoomed in to editor scale {}", self.editor_zoom_scale);
        } else {
            tracing::trace!("Already at minimum zoom level: {}", self.editor_zoom_scale);
        }
    }

    /// Zoom out (decrease scale)
    pub fn zoom_out(&mut self) {
        let next_scale = next_zoom_out_scale(self.editor_zoom_scale);
        if (next_scale - self.editor_zoom_scale).abs() > f32::EPSILON {
            self.editor_zoom_scale = next_scale;
            self.request_render();
            tracing::debug!("Zoomed out to editor scale {}", self.editor_zoom_scale);
        } else {
            tracing::trace!("Already at maximum zoom level: {}", self.editor_zoom_scale);
        }
    }

    /// Handle keyboard input for zoom controls using logical keys (respects keyboard layout)
    pub fn handle_keyboard_input(
        &mut self,
        logical_key: &winit::keyboard::Key,
        _modifiers: winit::event::Modifiers,
        pressed: bool,
    ) -> bool {
        tracing::trace!(
            "Viewport keyboard input: {:?}, pressed: {}",
            logical_key,
            pressed
        );
        if pressed {
            match logical_key {
                winit::keyboard::Key::Character(ch) => {
                    let ch_str = ch.as_str();
                    match ch_str {
                        "+" => {
                            tracing::debug!("Zoom in key pressed (+)");
                            self.zoom_in();
                            return true;
                        }
                        "-" => {
                            tracing::debug!("Zoom out key pressed (-)");
                            self.zoom_out();
                            return true;
                        }
                        _ => {}
                    }
                }
                winit::keyboard::Key::Named(_) => {} // Future: camera panning via arrow keys
                _ => {}
            }
            tracing::trace!("Viewport: Unhandled key {:?}", logical_key);
        }
        false // Event not handled
    }

    // Note: Additional methods like toggle_collision_boxes, etc. can be added when needed
}

#[cfg(test)]
#[path = "viewport_tests.rs"]
mod tests;
