use anyhow::Result;
use crate::rendering::SceneRenderer;
use crate::scene::SceneManager;

/// Handles the scene viewport - integration between scene data and rendering
pub struct SceneViewport {
    scene_manager: SceneManager,
    scene_renderer: SceneRenderer,
    is_initialized: bool,
}

impl SceneViewport {
    /// Create viewport with existing game state
    pub fn with_game_state(game_state: toki_core::GameState) -> Result<Self> {
        let scene_manager = SceneManager::with_game_state(game_state)?;
        let scene_renderer = SceneRenderer::new()?;
        
        Ok(Self {
            scene_manager,
            scene_renderer,
            is_initialized: false,
        })
    }
    
    /// Initialize the viewport
    pub fn initialize(&mut self) {
        self.is_initialized = true;
        tracing::info!("Scene viewport initialized");
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
    
    /// Render the viewport using egui
    pub fn render(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        if !self.is_initialized {
            // Show placeholder when not initialized
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
            return;
        }
        
        // Use the scene renderer to draw the scene
        self.scene_renderer.render_for_editor(
            ui,
            self.scene_manager.game_state(),
            self.scene_manager.camera(),
            rect,
        );
    }
    
    /// Handle click events for entity selection
    pub fn handle_click(&mut self, screen_pos: glam::Vec2, viewport_rect: egui::Rect) -> Option<u32> {
        if !self.is_initialized {
            return None;
        }
        
        // Test if we clicked on an entity
        if let Some(entity_id) = self.scene_renderer.entity_at_position(
            self.scene_manager.game_state(),
            self.scene_manager.camera(),
            screen_pos,
            viewport_rect,
        ) {
            // Select the clicked entity
            self.scene_renderer.select_entity(Some(entity_id));
            tracing::info!("Selected entity {}", entity_id);
            Some(entity_id)
        } else {
            // Clear selection if clicked on empty space
            self.scene_renderer.select_entity(None);
            tracing::info!("Cleared selection");
            None
        }
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
        self.scene_renderer.selected_entity()
    }
    
    // Note: Additional methods like toggle_collision_boxes, etc. can be added when needed
}