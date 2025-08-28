use toki_core::entity::EntityId;
use crate::scene::SceneViewport;
use crate::ui::panels;

/// Manages the editor's UI state and rendering
pub struct EditorUI {
    pub selected_entity_id: Option<EntityId>,
    pub show_hierarchy: bool,
    pub show_inspector: bool,
    pub should_exit: bool,
    pub create_test_entities: bool,
}

impl EditorUI {
    pub fn new() -> Self {
        Self {
            selected_entity_id: None,
            show_hierarchy: true,
            show_inspector: true,
            should_exit: false,
            create_test_entities: false,
        }
    }
    
    /// Render the entire UI
    pub fn render(&mut self, ctx: &egui::Context, scene_viewport: Option<&mut SceneViewport>) {
        self.render_top_menu(ctx);
        
        // Render hierarchy and inspector panels
        let game_state = scene_viewport.as_ref().map(|v| v.scene_manager().game_state());
        
        if self.show_hierarchy {
            panels::render_hierarchy(ctx, game_state, &mut self.selected_entity_id);
        }
        
        if self.show_inspector {
            panels::render_inspector(ctx, game_state, self.selected_entity_id);
        }
        
        // Render viewport last (mutable access)
        self.render_viewport(ctx, scene_viewport);
    }
    
    fn render_top_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project").clicked() {
                        tracing::info!("New Project clicked");
                    }
                    if ui.button("Open Project").clicked() {
                        tracing::info!("Open Project clicked");
                    }
                    ui.separator();
                    if ui.button("Create Test Entities").clicked() {
                        tracing::info!("Create Test Entities clicked");
                        self.create_test_entities = true;
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        tracing::info!("Exit clicked");
                        self.should_exit = true;
                    }
                });
                
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_hierarchy, "Hierarchy");
                    ui.checkbox(&mut self.show_inspector, "Inspector");
                });
            });
        });
    }
    
    fn render_viewport(&mut self, ctx: &egui::Context, scene_viewport: Option<&mut SceneViewport>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Scene Viewport");
            ui.separator();
            
            // Collect stats before updating viewport to avoid borrowing conflicts
            let (entity_count, selected_entity) = if let Some(ref viewport) = scene_viewport {
                let count = viewport.scene_manager().game_state().entity_manager().active_entities().len();
                let selected = viewport.selected_entity();
                (count, selected)
            } else {
                (0, None)
            };
            
            // Update and render the scene viewport
            if let Some(viewport) = scene_viewport {
                // Update the viewport systems
                if let Err(e) = viewport.update() {
                    tracing::error!("Scene viewport update error: {e}");
                }
                
                // Handle viewport interactions
                let available_size = ui.available_size();
                let (rect, response) = ui.allocate_exact_size(available_size, egui::Sense::click_and_drag());

                // Handle click events for entity selection
                if response.clicked() {
                    if let Some(click_pos) = response.interact_pointer_pos() {
                        let screen_pos = glam::Vec2::new(click_pos.x, click_pos.y);
                        if let Some(entity_id) = viewport.handle_click(screen_pos, rect) {
                            self.selected_entity_id = Some(entity_id);
                        } else {
                            self.selected_entity_id = None;
                        }
                    }
                }
                
                // Render the scene content
                viewport.render(ui, rect);
            } else {
                // Show placeholder when no viewport
                let available_size = ui.available_size();
                ui.allocate_response(available_size, egui::Sense::click())
                    .on_hover_text("Scene viewport not initialized");
            }
                
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.label("📊 Stats:");
                    ui.label(format!("Entities: {} | Selected: {:?}", entity_count, selected_entity));
                    ui.label("Press F1/F2 to toggle panels");
                });
            });
        });
    }
}