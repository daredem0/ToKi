use crate::scene::SceneViewport;
use std::path::PathBuf;
use toki_core::entity::EntityId;

/// Manages the editor's UI state and rendering
pub struct EditorUI {
    pub selected_entity_id: Option<EntityId>,
    pub show_hierarchy: bool,
    pub show_inspector: bool,
    pub show_maps: bool,
    pub should_exit: bool,
    pub show_console: bool,
    pub create_test_entities: bool,
    
    // Map selection state
    pub selected_map: Option<String>,

    // Project management flags
    pub new_project_requested: bool,
    pub open_project_requested: bool,
    pub browse_for_project_requested: bool,
    pub save_project_requested: bool,
    pub save_as_project_requested: bool,
    pub init_config_requested: bool,
    pub open_recent_project_requested: Option<PathBuf>,
    pub open_last_project_requested: bool,
}

impl EditorUI {
    pub fn new() -> Self {
        Self {
            selected_entity_id: None,
            show_hierarchy: true,
            show_inspector: true,
            show_maps: true,
            should_exit: false,
            show_console: true,
            create_test_entities: false,
            
            // Map selection state
            selected_map: None,

            // Project management flags
            new_project_requested: false,
            open_project_requested: false,
            browse_for_project_requested: false,
            save_project_requested: false,
            save_as_project_requested: false,
            init_config_requested: false,
            open_recent_project_requested: None,
            open_last_project_requested: false,
        }
    }

    /// Render the entire UI
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        scene_viewport: Option<&mut SceneViewport>,
        config: Option<&crate::config::EditorConfig>,
        log_capture: Option<&crate::logging::LogCapture>,
    ) {
        self.render_top_menu(ctx, config);

        // Render log panel first to claim full width at bottom
        if self.show_console {
            self.render_log_panel(ctx, log_capture);
        }

        // Render hierarchy and inspector panels
        let game_state = scene_viewport
            .as_ref()
            .map(|v| v.scene_manager().game_state());

        if self.show_hierarchy {
            self.render_hierarchy_and_maps_panel(ctx, game_state, config);
        }

        if self.show_inspector {
            self.render_inspector_panel(ctx, game_state, config);
        }

        // Render viewport last (mutable access)
        self.render_viewport(ctx, scene_viewport);
    }

    /// Apply config settings to UI state
    pub fn apply_config(&mut self, config: &crate::config::EditorConfig) {
        self.show_hierarchy = config.editor_settings.panels.hierarchy_visible;
        self.show_inspector = config.editor_settings.panels.inspector_visible;
        self.show_console = config.editor_settings.panels.console_visible;
    }

    fn render_top_menu(
        &mut self,
        ctx: &egui::Context,
        config: Option<&crate::config::EditorConfig>,
    ) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project...").clicked() {
                        tracing::info!("New Project clicked");
                        self.new_project_requested = true;
                    }
                    // Smart open project - try recent first
                    if let Some(config) = config {
                        if !config.recent_projects.is_empty() {
                            if ui.button("Open Last Project").clicked() {
                                tracing::info!("Open Last Project clicked");
                                self.open_last_project_requested = true;
                            }
                        }
                    }

                    // Auto-open the project from config
                    if let Some(config) = config {
                        if config.has_project_path() {
                            if ui.button("Open Project").clicked() {
                                tracing::info!(
                                    "Open Project clicked - opening project from config"
                                );
                                self.open_project_requested = true;
                            }
                            if ui.button("Browse for Project...").clicked() {
                                tracing::info!("Browse for Project clicked");
                                self.browse_for_project_requested = true;
                            }
                        } else {
                            if ui.button("Open Project...").clicked() {
                                tracing::info!(
                                    "Open Project... clicked - no project path in config"
                                );
                                self.browse_for_project_requested = true;
                            }
                        }
                    } else {
                        if ui.button("Open Project...").clicked() {
                            tracing::info!("Open Project... clicked - no config available");
                            self.browse_for_project_requested = true;
                        }
                    }

                    // Recent projects submenu
                    if let Some(config) = config {
                        if !config.recent_projects.is_empty() {
                            ui.menu_button("Open Recent", |ui| {
                                for project_path in &config.recent_projects {
                                    let project_name = project_path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("Unknown Project");

                                    if ui.button(project_name).clicked() {
                                        tracing::info!(
                                            "Opening recent project: {:?}",
                                            project_path
                                        );
                                        self.open_recent_project_requested =
                                            Some(project_path.clone());
                                    }
                                }
                            });
                        }
                    }

                    ui.separator();
                    if ui.button("Save Project").clicked() {
                        tracing::info!("Save Project clicked");
                        self.save_project_requested = true;
                    }
                    if ui.button("Save As...").clicked() {
                        tracing::info!("Save As clicked");
                        self.save_as_project_requested = true;
                    }
                    ui.separator();
                    if ui.button("Create Test Entities").clicked() {
                        tracing::info!("Create Test Entities clicked");
                        self.create_test_entities = true;
                    }
                    ui.separator();
                    if ui.button("Init Config").clicked() {
                        tracing::info!("Init Config clicked");
                        self.init_config_requested = true;
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
                    ui.checkbox(&mut self.show_maps, "Maps");
                    ui.checkbox(&mut self.show_console, "Console");
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
                let count = viewport
                    .scene_manager()
                    .game_state()
                    .entity_manager()
                    .active_entities()
                    .len();
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
                let (rect, response) =
                    ui.allocate_exact_size(available_size, egui::Sense::click_and_drag());

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
                    ui.label(format!(
                        "Entities: {} | Selected: {:?}",
                        entity_count, selected_entity
                    ));
                    ui.label("Press F1/F2 to toggle panels");
                });
            });
        });
    }

    fn render_log_panel(
        &mut self,
        ctx: &egui::Context,
        log_capture: Option<&crate::logging::LogCapture>,
    ) {
        egui::TopBottomPanel::bottom("log_panel")
            .resizable(true)
            .default_height(200.0)
            .show(ctx, |ui| {
                ui.heading("📝 Console");
                ui.separator();

                if let Some(capture) = log_capture {
                    let logs = capture.get_logs();
                    let scroll_area = egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true);

                    scroll_area.show(ui, |ui| {
                        for log_entry in &logs {
                            ui.horizontal(|ui| {
                                ui.label(&log_entry.timestamp);
                                ui.label(&log_entry.level);
                                ui.label(&log_entry.message);
                            });
                        }
                    });
                } else {
                    ui.label("Logs are being sent to terminal (check log_to_terminal config)");
                }
            });
    }

    fn render_hierarchy_and_maps_panel(&mut self, ctx: &egui::Context, game_state: Option<&toki_core::GameState>, config: Option<&crate::config::EditorConfig>) {
        egui::SidePanel::left("hierarchy_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                // Hierarchy section
                ui.heading("📋 Hierarchy");
                ui.separator();

                if let Some(game_state) = game_state {
                    let entity_ids = game_state.entity_manager().active_entities();
                    
                    if entity_ids.is_empty() {
                        ui.label("No entities in scene");
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(200.0)  // Limit height to make room for Maps section
                            .show(ui, |ui| {
                                for entity_id in &entity_ids {
                                    if let Some(entity) = game_state.entity_manager().get_entity(*entity_id) {
                                        let is_selected = self.selected_entity_id == Some(*entity_id);
                                        
                                        ui.horizontal(|ui| {
                                            let response = ui.selectable_label(
                                                is_selected,
                                                format!("Entity {}", entity_id)
                                            );
                                            
                                            if response.clicked() {
                                                self.selected_entity_id = Some(*entity_id);
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

                // Maps section (only show if Maps toggle is enabled)
                if self.show_maps {
                    ui.add_space(10.0);  // Add some spacing
                    ui.heading("🗺️ Maps");
                    ui.separator();

                    if let Some(config) = config {
                        if let Some(project_path) = config.current_project_path() {
                            let tilemaps_path = project_path.join("assets").join("tilemaps");
                            
                            if tilemaps_path.exists() {
                                // Discover tilemap files
                                if let Ok(entries) = std::fs::read_dir(&tilemaps_path) {
                                    let mut found_maps = false;
                                    
                                    egui::ScrollArea::vertical()
                                        .max_height(150.0)  // Limit height for Maps section
                                        .show(ui, |ui| {
                                            for entry in entries.flatten() {
                                                if let Some(name) = entry.file_name().to_str() {
                                                    if name.ends_with(".json") {
                                                        let map_name = name.trim_end_matches(".json").to_string();
                                                        found_maps = true;
                                                        
                                                        let is_selected = self.selected_map.as_ref() == Some(&map_name);
                                                        
                                                        if ui.selectable_label(is_selected, &map_name).clicked() {
                                                            tracing::info!("Map selected: {}", map_name);
                                                            self.selected_map = Some(map_name);
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                    
                                    if !found_maps {
                                        tracing::info!("No tilemap (.json) files found in assets/tilemaps/");
                                    }
                                } else {
                                    tracing::warn!("Could not read tilemaps directory");
                                }
                            } else {
                                tracing::info!("No tilemaps directory found, expected: assets/tilemaps/");
                            }
                        } else {
                            tracing::info!("No project loaded for Maps panel");
                        }
                    } else {
                        tracing::warn!("No project configuration available for Maps panel");
                    }
                }
            });
    }

    fn render_inspector_panel(&mut self, ctx: &egui::Context, game_state: Option<&toki_core::GameState>, config: Option<&crate::config::EditorConfig>) {
        egui::SidePanel::right("inspector_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("🔍 Inspector");
                ui.separator();

                // Show map details if a map is selected
                if let Some(ref selected_map) = self.selected_map {
                    ui.label(format!("Map: {}", selected_map));
                    ui.separator();
                    
                    // Try to load and show map details
                    if let Some(config) = config {
                        if let Some(project_path) = config.current_project_path() {
                            let map_file = project_path
                                .join("assets")
                                .join("tilemaps")
                                .join(format!("{}.json", selected_map));
                            
                            if map_file.exists() {
                                // Try to read the tilemap file
                                match std::fs::read_to_string(&map_file) {
                                    Ok(content) => {
                                        // Try to parse as JSON to show basic info
                                        match serde_json::from_str::<serde_json::Value>(&content) {
                                            Ok(json) => {
                                                // Show file info
                                                ui.horizontal(|ui| {
                                                    ui.label("File:");
                                                    ui.label(format!("{}.json", selected_map));
                                                });
                                                
                                                // Show file size
                                                ui.horizontal(|ui| {
                                                    ui.label("Size:");
                                                    ui.label(format!("{} bytes", content.len()));
                                                });
                                                
                                                // Show JSON properties and values
                                                if let Some(obj) = json.as_object() {
                                                    ui.horizontal(|ui| {
                                                        ui.label("Properties:");
                                                        ui.label(format!("{}", obj.keys().count()));
                                                    });
                                                    
                                                    ui.separator();
                                                    ui.label("Map Properties:");
                                                    
                                                    egui::ScrollArea::vertical()
                                                        .max_height(200.0)
                                                        .show(ui, |ui| {
                                                            for (key, value) in obj {
                                                                ui.horizontal(|ui| {
                                                                    ui.label(format!("{}:", key));
                                                                    
                                                                    // Format value based on type
                                                                    let value_str = match value {
                                                                        serde_json::Value::String(s) => format!("\"{}\"", s),
                                                                        serde_json::Value::Number(n) => n.to_string(),
                                                                        serde_json::Value::Bool(b) => b.to_string(),
                                                                        serde_json::Value::Array(arr) => {
                                                                            if arr.len() <= 5 {
                                                                                // Show actual values for small arrays
                                                                                let items: Vec<String> = arr.iter().map(|v| match v {
                                                                                    serde_json::Value::String(s) => format!("\"{}\"", s),
                                                                                    serde_json::Value::Number(n) => n.to_string(),
                                                                                    serde_json::Value::Bool(b) => b.to_string(),
                                                                                    serde_json::Value::Null => "null".to_string(),
                                                                                    _ => format!("{{ complex }}"),
                                                                                }).collect();
                                                                                format!("[{}]", items.join(", "))
                                                                            } else {
                                                                                // Show count for large arrays
                                                                                format!("[{} items]", arr.len())
                                                                            }
                                                                        },
                                                                        serde_json::Value::Object(obj) => format!("{{ {} properties }}", obj.len()),
                                                                        serde_json::Value::Null => "null".to_string(),
                                                                    };
                                                                    
                                                                    ui.label(value_str);
                                                                });
                                                            }
                                                        });
                                                }
                                                
                                                ui.separator();
                                                if ui.button("🎮 Load Map").clicked() {
                                                    tracing::info!("Load Map button clicked for: {}", selected_map);
                                                    // TODO: Implement map loading
                                                }
                                            }
                                            Err(e) => {
                                                ui.label("❌ Invalid JSON file");
                                                ui.label(format!("Error: {}", e));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        ui.label("❌ Could not read map file");
                                        ui.label(format!("Error: {}", e));
                                    }
                                }
                            } else {
                                ui.label("❌ Map file not found");
                            }
                        }
                    }
                }
                // Show entity details if an entity is selected and no map is selected
                else if let (Some(game_state), Some(entity_id)) = (game_state, self.selected_entity_id) {
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
                    ui.label("No selection");
                    ui.separator();
                    ui.label("Select an entity from the hierarchy or a map to see details.");
                }
            });
    }
}
