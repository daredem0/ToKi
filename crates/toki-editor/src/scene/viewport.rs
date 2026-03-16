use crate::project::assets::{ObjectSheetAsset, SpriteAtlasAsset};
use crate::project::ProjectAssets;
use crate::scene::SceneManager;
use anyhow::Result;
use toki_core::assets::{
    atlas::AtlasMeta, object_sheet::ObjectSheetMeta, tilemap::MapObjectInstance,
};
use toki_core::Camera;
use toki_render::{OffscreenTarget, SceneData, SceneRenderer};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportSizingMode {
    Fixed,
    Responsive,
}

/// Handles the scene viewport - integration between scene data and rendering
pub struct SceneViewport {
    scene_manager: SceneManager,
    scene_renderer: Option<SceneRenderer>,
    offscreen_target: Option<OffscreenTarget>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    is_initialized: bool,
    sizing_mode: ViewportSizingMode,
    viewport_size: (u32, u32),
    requested_viewport_size: Option<(u32, u32)>,
    atlas_cache: Option<AtlasMeta>,
    needs_render: bool, // Track if scene needs re-rendering
    camera: Camera,     // Camera for zoom and pan
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
}

impl SceneViewport {
    /// Create viewport with existing game state
    pub fn with_game_state(game_state: toki_core::GameState) -> Result<Self> {
        Self::with_game_state_and_sizing_mode(game_state, ViewportSizingMode::Fixed)
    }

    pub fn with_game_state_responsive(game_state: toki_core::GameState) -> Result<Self> {
        Self::with_game_state_and_sizing_mode(game_state, ViewportSizingMode::Responsive)
    }

    fn with_game_state_and_sizing_mode(
        game_state: toki_core::GameState,
        sizing_mode: ViewportSizingMode,
    ) -> Result<Self> {
        let scene_manager = SceneManager::with_game_state(game_state)?;

        // Initialize camera with default toki-runtime settings
        let mut camera = Camera::new();
        camera.viewport_size = glam::UVec2::new(160, 144); // Match toki-runtime native resolution
        camera.scale = 1; // Default zoom same as toki-runtime
        camera.center_on(glam::IVec2::new(80, 72)); // Center on viewport

        Ok(Self {
            scene_manager,
            scene_renderer: None,
            offscreen_target: None,
            device: None,
            queue: None,
            is_initialized: false,
            sizing_mode,
            viewport_size: (160, 144), // Native runtime resolution
            requested_viewport_size: None,
            atlas_cache: None,
            needs_render: true, // Initial render required
            camera,
            editor_zoom_scale: 1.0,
            last_mouse_pos: None,
            is_dragging_camera: false,
            suppressed_entity_ids: std::collections::HashSet::new(),
            loaded_sprite_atlases: std::collections::HashMap::new(),
            loaded_object_sheets: std::collections::HashMap::new(),
        })
    }

    /// Initialize the viewport with WGPU context
    pub async fn initialize(&mut self, device: wgpu::Device, queue: wgpu::Queue) -> Result<()> {
        // Create scene renderer
        let scene_renderer = SceneRenderer::new(
            device.clone(),
            queue.clone(),
            wgpu::TextureFormat::Bgra8UnormSrgb, // Match pipeline format
            None,                                // No default tilemap texture
            None,                                // No default sprite texture
        )
        .map_err(|e| anyhow::anyhow!("Failed to create scene renderer: {}", e))?;

        // Create offscreen render target
        let offscreen_target = OffscreenTarget::new(
            device.clone(),
            self.viewport_size,
            wgpu::TextureFormat::Bgra8UnormSrgb, // Match pipeline format
        )
        .map_err(|e| anyhow::anyhow!("Failed to create offscreen target: {}", e))?;

        self.scene_renderer = Some(scene_renderer);
        self.offscreen_target = Some(offscreen_target);
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
        self.camera.scale as f32 * self.editor_zoom_scale
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
        if let Some(target) = &mut self.offscreen_target {
            toki_render::RenderTarget::resize(target, new_size)
                .map_err(|e| anyhow::anyhow!("Failed to resize offscreen target: {}", e))?;
        }
        self.needs_render = true;
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
        self.needs_render = true;
        true
    }

    /// Update the viewport (called every frame if needed)
    pub fn update(&mut self) -> Result<()> {
        if !self.is_initialized {
            return Ok(());
        }

        // Scene doesn't need per-frame updates like runtime does
        // The scene is static until the user modifies it
        Ok(())
    }

    /// Render scene to offscreen texture (called before egui UI construction)
    pub fn render_to_texture(
        &mut self,
        project_path: &std::path::Path,
        project_assets: &ProjectAssets,
        renderer: &mut egui_wgpu::Renderer,
        preview_data: Option<(&str, glam::Vec2, toki_core::sprite::SpriteFrame, bool)>,
        drag_preview_data: Option<&[DragPreviewSprite]>,
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
        let scene_data = self.prepare_scene_data(
            Some(project_path),
            project_assets,
            preview_data,
            drag_preview_data,
        );

        // Render to offscreen target
        let projection = self.calculate_editor_projection();

        if let (Some(scene_renderer), Some(target)) =
            (&mut self.scene_renderer, &mut self.offscreen_target)
        {
            tracing::trace!("About to render scene with data: tilemap={}, atlas={}, sprites={}, debug_shapes={}",
                           scene_data.tilemap.is_some(),
                           scene_data.atlas.is_some(),
                           scene_data.sprites.len(),
                           scene_data.debug_shapes.len());

            // Render scene to texture with camera projection
            scene_renderer.render_scene_with_projection(target, &scene_data, projection)?;

            // Register texture with egui for later use
            let texture_id = target.register_with_egui(renderer);
            tracing::trace!("Registered texture with egui, texture_id: {:?}", texture_id);

            tracing::trace!("Scene rendered to texture successfully");

            // Clear dirty flag after successful render
            self.needs_render = false;
        } else {
            tracing::warn!(
                "Scene renderer or offscreen target not available: renderer={}, target={}",
                self.scene_renderer.is_some(),
                self.offscreen_target.is_some()
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
        _renderer: Option<&mut egui_wgpu::Renderer>,
    ) {
        if !self.is_initialized {
            self.render_placeholder(ui, rect);
            return;
        }

        // Keep native resolution - don't resize offscreen target based on UI size
        // The texture will be stretched by egui to fit the UI rect

        // Display the pre-rendered texture or show fallback message
        if let Some(_target) = &self.offscreen_target {
            // Access the texture ID if available (compiled with editor feature)
            #[cfg(feature = "editor")]
            {
                if let Some(texture_id) = _target.egui_texture_id {
                    // Calculate aspect ratio preserving viewport size
                    let display_rect = if self.sizing_mode == ViewportSizingMode::Responsive {
                        rect
                    } else {
                        let viewport_aspect =
                            self.viewport_size.0 as f32 / self.viewport_size.1 as f32;
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

                    // Fill background with dark color for letterbox areas
                    ui.painter()
                        .rect_filled(rect, 0.0, egui::Color32::from_rgb(20, 20, 25));

                    // Draw the viewport texture with preserved aspect ratio
                    ui.painter().image(
                        texture_id,
                        display_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    // Only log occasionally to avoid spam
                    // tracing::debug!("Displayed pre-rendered texture in viewport with aspect ratio preservation");
                } else {
                    // Show status instead of error - this is normal during initialization
                    self.render_debug_status(ui, rect, "Texture rendering in progress...");
                }
            }

            // If editor feature not enabled, show error
            #[cfg(not(feature = "editor"))]
            self.render_error(ui, rect, "Editor features not enabled");
        } else {
            self.render_error(ui, rect, "Offscreen target not initialized");
        }
    }

    /// Handle click events for entity selection
    #[allow(dead_code)]
    pub fn handle_click(
        &mut self,
        screen_pos: glam::Vec2,
        _viewport_rect: egui::Rect,
    ) -> Option<u32> {
        if !self.is_initialized {
            return None;
        }

        // TODO: Implement entity picking with unified renderer
        // For now, clear any existing selection
        tracing::info!(
            "Click at ({:.2}, {:.2}) - entity picking not yet implemented",
            screen_pos.x,
            screen_pos.y
        );
        None
    }

    /// Get reference to scene manager
    pub fn scene_manager(&self) -> &SceneManager {
        &self.scene_manager
    }

    /// Get mutable reference to scene manager
    pub fn scene_manager_mut(&mut self) -> &mut SceneManager {
        &mut self.scene_manager
    }

    /// Get currently selected entity
    pub fn selected_entity(&self) -> Option<u32> {
        // TODO: Implement with unified renderer
        None
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
        // Get entity IDs from the active scene
        let entity_ids = self
            .scene_manager
            .game_state()
            .entity_manager()
            .active_entities();

        // Convert world position to integer coordinates for comparison
        let world_pos_i32 = world_to_i32_floor(world_pos);

        // Iterate through entity IDs in reverse order (top layer first)
        // This ensures we select the topmost entity if they overlap
        for &entity_id in entity_ids.iter().rev() {
            if let Some(entity) = self
                .scene_manager
                .game_state()
                .entity_manager()
                .get_entity(entity_id)
            {
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
        self.needs_render = true;
    }

    pub fn needs_render(&self) -> bool {
        self.needs_render
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
            self.mark_dirty();
        }
    }

    /// Clear temporary entity render suppression.
    pub fn clear_suppressed_entity_rendering(&mut self) {
        if !self.suppressed_entity_ids.is_empty() {
            self.suppressed_entity_ids.clear();
            self.mark_dirty();
        }
    }

    /// Zoom in (increase scale)
    pub fn zoom_in(&mut self) {
        let next_scale = next_zoom_in_scale(self.editor_zoom_scale);
        if (next_scale - self.editor_zoom_scale).abs() > f32::EPSILON {
            self.editor_zoom_scale = next_scale;
            self.mark_dirty();
            tracing::info!("Zoomed in to editor scale {}", self.editor_zoom_scale);
        } else {
            tracing::trace!("Already at minimum zoom level: {}", self.editor_zoom_scale);
        }
    }

    /// Zoom out (decrease scale)
    pub fn zoom_out(&mut self) {
        let next_scale = next_zoom_out_scale(self.editor_zoom_scale);
        if (next_scale - self.editor_zoom_scale).abs() > f32::EPSILON {
            self.editor_zoom_scale = next_scale;
            self.mark_dirty();
            tracing::info!("Zoomed out to editor scale {}", self.editor_zoom_scale);
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
                            tracing::info!("Zoom in key pressed (+)");
                            self.zoom_in();
                            return true;
                        }
                        "-" => {
                            tracing::info!("Zoom out key pressed (-)");
                            self.zoom_out();
                            return true;
                        }
                        _ => {
                            tracing::trace!("Viewport: Unhandled character key '{}'", ch_str);
                        }
                    }
                }
                winit::keyboard::Key::Named(named_key) => {
                    match named_key {
                        winit::keyboard::NamedKey::ArrowUp => {
                            // Could add camera panning here in the future
                            tracing::trace!("Viewport: Arrow key up (not handled)");
                        }
                        _ => {
                            tracing::trace!("Viewport: Unhandled named key {:?}", named_key);
                        }
                    }
                }
                _ => {
                    tracing::trace!("Viewport: Unhandled key type {:?}", logical_key);
                }
            }
        }
        false // Event not handled
    }

    // Note: Additional methods like toggle_collision_boxes, etc. can be added when needed
}

#[cfg(test)]
#[path = "viewport_tests.rs"]
mod tests;
