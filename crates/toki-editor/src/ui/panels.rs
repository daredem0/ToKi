use crate::scene::SceneViewport;
use crate::config::EditorConfig;

/// Handles panel rendering for the editor (viewport and log panels)
pub struct PanelSystem;

impl PanelSystem {
    /// Renders the main scene viewport panel in the center of the screen
    pub fn render_viewport(
        _ui_state: &mut super::EditorUI,
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
                //             ui_state.selected_entity_id = Some(entity_id);
                //         } else {
                //             ui_state.selected_entity_id = None;
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