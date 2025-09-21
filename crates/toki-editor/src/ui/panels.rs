use crate::scene::SceneViewport;
use crate::config::EditorConfig;

/// Handles panel rendering for the editor (viewport and log panels)
pub struct PanelSystem;

impl PanelSystem {
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

                // Track mouse position for placement preview
                if ui_state.is_in_placement_mode() {
                    if let Some(hover_pos) = response.hover_pos() {
                        // Use raw world position conversion for preview (without placement offsets)
                        let world_pos = viewport.screen_to_world_pos_raw(hover_pos, rect);
                        ui_state.placement_preview_position = Some(world_pos);

                        // Check collision validity for visual feedback
                        let is_valid = if let Some(entity_def_name) = &ui_state.placement_entity_definition {
                            // Load entity definition and check collision
                            if let Some(config) = config {
                                if let Some(project_path) = config.current_project_path() {
                                    let entity_file = project_path.join("entities").join(format!("{}.json", entity_def_name));
                                    if entity_file.exists() {
                                        match std::fs::read_to_string(&entity_file) {
                                            Ok(content) => {
                                                match serde_json::from_str::<toki_core::entity::EntityDefinition>(&content) {
                                                    Ok(entity_def) => {
                                                        // Calculate placement position (same logic as click)
                                                        let sprite_size = glam::UVec2::new(entity_def.rendering.size[0], entity_def.rendering.size[1]);
                                                        let half_size = glam::Vec2::new(sprite_size.x as f32 / 2.0, sprite_size.y as f32 / 2.0);
                                                        let centered_world_pos = world_pos - half_size;
                                                        let world_pos_i32 = glam::IVec2::new(centered_world_pos.x as i32, centered_world_pos.y as i32);

                                                        // Get collision box and check directly (no entity creation needed!)
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
                                                    Err(_) => false, // Entity definition parsing failed
                                                }
                                            }
                                            Err(_) => false, // File read failed
                                        }
                                    } else {
                                        false // Entity file doesn't exist
                                    }
                                } else {
                                    false // No project path
                                }
                            } else {
                                false // No config
                            }
                        } else {
                            false // No entity definition
                        };

                        ui_state.placement_preview_valid = Some(is_valid);
                        // Mark viewport as needing re-render to show preview
                        viewport.mark_dirty();
                    } else {
                        ui_state.placement_preview_position = None;
                        ui_state.placement_preview_valid = None;
                        // Mark viewport as needing re-render to hide preview
                        viewport.mark_dirty();
                    }
                }

                // Handle viewport clicks (entity placement or selection)
                if response.clicked() {
                    if let Some(click_pos) = response.hover_pos() {
                        
                        // Check if we're in placement mode
                        if ui_state.is_in_placement_mode() {
                            tracing::info!("Placement click detected at screen pos: {:?}", click_pos);
                            if let Some(entity_def_name) = &ui_state.placement_entity_definition {
                                // Convert screen coordinates to world coordinates
                                tracing::debug!("Click at screen pos: {:?}, rect: {:?}", click_pos, rect);
                                tracing::debug!("Rect dimensions: width={:.1}, height={:.1}", rect.width(), rect.height());
                                let world_pos = viewport.screen_to_world_pos(click_pos, rect);

                                tracing::info!("Placing entity '{}' at world coordinates ({}, {}) [converted from screen ({}, {})]",
                                    entity_def_name, world_pos.x, world_pos.y, click_pos.x, click_pos.y);
                                
                                // Create entity from definition
                                if let Some(config) = config {
                                    if let Some(project_path) = config.current_project_path() {
                                        let entity_file = project_path
                                            .join("entities")
                                            .join(format!("{}.json", entity_def_name));

                                        if entity_file.exists() {
                                            match std::fs::read_to_string(&entity_file) {
                                                Ok(content) => {
                                                    match serde_json::from_str::<toki_core::entity::EntityDefinition>(&content) {
                                                        Ok(entity_def) => {
                                                            // Calculate center-based position by offsetting with half the sprite size
                                                            let sprite_size = glam::UVec2::new(entity_def.rendering.size[0], entity_def.rendering.size[1]);
                                                            let half_size = glam::Vec2::new(sprite_size.x as f32 / 2.0, sprite_size.y as f32 / 2.0);
                                                            let centered_world_pos = world_pos - half_size;
                                                            let world_pos_i32 = glam::IVec2::new(centered_world_pos.x as i32, centered_world_pos.y as i32);

                                                            tracing::debug!("Sprite size: {}x{}, offset: ({:.1}, {:.1}), final position: ({}, {})",
                                                                sprite_size.x, sprite_size.y, half_size.x, half_size.y, world_pos_i32.x, world_pos_i32.y);

                                                            // Find active scene
                                                            if let Some(active_scene_name) = &ui_state.active_scene {
                                                                if let Some(target_scene) = ui_state.scenes.iter_mut().find(|s| s.name == *active_scene_name) {
                                                                    // Generate new entity ID
                                                                    let new_id = target_scene.entities.iter()
                                                                        .map(|e| e.id)
                                                                        .max()
                                                                        .unwrap_or(0) + 1;

                                                                    // Create entity at corrected position (center-based)
                                                                    match entity_def.create_entity(world_pos_i32, new_id) {
                                                                        Ok(entity) => {
                                                                            // Check collision before placing the entity
                                                                            let can_place = if let Some(tilemap) = viewport.scene_manager().tilemap() {
                                                                                let terrain_atlas = viewport.scene_manager().resources().get_terrain_atlas();
                                                                                toki_core::collision::can_entity_move_to_position(&entity, world_pos_i32, tilemap, terrain_atlas)
                                                                            } else {
                                                                                // No tilemap loaded, allow placement
                                                                                true
                                                                            };

                                                                            if can_place {
                                                                                target_scene.entities.push(entity);
                                                                                tracing::info!("Successfully placed entity '{}' (ID: {}) in scene '{}' at world position ({}, {})",
                                                                                    entity_def_name, new_id, active_scene_name, world_pos_i32.x, world_pos_i32.y);
                                                                                ui_state.scene_content_changed = true;
                                                                            } else {
                                                                                tracing::warn!("Cannot place entity '{}' at position ({}, {}) - collision detected with solid terrain",
                                                                                    entity_def_name, world_pos_i32.x, world_pos_i32.y);
                                                                            }
                                                                        }
                                                                        Err(e) => {
                                                                            tracing::error!("Failed to create entity '{}': {}", entity_def_name, e);
                                                                        }
                                                                    }
                                                                } else {
                                                                    tracing::error!("Active scene '{}' not found", active_scene_name);
                                                                }
                                                            } else {
                                                                tracing::error!("No active scene for entity placement");
                                                            }
                                                        }
                                                        Err(e) => {
                                                            tracing::error!("Failed to parse entity definition '{}': {}", entity_def_name, e);
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!("Failed to read entity file '{}': {}", entity_def_name, e);
                                                }
                                            }
                                        } else {
                                            tracing::error!("Entity definition file not found: {:?}", entity_file);
                                        }
                                    } else {
                                        tracing::error!("No project path available for entity creation");
                                    }
                                } else {
                                    tracing::error!("No config available for entity creation");
                                }
                                
                                // Always exit placement mode after attempting placement
                                ui_state.exit_placement_mode();
                            }
                        } else {
                            // Normal entity selection (original TODO logic)
                            tracing::info!("Regular click detected at screen pos: {:?}", click_pos);
                            // TODO: Implement entity selection when viewport.handle_click exists
                            // if let Some(entity_id) = viewport.handle_click(screen_pos, rect) {
                            //     ui_state.selected_entity_id = Some(entity_id);
                            // } else {
                            //     ui_state.selected_entity_id = None;
                            // }
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