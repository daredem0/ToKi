use toki_core::{GameState, entity::EntityId};

/// Renders the hierarchy panel showing all entities
pub fn render_hierarchy(ctx: &egui::Context, game_state: Option<&GameState>, selected_entity: &mut Option<EntityId>) {
    egui::SidePanel::left("hierarchy_panel")
        .resizable(true)
        .default_width(250.0)
        .show(ctx, |ui| {
            ui.heading("📋 Hierarchy");
            ui.separator();

            if let Some(game_state) = game_state {
                let entity_ids = game_state.entity_manager().active_entities();
                
                if entity_ids.is_empty() {
                    ui.label("No entities in scene");
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for entity_id in &entity_ids {
                            if let Some(entity) = game_state.entity_manager().get_entity(*entity_id) {
                                let is_selected = *selected_entity == Some(*entity_id);
                                
                                ui.horizontal(|ui| {
                                    let response = ui.selectable_label(
                                        is_selected,
                                        format!("Entity {}", entity_id)
                                    );
                                    
                                    if response.clicked() {
                                        *selected_entity = Some(*entity_id);
                                    }
                                    
                                    // Show entity type or position as subtitle
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(format!("({}, {})", entity.position.x, entity.position.y));
                                    });
                                });
                            }
                        }
                    });
                }
            } else {
                ui.label("No scene loaded");
            }
        });
}

/// Renders the inspector panel showing entity properties
pub fn render_inspector(ctx: &egui::Context, game_state: Option<&GameState>, selected_entity: Option<EntityId>) {
    egui::SidePanel::right("inspector_panel")
        .resizable(true)
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.heading("🔍 Inspector");
            ui.separator();

            if let (Some(game_state), Some(entity_id)) = (game_state, selected_entity) {
                if let Some(entity) = game_state.entity_manager().get_entity(entity_id) {
                    ui.label(format!("Entity ID: {}", entity_id));
                    ui.separator();
                    
                    // Position
                    ui.horizontal(|ui| {
                        ui.label("Position:");
                        ui.label(format!("({}, {})", entity.position.x, entity.position.y));
                    });
                    
                    // Size
                    ui.horizontal(|ui| {
                        ui.label("Size:");
                        ui.label(format!("({}, {})", entity.size.x, entity.size.y));
                    });
                    
                    // Entity type
                    ui.horizontal(|ui| {
                        ui.label("Type:");
                        ui.label(format!("{:?}", entity.entity_type));
                    });
                    
                    ui.separator();
                    
                    // Attributes
                    ui.heading("Attributes");
                    if let Some(health) = entity.attributes.health {
                        ui.horizontal(|ui| {
                            ui.label("Health:");
                            ui.label(health.to_string());
                        });
                    }
                    
                    ui.horizontal(|ui| {
                        ui.label("Speed:");
                        ui.label(entity.attributes.speed.to_string());
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Solid:");
                        ui.label(entity.attributes.solid.to_string());
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Visible:");
                        ui.label(entity.attributes.visible.to_string());
                    });
                    
                    // Collision box
                    if let Some(collision_box) = &entity.collision_box {
                        ui.separator();
                        ui.heading("Collision Box");
                        ui.horizontal(|ui| {
                            ui.label("Offset:");
                            ui.label(format!("({}, {})", collision_box.offset.x, collision_box.offset.y));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Size:");
                            ui.label(format!("({}, {})", collision_box.size.x, collision_box.size.y));
                        });
                    }
                } else {
                    ui.label("Entity not found");
                }
            } else {
                ui.label("No entity selected");
                ui.separator();
                ui.label("Select an entity from the hierarchy or viewport to see its properties.");
            }
        });
}