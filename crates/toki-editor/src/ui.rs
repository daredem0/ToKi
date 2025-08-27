use toki_core::{GameState, entity::EntityId};

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
    pub fn render(&mut self, ctx: &egui::Context, game_state: Option<&GameState>) {
        self.render_top_menu(ctx);
        self.render_hierarchy(ctx, game_state);
        self.render_inspector(ctx, game_state);
        self.render_viewport(ctx, game_state);
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
    
    fn render_hierarchy(&mut self, ctx: &egui::Context, game_state: Option<&GameState>) {
        if !self.show_hierarchy {
            return;
        }
        
        egui::SidePanel::left("hierarchy_panel")
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Hierarchy");
                ui.separator();
                
                if let Some(game_state) = game_state {
                    let entity_manager = game_state.entity_manager();
                    let active_entities = entity_manager.active_entities();
                    
                    for entity_id in active_entities {
                        if let Some(entity) = entity_manager.get_entity(entity_id) {
                            let entity_name = format!("{:?} (ID: {})", entity.entity_type, entity_id);
                            
                            let response = ui.selectable_label(
                                self.selected_entity_id == Some(entity_id),
                                entity_name
                            );
                            
                            if response.clicked() {
                                self.selected_entity_id = Some(entity_id);
                                tracing::info!("Selected entity: {:?}", entity_id);
                            }
                        }
                    }
                } else {
                    ui.label("No game state loaded");
                }
            });
    }
    
    fn render_inspector(&mut self, ctx: &egui::Context, game_state: Option<&GameState>) {
        if !self.show_inspector {
            return;
        }
        
        egui::SidePanel::right("inspector_panel")
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Inspector");
                ui.separator();
                
                if let Some(selected_id) = self.selected_entity_id {
                    if let Some(game_state) = game_state {
                        if let Some(entity) = game_state.entity_manager().get_entity(selected_id) {
                            ui.label(format!("Entity ID: {}", selected_id));
                            ui.label(format!("Type: {:?}", entity.entity_type));
                            ui.label(format!("Position: {:?}", entity.position));
                            ui.label(format!("Size: {:?}", entity.size));
                            ui.label(format!("Visible: {}", entity.attributes.visible));
                            
                            if let Some(collision_box) = &entity.collision_box {
                                ui.separator();
                                ui.label("Collision Box:");
                                ui.label(format!("  Offset: {:?}", collision_box.offset));
                                ui.label(format!("  Size: {:?}", collision_box.size));
                            }
                            
                            if entity.attributes.animation_controller.is_some() {
                                ui.separator();
                                ui.label("Has Animation Controller");
                            }
                        }
                    }
                } else {
                    ui.label("No entity selected");
                    ui.label("Click an entity in the Hierarchy to inspect it");
                }
            });
    }
    
    fn render_viewport(&mut self, ctx: &egui::Context, game_state: Option<&GameState>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Game Viewport");
            ui.separator();
            
            let available_size = ui.available_size();
            ui.allocate_response(available_size, egui::Sense::click())
                .on_hover_text("Game will render here");
                
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.label("📊 Stats:");
                    if let Some(game_state) = game_state {
                        ui.label(format!("Entities: {}", game_state.entity_manager().active_entities().len()));
                    }
                    ui.label("Press F1/F2 to toggle panels");
                });
            });
        });
    }
}