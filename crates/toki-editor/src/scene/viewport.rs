use anyhow::Result;
use crate::scene::SceneManager;
use toki_render::{SceneRenderer, SceneData, OffscreenTarget, RenderTarget};
use toki_core::assets::atlas::AtlasMeta;

/// Handles the scene viewport - integration between scene data and rendering
pub struct SceneViewport {
    scene_manager: SceneManager,
    scene_renderer: Option<SceneRenderer>,
    offscreen_target: Option<OffscreenTarget>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    is_initialized: bool,
    viewport_size: (u32, u32),
    atlas_cache: Option<AtlasMeta>,
    needs_render: bool, // Track if scene needs re-rendering
}

impl SceneViewport {
    /// Create viewport with existing game state
    pub fn with_game_state(game_state: toki_core::GameState) -> Result<Self> {
        let scene_manager = SceneManager::with_game_state(game_state)?;
        
        Ok(Self {
            scene_manager,
            scene_renderer: None,
            offscreen_target: None,
            device: None,
            queue: None,
            is_initialized: false,
            viewport_size: (800, 600),
            atlas_cache: None,
            needs_render: true, // Initial render required
        })
    }
    
    /// Initialize the viewport with WGPU context
    pub async fn initialize(&mut self, device: wgpu::Device, queue: wgpu::Queue) -> Result<()> {
        // Create scene renderer
        let scene_renderer = SceneRenderer::new(
            device.clone(),
            queue.clone(),
            wgpu::TextureFormat::Rgba8UnormSrgb, // Standard format for offscreen rendering
            None, // No default tilemap texture
            None, // No default sprite texture
        ).map_err(|e| anyhow::anyhow!("Failed to create scene renderer: {}", e))?;
        
        // Create offscreen render target
        let offscreen_target = OffscreenTarget::new(
            device.clone(),
            self.viewport_size,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ).map_err(|e| anyhow::anyhow!("Failed to create offscreen target: {}", e))?;
        
        self.scene_renderer = Some(scene_renderer);
        self.offscreen_target = Some(offscreen_target);
        self.device = Some(device);
        self.queue = Some(queue);
        self.is_initialized = true;
        
        tracing::info!("Scene viewport initialized with unified rendering");
        Ok(())
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
    pub fn render_to_texture(&mut self, project_path: &std::path::Path, renderer: &mut egui_wgpu::Renderer) -> Result<()> {
        if !self.is_initialized {
            return Ok(()); // Skip if not initialized
        }
        
        // Only render if scene needs updating
        if !self.needs_render {
            return Ok(()); // Skip silently - no need to log this every frame
        }
        
        tracing::debug!("Scene needs re-rendering, proceeding with render");
        
        // Prepare scene data
        let scene_data = self.prepare_scene_data(Some(project_path));
        
        // Render to offscreen target
        if let (Some(scene_renderer), Some(target)) = (&mut self.scene_renderer, &mut self.offscreen_target) {
            tracing::debug!("About to render scene with data: tilemap={}, atlas={}, sprites={}, debug_shapes={}", 
                           scene_data.tilemap.is_some(),
                           scene_data.atlas.is_some(), 
                           scene_data.sprites.len(),
                           scene_data.debug_shapes.len());
                           
            // Render scene to texture
            scene_renderer.render_scene(target, &scene_data)?;
            
            // Register texture with egui for later use
            let texture_id = target.register_with_egui(renderer);
            tracing::debug!("Registered texture with egui, texture_id: {:?}", texture_id);
            
            tracing::debug!("Scene rendered to texture successfully");
            
            // Clear dirty flag after successful render
            self.needs_render = false;
        } else {
            tracing::warn!("Scene renderer or offscreen target not available: renderer={}, target={}", 
                          self.scene_renderer.is_some(), self.offscreen_target.is_some());
        }
        
        Ok(())
    }
    
    /// Display the pre-rendered texture in egui UI
    pub fn render(&mut self, ui: &mut egui::Ui, rect: egui::Rect, _project_path: Option<&std::path::Path>, _renderer: Option<&mut egui_wgpu::Renderer>) {
        if !self.is_initialized {
            self.render_placeholder(ui, rect);
            return;
        }
        
        // Update viewport size if it changed
        let new_size = (rect.width() as u32, rect.height() as u32);
        if new_size != self.viewport_size && new_size.0 > 0 && new_size.1 > 0 {
            self.viewport_size = new_size;
            if let (Some(target), Some(_device)) = (&mut self.offscreen_target, &self.device) {
                if let Err(e) = target.resize(new_size) {
                    tracing::error!("Failed to resize offscreen target: {}", e);
                    return;
                }
            }
        }
        
        // Display the pre-rendered texture or show fallback message
        if let Some(_target) = &self.offscreen_target {
            // Access the texture ID if available (compiled with editor feature)
            #[cfg(feature = "editor")]
            {
                if let Some(texture_id) = _target.egui_texture_id {
                    let texture_rect = egui::Rect::from_min_size(rect.min, rect.size());
                    let _response = ui.allocate_response(rect.size(), egui::Sense::hover());
                    ui.painter().image(
                        texture_id,
                        texture_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    // Only log occasionally to avoid spam
            // tracing::debug!("Displayed pre-rendered texture in viewport");
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
    pub fn handle_click(&mut self, screen_pos: glam::Vec2, _viewport_rect: egui::Rect) -> Option<u32> {
        if !self.is_initialized {
            return None;
        }
        
        // TODO: Implement entity picking with unified renderer
        // For now, clear any existing selection
        tracing::info!("Click at ({:.2}, {:.2}) - entity picking not yet implemented", screen_pos.x, screen_pos.y);
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
    
    /// Render without WGPU renderer (shows message to user)
    pub fn render_fallback_egui(&mut self, ui: &mut egui::Ui, rect: egui::Rect, _project_path: Option<&std::path::Path>) {
        if !self.is_initialized {
            self.render_placeholder(ui, rect);
            return;
        }
        
        // Show that we're working with the unified renderer but need proper integration
        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(40, 40, 50));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Unified WGPU Renderer Initialized\n\nRenderer integration in progress...\nScene data will be displayed here once\nthe render-to-texture pipeline is complete.",
            egui::FontId::default(),
            egui::Color32::WHITE,
        );
        
        // Show some debug info about the initialized renderer
        let debug_y = rect.min.y + 20.0;
        let debug_text = format!(
            "✓ SceneRenderer: {}\n✓ OffscreenTarget: {}\n✓ Atlas Cache: {}",
            if self.scene_renderer.is_some() { "Ready" } else { "Not Ready" },
            if self.offscreen_target.is_some() { "Ready" } else { "Not Ready" },
            if self.atlas_cache.is_some() { "Loaded" } else { "Empty" }
        );
        
        ui.painter().text(
            egui::pos2(rect.min.x + 10.0, debug_y),
            egui::Align2::LEFT_TOP,
            debug_text,
            egui::FontId::monospace(10.0),
            egui::Color32::LIGHT_GRAY,
        );
    }
    
    /// Prepare scene data for rendering
    fn prepare_scene_data(&mut self, project_path: Option<&std::path::Path>) -> SceneData {
        let mut scene_data = SceneData::default();
        
        tracing::debug!("Preparing scene data for rendering...");
        
        // Add tilemap if present
        if let Some(tilemap) = self.scene_manager.tilemap() {
            let tilemap = tilemap.clone(); // Clone to avoid borrow issues
            tracing::debug!("Found tilemap: size={}x{}, atlas={}", 
                           tilemap.size.x, tilemap.size.y, tilemap.atlas.display());
            scene_data.tilemap = Some(tilemap.clone());
            
            // Load atlas for the tilemap
            if let Some(project_path) = project_path {
                tracing::debug!("Loading atlas for tilemap from project path: {}", project_path.display());
                match self.load_atlas_for_tilemap(&tilemap.atlas.to_string_lossy(), project_path) {
                    Ok(atlas) => {
                        tracing::debug!("Successfully loaded atlas with {} tiles", atlas.tiles.len());
                        scene_data.atlas = Some(atlas);
                        scene_data.texture_size = glam::UVec2::new(256, 256); // TODO: Get from atlas
                    }
                    Err(e) => {
                        tracing::error!("Failed to load atlas: {}", e);
                    }
                }
            } else {
                tracing::warn!("No project path provided for atlas loading");
            }
        } else {
            tracing::debug!("No tilemap found in scene manager");
        }
        
        // TODO: Add sprites from game state
        // TODO: Add debug shapes for selected entities, collision boxes, etc.
        
        tracing::debug!("Scene data prepared: tilemap={}, atlas={}", 
                       scene_data.tilemap.is_some(), scene_data.atlas.is_some());
        scene_data
    }
    
    /// Load atlas for tilemap (with caching)
    fn load_atlas_for_tilemap(&mut self, atlas_name: &str, project_path: &std::path::Path) -> Result<AtlasMeta> {
        // Check cache first
        if let Some(cached_atlas) = &self.atlas_cache {
            // Simple check - in production you'd want to compare the atlas file path
            return Ok(cached_atlas.clone());
        }
        
        // Try tilemaps directory first, then sprites directory
        let atlas_path = {
            let tilemaps_path = project_path.join("assets").join("tilemaps").join(atlas_name);
            if tilemaps_path.exists() {
                tilemaps_path
            } else {
                project_path.join("assets").join("sprites").join(atlas_name)
            }
        };
        
        let atlas = AtlasMeta::load_from_file(&atlas_path)
            .map_err(|e| anyhow::anyhow!("Failed to load atlas '{}': {}", atlas_path.display(), e))?;
        
        // Cache the loaded atlas
        self.atlas_cache = Some(atlas.clone());
        tracing::info!("Loaded and cached atlas: {}", atlas_path.display());
        
        Ok(atlas)
    }
    
    /// Render placeholder when not initialized
    fn render_placeholder(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        ui.painter().rect_filled(
            rect,
            4.0,
            egui::Color32::from_rgb(32, 32, 40),
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Initializing Scene Viewport...",
            egui::FontId::default(),
            egui::Color32::WHITE,
        );
    }
    
    /// Render error message
    fn render_error(&self, ui: &mut egui::Ui, rect: egui::Rect, error_msg: &str) {
        ui.painter().rect_filled(
            rect,
            4.0,
            egui::Color32::from_rgb(60, 32, 32),
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            error_msg,
            egui::FontId::default(),
            egui::Color32::WHITE,
        );
    }
    
    /// Render debug status message
    fn render_debug_status(&self, ui: &mut egui::Ui, rect: egui::Rect, status_msg: &str) {
        ui.painter().rect_filled(
            rect,
            4.0,
            egui::Color32::from_rgb(40, 40, 50),
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            status_msg,
            egui::FontId::default(),
            egui::Color32::LIGHT_BLUE,
        );
        
        // Show some debug info about the initialized renderer
        let debug_y = rect.min.y + 30.0;
        let debug_text = format!(
            "✓ SceneRenderer: {}\n✓ OffscreenTarget: {}\n✓ Tilemap: {}",
            if self.scene_renderer.is_some() { "Ready" } else { "Not Ready" },
            if self.offscreen_target.is_some() { "Ready" } else { "Not Ready" },
            if self.scene_manager.tilemap().is_some() { "Loaded" } else { "None" }
        );
        
        ui.painter().text(
            egui::pos2(rect.min.x + 10.0, debug_y),
            egui::Align2::LEFT_TOP,
            debug_text,
            egui::FontId::monospace(10.0),
            egui::Color32::LIGHT_GRAY,
        );
    }
    
    /// Mark the scene as needing a re-render
    pub fn mark_dirty(&mut self) {
        tracing::debug!("Scene viewport marked dirty - will re-render on next frame");
        self.needs_render = true;
    }
    
    /// Check if scene needs re-rendering
    pub fn needs_render(&self) -> bool {
        self.needs_render
    }
    
    // Note: Additional methods like toggle_collision_boxes, etc. can be added when needed
}