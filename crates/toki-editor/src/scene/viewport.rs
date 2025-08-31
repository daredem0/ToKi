use anyhow::Result;
use crate::scene::SceneManager;
use toki_render::{SceneRenderer, SceneData, OffscreenTarget};
use toki_core::assets::atlas::AtlasMeta;
use toki_core::Camera;

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
    camera: Camera, // Camera for zoom and pan
    // Mouse interaction state
    last_mouse_pos: Option<glam::Vec2>, // For camera panning
    is_dragging_camera: bool,
}

impl SceneViewport {
    /// Create viewport with existing game state
    pub fn with_game_state(game_state: toki_core::GameState) -> Result<Self> {
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
            viewport_size: (160, 144), // Native runtime resolution
            atlas_cache: None,
            needs_render: true, // Initial render required
            camera,
            last_mouse_pos: None,
            is_dragging_camera: false,
        })
    }
    
    /// Initialize the viewport with WGPU context
    pub async fn initialize(&mut self, device: wgpu::Device, queue: wgpu::Queue) -> Result<()> {
        // Create scene renderer
        let scene_renderer = SceneRenderer::new(
            device.clone(),
            queue.clone(),
            wgpu::TextureFormat::Bgra8UnormSrgb, // Match pipeline format
            None, // No default tilemap texture
            None, // No default sprite texture
        ).map_err(|e| anyhow::anyhow!("Failed to create scene renderer: {}", e))?;
        
        // Create offscreen render target
        let offscreen_target = OffscreenTarget::new(
            device.clone(),
            self.viewport_size,
            wgpu::TextureFormat::Bgra8UnormSrgb, // Match pipeline format
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
                           
            // Calculate projection matrix using camera
            let projection = self.camera.calculate_projection();
            
            // Render scene to texture with camera projection
            scene_renderer.render_scene_with_projection(target, &scene_data, projection)?;
            
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
        
        // Keep native resolution - don't resize offscreen target based on UI size
        // The texture will be stretched by egui to fit the UI rect
        
        // Display the pre-rendered texture or show fallback message
        if let Some(_target) = &self.offscreen_target {
            // Access the texture ID if available (compiled with editor feature)
            #[cfg(feature = "editor")]
            {
                if let Some(texture_id) = _target.egui_texture_id {
                    // Calculate aspect ratio preserving viewport size
                    let viewport_aspect = 160.0 / 144.0; // Native aspect ratio (10:9)
                    let available_size = rect.size();
                    let available_aspect = available_size.x / available_size.y;
                    
                    let display_size = if available_aspect > viewport_aspect {
                        // Available space is wider than viewport - letterbox horizontally
                        egui::Vec2::new(available_size.y * viewport_aspect, available_size.y)
                    } else {
                        // Available space is taller than viewport - letterbox vertically  
                        egui::Vec2::new(available_size.x, available_size.x / viewport_aspect)
                    };
                    
                    // Center the viewport within the available rect
                    let offset = (available_size - display_size) * 0.5;
                    let display_rect = egui::Rect::from_min_size(rect.min + offset, display_size);
                    
                    // Handle mouse interaction for camera panning and future entity selection
                    let response = ui.allocate_response(rect.size(), egui::Sense::click_and_drag());
                    
                    // Log once when UI response is created (only if mouse is interacting)
                    if response.hovered() || response.clicked() || response.dragged() {
                        tracing::debug!("UI response - rect size: {:?}, hovered: {}, clicked: {}, dragged: {}", 
                                      rect.size(), response.hovered(), response.clicked(), response.dragged());
                    }
                    
                    // Mouse interaction now handled in editor_ui.rs
                    
                    // Fill background with dark color for letterbox areas
                    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(20, 20, 25));
                    
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
                        // Calculate texture size from atlas: tile_size * grid_dimensions
                        let texture_size = atlas.image_size().unwrap_or(glam::UVec2::new(64, 8));
                        tracing::debug!("Calculated atlas texture size: {}x{}", texture_size.x, texture_size.y);
                        scene_data.atlas = Some(atlas);
                        scene_data.texture_size = texture_size;
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
        
        // Add sprites from game state (same logic as toki-runtime)
        let renderable_entities = self.scene_manager.game_state().get_renderable_entities();
        tracing::debug!("Found {} renderable entities", renderable_entities.len());
        
        for (entity_id, position, size) in renderable_entities {
            // Get the entity to find its atlas
            if let Some(entity) = self.scene_manager.game_state().entity_manager().get_entity(entity_id) {
                if let Some(animation_controller) = &entity.attributes.animation_controller {
                    if let Ok(atlas_name) = animation_controller.current_atlas_name() {
                        tracing::debug!("Processing entity {} with atlas '{}'", entity_id, atlas_name);
                        
                        // Load sprite atlas if needed (similar to runtime's creature_atlas loading)
                        if let Some(project_path) = project_path {
                            match self.load_sprite_atlas_for_entity(&atlas_name, project_path) {
                                Ok(sprite_atlas) => {
                                    let sprite_texture_size = sprite_atlas.image_size().unwrap_or(glam::UVec2::new(64, 16));
                                    tracing::debug!("Loaded sprite atlas '{}' with texture size {}x{}", atlas_name, sprite_texture_size.x, sprite_texture_size.y);
                                    
                                    if let Some(frame) = self.scene_manager.game_state().get_entity_sprite_frame(entity_id, &sprite_atlas, sprite_texture_size) {
                                        let sprite_instance = toki_render::SpriteInstance {
                                            frame,
                                            position,
                                            size,
                                        };
                                        scene_data.sprites.push(sprite_instance);
                                        tracing::debug!("Added sprite for entity {} at ({}, {}) with size {}x{}", 
                                                       entity_id, position.x, position.y, size.x, size.y);
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to load sprite atlas '{}': {}", atlas_name, e);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Add debug shapes for collision boxes (same logic as toki-runtime)
        if self.scene_manager.game_state().is_debug_collision_rendering_enabled() {
            tracing::debug!("Debug collision rendering enabled - adding debug shapes");
            
            // Define colors (same as runtime)
            let entity_color = [1.0, 0.0, 0.0, 0.8]; // Red for entity collision boxes
            let solid_tile_color = [0.0, 0.0, 1.0, 0.6]; // Blue for solid tiles  
            let trigger_tile_color = [1.0, 1.0, 0.0, 0.6]; // Yellow for trigger tiles
            
            // Add entity position/size rectangles (to visualize where entities are)
            let renderable_entities = self.scene_manager.game_state().get_renderable_entities();
            for (entity_id, position, size) in renderable_entities {
                let debug_shape = toki_render::DebugShape {
                    shape_type: toki_render::DebugShapeType::Rectangle,
                    position: position.as_vec2(),
                    size: size.as_vec2(),
                    color: [0.0, 1.0, 0.0, 0.5], // Green for entity bounds
                };
                scene_data.debug_shapes.push(debug_shape);
                tracing::debug!("Added entity bounds for entity {} at ({}, {}) with size {}x{}", 
                               entity_id, position.x, position.y, size.x, size.y);
            }
            
            // Add entity collision boxes
            let entity_boxes = self.scene_manager.game_state().get_entity_collision_boxes();
            for (pos, size, is_trigger) in entity_boxes {
                let color = if is_trigger {
                    trigger_tile_color
                } else {
                    entity_color
                };
                
                let debug_shape = toki_render::DebugShape {
                    shape_type: toki_render::DebugShapeType::Rectangle,
                    position: pos.as_vec2(),
                    size: size.as_vec2(), 
                    color,
                };
                scene_data.debug_shapes.push(debug_shape);
                tracing::debug!("Added entity collision box at ({}, {}) with size {}x{}", 
                               pos.x, pos.y, size.x, size.y);
            }
            
            // Add solid tile debug boxes (if tilemap and atlas are available)
            if let (Some(tilemap), Some(atlas)) = (&scene_data.tilemap, &scene_data.atlas) {
                let solid_tiles = self.scene_manager.game_state().get_solid_tile_positions(tilemap, atlas);
                for (tile_x, tile_y) in solid_tiles {
                    let world_x = tile_x * tilemap.tile_size.x;
                    let world_y = tile_y * tilemap.tile_size.y;
                    
                    let debug_shape = toki_render::DebugShape {
                        shape_type: toki_render::DebugShapeType::Rectangle,
                        position: glam::Vec2::new(world_x as f32, world_y as f32),
                        size: tilemap.tile_size.as_vec2(),
                        color: solid_tile_color,
                    };
                    scene_data.debug_shapes.push(debug_shape);
                }
                
                // Add trigger tile debug boxes
                let trigger_tiles = self.scene_manager.game_state().get_trigger_tile_positions(tilemap, atlas);
                for (tile_x, tile_y) in trigger_tiles {
                    let world_x = tile_x * tilemap.tile_size.x;
                    let world_y = tile_y * tilemap.tile_size.y;
                    
                    let debug_shape = toki_render::DebugShape {
                        shape_type: toki_render::DebugShapeType::Rectangle,
                        position: glam::Vec2::new(world_x as f32, world_y as f32),
                        size: tilemap.tile_size.as_vec2(),
                        color: trigger_tile_color,
                    };
                    scene_data.debug_shapes.push(debug_shape);
                }
            }
            
            tracing::debug!("Added {} debug shapes total", scene_data.debug_shapes.len());
        }
        
        tracing::debug!("Scene data prepared: tilemap={}, atlas={}, sprites={}, debug_shapes={}", 
                       scene_data.tilemap.is_some(), scene_data.atlas.is_some(), scene_data.sprites.len(), scene_data.debug_shapes.len());
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
        
        // Load the corresponding texture image into the renderer
        tracing::debug!("Atlas image field contains: {:?}", atlas.image);
        if let Some(scene_renderer) = &mut self.scene_renderer {
            tracing::debug!("Scene renderer available, proceeding with texture load");
            // Construct the texture path relative to the atlas file
            let texture_path = atlas_path.parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join(&atlas.image);
            
            if texture_path.exists() {
                tracing::info!("Loading tilemap texture: {}", texture_path.display());
                scene_renderer.load_tilemap_texture(texture_path)
                    .map_err(|e| anyhow::anyhow!("Failed to load tilemap texture: {}", e))?;
                tracing::info!("Successfully loaded tilemap texture");
            } else {
                tracing::warn!("Tilemap texture not found: {}", texture_path.display());
            }
        }
        
        // Cache the loaded atlas
        self.atlas_cache = Some(atlas.clone());
        tracing::info!("Loaded and cached atlas: {}", atlas_path.display());
        
        Ok(atlas)
    }
    
    /// Load atlas for sprite entities (similar to tilemap loading)
    fn load_sprite_atlas_for_entity(&mut self, atlas_name: &str, project_path: &std::path::Path) -> Result<AtlasMeta> {
        // Try sprites directory first, then tilemaps directory
        let atlas_path = {
            let sprites_path = project_path.join("assets").join("sprites").join(format!("{}.json", atlas_name));
            if sprites_path.exists() {
                sprites_path
            } else {
                let tilemaps_path = project_path.join("assets").join("tilemaps").join(format!("{}.json", atlas_name));
                if tilemaps_path.exists() {
                    tilemaps_path
                } else {
                    // Fallback to original name without .json extension
                    project_path.join("assets").join("sprites").join(atlas_name)
                }
            }
        };
        
        let atlas = AtlasMeta::load_from_file(&atlas_path)
            .map_err(|e| anyhow::anyhow!("Failed to load sprite atlas '{}' from '{}': {}", atlas_name, atlas_path.display(), e))?;
        
        // Load the corresponding sprite texture into the renderer
        tracing::debug!("Sprite atlas image field contains: {:?}", atlas.image);
        if let Some(scene_renderer) = &mut self.scene_renderer {
            tracing::debug!("Scene renderer available, proceeding with sprite texture load");
            // Construct the texture path relative to the atlas file
            let texture_path = atlas_path.parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join(&atlas.image);
            
            if texture_path.exists() {
                tracing::info!("Loading sprite texture: {}", texture_path.display());
                scene_renderer.load_sprite_texture(texture_path)
                    .map_err(|e| anyhow::anyhow!("Failed to load sprite texture: {}", e))?;
                tracing::info!("Successfully loaded sprite texture");
            } else {
                tracing::warn!("Sprite texture not found: {}", texture_path.display());
            }
        }
        
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
    
    /// Zoom in (increase scale)
    pub fn zoom_in(&mut self) {
        if self.camera.scale > 1 { // Min zoom level
            self.camera.scale -= 1;
            self.mark_dirty();
            tracing::info!("Zoomed in to scale {}", self.camera.scale);
        } else {
            tracing::debug!("Already at minimum zoom level: {}", self.camera.scale);
        }
    }
    
    /// Zoom out (decrease scale)  
    pub fn zoom_out(&mut self) {
        if self.camera.scale < 8 { // Max zoom level
            self.camera.scale += 1;
            self.mark_dirty();
            tracing::info!("Zoomed out to scale {}", self.camera.scale);
        } else {
            tracing::debug!("Already at maximum zoom level: {}", self.camera.scale);
        }
    }
    
    
    /// Handle keyboard input for zoom controls using logical keys (respects keyboard layout)
    pub fn handle_keyboard_input(&mut self, logical_key: &winit::keyboard::Key, _modifiers: winit::event::Modifiers, pressed: bool) -> bool {
        tracing::debug!("Viewport keyboard input: {:?}, pressed: {}", logical_key, pressed);
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
                            tracing::debug!("Viewport: Unhandled character key '{}'", ch_str);
                        }
                    }
                }
                winit::keyboard::Key::Named(named_key) => {
                    match named_key {
                        winit::keyboard::NamedKey::ArrowUp => {
                            // Could add camera panning here in the future
                            tracing::debug!("Viewport: Arrow key up (not handled)");
                        }
                        _ => {
                            tracing::debug!("Viewport: Unhandled named key {:?}", named_key);
                        }
                    }
                }
                _ => {
                    tracing::debug!("Viewport: Unhandled key type {:?}", logical_key);
                }
            }
        }
        false // Event not handled
    }
    
    /// Handle mouse interaction for camera panning and entity selection
    #[allow(dead_code)]
    fn handle_mouse_interaction(&mut self, response: &egui::Response, display_rect: egui::Rect) {
        // Debug: Log mouse interaction state (less spammy)
        if response.clicked() {
            tracing::info!("Mouse clicked! Response: hovered={}, clicked={}, dragged={}", 
                          response.hovered(), response.clicked(), response.dragged());
        }
        
        if response.drag_started() {
            tracing::info!("Drag started!");
        }
        
        if response.dragged() {
            tracing::info!("Dragging...");
        }
        
        if response.drag_stopped() {
            tracing::info!("Drag stopped!");
        }
        
        // Try multiple mouse position detection methods in order of preference
        let mouse_pos_opt = response.interact_pointer_pos()
            .or_else(|| response.hover_pos()) 
            .or_else(|| response.ctx.pointer_hover_pos());
            
        if let Some(mouse_pos) = mouse_pos_opt {
            if display_rect.contains(mouse_pos) {
                tracing::debug!("Mouse at {:?} within display rect, handling interaction", mouse_pos);
                self.handle_viewport_mouse_interaction(response, mouse_pos, display_rect);
            } else {
                // Mouse is in letterbox area - stop any ongoing camera drag
                if self.is_dragging_camera {
                    tracing::debug!("Mouse in letterbox area at {:?}, stopping camera drag", mouse_pos);
                    self.stop_camera_drag();
                }
            }
        } else if self.is_dragging_camera {
            // Mouse left the viewport area - only log if we're actually dragging
            tracing::debug!("Mouse left viewport area while dragging, stopping camera drag");
            self.stop_camera_drag();
        }
    }
    
    /// Handle mouse interaction within the actual viewport area
    fn handle_viewport_mouse_interaction(&mut self, response: &egui::Response, mouse_pos: egui::Pos2, display_rect: egui::Rect) {
        let mouse_vec2 = glam::Vec2::new(mouse_pos.x, mouse_pos.y);
        
        if response.drag_started() {
            // Start drag - could be camera pan or entity selection/movement
            let _world_pos = self.screen_to_world_pos(mouse_pos, display_rect);
            
            // TODO: Check if we clicked on an entity first
            // let clicked_entity = self.get_entity_at_world_pos(_world_pos);
            // if let Some(entity_id) = clicked_entity {
            //     self.start_entity_drag(entity_id, _world_pos);
            // } else {
                // No entity clicked - start camera panning
                self.start_camera_drag(mouse_vec2);
            // }
        } else if response.dragged() {
            // Continue ongoing drag
            if self.is_dragging_camera {
                // NOTE: This is now handled in editor_ui.rs
                // self.update_camera_drag(mouse_vec2, 1.0);
            }
            // TODO: Handle entity dragging
        } else if response.drag_stopped() {
            // End any ongoing drag
            self.stop_camera_drag();
            // TODO: Stop entity drag
        }
        
        // Handle single clicks for selection (when not dragging)
        if response.clicked() && !response.dragged() {
            let world_pos = self.screen_to_world_pos(mouse_pos, display_rect);
            tracing::debug!("Viewport clicked at world position: {:?}", world_pos);
            // TODO: Handle entity selection
        }
    }
    
    /// Convert screen position to world position
    fn screen_to_world_pos(&self, screen_pos: egui::Pos2, display_rect: egui::Rect) -> glam::Vec2 {
        // Convert screen position relative to display rect to 0-1 normalized coordinates
        let normalized_x = (screen_pos.x - display_rect.min.x) / display_rect.width();
        let normalized_y = (screen_pos.y - display_rect.min.y) / display_rect.height();
        
        // Convert to viewport coordinates (160x144)
        let viewport_x = normalized_x * self.viewport_size.0 as f32;
        let viewport_y = normalized_y * self.viewport_size.1 as f32;
        
        // Convert to world coordinates using camera
        let world_x = self.camera.position.x as f32 + viewport_x * self.camera.scale as f32;
        let world_y = self.camera.position.y as f32 + viewport_y * self.camera.scale as f32;
        
        glam::Vec2::new(world_x, world_y)
    }
    
    /// Start camera panning drag
    pub fn start_camera_drag(&mut self, mouse_pos: glam::Vec2) {
        self.is_dragging_camera = true;
        self.last_mouse_pos = Some(mouse_pos);
        tracing::info!("Started camera drag at: {:?}", mouse_pos);
    }
    
    /// Update camera position during drag
    pub fn update_camera_drag(&mut self, mouse_pos: glam::Vec2, pan_speed: f32) {
        if let Some(last_pos) = self.last_mouse_pos {
            // Calculate mouse movement in screen space
            let screen_delta = mouse_pos - last_pos;
            
            // Convert screen delta to world delta (account for camera scale, aspect ratio, and pan speed)
            let world_delta_x = -screen_delta.x * self.camera.scale as f32 * pan_speed;
            let world_delta_y = -screen_delta.y * self.camera.scale as f32 * pan_speed;
            
            // Apply camera movement (negative for natural drag feel)
            self.camera.move_by(glam::IVec2::new(world_delta_x as i32, world_delta_y as i32));
            
            // Mark for re-render
            self.mark_dirty();
            
            tracing::trace!("Camera dragged by screen delta: {:?}, world delta: ({}, {}) [pan_speed: {}]", 
                          screen_delta, world_delta_x, world_delta_y, pan_speed);
        }
        
        self.last_mouse_pos = Some(mouse_pos);
    }
    
    /// Stop camera panning drag
    pub fn stop_camera_drag(&mut self) {
        if self.is_dragging_camera {
            tracing::info!("Stopped camera drag");
            self.is_dragging_camera = false;
            self.last_mouse_pos = None;
        }
    }
    
    // Note: Additional methods like toggle_collision_boxes, etc. can be added when needed
}