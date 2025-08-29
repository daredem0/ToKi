use anyhow::Result;
use toki_core::{GameState, Camera};
use toki_core::assets::tilemap::TileMap;
use toki_core::assets::atlas::AtlasMeta;
use std::path::PathBuf;

/// Handles scene visualization for editing purposes
/// Renders game entities, collision boxes, and selection highlights
pub struct SceneRenderer {
    // Editor-specific state
    selected_entity_id: Option<u32>,
    show_collision_boxes: bool,
    show_entity_centers: bool,
    // Atlas cache for tilemap rendering
    loaded_atlas: Option<(PathBuf, AtlasMeta)>, // (atlas_path, atlas_data)
}

impl SceneRenderer {
    /// Create a new editor renderer
    pub fn new() -> Result<Self> {
        Ok(Self {
            selected_entity_id: None,
            show_collision_boxes: true,
            show_entity_centers: true,
            loaded_atlas: None,
        })
    }
    
    /// Render the game state for editing purposes using egui
    pub fn render_for_editor(
        &mut self,
        ui: &mut egui::Ui,
        game_state: &GameState,
        camera: &Camera,
        viewport_rect: egui::Rect,
        tilemap: Option<&TileMap>,
        project_path: Option<&std::path::Path>,
    ) {
        // Draw background
        ui.painter().rect_filled(
            viewport_rect,
            0.0,
            egui::Color32::from_rgb(32, 48, 64), // Dark blue-gray background
        );
        
        // Calculate camera transform for rendering
        let camera_offset = glam::Vec2::new(camera.position.x as f32, camera.position.y as f32);
        let scale = camera.scale as f32;
        
        // Render tilemap if present (render first so it appears behind entities)
        if let Some(tilemap) = tilemap {
            tracing::debug!("Rendering tilemap: {}x{}, tile_size: {}x{}, atlas: {}", 
                           tilemap.size.x, tilemap.size.y, 
                           tilemap.tile_size.x, tilemap.tile_size.y,
                           tilemap.atlas.display());
            self.render_tilemap_egui(ui, tilemap, camera_offset, scale, viewport_rect, project_path);
        } else {
            tracing::debug!("No tilemap to render");
        }
        
        // Get entity IDs and then get the actual entities
        let entity_ids = game_state.entity_manager().active_entities();
        let entities: Vec<&toki_core::entity::Entity> = entity_ids
            .iter()
            .filter_map(|id| game_state.entity_manager().get_entity(*id))
            .collect();
        
        // Render all entities
        for entity in &entities {
            self.render_entity_egui(ui, entity, camera_offset, scale, viewport_rect);
        }
        
        // Render editor overlays on top
        for entity in &entities {
            if self.show_entity_centers {
                self.render_entity_center_egui(ui, entity, camera_offset, scale, viewport_rect);
            }
            
            if self.show_collision_boxes {
                self.render_collision_box_egui(ui, entity, camera_offset, scale, viewport_rect);
            }
            
            // Highlight selected entity
            if Some(entity.id) == self.selected_entity_id {
                self.render_selection_highlight_egui(ui, entity, camera_offset, scale, viewport_rect);
            }
        }
    }
    
    /// Render a single entity using egui
    fn render_entity_egui(
        &self,
        ui: &mut egui::Ui,
        entity: &toki_core::entity::Entity,
        camera_offset: glam::Vec2,
        scale: f32,
        viewport_rect: egui::Rect,
    ) {
        let entity_pos = entity.position;
        let screen_pos = self.world_to_screen(entity_pos.as_vec2(), camera_offset, scale, viewport_rect);
        
        // For now, render entities as simple colored rectangles
        let entity_size = if let Some(collision_box) = &entity.collision_box {
            collision_box.size.as_vec2() * scale
        } else {
            glam::Vec2::new(16.0, 16.0) * scale // default size
        };
        
        let entity_rect = egui::Rect::from_center_size(
            egui::pos2(screen_pos.x, screen_pos.y),
            egui::vec2(entity_size.x, entity_size.y),
        );
        
        // Color entities differently based on type or ID
        let color = match entity.id % 6 {
            0 => egui::Color32::from_rgb(255, 100, 100), // Red
            1 => egui::Color32::from_rgb(100, 255, 100), // Green  
            2 => egui::Color32::from_rgb(100, 100, 255), // Blue
            3 => egui::Color32::from_rgb(255, 255, 100), // Yellow
            4 => egui::Color32::from_rgb(255, 100, 255), // Magenta
            5 => egui::Color32::from_rgb(100, 255, 255), // Cyan
            _ => egui::Color32::WHITE,
        };
        
        ui.painter().rect_filled(entity_rect, 2.0, color);
        
        // Draw entity ID
        ui.painter().text(
            entity_rect.center(),
            egui::Align2::CENTER_CENTER,
            entity.id.to_string(),
            egui::FontId::monospace(10.0),
            egui::Color32::BLACK,
        );
    }
    
    /// Render entity center dot using egui
    fn render_entity_center_egui(
        &self,
        ui: &mut egui::Ui,
        entity: &toki_core::entity::Entity,
        camera_offset: glam::Vec2,
        scale: f32,
        viewport_rect: egui::Rect,
    ) {
        let entity_pos = entity.position;
        let screen_pos = self.world_to_screen(entity_pos.as_vec2(), camera_offset, scale, viewport_rect);
        
        ui.painter().circle_filled(
            egui::pos2(screen_pos.x, screen_pos.y),
            2.0,
            egui::Color32::YELLOW,
        );
    }
    
    /// Render collision box outline using egui
    fn render_collision_box_egui(
        &self,
        ui: &mut egui::Ui,
        entity: &toki_core::entity::Entity,
        camera_offset: glam::Vec2,
        scale: f32,
        viewport_rect: egui::Rect,
    ) {
        if let Some(collision_box) = &entity.collision_box {
            let entity_pos = entity.position;
            let screen_pos = self.world_to_screen(entity_pos.as_vec2(), camera_offset, scale, viewport_rect);
            let size = collision_box.size.as_vec2() * scale;
            
            let rect = egui::Rect::from_center_size(
                egui::pos2(screen_pos.x, screen_pos.y),
                egui::vec2(size.x, size.y),
            );
            
            // Draw collision box outline (simplified for now)
            ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgba_premultiplied(0, 255, 0, 50));
        }
    }
    
    /// Render selection highlight using egui
    fn render_selection_highlight_egui(
        &self,
        ui: &mut egui::Ui,
        entity: &toki_core::entity::Entity,
        camera_offset: glam::Vec2,
        scale: f32,
        viewport_rect: egui::Rect,
    ) {
        let entity_pos = entity.position;
        let screen_pos = self.world_to_screen(entity_pos.as_vec2(), camera_offset, scale, viewport_rect);
        
        let size = if let Some(collision_box) = &entity.collision_box {
            collision_box.size.as_vec2() * scale
        } else {
            glam::Vec2::new(16.0, 16.0) * scale
        };
        
        let rect = egui::Rect::from_center_size(
            egui::pos2(screen_pos.x, screen_pos.y),
            egui::vec2(size.x + 8.0, size.y + 8.0), // slightly larger
        );
        
        // Draw selection highlight (simplified for now)  
        ui.painter().rect_filled(rect, 2.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 80));
    }
    
    /// Convert world coordinates to screen coordinates
    fn world_to_screen(
        &self,
        world_pos: glam::Vec2,
        camera_offset: glam::Vec2,
        scale: f32,
        viewport_rect: egui::Rect,
    ) -> glam::Vec2 {
        let relative_pos = world_pos - camera_offset;
        let scaled_pos = relative_pos * scale;
        
        glam::Vec2::new(
            viewport_rect.center().x + scaled_pos.x,
            viewport_rect.center().y + scaled_pos.y,
        )
    }
    
    /// Select entity by ID
    pub fn select_entity(&mut self, entity_id: Option<u32>) {
        self.selected_entity_id = entity_id;
    }
    
    /// Get currently selected entity
    pub fn selected_entity(&self) -> Option<u32> {
        self.selected_entity_id
    }
    
    // Note: Toggle methods can be added back when UI controls for them are implemented
    
    /// Test if a screen position hits an entity (for selection)
    pub fn entity_at_position(
        &self,
        game_state: &GameState,
        camera: &Camera,
        screen_pos: glam::Vec2,
        viewport_rect: egui::Rect,
    ) -> Option<u32> {
        // Convert screen coordinates to world coordinates
        let camera_offset = glam::Vec2::new(camera.position.x as f32, camera.position.y as f32);
        let scale = camera.scale as f32;
        let world_pos = self.screen_to_world(screen_pos, camera_offset, scale, viewport_rect);
        
        // Get entity IDs and then get the actual entities, test in reverse order (top to bottom)
        let entity_ids = game_state.entity_manager().active_entities();
        for entity_id in entity_ids.iter().rev() {
            if let Some(entity) = game_state.entity_manager().get_entity(*entity_id) {
                let entity_pos = entity.position.as_vec2();
                
                let size = if let Some(collision_box) = &entity.collision_box {
                    collision_box.size.as_vec2()
                } else {
                    glam::Vec2::new(16.0, 16.0) // default size
                };
                
                // Check if click is within entity bounds
                if world_pos.x >= entity_pos.x - size.x / 2.0 && 
                   world_pos.x <= entity_pos.x + size.x / 2.0 &&
                   world_pos.y >= entity_pos.y - size.y / 2.0 && 
                   world_pos.y <= entity_pos.y + size.y / 2.0 {
                    return Some(entity.id);
                }
            }
        }
        None
    }
    
    /// Convert screen coordinates to world coordinates
    fn screen_to_world(
        &self,
        screen_pos: glam::Vec2,
        camera_offset: glam::Vec2,
        scale: f32,
        viewport_rect: egui::Rect,
    ) -> glam::Vec2 {
        let relative_screen_pos = glam::Vec2::new(
            screen_pos.x - viewport_rect.center().x,
            screen_pos.y - viewport_rect.center().y,
        );
        
        let scaled_pos = relative_screen_pos / scale;
        camera_offset + scaled_pos
    }
    
    /// Render tilemap using egui (improved with proper atlas support)
    fn render_tilemap_egui(
        &mut self,
        ui: &mut egui::Ui,
        tilemap: &TileMap,
        camera_offset: glam::Vec2,
        scale: f32,
        viewport_rect: egui::Rect,
        project_path: Option<&std::path::Path>,
    ) {
        // Load atlas data for proper tile mapping
        if let Err(e) = self.ensure_atlas_loaded(tilemap, project_path) {
            tracing::warn!("Failed to load atlas for tilemap: {}. Using fallback rendering.", e);
            self.render_tilemap_fallback(ui, tilemap, camera_offset, scale, viewport_rect);
            return;
        }
        
        // Get the cached atlas (we know it's loaded now)
        let atlas = self.loaded_atlas.as_ref().unwrap().1.clone();

        // For now, still use the simplified egui rendering, but with proper atlas-based colors
        let tile_size = glam::Vec2::new(
            tilemap.tile_size.x as f32 * scale,
            tilemap.tile_size.y as f32 * scale
        );
        
        // Calculate visible tile range based on camera and viewport
        let viewport_world_start = glam::Vec2::new(
            camera_offset.x - viewport_rect.width() / (2.0 * scale),
            camera_offset.y - viewport_rect.height() / (2.0 * scale),
        );
        let viewport_world_end = glam::Vec2::new(
            camera_offset.x + viewport_rect.width() / (2.0 * scale),
            camera_offset.y + viewport_rect.height() / (2.0 * scale),
        );
        
        let start_tile_x = (viewport_world_start.x / tilemap.tile_size.x as f32).floor().max(0.0) as u32;
        let start_tile_y = (viewport_world_start.y / tilemap.tile_size.y as f32).floor().max(0.0) as u32;
        let end_tile_x = (viewport_world_end.x / tilemap.tile_size.x as f32).ceil().min(tilemap.size.x as f32) as u32;
        let end_tile_y = (viewport_world_end.y / tilemap.tile_size.y as f32).ceil().min(tilemap.size.y as f32) as u32;
        
        tracing::debug!("Rendering tiles from ({}, {}) to ({}, {})", start_tile_x, start_tile_y, end_tile_x, end_tile_y);
        
        // Render visible tiles using atlas information
        for tile_y in start_tile_y..end_tile_y.min(tilemap.size.y) {
            for tile_x in start_tile_x..end_tile_x.min(tilemap.size.x) {
                if let Ok(tile_name) = tilemap.get_tile_name(tile_x, tile_y) {
                    let world_pos = glam::Vec2::new(
                        tile_x as f32 * tilemap.tile_size.x as f32,
                        tile_y as f32 * tilemap.tile_size.y as f32
                    );
                    
                    let screen_pos = self.world_to_screen(world_pos, camera_offset, scale, viewport_rect);
                    
                    let tile_rect = egui::Rect::from_min_size(
                        egui::pos2(screen_pos.x, screen_pos.y),
                        egui::vec2(tile_size.x, tile_size.y),
                    );
                    
                    // Get tile-specific color based on atlas position or properties
                    let color = Self::get_tile_color_from_atlas_static(tile_name, &atlas);
                    
                    // Render tile
                    ui.painter().rect_filled(tile_rect, 1.0, color);
                    
                    // Optionally draw tile name for debugging
                    if scale > 2.0 {
                        let font_size = (scale * 4.0).min(12.0);
                        ui.painter().text(
                            tile_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            tile_name,
                            egui::FontId::monospace(font_size),
                            egui::Color32::WHITE,
                        );
                    }
                }
            }
        }
    }

    /// Fallback tilemap rendering when atlas loading fails
    fn render_tilemap_fallback(
        &self,
        ui: &mut egui::Ui,
        tilemap: &TileMap,
        camera_offset: glam::Vec2,
        scale: f32,
        viewport_rect: egui::Rect,
    ) {
        // Use the old hash-based coloring as fallback
        let tile_size = glam::Vec2::new(
            tilemap.tile_size.x as f32 * scale,
            tilemap.tile_size.y as f32 * scale
        );
        
        for tile_y in 0..tilemap.size.y.min(50) { // Limit for performance
            for tile_x in 0..tilemap.size.x.min(50) {
                if let Ok(tile_name) = tilemap.get_tile_name(tile_x, tile_y) {
                    let world_pos = glam::Vec2::new(
                        tile_x as f32 * tilemap.tile_size.x as f32,
                        tile_y as f32 * tilemap.tile_size.y as f32
                    );
                    
                    let screen_pos = self.world_to_screen(world_pos, camera_offset, scale, viewport_rect);
                    let tile_rect = egui::Rect::from_min_size(
                        egui::pos2(screen_pos.x, screen_pos.y),
                        egui::vec2(tile_size.x, tile_size.y),
                    );
                    
                    // Hash-based color fallback
                    let hash = tile_name.chars().map(|c| c as u32).sum::<u32>();
                    let color = match hash % 6 {
                        0 => egui::Color32::from_rgb(160, 82, 45),
                        1 => egui::Color32::from_rgb(34, 139, 34),
                        2 => egui::Color32::from_rgb(70, 130, 180),
                        3 => egui::Color32::from_rgb(210, 180, 140),
                        4 => egui::Color32::from_rgb(128, 128, 128),
                        _ => egui::Color32::from_rgb(139, 69, 19),
                    };
                    
                    ui.painter().rect_filled(tile_rect, 1.0, color.gamma_multiply(0.7));
                }
            }
        }
    }

    /// Get tile color based on atlas information (static version)
    fn get_tile_color_from_atlas_static(tile_name: &str, atlas: &AtlasMeta) -> egui::Color32 {
        // Use atlas position to determine color, or tile properties
        if let Some(tile_rect) = atlas.get_tile_rect(tile_name) {
            // tile_rect format: [x, y, width, height]
            // Create distinct colors based on tile position in atlas
            let pos_hash = (tile_rect[0] * 7 + tile_rect[1] * 13) as u32;
            match pos_hash % 8 {
                0 => egui::Color32::from_rgb(34, 139, 34),   // Grass green
                1 => egui::Color32::from_rgb(139, 69, 19),   // Dirt brown
                2 => egui::Color32::from_rgb(128, 128, 128), // Stone gray
                3 => egui::Color32::from_rgb(30, 144, 255),  // Water blue  
                4 => egui::Color32::from_rgb(238, 203, 173), // Sand beige
                5 => egui::Color32::from_rgb(160, 82, 45),   // Wood brown
                6 => egui::Color32::from_rgb(205, 92, 92),   // Brick red
                _ => egui::Color32::from_rgb(75, 0, 130),    // Roof purple
            }
        } else {
            // Fallback for unknown tiles
            egui::Color32::from_rgb(255, 0, 255) // Magenta for missing tiles
        }
    }

    /// Load atlas for tilemap rendering (with caching)
    fn ensure_atlas_loaded(&mut self, tilemap: &TileMap, project_path: Option<&std::path::Path>) -> Result<()> {
        // Extract atlas filename from tilemap
        let atlas_filename = &tilemap.atlas;
        
        // Determine atlas path - try tilemaps directory first, then sprites directory
        let atlas_path = if let Some(project_path) = project_path {
            let tilemaps_path = project_path.join("assets").join("tilemaps").join(atlas_filename);
            if tilemaps_path.exists() {
                tilemaps_path
            } else {
                project_path.join("assets").join("sprites").join(atlas_filename)
            }
        } else {
            return Err(anyhow::anyhow!("No project path available for atlas loading"));
        };

        // Check if we already have this atlas loaded
        if let Some((cached_path, _)) = &self.loaded_atlas {
            if cached_path == &atlas_path {
                // Atlas already loaded
                return Ok(());
            }
        }

        // Load new atlas
        tracing::info!("Loading atlas from: {}", atlas_path.display());
        let atlas = AtlasMeta::load_from_file(&atlas_path)
            .map_err(|e| anyhow::anyhow!("Failed to load atlas '{}': {}", atlas_path.display(), e))?;
        
        // Cache the loaded atlas
        self.loaded_atlas = Some((atlas_path, atlas));
        Ok(())
    }
}