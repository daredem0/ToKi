use crate::scene::SceneViewport;
use toki_core::{entity::EntityId, Scene};

#[derive(Debug, Clone)]
pub enum Selection {
    Scene(String),
    Map(String, String), // (scene_name, map_name)
    Entity(EntityId),
    StandaloneMap(String), // Map selected from Maps panel (not in scene context)
    EntityDefinition(String), // Entity definition from palette
}


/// Manages the editor's UI state and rendering
pub struct EditorUI {
    // Scene management
    pub scenes: Vec<Scene>,
    pub selection: Option<Selection>,
    pub active_scene: Option<String>, // Name of currently active scene
    pub scene_content_changed: bool, // Flag to signal that scene content changed
    
    // Legacy entity selection (keep for backward compatibility)
    pub selected_entity_id: Option<EntityId>,

    // UI Panel visibility
    pub show_hierarchy: bool,
    pub show_inspector: bool,
    pub show_maps: bool,
    pub should_exit: bool,
    pub show_console: bool,
    pub create_test_entities: bool,

    // Project management flags
    pub new_project_requested: bool,
    pub open_project_requested: bool,
    pub browse_for_project_requested: bool,
    pub save_project_requested: bool,
    pub init_config_requested: bool,
    pub window_title: Option<String>,
    
    // Map loading request
    pub map_load_requested: Option<(String, String)>, // (scene_name, map_name)
    
    // Asset validation
    pub validate_assets_requested: bool,
}

impl EditorUI {
    pub fn new() -> Self {
        Self {
            // Scene management
            scenes: vec![Scene::new("Main Scene".to_string())], // Start with default scene
            selection: None,
            active_scene: Some("Main Scene".to_string()), // Default scene starts active
            scene_content_changed: false,
            
            // Legacy fields (keep for backward compatibility)
            selected_entity_id: None,

            // UI Panel visibility
            show_hierarchy: true,
            show_inspector: true,
            show_maps: true,
            should_exit: false,
            show_console: true,
            create_test_entities: false,

            // Project management flags
            new_project_requested: false,
            open_project_requested: false,
            browse_for_project_requested: false,
            save_project_requested: false,
            init_config_requested: false,
            window_title: Some("No project open".to_string()),
            
            // Map loading request
            map_load_requested: None,
            
            // Asset validation  
            validate_assets_requested: false,
        }
    }

    // Scene management methods
    pub fn add_scene(&mut self, name: String) -> &mut Scene {
        self.scenes.push(Scene::new(name));
        self.scenes.last_mut().unwrap()
    }

    pub fn get_scene(&self, name: &str) -> Option<&Scene> {
        self.scenes.iter().find(|s| s.name == name)
    }

    pub fn load_scenes_from_project(&mut self, loaded_scenes: Vec<Scene>) {
        tracing::info!("Loading {} scenes into UI hierarchy", loaded_scenes.len());
        self.scenes = loaded_scenes;
        
        // Set the first scene as active if we have scenes and no active scene is set
        if !self.scenes.is_empty() && self.active_scene.is_none() {
            self.active_scene = Some(self.scenes[0].name.clone());
            tracing::info!("Set '{}' as active scene", self.scenes[0].name);
        }
    }

    pub fn set_selection(&mut self, selection: Selection) {
        self.selection = Some(selection);
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Render the entire UI
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        scene_viewport: Option<&mut SceneViewport>,
        config: Option<&crate::config::EditorConfig>,
        log_capture: Option<&crate::logging::LogCapture>,
        renderer: Option<&mut egui_wgpu::Renderer>,
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
            self.render_hierarchy_and_maps_combined_panel(ctx, game_state, config);
        }

        if self.show_inspector {
            self.render_inspector_panel(ctx, game_state, config);
        }

        // Render viewport last (mutable access)
        self.render_viewport(ctx, scene_viewport, config, renderer);
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
                        } else if ui.button("Open Project...").clicked() {
                                tracing::info!(
                                    "Open Project... clicked - no project path in config"
                                );
                                self.browse_for_project_requested = true;
                            
                        }
                    } else if ui.button("Open Project...").clicked() {
                            tracing::info!("Open Project... clicked - no config available");
                            self.browse_for_project_requested = true;
                        
                    }

                    ui.separator();
                    if ui.button("Save Project").clicked() {
                        tracing::info!("Save Project clicked");
                        self.save_project_requested = true;
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

                ui.menu_button("Edit", |ui| {
                    if ui.button("Validate Project Assets").clicked() {
                        tracing::info!("Validate Project Assets clicked");
                        self.validate_assets_requested = true;
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_hierarchy, "Hierarchy");
                    ui.checkbox(&mut self.show_inspector, "Inspector");
                    ui.checkbox(&mut self.show_maps, "Maps");
                    ui.checkbox(&mut self.show_console, "Console");
                });
                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                        ui.label(self.window_title.as_ref().unwrap());
        });
            });
        });
    }

    pub fn set_title(&mut self, title: &str){
        self.window_title = Some(title.to_string());
    }

    fn render_viewport(&mut self, ctx: &egui::Context, scene_viewport: Option<&mut SceneViewport>, config: Option<&crate::config::EditorConfig>, renderer: Option<&mut egui_wgpu::Renderer>) {
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

                // Handle camera panning with drag
                if response.drag_started() {
                    if let Some(start_pos) = response.interact_pointer_pos() {
                        tracing::info!("Camera drag started at {:?}", start_pos);
                        let start_vec = glam::Vec2::new(start_pos.x, start_pos.y);
                        viewport.start_camera_drag(start_vec);
                    }
                } else if response.dragged() {
                    if let Some(drag_pos) = response.interact_pointer_pos() {
                        tracing::debug!("Camera dragging to {:?}", drag_pos);
                        let drag_vec = glam::Vec2::new(drag_pos.x, drag_pos.y);
                        let pan_speed = config.map(|c| c.editor_settings.camera.pan_speed).unwrap_or(1.0);
                        viewport.update_camera_drag(drag_vec, pan_speed);
                    }
                } else if response.drag_stopped() {
                    tracing::info!("Camera drag stopped");
                    viewport.stop_camera_drag();
                }
                
                // TODO: Entity click system - commented out for now
                // if response.clicked() {
                //     if let Some(click_pos) = response.interact_pointer_pos() {
                //         let screen_pos = glam::Vec2::new(click_pos.x, click_pos.y);
                //         if let Some(entity_id) = viewport.handle_click(screen_pos, rect) {
                //             self.selected_entity_id = Some(entity_id);
                //         } else {
                //             self.selected_entity_id = None;
                //         }
                //     }
                // }

                // Render the scene content
                let project_path = config.and_then(|c| c.current_project_path());
                viewport.render(ui, rect, project_path.map(|p| p.as_path()), renderer);
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

    fn render_hierarchy_and_maps_combined_panel(&mut self, ctx: &egui::Context, game_state: Option<&toki_core::GameState>, config: Option<&crate::config::EditorConfig>) {
        egui::SidePanel::left("hierarchy_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("📋 Scene Hierarchy");
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("hierarchy_scroll")
                    .show(ui, |ui| {
                    // Collect actions to perform after UI iteration
                    let mut map_removals: Vec<(usize, usize)> = Vec::new();
                    let mut entity_removals: Vec<(String, u32)> = Vec::new(); // (scene_name, entity_id)
                    let mut selection_changes: Vec<Selection> = Vec::new();
                    let mut active_scene_change: Option<String> = None;
                    
                    for (scene_index, scene) in self.scenes.iter().enumerate() {
                        let is_active_scene = self.active_scene.as_ref() == Some(&scene.name);
                        let scene_header_text = if is_active_scene {
                            format!("🎬 {} ⭐", scene.name) // Active scene gets a star
                        } else {
                            format!("🎬 {}", scene.name)
                        };
                        
                        let scene_header_response = ui.collapsing(&scene_header_text, |ui| {
                            
                            // Maps section within the scene - only show configured maps
                            if !scene.maps.is_empty() {
                                ui.label("Maps:");
                                ui.indent("scene_maps", |ui| {
                                    for (map_index, map_name) in scene.maps.iter().enumerate() {
                                        let is_selected = matches!(
                                            &self.selection,
                                            Some(Selection::Map(s, m)) if s == &scene.name && m == map_name
                                        );
                                        
                                        ui.horizontal(|ui| {
                                            let response = ui.selectable_label(is_selected, format!("🗺️ {}", map_name));
                                            if response.clicked() {
                                                selection_changes.push(Selection::Map(scene.name.clone(), map_name.clone()));
                                                tracing::info!("Selected map {} in scene {}", map_name, scene.name);
                                            }
                                            
                                            // Remove map button
                                            if ui.small_button("✕").clicked() {
                                                map_removals.push((scene_index, map_index));
                                                tracing::info!("Removed map {} from scene {}", map_name, scene.name);
                                            }
                                        });
                                    }
                                });
                                ui.add_space(5.0);
                            }
                            
                            // Scene entities section (design-time entities in scene definition)
                            if !scene.entities.is_empty() {
                                ui.label("Scene Entities:");
                                ui.indent("scene_design_entities", |ui| {
                                    for entity in &scene.entities {
                                        let is_selected = matches!(
                                            &self.selection,
                                            Some(Selection::Entity(id)) if id == &entity.id
                                        );
                                        
                                        ui.horizontal(|ui| {
                                            let entity_display = match entity.entity_type {
                                                toki_core::entity::EntityType::Player => format!("👤 Player (ID: {})", entity.id),
                                                toki_core::entity::EntityType::Npc => format!("🧙 NPC (ID: {})", entity.id),
                                                toki_core::entity::EntityType::Item => format!("📦 Item (ID: {})", entity.id),
                                                toki_core::entity::EntityType::Decoration => format!("🎨 Decoration (ID: {})", entity.id),
                                                toki_core::entity::EntityType::Trigger => format!("⚡ Trigger (ID: {})", entity.id),
                                            };
                                            
                                            let response = ui.selectable_label(is_selected, entity_display);
                                            
                                            if response.clicked() {
                                                selection_changes.push(Selection::Entity(entity.id));
                                                tracing::info!("Selected scene entity ID: {}", entity.id);
                                            }
                                            
                                            // Show entity position
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(format!("({}, {})", entity.position.x, entity.position.y));
                                            });
                                        });
                                        
                                        // Right-click context menu for entity actions
                                        ui.horizontal(|ui| {
                                            ui.add_space(20.0); // Indent for context options
                                            if ui.small_button("🗑️").on_hover_text("Remove from scene").clicked() {
                                                // Add to removal list - will be processed after UI rendering
                                                entity_removals.push((scene.name.clone(), entity.id));
                                            }
                                        });
                                    }
                                });
                            }
                            
                            // Runtime entities section (entities from game state)
                            ui.label("Runtime Entities:");
                            ui.indent("scene_runtime_entities", |ui| {
                                if let Some(game_state) = game_state {
                                    let entity_ids = game_state.entity_manager().active_entities();
                                    
                                    if entity_ids.is_empty() {
                                        ui.label("No runtime entities");
                                    } else {
                                        for entity_id in &entity_ids {
                                            if let Some(entity) = game_state.entity_manager().get_entity(*entity_id) {
                                                let is_selected = matches!(
                                                    &self.selection,
                                                    Some(Selection::Entity(id)) if id == entity_id
                                                );
                                                
                                                ui.horizontal(|ui| {
                                                    let response = ui.selectable_label(
                                                        is_selected,
                                                        format!("⚙️ Runtime Entity {}", entity_id)
                                                    );
                                                    
                                                    if response.clicked() {
                                                        selection_changes.push(Selection::Entity(*entity_id));
                                                        self.selected_entity_id = Some(*entity_id);
                                                    }
                                                    
                                                    // Show entity position
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        ui.label(format!("({}, {})", entity.position.x, entity.position.y));
                                                    });
                                                });
                                            }
                                        }
                                    }
                                } else {
                                    ui.label("No game state available");
                                }
                            });
                        });
                        
                        // Scene selection (clicking on header)
                        if scene_header_response.header_response.clicked() {
                            selection_changes.push(Selection::Scene(scene.name.clone()));
                            tracing::info!("Selected scene: {}", scene.name);
                        }

                        // Right-click context menu for scene
                        scene_header_response.header_response.context_menu(|ui| {
                            let is_active = self.active_scene.as_ref() == Some(&scene.name);
                            
                            ui.horizontal(|ui| {
                                if is_active {
                                    ui.label("✅ Active Scene");
                                } else if ui.button("🎯 Set as Active Scene").clicked() {
                                        active_scene_change = Some(scene.name.clone());
                                        tracing::info!("Setting {} as active scene", scene.name);
                                        ui.close();
                                    
                                }
                            });
                        });
                    }
                    
                    // Process removals in reverse order to maintain correct indices
                    map_removals.sort_by(|a, b| b.1.cmp(&a.1));
                    for (scene_index, map_index) in map_removals {
                        if let Some(scene) = self.scenes.get_mut(scene_index) {
                            if map_index < scene.maps.len() {
                                let removed_map = scene.maps.remove(map_index);
                                // Clear selection if it was the removed map
                                if matches!(&self.selection, Some(Selection::Map(s, m)) if s == &scene.name && m == &removed_map) {
                                    self.clear_selection();
                                }
                            }
                        }
                    }
                    
                    // Process entity removals
                    for (scene_name, entity_id) in entity_removals {
                        if let Some(scene) = self.scenes.iter_mut().find(|s| s.name == scene_name) {
                            if let Some(index) = scene.entities.iter().position(|e| e.id == entity_id) {
                                scene.entities.remove(index);
                                tracing::info!("Removed entity {} from scene {}", entity_id, scene_name);
                                
                                // Clear selection if it was the removed entity
                                if matches!(&self.selection, Some(Selection::Entity(id)) if id == &entity_id) {
                                    self.clear_selection();
                                }
                                
                                self.scene_content_changed = true;
                            }
                        }
                    }
                    
                    // Apply selection changes (only apply the last one)
                    if let Some(selection) = selection_changes.last() {
                        self.set_selection(selection.clone());
                    }
                    
                    // Apply active scene change
                    if let Some(new_active_scene) = active_scene_change {
                        self.active_scene = Some(new_active_scene);
                    }
                });

                ui.separator();
                
                // Add new scene button
                if ui.button("+ Add Scene").clicked() {
                    let new_scene_name = format!("Scene {}", self.scenes.len() + 1);
                    self.add_scene(new_scene_name.clone());
                    tracing::info!("Created new scene: {}", new_scene_name);
                }
                
                // Add Maps section if enabled
                if self.show_maps {
                    ui.add_space(10.0);
                    ui.heading("🗺️ Maps");
                    ui.separator();

                    if let Some(config) = config {
                        if let Some(project_path) = config.current_project_path() {
                            let tilemaps_path = project_path.join("assets").join("tilemaps");
                            
                            if tilemaps_path.exists() {
                                // Discover tilemap files
                                if let Ok(entries) = std::fs::read_dir(&tilemaps_path) {
                                    let mut found_maps = false;
                                    
                                    // Collect actions to perform after UI iteration
                                    let mut map_selections: Vec<String> = Vec::new();
                                    let mut scene_map_additions: Vec<(String, String)> = Vec::new(); // (scene_name, map_name)
                                    
                                    egui::ScrollArea::vertical()
                                        .id_salt("maps_scroll")
                                        .max_height(150.0) // Limit height so hierarchy doesn't get too tall
                                        .show(ui, |ui| {
                                            for entry in entries.flatten() {
                                                if let Some(name) = entry.file_name().to_str() {
                                                    if name.ends_with(".json") {
                                                        let map_name = name.trim_end_matches(".json").to_string();
                                                        found_maps = true;
                                                        
                                                        let is_selected = matches!(
                                                            &self.selection,
                                                            Some(Selection::StandaloneMap(name)) if name == &map_name
                                                        );
                                                        
                                                        let response = ui.selectable_label(is_selected, &map_name);
                                                        
                                                        if response.clicked() {
                                                            tracing::info!("Map selected: {}", map_name);
                                                            map_selections.push(map_name.clone());
                                                        }
                                                        
                                                        // Right-click context menu for "Add to Scene"
                                                        response.context_menu(|ui| {
                                                            ui.label("Add to Scene:");
                                                            ui.separator();
                                                            
                                                            // Show available scenes - create a copy to avoid borrowing issues
                                                            let scene_names: Vec<(String, bool)> = self.scenes.iter()
                                                                .map(|s| (s.name.clone(), s.maps.contains(&map_name)))
                                                                .collect();
                                                            
                                                            for (scene_name, already_added) in scene_names {
                                                                if !already_added {
                                                                    if ui.button(&scene_name).clicked() {
                                                                        scene_map_additions.push((scene_name.clone(), map_name.clone()));
                                                                        ui.close();
                                                                    }
                                                                } else {
                                                                    ui.add_enabled(false, egui::Button::new(format!("{} (already added)", scene_name)));
                                                                }
                                                            }
                                                            
                                                            if self.scenes.is_empty() {
                                                                ui.label("No scenes available");
                                                            }
                                                        });
                                                    }
                                                }
                                            }
                                        });
                                    
                                    // Apply collected actions
                                    for map_name in map_selections {
                                        self.set_selection(Selection::StandaloneMap(map_name));
                                    }
                                    
                                    for (scene_name, map_name) in scene_map_additions {
                                        if let Some(target_scene) = self.scenes.iter_mut().find(|s| s.name == scene_name) {
                                            target_scene.maps.push(map_name.clone());
                                            tracing::info!("Added map '{}' to scene '{}'", map_name, scene_name);
                                            // Signal that scene content changed by setting a flag
                                            self.scene_content_changed = true;
                                        }
                                    }
                                    
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
                
                // Add Entity Palette section
                ui.add_space(10.0);
                ui.heading("🧙 Entities");
                ui.separator();
                
                if let Some(config) = config {
                    if let Some(project_path) = config.current_project_path() {
                        let (selected_entity, entity_additions) = self.render_entity_palette(ui, project_path);
                        
                        // Handle entity selection
                        if let Some(selected_entity) = selected_entity {
                            self.set_selection(Selection::EntityDefinition(selected_entity));
                        }
                        
                        // Process entity additions to scenes
                        for (scene_name, entity_name) in entity_additions {
                            if let Some(target_scene) = self.scenes.iter_mut().find(|s| s.name == scene_name) {
                                // Try to load and create entity from definition
                                if let Some(project_path) = config.current_project_path() {
                                    let entity_file = project_path
                                        .join("entities")
                                        .join(format!("{}.json", entity_name));
                                    
                                    if entity_file.exists() {
                                        match std::fs::read_to_string(&entity_file) {
                                            Ok(content) => {
                                                match serde_json::from_str::<toki_core::entity::EntityDefinition>(&content) {
                                                    Ok(entity_def) => {
                                                        // Generate a new entity ID (simple increment from existing entities)
                                                        let new_id = target_scene.entities.iter()
                                                            .map(|e| e.id)
                                                            .max()
                                                            .unwrap_or(0) + 1;
                                                        
                                                        // Default position at (100, 100) - user can move it later
                                                        let default_position = glam::IVec2::new(100, 100);
                                                        
                                                        match entity_def.create_entity(default_position, new_id) {
                                                            Ok(entity) => {
                                                                target_scene.entities.push(entity);
                                                                tracing::info!("Successfully added entity '{}' (ID: {}) to scene '{}' at position ({}, {})", 
                                                                    entity_name, new_id, scene_name, default_position.x, default_position.y);
                                                                self.scene_content_changed = true;
                                                            }
                                                            Err(e) => {
                                                                tracing::error!("Failed to create entity '{}': {}", entity_name, e);
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        tracing::error!("Failed to parse entity definition '{}': {}", entity_name, e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                tracing::error!("Failed to read entity file '{}': {}", entity_name, e);
                                            }
                                        }
                                    } else {
                                        tracing::error!("Entity definition file not found: {:?}", entity_file);
                                    }
                                } else {
                                    tracing::error!("No project path available for entity creation");
                                }
                            }
                        }
                    } else {
                        ui.label("No project loaded for Entity palette");
                    }
                } else {
                    ui.label("No project configuration available for Entity palette");
                }
            });
    }

    fn render_entity_palette(&self, ui: &mut egui::Ui, project_path: &std::path::Path) -> (Option<String>, Vec<(String, String)>) {
        let entities_path = project_path.join("entities");
        
        if entities_path.exists() {
            // Try to read entity definition files
            if let Ok(entries) = std::fs::read_dir(&entities_path) {
                let mut found_entities = false;
                let mut categories: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
                
                // First pass: collect entities and group by category
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".json") {
                            let entity_name = name.trim_end_matches(".json").to_string();
                            found_entities = true;
                            
                            // Try to read the entity file to get its category
                            let entity_path = entry.path();
                            if let Ok(content) = std::fs::read_to_string(&entity_path) {
                                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&content) {
                                    let category = json_value
                                        .get("category")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("uncategorized")
                                        .to_string();
                                    
                                    categories.entry(category).or_default().push(entity_name);
                                } else {
                                    // If we can't parse JSON, put in uncategorized
                                    categories.entry("uncategorized".to_string()).or_default().push(entity_name);
                                }
                            } else {
                                // If we can't read file, put in uncategorized
                                categories.entry("uncategorized".to_string()).or_default().push(entity_name);
                            }
                        }
                    }
                }
                
                if found_entities {
                    let mut selected_entity = None;
                    let mut scene_entity_additions: Vec<(String, String)> = Vec::new(); // (scene_name, entity_name)
                    
                    egui::ScrollArea::vertical()
                        .id_salt("entities_scroll")
                        .max_height(150.0)
                        .show(ui, |ui| {
                            // Sort categories for consistent display
                            let mut sorted_categories: Vec<_> = categories.into_iter().collect();
                            sorted_categories.sort_by(|a, b| a.0.cmp(&b.0));
                            
                            for (category, mut entity_names) in sorted_categories {
                                // Show category header
                                ui.label(format!("📂 {}", category));
                                ui.indent(format!("category_{}", category), |ui| {
                                    // Sort entity names
                                    entity_names.sort();
                                    
                                    for entity_name in entity_names {
                                        // Show entity button with selection capability
                                        let is_selected = matches!(
                                            &self.selection,
                                            Some(Selection::EntityDefinition(name)) if name == &entity_name
                                        );
                                        
                                        let button = ui.selectable_label(is_selected, format!("🧙 {}", entity_name));
                                        
                                        if button.clicked() {
                                            tracing::info!("Entity '{}' selected", entity_name);
                                            selected_entity = Some(entity_name.clone());
                                        }
                                        
                                        // Right-click context menu for entity actions
                                        button.context_menu(|ui| {
                                            ui.label(format!("Entity: {}", entity_name));
                                            ui.separator();
                                            
                                            // Add to Scene section
                                            ui.label("Add to Scene:");
                                            ui.separator();
                                            
                                            // Show available scenes
                                            let scene_names: Vec<String> = self.scenes.iter()
                                                .map(|s| s.name.clone())
                                                .collect();
                                            
                                            for scene_name in scene_names {
                                                if ui.button(&scene_name).clicked() {
                                                    tracing::info!("Adding entity '{}' to scene '{}'", entity_name, scene_name);
                                                    scene_entity_additions.push((scene_name.clone(), entity_name.clone()));
                                                    ui.close();
                                                }
                                            }
                                            
                                            ui.separator();
                                            ui.label("Actions:");
                                            if ui.button("📖 View Definition").clicked() {
                                                tracing::info!("View definition for entity: {}", entity_name);
                                                ui.close();
                                            }
                                        });
                                    }
                                });
                                ui.add_space(5.0);
                            }
                        });
                    
                    return (selected_entity, scene_entity_additions);
                } else {
                    ui.label("No entity definition files found in entities/");
                }
            } else {
                ui.label("Could not read entities directory");
            }
        } else {
            ui.label("No entities directory found, expected: entities/");
        }
        
        (None, Vec::new())
    }

    fn render_inspector_panel(&mut self, ctx: &egui::Context, game_state: Option<&toki_core::GameState>, config: Option<&crate::config::EditorConfig>) {
        egui::SidePanel::right("inspector_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("🔍 Inspector");
                ui.separator();

                // Wrap all inspector content in a scrollable area
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        match &self.selection {
                            Some(Selection::Scene(scene_name)) => {
                                ui.heading(format!("🎬 {}", scene_name));
                                ui.separator();
                        
                        if let Some(scene) = self.get_scene(scene_name) {
                            ui.horizontal(|ui| {
                                ui.label("Maps:");
                                ui.label(format!("{}", scene.maps.len()));
                            });
                            
                            ui.horizontal(|ui| {
                                ui.label("Entities:");
                                ui.label(format!("{}", scene.entities.len()));
                            });
                            
                            ui.separator();
                            ui.label("Scene Actions:");
                            
                            if ui.button("🗺️ Add Map").clicked() {
                                tracing::info!("Add Map to scene: {}", scene_name);
                                // Maps are added via the hierarchy panel, this could open a dialog
                            }
                            
                            if ui.button("👤 Add Entity").clicked() {
                                tracing::info!("Add Entity to scene: {}", scene_name);
                                // TODO: Entity creation
                            }
                        }
                    },
                    
                    Some(Selection::Map(scene_name, map_name)) => {
                        ui.heading(format!("🗺️ {}", map_name));
                        ui.label(format!("Scene: {}", scene_name));
                        ui.separator();
                        
                        Self::render_map_details(ui, map_name, config, Some(scene_name), &mut self.map_load_requested);
                    },
                    
                    Some(Selection::StandaloneMap(map_name)) => {
                        ui.heading(format!("🗺️ {}", map_name));
                        ui.separator();
                        
                        Self::render_map_details(ui, map_name, config, None, &mut self.map_load_requested);
                    },
                    
                    Some(Selection::Entity(entity_id)) => {
                        if let Some(game_state) = game_state {
                            if let Some(entity) = game_state.entity_manager().get_entity(*entity_id) {
                                ui.heading(format!("👤 Entity {}", entity_id));
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
                                ui.label("❌ Entity not found");
                            }
                        } else {
                            ui.label("❌ No game state available");
                        }
                    },
                    
                    Some(Selection::EntityDefinition(entity_name)) => {
                        ui.heading(format!("🧙 {}", entity_name));
                        ui.separator();
                        
                        Self::render_entity_definition_details(ui, entity_name, config);
                    },
                    
                            None => {
                                ui.label("No selection");
                                ui.separator();
                                ui.label("Select a scene, map, entity, or entity definition from the hierarchy to see details.");
                            }
                        }
                    });
            });
    }

    fn render_map_details(ui: &mut egui::Ui, map_name: &str, config: Option<&crate::config::EditorConfig>, scene_name: Option<&str>, map_load_requested: &mut Option<(String, String)>) {
        // Try to load and show map details
        if let Some(config) = config {
            if let Some(project_path) = config.current_project_path() {
                let map_file = project_path
                    .join("assets")
                    .join("tilemaps")
                    .join(format!("{}.json", map_name));
                
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
                                        ui.label(format!("{}.json", map_name));
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
                                            .id_salt("map_properties_scroll")
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
                                                                        _ => "{ complex }".to_string(),
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
                                        // Handle load map click - use scene name if available, otherwise use a default
                                        let scene_for_loading = scene_name.unwrap_or("Main Scene").to_string();
                                        tracing::info!("Load Map button clicked for: {} in scene {}", map_name, scene_for_loading);
                                        *map_load_requested = Some((scene_for_loading, map_name.to_string()));
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

    fn render_entity_definition_details(ui: &mut egui::Ui, entity_name: &str, config: Option<&crate::config::EditorConfig>) {
        // Try to load and show entity definition details
        if let Some(config) = config {
            if let Some(project_path) = config.current_project_path() {
                let entity_file = project_path
                    .join("entities")
                    .join(format!("{}.json", entity_name));
                
                if entity_file.exists() {
                    // Try to read the entity definition file
                    match std::fs::read_to_string(&entity_file) {
                        Ok(content) => {
                            // Try to parse as JSON to show detailed info
                            match serde_json::from_str::<serde_json::Value>(&content) {
                                Ok(json) => {
                                    // Show file info
                                    ui.horizontal(|ui| {
                                        ui.label("File:");
                                        ui.label(format!("{}.json", entity_name));
                                    });
                                    
                                    if let Some(obj) = json.as_object() {
                                        // Show basic entity information
                                        if let Some(display_name) = obj.get("display_name").and_then(|v| v.as_str()) {
                                            ui.horizontal(|ui| {
                                                ui.label("Display Name:");
                                                ui.label(display_name);
                                            });
                                        }
                                        
                                        if let Some(description) = obj.get("description").and_then(|v| v.as_str()) {
                                            ui.horizontal(|ui| {
                                                ui.label("Description:");
                                                ui.label(description);
                                            });
                                        }
                                        
                                        if let Some(entity_type) = obj.get("entity_type").and_then(|v| v.as_str()) {
                                            ui.horizontal(|ui| {
                                                ui.label("Type:");
                                                ui.label(entity_type);
                                            });
                                        }
                                        
                                        if let Some(category) = obj.get("category").and_then(|v| v.as_str()) {
                                            ui.horizontal(|ui| {
                                                ui.label("Category:");
                                                ui.label(category);
                                            });
                                        }
                                        
                                        ui.separator();
                                        
                                        // Show attributes if present
                                        if let Some(attributes) = obj.get("attributes").and_then(|v| v.as_object()) {
                                            ui.heading("Attributes");
                                            
                                            if let Some(health) = attributes.get("health") {
                                                ui.horizontal(|ui| {
                                                    ui.label("Health:");
                                                    if health.is_null() {
                                                        ui.label("None");
                                                    } else if let Some(h) = health.as_u64() {
                                                        ui.label(h.to_string());
                                                    } else {
                                                        ui.label("Invalid");
                                                    }
                                                });
                                            }
                                            
                                            if let Some(speed) = attributes.get("speed").and_then(|v| v.as_u64()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Speed:");
                                                    ui.label(speed.to_string());
                                                });
                                            }
                                            
                                            if let Some(solid) = attributes.get("solid").and_then(|v| v.as_bool()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Solid:");
                                                    ui.label(solid.to_string());
                                                });
                                            }
                                            
                                            if let Some(active) = attributes.get("active").and_then(|v| v.as_bool()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Active:");
                                                    ui.label(active.to_string());
                                                });
                                            }
                                            
                                            if let Some(can_move) = attributes.get("can_move").and_then(|v| v.as_bool()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Can Move:");
                                                    ui.label(can_move.to_string());
                                                });
                                            }
                                            
                                            if let Some(has_inventory) = attributes.get("has_inventory").and_then(|v| v.as_bool()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Has Inventory:");
                                                    ui.label(has_inventory.to_string());
                                                });
                                            }
                                        }
                                        
                                        ui.separator();
                                        
                                        // Show collision info if present
                                        if let Some(collision) = obj.get("collision").and_then(|v| v.as_object()) {
                                            ui.heading("Collision");
                                            
                                            if let Some(enabled) = collision.get("enabled").and_then(|v| v.as_bool()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Enabled:");
                                                    ui.label(enabled.to_string());
                                                });
                                            }
                                            
                                            if let Some(offset) = collision.get("offset").and_then(|v| v.as_array()) {
                                                if offset.len() == 2 {
                                                    if let (Some(x), Some(y)) = (offset[0].as_i64(), offset[1].as_i64()) {
                                                        ui.horizontal(|ui| {
                                                            ui.label("Offset:");
                                                            ui.label(format!("({}, {})", x, y));
                                                        });
                                                    }
                                                }
                                            }
                                            
                                            if let Some(size) = collision.get("size").and_then(|v| v.as_array()) {
                                                if size.len() == 2 {
                                                    if let (Some(w), Some(h)) = (size[0].as_u64(), size[1].as_u64()) {
                                                        ui.horizontal(|ui| {
                                                            ui.label("Size:");
                                                            ui.label(format!("{}x{}", w, h));
                                                        });
                                                    }
                                                }
                                            }
                                            
                                            if let Some(trigger) = collision.get("trigger").and_then(|v| v.as_bool()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Trigger:");
                                                    ui.label(trigger.to_string());
                                                });
                                            }
                                            
                                            ui.separator();
                                        }
                                        
                                        // Show rendering info if present
                                        if let Some(rendering) = obj.get("rendering").and_then(|v| v.as_object()) {
                                            ui.heading("Rendering");
                                            
                                            if let Some(size) = rendering.get("size").and_then(|v| v.as_array()) {
                                                if size.len() == 2 {
                                                    if let (Some(w), Some(h)) = (size[0].as_u64(), size[1].as_u64()) {
                                                        ui.horizontal(|ui| {
                                                            ui.label("Size:");
                                                            ui.label(format!("{}x{}", w, h));
                                                        });
                                                    }
                                                }
                                            }
                                            
                                            if let Some(render_layer) = rendering.get("render_layer").and_then(|v| v.as_i64()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Render Layer:");
                                                    ui.label(render_layer.to_string());
                                                });
                                            }
                                        }
                                        
                                        // Show animation info if present
                                        if let Some(animations) = obj.get("animations").and_then(|v| v.as_object()) {
                                            ui.separator();
                                            ui.heading("Animations");
                                            
                                            if let Some(atlas_name) = animations.get("atlas_name").and_then(|v| v.as_str()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Atlas:");
                                                    ui.label(atlas_name);
                                                });
                                            }
                                            
                                            if let Some(default_state) = animations.get("default_state").and_then(|v| v.as_str()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Default State:");
                                                    ui.label(default_state);
                                                });
                                            }
                                            
                                            if let Some(clips) = animations.get("clips").and_then(|v| v.as_array()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Available Clips:");
                                                    ui.label(format!("{}", clips.len()));
                                                });
                                                
                                                ui.indent("animation_clips", |ui| {
                                                    for clip in clips.iter() {
                                                        if let Some(clip_obj) = clip.as_object() {
                                                            let state = clip_obj.get("state").and_then(|v| v.as_str()).unwrap_or("unknown");
                                                            let frame_count = clip_obj.get("frame_tiles").and_then(|v| v.as_array()).map(|arr| arr.len()).unwrap_or(0);
                                                            let duration = clip_obj.get("frame_duration_ms").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                                            let loop_mode = clip_obj.get("loop_mode").and_then(|v| v.as_str()).unwrap_or("unknown");
                                                            
                                                            ui.collapsing(format!("🎬 {} ({} frames)", state, frame_count), |ui| {
                                                                ui.horizontal(|ui| {
                                                                    ui.label("Frame Duration:");
                                                                    ui.label(format!("{} ms", duration));
                                                                });
                                                                ui.horizontal(|ui| {
                                                                    ui.label("Loop Mode:");
                                                                    ui.label(loop_mode);
                                                                });
                                                                
                                                                if let Some(frame_tiles) = clip_obj.get("frame_tiles").and_then(|v| v.as_array()) {
                                                                    ui.label("Frame Tiles:");
                                                                    ui.indent("frame_tiles", |ui| {
                                                                        for (j, tile) in frame_tiles.iter().enumerate() {
                                                                            if let Some(tile_name) = tile.as_str() {
                                                                                ui.label(format!("{}. {}", j + 1, tile_name));
                                                                            }
                                                                        }
                                                                    });
                                                                }
                                                            });
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                        
                                        // Show audio info if present
                                        if let Some(audio) = obj.get("audio").and_then(|v| v.as_object()) {
                                            ui.separator();
                                            ui.heading("Audio");
                                            
                                            if let Some(footstep_distance) = audio.get("footstep_trigger_distance").and_then(|v| v.as_f64()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Footstep Distance:");
                                                    ui.label(format!("{} pixels", footstep_distance));
                                                });
                                            }
                                            
                                            if let Some(movement_sound) = audio.get("movement_sound").and_then(|v| v.as_str()) {
                                                ui.horizontal(|ui| {
                                                    ui.label("Movement Sound:");
                                                    ui.label(movement_sound);
                                                });
                                            }
                                        }
                                        
                                        // Show tags if present
                                        if let Some(tags) = obj.get("tags").and_then(|v| v.as_array()) {
                                            ui.separator();
                                            ui.horizontal(|ui| {
                                                ui.label("Tags:");
                                                let tag_strings: Vec<String> = tags
                                                    .iter()
                                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                                    .collect();
                                                ui.label(tag_strings.join(", "));
                                            });
                                        }
                                        
                                        ui.separator();
                                        if ui.button("🎮 Place in Scene").clicked() {
                                            tracing::info!("Place entity '{}' button clicked", entity_name);
                                            // TODO: Implement entity placement functionality
                                        }
                                    }
                                }
                                Err(e) => {
                                    ui.label("❌ Invalid JSON file");
                                    ui.label(format!("Error: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            ui.label("❌ Could not read entity definition file");
                            ui.label(format!("Error: {}", e));
                        }
                    }
                } else {
                    ui.label("❌ Entity definition file not found");
                }
            }
        }
    }
}
