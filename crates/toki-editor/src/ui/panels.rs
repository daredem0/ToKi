use super::editor_ui::CenterPanelTab;
use super::interactions::{CameraInteraction, PlacementInteraction, SelectionInteraction};
use super::rule_graph::{RuleGraph, RuleGraphNodeKind};
use crate::config::EditorConfig;
use crate::scene::SceneViewport;
use std::collections::HashMap;

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
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ui_state.center_panel_tab,
                    CenterPanelTab::SceneViewport,
                    "Scene Viewport",
                );
                ui.selectable_value(
                    &mut ui_state.center_panel_tab,
                    CenterPanelTab::SceneGraph,
                    "Scene Graph",
                );
            });
            ui.separator();

            if ui_state.center_panel_tab == CenterPanelTab::SceneGraph {
                Self::render_scene_graph(ui, ui_state);
                return;
            }

            // Update and render the scene viewport
            if let Some(viewport) = scene_viewport {
                // Collect stats before updating viewport to avoid borrowing conflicts
                let entity_count = viewport
                    .scene_manager()
                    .game_state()
                    .entity_manager()
                    .active_entities()
                    .len();
                let selected_entity = viewport.selected_entity();

                // Update the viewport systems
                if let Err(e) = viewport.update() {
                    tracing::error!("Scene viewport update error: {e}");
                }

                // Handle viewport interactions
                let available_size = ui.available_size();
                let (rect, response) = ui.allocate_exact_size(
                    available_size,
                    egui::Sense::click_and_drag().union(egui::Sense::hover()),
                );

                // Safety reset: don't keep entities hidden when no move drag is active.
                if !ui_state.is_entity_move_drag_active() {
                    viewport.clear_suppressed_entity_rendering();
                }

                // Start entity move drag if dragging began over an entity.
                if response.drag_started() {
                    if let Some(drag_start_pos) = response.interact_pointer_pos() {
                        SelectionInteraction::handle_drag_start(
                            ui_state,
                            viewport,
                            drag_start_pos,
                            rect,
                            config,
                        );
                    }
                }

                // Handle drag release for entity move operations.
                if response.drag_stopped() {
                    let drop_pos = response
                        .interact_pointer_pos()
                        .or_else(|| response.hover_pos());
                    SelectionInteraction::handle_drag_release(ui_state, viewport, drop_pos, rect);
                }

                // Handle camera panning with drag (disabled while moving an entity).
                if !ui_state.is_entity_move_drag_active() {
                    CameraInteraction::handle_drag(viewport, &response, config);
                } else {
                    viewport.stop_camera_drag();
                }

                // Handle placement mode hover logic
                PlacementInteraction::handle_hover(ui_state, viewport, &response, rect, config);

                // Handle viewport clicks (entity placement or selection)
                if response.clicked() {
                    if let Some(click_pos) = response.hover_pos() {
                        // Check if we're in placement mode
                        if ui_state.is_in_placement_mode() {
                            PlacementInteraction::handle_click(
                                ui_state, viewport, click_pos, rect, config,
                            );
                        } else {
                            // Normal entity selection
                            SelectionInteraction::handle_click(ui_state, viewport, click_pos, rect);
                        }
                    }
                }

                // Render the scene content
                let project_path = config.and_then(|c| c.current_project_path());
                viewport.render(ui, rect, project_path.map(|p| p.as_path()), renderer);

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
            } else {
                // Show placeholder when no viewport
                let available_size = ui.available_size();
                ui.allocate_response(available_size, egui::Sense::click())
                    .on_hover_text("Scene viewport not initialized");
            }
        });
    }

    fn render_scene_graph(ui: &mut egui::Ui, ui_state: &super::EditorUI) {
        ui.heading("Active Scene Graph");
        ui.separator();

        let Some(active_scene_name) = ui_state.active_scene.as_deref() else {
            ui.label("No active scene selected.");
            return;
        };

        let Some(scene) = ui_state
            .scenes
            .iter()
            .find(|scene| scene.name == active_scene_name)
        else {
            ui.label(format!(
                "Active scene '{}' is not loaded.",
                active_scene_name
            ));
            return;
        };

        if scene.rules.rules.is_empty() {
            ui.label("No rules in active scene. Add rules in the Inspector first.");
            return;
        }

        let graph = RuleGraph::from_rule_set(&scene.rules);
        ui.label(format!(
            "Chains: {} | Nodes: {} | Edges: {}",
            graph.chains.len(),
            graph.nodes.len(),
            graph.edges.len()
        ));

        if let Err(error) = graph.to_rule_set() {
            ui.colored_label(
                egui::Color32::from_rgb(255, 120, 120),
                format!("Graph validation failed: {:?}", error),
            );
        }

        let node_by_id = graph
            .nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<HashMap<_, _>>();
        let mut outgoing = HashMap::<u64, Vec<u64>>::new();
        for edge in &graph.edges {
            outgoing.entry(edge.from).or_default().push(edge.to);
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (rule_index, chain) in graph.chains.iter().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.strong(format!("Rule {}: {}", rule_index + 1, chain.rule_id));
                        if !chain.enabled {
                            ui.label("(disabled)");
                        }
                    });

                    let mut node_id = chain.trigger_node_id;
                    let mut visited = std::collections::HashSet::new();
                    loop {
                        if !visited.insert(node_id) {
                            ui.monospace("Cycle detected");
                            break;
                        }
                        let Some(node) = node_by_id.get(&node_id) else {
                            ui.monospace(format!("Missing node {}", node_id));
                            break;
                        };
                        match &node.kind {
                            RuleGraphNodeKind::Trigger(trigger) => {
                                ui.monospace(format!("Trigger -> {:?}", trigger));
                            }
                            RuleGraphNodeKind::Condition(condition) => {
                                ui.monospace(format!("Condition -> {:?}", condition));
                            }
                            RuleGraphNodeKind::Action(action) => {
                                ui.monospace(format!("Action -> {:?}", action));
                            }
                        }

                        let next = outgoing.get(&node_id).cloned().unwrap_or_default();
                        if next.is_empty() {
                            break;
                        }
                        if next.len() > 1 {
                            ui.monospace("Branching chain");
                            break;
                        }
                        node_id = next[0];
                    }
                });
                ui.add_space(6.0);
            }
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
