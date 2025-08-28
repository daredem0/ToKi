use anyhow::Result;
use toki_core::{GameState, Camera, CameraController, CameraMode};
use toki_runtime::systems::ResourceManager;
use crate::editor_renderer::EditorRenderer;

/// Editor viewport for visualizing and editing game scenes
pub struct GameViewport {
    // Game data (reused from toki-runtime)
    game_state: GameState,
    camera: Camera,
    camera_controller: CameraController,
    resources: ResourceManager,
    
    // Editor-specific renderer and state
    editor_renderer: EditorRenderer,
    is_initialized: bool,
}

impl GameViewport {
    /// Create a new GameViewport for editing
    pub fn new(initial_game_state: GameState) -> Result<Self> {
        // Load resources (reuse from toki-runtime)
        let resources = ResourceManager::load_all()
            .map_err(|e| anyhow::anyhow!("Failed to load resources: {e}"))?;
        
        // Set up camera for editing
        let mut camera = Camera {
            position: glam::IVec2::ZERO,
            viewport_size: glam::UVec2::new(800, 600),
            scale: 2, // Good zoom level for editing
        };
        camera.center_on(glam::IVec2::new(400, 300)); // Center on viewport
        
        // Camera controller for editor (no entity following, just free camera)
        let camera_controller = CameraController {
            mode: CameraMode::FollowEntity(0), // We'll modify this for editor use
        };
        
        // Create editor renderer
        let editor_renderer = EditorRenderer::new()?;
        
        tracing::info!("Editor viewport created successfully");
        Ok(Self {
            game_state: initial_game_state,
            camera,
            camera_controller,
            resources,
            editor_renderer,
            is_initialized: false,
        })
    }

    /// Initialize the viewport (called during editor setup)
    pub fn initialize_wgpu(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue, _format: wgpu::TextureFormat) {
        // Editor renderer doesn't need WGPU initialization (uses egui)
        self.is_initialized = true;
        tracing::info!("Editor viewport initialized");
    }

    /// Update the viewport (called every frame)
    pub fn update(&mut self) -> Result<()> {
        if !self.is_initialized {
            return Ok(());
        }
        
        // Editor doesn't need per-frame updates like the runtime does
        // The game state is static until the user modifies it
        Ok(())
    }

    /// Render the editor viewport
    pub fn render_viewport(&mut self, ui: &mut egui::Ui) {
        let available_size = ui.available_size();
        
        if !self.is_initialized {
            // Show placeholder when not initialized
            let (rect, _response) = ui.allocate_exact_size(available_size, egui::Sense::hover());
            ui.painter().rect_filled(
                rect,
                4.0,
                egui::Color32::from_rgb(32, 32, 40),
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Initializing Editor Viewport...",
                egui::FontId::default(),
                egui::Color32::WHITE,
            );
            return;
        }

        // Allocate the space for our editor rendering
        let (rect, response) = ui.allocate_exact_size(available_size, egui::Sense::click_and_drag());

        // Handle input events for entity selection
        if response.clicked() {
            if let Some(click_pos) = response.interact_pointer_pos() {
                let screen_pos = glam::Vec2::new(click_pos.x, click_pos.y);
                
                // Test if we clicked on an entity
                if let Some(entity_id) = self.editor_renderer.entity_at_position(
                    &self.game_state,
                    &self.camera,
                    screen_pos,
                    rect,
                ) {
                    // Select the clicked entity
                    self.editor_renderer.select_entity(Some(entity_id));
                    tracing::info!("Selected entity {}", entity_id);
                } else {
                    // Clear selection if clicked on empty space
                    self.editor_renderer.select_entity(None);
                    tracing::info!("Cleared selection");
                }
            }
        }

        // Use the editor renderer to draw the scene
        self.editor_renderer.render_for_editor(
            ui,
            &self.game_state,
            &self.camera,
            rect,
        );
        
        // Show debug info overlay
        ui.painter().text(
            rect.min + egui::Vec2::new(10.0, 10.0),
            egui::Align2::LEFT_TOP,
            format!("📝 Editor Mode | Entities: {} | Selected: {:?}", 
                self.game_state.entity_manager().active_entities().len(),
                self.editor_renderer.selected_entity()
            ),
            egui::FontId::monospace(10.0),
            egui::Color32::LIGHT_GREEN,
        );
    }

    /// Get current game state (for editor inspection)
    pub fn game_state(&self) -> &GameState {
        &self.game_state
    }

    /// Get mutable game state (for editor modifications)
    pub fn game_state_mut(&mut self) -> &mut GameState {
        &mut self.game_state
    }
    
    /// Get the selected entity ID
    pub fn selected_entity(&self) -> Option<u32> {
        self.editor_renderer.selected_entity()
    }
    
    /// Select an entity by ID
    pub fn select_entity(&mut self, entity_id: Option<u32>) {
        self.editor_renderer.select_entity(entity_id);
    }
    
    /// Toggle collision box visibility
    pub fn toggle_collision_boxes(&mut self) {
        self.editor_renderer.toggle_collision_boxes();
    }
    
    /// Toggle entity center visibility
    pub fn toggle_entity_centers(&mut self) {
        self.editor_renderer.toggle_entity_centers();
    }
}