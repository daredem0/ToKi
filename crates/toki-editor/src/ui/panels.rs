use crate::scene::SceneViewport;
use crate::config::EditorConfig;

/// Handles panel rendering for the editor (viewport and log panels)
pub struct PanelSystem;

impl PanelSystem {
    /// Handle placement mode hover logic for preview updates
    fn handle_placement_hover(
        ui_state: &mut super::EditorUI,
        viewport: &mut SceneViewport,
        response: &egui::Response,
        rect: egui::Rect,
        config: Option<&crate::config::EditorConfig>,
    ) {
        if ui_state.is_in_placement_mode() {
            if let Some(hover_pos) = response.hover_pos() {
                // Use raw world position conversion for preview
                let world_pos = viewport.screen_to_world_pos_raw(hover_pos, rect);
                ui_state.placement_preview_position = Some(world_pos);

                // Check collision validity for visual feedback
                let is_valid = Self::check_placement_validity(ui_state, viewport, world_pos, config);
                ui_state.placement_preview_valid = Some(is_valid);
                viewport.mark_dirty();
            } else {
                ui_state.placement_preview_position = None;
                ui_state.placement_preview_valid = None;
                viewport.mark_dirty();
            }
        }
    }

    /// Handle placement click - creates entity at clicked position
    fn handle_placement_click(
        ui_state: &mut super::EditorUI,
        viewport: &mut SceneViewport,
        click_pos: egui::Pos2,
        rect: egui::Rect,
        config: Option<&crate::config::EditorConfig>,
    ) {
        tracing::info!("Placement click detected at screen pos: {:?}", click_pos);

        let Some(entity_def_name) = &ui_state.placement_entity_definition.clone() else {
            tracing::warn!("No entity definition for placement");
            return;
        };

        // Convert screen coordinates to world coordinates
        let world_pos = viewport.screen_to_world_pos(click_pos, rect);
        tracing::info!("Placing entity '{}' at world coordinates ({}, {}) [converted from screen ({}, {})]",
            entity_def_name, world_pos.x, world_pos.y, click_pos.x, click_pos.y);

        // Attempt to place the entity
        if Self::try_place_entity(ui_state, entity_def_name, world_pos, config, viewport) {
            // Only exit placement mode on successful placement
            ui_state.exit_placement_mode();
        }
        // If placement failed, stay in placement mode so user can try again
    }

    /// Try to place entity at given world position, returns true if successful
    fn try_place_entity(
        ui_state: &mut super::EditorUI,
        entity_def_name: &str,
        world_pos: glam::Vec2,
        config: Option<&crate::config::EditorConfig>,
        viewport: &SceneViewport,
    ) -> bool {
        let Some(config) = config else {
            tracing::error!("No config available for entity creation");
            ui_state.exit_placement_mode();
            return false;
        };

        let Some(project_path) = config.current_project_path() else {
            tracing::error!("No project path available for entity creation");
            ui_state.exit_placement_mode();
            return false;
        };

        let entity_file = project_path.join("entities").join(format!("{}.json", entity_def_name));
        if !entity_file.exists() {
            tracing::error!("Entity definition file not found: {:?}", entity_file);
            ui_state.exit_placement_mode();
            return false;
        }

        let content = match std::fs::read_to_string(&entity_file) {
            Ok(content) => content,
            Err(e) => {
                tracing::error!("Failed to read entity file '{}': {}", entity_def_name, e);
                ui_state.exit_placement_mode();
                return false;
            }
        };

        let entity_def = match serde_json::from_str::<toki_core::entity::EntityDefinition>(&content) {
            Ok(entity_def) => entity_def,
            Err(e) => {
                tracing::error!("Failed to parse entity definition '{}': {}", entity_def_name, e);
                ui_state.exit_placement_mode();
                return false;
            }
        };

        // Calculate center-based position
        let sprite_size = glam::UVec2::new(entity_def.rendering.size[0], entity_def.rendering.size[1]);
        let half_size = glam::Vec2::new(sprite_size.x as f32 / 2.0, sprite_size.y as f32 / 2.0);
        let centered_world_pos = world_pos - half_size;
        let world_pos_i32 = glam::IVec2::new(centered_world_pos.x as i32, centered_world_pos.y as i32);

        Self::create_entity_in_scene(ui_state, entity_def, entity_def_name, world_pos_i32, viewport)
    }

    /// Create entity in the active scene, returns true if successful
    fn create_entity_in_scene(
        ui_state: &mut super::EditorUI,
        entity_def: toki_core::entity::EntityDefinition,
        entity_def_name: &str,
        world_pos_i32: glam::IVec2,
        viewport: &SceneViewport,
    ) -> bool {
        let Some(active_scene_name) = &ui_state.active_scene else {
            tracing::error!("No active scene for entity placement");
            ui_state.exit_placement_mode();
            return false;
        };

        let Some(target_scene) = ui_state.scenes.iter_mut().find(|s| s.name == *active_scene_name) else {
            tracing::error!("Active scene '{}' not found", active_scene_name);
            ui_state.exit_placement_mode();
            return false;
        };

        // Generate new entity ID
        let new_id = target_scene.entities.iter()
            .map(|e| e.id)
            .max()
            .unwrap_or(0) + 1;

        // Create entity at position
        let entity = match entity_def.create_entity(world_pos_i32, new_id) {
            Ok(entity) => entity,
            Err(e) => {
                tracing::error!("Failed to create entity '{}': {}", entity_def_name, e);
                ui_state.exit_placement_mode();
                return false;
            }
        };

        // Check collision before placing the entity
        let can_place = if let Some(tilemap) = viewport.scene_manager().tilemap() {
            let terrain_atlas = viewport.scene_manager().resources().get_terrain_atlas();
            toki_core::collision::can_entity_move_to_position(&entity, world_pos_i32, tilemap, terrain_atlas)
        } else {
            true // No tilemap loaded, allow placement
        };

        if can_place {
            target_scene.entities.push(entity);
            tracing::info!("Successfully placed entity '{}' (ID: {}) in scene '{}' at world position ({}, {})",
                entity_def_name, new_id, active_scene_name, world_pos_i32.x, world_pos_i32.y);
            ui_state.scene_content_changed = true;
            true
        } else {
            tracing::warn!("Cannot place entity '{}' at position ({}, {}) - collision detected with solid terrain (staying in placement mode)",
                entity_def_name, world_pos_i32.x, world_pos_i32.y);
            false
        }
    }

    /// Handle entity selection click - starts drag operation
    fn handle_entity_selection_click(
        ui_state: &mut super::EditorUI,
        viewport: &mut SceneViewport,
        click_pos: egui::Pos2,
        rect: egui::Rect,
    ) {
        tracing::info!("Regular click detected at screen pos: {:?}", click_pos);
        let world_pos = viewport.screen_to_world_pos(click_pos, rect);

        if let Some(entity_id) = viewport.get_entity_at_world_pos(world_pos) {
            Self::start_entity_drag_operation(ui_state, viewport, entity_id);
        } else {
            tracing::info!("No entity clicked at world position ({:.1}, {:.1})", world_pos.x, world_pos.y);
        }
    }

    /// Start drag operation for selected entity
    fn start_entity_drag_operation(
        ui_state: &mut super::EditorUI,
        viewport: &SceneViewport,
        entity_id: toki_core::entity::EntityId,
    ) {
        tracing::info!("Entity {} clicked - starting drag operation", entity_id);

        // Get the entity to determine its type before removing it
        let Some(entity) = viewport.scene_manager().game_state().entity_manager().get_entity(entity_id) else {
            tracing::warn!("Could not find entity {} for drag operation", entity_id);
            return;
        };

        // Map entity type to definition name (simple mapping for now)
        let entity_def_name = match entity.entity_type {
            toki_core::entity::EntityType::Player => "player",
            toki_core::entity::EntityType::Npc => "slime", // Use slime for NPCs
            _ => "slime", // Default fallback
        };

        tracing::info!("Removing entity {} and entering placement mode with type: '{}'", entity_id, entity_def_name);

        // Remove the entity from the scene (this makes it a move/drag operation)
        Self::remove_entity_from_scene(ui_state, entity_id);

        // Enter placement mode to "place" the entity at a new location
        ui_state.enter_placement_mode(entity_def_name.to_string());
    }

    /// Remove entity from the active scene
    fn remove_entity_from_scene(
        ui_state: &mut super::EditorUI,
        entity_id: toki_core::entity::EntityId,
    ) {
        let Some(active_scene_name) = &ui_state.active_scene else {
            tracing::warn!("No active scene to remove entity from");
            return;
        };

        let Some(scene) = ui_state.scenes.iter_mut().find(|s| s.name == *active_scene_name) else {
            tracing::warn!("Active scene '{}' not found", active_scene_name);
            return;
        };

        scene.entities.retain(|e| e.id != entity_id);
        ui_state.scene_content_changed = true;
        tracing::info!("Removed entity {} from scene '{}'", entity_id, active_scene_name);
    }

    /// Check if placement is valid at given world position
    fn check_placement_validity(
        ui_state: &super::EditorUI,
        viewport: &mut SceneViewport,
        world_pos: glam::Vec2,
        config: Option<&crate::config::EditorConfig>,
    ) -> bool {
        let Some(entity_def_name) = &ui_state.placement_entity_definition else {
            return false;
        };

        let Some(config) = config else {
            return false;
        };

        let Some(project_path) = config.current_project_path() else {
            return false;
        };

        let entity_file = project_path.join("entities").join(format!("{}.json", entity_def_name));
        if !entity_file.exists() {
            return false;
        }

        let content = match std::fs::read_to_string(&entity_file) {
            Ok(content) => content,
            Err(_) => return false,
        };

        let entity_def = match serde_json::from_str::<toki_core::entity::EntityDefinition>(&content) {
            Ok(entity_def) => entity_def,
            Err(_) => return false,
        };

        // Calculate placement position (same logic as click)
        let sprite_size = glam::UVec2::new(entity_def.rendering.size[0], entity_def.rendering.size[1]);
        let half_size = glam::Vec2::new(sprite_size.x as f32 / 2.0, sprite_size.y as f32 / 2.0);
        let centered_world_pos = world_pos - half_size;
        let world_pos_i32 = glam::IVec2::new(centered_world_pos.x as i32, centered_world_pos.y as i32);

        // Get collision box and check directly
        let collision_box = entity_def.get_collision_box();
        if let Some(tilemap) = viewport.scene_manager().tilemap() {
            let terrain_atlas = viewport.scene_manager().resources().get_terrain_atlas();
            toki_core::collision::can_place_collision_box_at_position(
                collision_box.as_ref(),
                world_pos_i32,
                tilemap,
                terrain_atlas
            )
        } else {
            true // No tilemap, allow placement
        }
    }

    /// Handle camera drag interactions
    fn handle_camera_drag(viewport: &mut SceneViewport, response: &egui::Response, config: Option<&crate::config::EditorConfig>) {
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
    }

    /// Renders the main scene viewport panel in the center of the screen
    pub fn render_viewport(
        ui_state: &mut super::EditorUI,
        ctx: &egui::Context,
        scene_viewport: Option<&mut SceneViewport>,
        config: Option<&EditorConfig>,
        renderer: Option<&mut egui_wgpu::Renderer>,
    ) {
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
                    ui.allocate_exact_size(available_size, egui::Sense::click_and_drag().union(egui::Sense::hover()));

                // Handle camera panning with drag
                Self::handle_camera_drag(viewport, &response, config);

                // Handle placement mode hover logic
                Self::handle_placement_hover(ui_state, viewport, &response, rect, config);

                // Handle viewport clicks (entity placement or selection)
                if response.clicked() {
                    if let Some(click_pos) = response.hover_pos() {
                        
                        // Check if we're in placement mode
                        if ui_state.is_in_placement_mode() {
                            Self::handle_placement_click(ui_state, viewport, click_pos, rect, config);
                        } else {
                            // Normal entity selection - use new hit detection
                            Self::handle_entity_selection_click(ui_state, viewport, click_pos, rect);
                        }
                    }
                }

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

    /// Renders the log/console panel at the bottom of the screen
    pub fn render_log_panel(
        _ui_state: &mut super::EditorUI,
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
}