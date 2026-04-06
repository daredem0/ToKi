use super::*;
use crate::editor_viewport::EditorViewportContext;
use crate::ui::editor_ui::MapEditorTool;
use crate::ui::EditorUI;

impl PanelSystem {
    #[allow(clippy::needless_option_as_deref)]
    pub(crate) fn render_map_editor(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
        map_editor_viewport: Option<&mut SceneViewport>,
        available_map_names: Option<Vec<String>>,
        mut config: Option<&mut EditorConfig>,
        mut renderer: Option<&mut crate::rendering::WindowRenderer>,
    ) {
        if let Some(names) = &available_map_names {
            crate::ui::editor_ui::sync_map_editor_selection(ui_state, names);
        } else {
            crate::ui::editor_ui::sync_map_editor_selection(ui_state, &[]);
        }

        let project_path = config
            .as_deref()
            .and_then(|cfg| cfg.current_project_path())
            .cloned();
        let available_brush_entries = project_path
            .as_deref()
            .and_then(|_| {
                crate::ui::editor_ui::load_map_editor_brush_source(ui_state, config.as_deref())
            })
            .map(|source| source.brush_entries)
            .unwrap_or_default();
        crate::ui::editor_ui::sync_map_editor_brush_selection(ui_state, &available_brush_entries);

        ui.horizontal(|ui| {
            ui.heading("Map Editor");
            ui.separator();
            if ui.button("New Map").clicked() {
                crate::ui::editor_ui::begin_new_map_dialog(ui_state);
            }
            if ui
                .add_enabled(
                    crate::ui::editor_context::map_state(ui_state)
                        .draft
                        .is_some(),
                    egui::Button::new("Resize Map..."),
                )
                .clicked()
            {
                crate::ui::editor_ui::begin_resize_map_dialog(ui_state);
            }
            if ui
                .add_enabled(
                    crate::ui::editor_ui::has_unsaved_map_editor_changes(ui_state),
                    egui::Button::new("Save Map"),
                )
                .clicked()
            {
                crate::ui::editor_context::map_state_mut(ui_state).save_requested = true;
            }
            ui.separator();
            ui.label("Map:");

            let selected_label = crate::ui::editor_ui::map_editor_selected_label(ui_state);
            egui::ComboBox::from_id_salt("map_editor_map_selector")
                .selected_text(selected_label)
                .show_ui(ui, |ui| {
                    if let Some(map_names) = &available_map_names {
                        if crate::ui::editor_ui::has_unsaved_map_editor_changes(ui_state) {
                            ui.label("Save the current draft before switching maps.");
                            return;
                        }
                        for map_name in map_names {
                            let is_selected = crate::ui::editor_context::map_state_mut(ui_state)
                                .active_map
                                .as_deref()
                                == Some(map_name.as_str());
                            if ui.selectable_label(is_selected, map_name).clicked() && !is_selected
                            {
                                crate::ui::editor_context::map_state_mut(ui_state).active_map =
                                    Some(map_name.clone());
                                crate::ui::editor_context::map_state_mut(ui_state)
                                    .map_load_requested = Some(map_name.clone());
                            }
                        }
                    }
                });

            if crate::ui::editor_ui::has_unsaved_map_editor_draft(ui_state) {
                ui.label("Unsaved draft");
            } else if crate::ui::editor_context::map_state_mut(ui_state).dirty {
                ui.label("Unsaved changes");
            } else if let Some(active_map) = crate::ui::editor_context::map_state_mut(ui_state)
                .active_map
                .as_deref()
            {
                ui.label(format!("Editing asset: {}", active_map));
            }
        });
        ui.horizontal(|ui| {
            ui.label("Tool:");
            ui.label(
                match crate::ui::editor_context::map_state_mut(ui_state).tool {
                    MapEditorTool::Drag => "Drag",
                    MapEditorTool::Brush => "Brush",
                    MapEditorTool::Fill => "Fill",
                    MapEditorTool::PickTile => "Pick Tile",
                },
            );
        });
        ui.separator();

        if crate::ui::editor_context::map_state_mut(ui_state).show_new_map_dialog {
            let mut open = crate::ui::editor_context::map_state_mut(ui_state).show_new_map_dialog;
            let mut create_clicked = false;
            let mut cancel_clicked = false;
            egui::Window::new("New Map")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(
                        &mut crate::ui::editor_context::map_state_mut(ui_state).new_map_name,
                    );
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Map Size (tiles):");
                        ui.add(
                            egui::DragValue::new(
                                &mut crate::ui::editor_context::map_state_mut(ui_state)
                                    .new_map_width,
                            )
                            .range(1..=512)
                            .speed(1),
                        );
                        ui.label("×");
                        ui.add(
                            egui::DragValue::new(
                                &mut crate::ui::editor_context::map_state_mut(ui_state)
                                    .new_map_height,
                            )
                            .range(1..=512)
                            .speed(1),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Tile Size (px):");
                        ui.add(
                            egui::DragValue::new(
                                &mut crate::ui::editor_context::map_state_mut(ui_state)
                                    .new_map_tile_width,
                            )
                            .range(1..=256)
                            .speed(1),
                        );
                        ui.label("×");
                        ui.add(
                            egui::DragValue::new(
                                &mut crate::ui::editor_context::map_state_mut(ui_state)
                                    .new_map_tile_height,
                            )
                            .range(1..=256)
                            .speed(1),
                        );
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            create_clicked = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel_clicked = true;
                        }
                    });
                });

            if create_clicked {
                crate::ui::editor_ui::submit_new_map_request(ui_state);
                open = false;
            }
            if cancel_clicked {
                open = false;
            }
            crate::ui::editor_context::map_state_mut(ui_state).show_new_map_dialog = open;
        }

        Self::render_resize_map_dialog(ui_state, ui.ctx());

        let Some(viewport) = map_editor_viewport else {
            ui.label("Map editor viewport not initialized.");
            return;
        };

        if let Some(cfg) = config.as_deref_mut() {
            if Self::render_grid_toolbar(ui, cfg) {
                viewport.mark_dirty();
            }
            ui.separator();
        }

        let available_size = ui.available_size();
        let requested_viewport_size = (
            available_size.x.max(1.0).round() as u32,
            available_size.y.max(1.0).round() as u32,
        );
        viewport.request_viewport_size(requested_viewport_size);

        if let Err(error) = viewport.update() {
            tracing::error!("Map editor viewport update error: {error}");
        }
        if viewport.has_active_tile_animations() {
            ui.ctx().request_repaint();
        }

        let (rect, response) =
            ui.allocate_exact_size(available_size, egui::Sense::click_and_drag());
        let (camera_position, camera_scale) = viewport.camera_state();
        let viewport_ctx = EditorViewportContext::new(
            rect,
            viewport.viewport_size(),
            viewport.sizing_mode() == crate::scene::viewport::ViewportSizingMode::Responsive,
            camera_position,
            camera_scale,
        );
        let display_rect = viewport_ctx.display_rect();

        match crate::ui::editor_context::map_state_mut(ui_state).tool {
            MapEditorTool::Drag => {
                crate::ui::editor_ui::cancel_map_editor_edit(ui_state);
                Self::handle_map_editor_primary_drag(viewport, &response, config.as_deref());
            }
            MapEditorTool::Brush => {
                Self::handle_map_editor_secondary_drag(ui, viewport, &response, config.as_deref());
            }
            MapEditorTool::Fill => {
                crate::ui::editor_ui::cancel_map_editor_edit(ui_state);
                Self::handle_map_editor_secondary_drag(ui, viewport, &response, config.as_deref());
            }
            MapEditorTool::PickTile => {
                crate::ui::editor_ui::cancel_map_editor_edit(ui_state);
                Self::handle_map_editor_secondary_drag(ui, viewport, &response, config.as_deref());
            }
        }

        if response.hovered() {
            let scroll_delta = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                let sensitivity = config
                    .as_deref()
                    .map(|c| c.editor_settings.camera.scroll_zoom_sensitivity)
                    .unwrap_or(0.02);
                // Scale the scroll delta by sensitivity; require threshold to trigger discrete zoom
                let scaled = scroll_delta.abs() * sensitivity;
                if scaled > 0.3 {
                    if scroll_delta > 0.0 {
                        viewport.zoom_in();
                    } else {
                        viewport.zoom_out();
                    }
                }
            }
        }

        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_rgb(34, 37, 41));
        viewport.render(ui, rect, project_path.as_deref(), renderer.as_deref_mut());
        Self::paint_map_editor_empty_tile_checkerboard(ui, viewport, &viewport_ctx);
        if let Some(cfg) = config.as_deref() {
            Self::paint_viewport_grid_overlay(ui, rect, viewport, cfg);
        }
        if let Some(project_path) = project_path.as_deref() {
            Self::paint_map_editor_brush_preview(
                ui,
                ui_state,
                viewport,
                &viewport_ctx,
                project_path,
                renderer.as_deref_mut(),
            );
        }

        match crate::ui::editor_context::map_state_mut(ui_state).tool {
            MapEditorTool::Drag => {
                if response.clicked() {
                    if let Some(project_path) = project_path.as_deref() {
                        if let Some(tile_info) = Self::handle_map_editor_tile_inspect(
                            ui,
                            viewport,
                            &response,
                            display_rect,
                            project_path,
                        ) {
                            crate::ui::editor_context::map_state_mut(ui_state).selected_tile_info =
                                tile_info;
                        }
                    }
                }
            }
            MapEditorTool::Brush => {
                let primary_down = ui.input(|input| input.pointer.primary_down());
                if !primary_down {
                    if let Some(tilemap) = viewport.tilemap() {
                        crate::ui::editor_ui::finish_map_editor_edit(ui_state, tilemap);
                    } else {
                        crate::ui::editor_ui::cancel_map_editor_edit(ui_state);
                    }
                }
                if let Some(selected_tile) = crate::ui::editor_context::map_state_mut(ui_state)
                    .selected_tile
                    .clone()
                {
                    let brush_size_tiles =
                        crate::ui::editor_context::map_state(ui_state).brush_size_tiles;
                    if Self::handle_map_editor_brush_paint(
                        ui,
                        ui_state,
                        viewport,
                        &response,
                        display_rect,
                        &selected_tile,
                        brush_size_tiles,
                    ) {
                        crate::ui::editor_ui::mark_map_editor_dirty(ui_state);
                    }
                }
            }
            MapEditorTool::Fill => {
                if let Some(selected_tile) = crate::ui::editor_context::map_state_mut(ui_state)
                    .selected_tile
                    .clone()
                {
                    if Self::handle_map_editor_fill_paint(
                        ui,
                        ui_state,
                        viewport,
                        &response,
                        &selected_tile,
                    ) {
                        crate::ui::editor_ui::mark_map_editor_dirty(ui_state);
                    }
                }
            }
            MapEditorTool::PickTile => {
                if let Some(tile_name) =
                    Self::handle_map_editor_tile_pick(ui, viewport, &response, display_rect)
                {
                    crate::ui::editor_ui::pick_map_editor_tile(ui_state, tile_name);
                }
            }
        }
    }

    fn render_resize_map_dialog(ui_state: &mut EditorUI, ctx: &egui::Context) {
        if !crate::ui::editor_context::map_state(ui_state).show_resize_map_dialog {
            return;
        }

        let Some(current_size) = crate::ui::editor_context::map_state(ui_state)
            .draft
            .as_ref()
            .map(|draft| draft.tilemap.size)
        else {
            crate::ui::editor_context::map_state_mut(ui_state).show_resize_map_dialog = false;
            return;
        };

        let spec = crate::ui::editor_ui::MapResizeSpec {
            remove_north: crate::ui::editor_context::map_state(ui_state).resize_remove_north,
            remove_east: crate::ui::editor_context::map_state(ui_state).resize_remove_east,
            remove_south: crate::ui::editor_context::map_state(ui_state).resize_remove_south,
            remove_west: crate::ui::editor_context::map_state(ui_state).resize_remove_west,
            add_north: crate::ui::editor_context::map_state(ui_state).resize_add_north,
            add_east: crate::ui::editor_context::map_state(ui_state).resize_add_east,
            add_south: crate::ui::editor_context::map_state(ui_state).resize_add_south,
            add_west: crate::ui::editor_context::map_state(ui_state).resize_add_west,
        };
        let result_width =
            i64::from(current_size.x) + i64::from(spec.add_west) + i64::from(spec.add_east)
                - i64::from(spec.remove_west)
                - i64::from(spec.remove_east);
        let result_height =
            i64::from(current_size.y) + i64::from(spec.add_north) + i64::from(spec.add_south)
                - i64::from(spec.remove_north)
                - i64::from(spec.remove_south);
        let valid = result_width >= 1 && result_height >= 1;

        let mut open = crate::ui::editor_context::map_state(ui_state).show_resize_map_dialog;
        let mut apply_clicked = false;
        let mut cancel_clicked = false;

        egui::Window::new("Resize Map")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Current Size:");
                    ui.label(format!("{} × {} tiles", current_size.x, current_size.y));
                });
                ui.horizontal(|ui| {
                    ui.label("Result Size:");
                    if valid {
                        ui.label(format!("{} × {} tiles", result_width, result_height));
                    } else {
                        ui.colored_label(
                            egui::Color32::LIGHT_RED,
                            "Invalid (must stay at least 1 × 1)",
                        );
                    }
                });
                ui.separator();

                ui.label("Remove Tiles");
                ui.horizontal(|ui| {
                    ui.label("All:");
                    let changed = ui
                        .add(
                            egui::DragValue::new(
                                &mut crate::ui::editor_context::map_state_mut(ui_state)
                                    .resize_remove_all,
                            )
                            .range(0..=512)
                            .speed(1),
                        )
                        .changed();
                    ui.label("tiles");
                    if changed {
                        let value =
                            crate::ui::editor_context::map_state(ui_state).resize_remove_all;
                        let state = crate::ui::editor_context::map_state_mut(ui_state);
                        state.resize_remove_north = value;
                        state.resize_remove_east = value;
                        state.resize_remove_south = value;
                        state.resize_remove_west = value;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("North:");
                    ui.add(
                        egui::DragValue::new(
                            &mut crate::ui::editor_context::map_state_mut(ui_state)
                                .resize_remove_north,
                        )
                        .range(0..=512)
                        .speed(1),
                    );
                    ui.label("East:");
                    ui.add(
                        egui::DragValue::new(
                            &mut crate::ui::editor_context::map_state_mut(ui_state)
                                .resize_remove_east,
                        )
                        .range(0..=512)
                        .speed(1),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("South:");
                    ui.add(
                        egui::DragValue::new(
                            &mut crate::ui::editor_context::map_state_mut(ui_state)
                                .resize_remove_south,
                        )
                        .range(0..=512)
                        .speed(1),
                    );
                    ui.label("West:");
                    ui.add(
                        egui::DragValue::new(
                            &mut crate::ui::editor_context::map_state_mut(ui_state)
                                .resize_remove_west,
                        )
                        .range(0..=512)
                        .speed(1),
                    );
                });

                ui.separator();
                ui.label("Add Tiles");
                ui.horizontal(|ui| {
                    ui.label("All:");
                    let changed = ui
                        .add(
                            egui::DragValue::new(
                                &mut crate::ui::editor_context::map_state_mut(ui_state)
                                    .resize_add_all,
                            )
                            .range(0..=512)
                            .speed(1),
                        )
                        .changed();
                    ui.label("tiles");
                    if changed {
                        let value = crate::ui::editor_context::map_state(ui_state).resize_add_all;
                        let state = crate::ui::editor_context::map_state_mut(ui_state);
                        state.resize_add_north = value;
                        state.resize_add_east = value;
                        state.resize_add_south = value;
                        state.resize_add_west = value;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("North:");
                    ui.add(
                        egui::DragValue::new(
                            &mut crate::ui::editor_context::map_state_mut(ui_state)
                                .resize_add_north,
                        )
                        .range(0..=512)
                        .speed(1),
                    );
                    ui.label("East:");
                    ui.add(
                        egui::DragValue::new(
                            &mut crate::ui::editor_context::map_state_mut(ui_state).resize_add_east,
                        )
                        .range(0..=512)
                        .speed(1),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("South:");
                    ui.add(
                        egui::DragValue::new(
                            &mut crate::ui::editor_context::map_state_mut(ui_state)
                                .resize_add_south,
                        )
                        .range(0..=512)
                        .speed(1),
                    );
                    ui.label("West:");
                    ui.add(
                        egui::DragValue::new(
                            &mut crate::ui::editor_context::map_state_mut(ui_state).resize_add_west,
                        )
                        .range(0..=512)
                        .speed(1),
                    );
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.add_enabled(valid, egui::Button::new("Apply")).clicked() {
                        apply_clicked = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel_clicked = true;
                    }
                });
            });

        if apply_clicked {
            let spec = crate::ui::editor_ui::MapResizeSpec {
                remove_north: crate::ui::editor_context::map_state(ui_state).resize_remove_north,
                remove_east: crate::ui::editor_context::map_state(ui_state).resize_remove_east,
                remove_south: crate::ui::editor_context::map_state(ui_state).resize_remove_south,
                remove_west: crate::ui::editor_context::map_state(ui_state).resize_remove_west,
                add_north: crate::ui::editor_context::map_state(ui_state).resize_add_north,
                add_east: crate::ui::editor_context::map_state(ui_state).resize_add_east,
                add_south: crate::ui::editor_context::map_state(ui_state).resize_add_south,
                add_west: crate::ui::editor_context::map_state(ui_state).resize_add_west,
            };
            let _ = crate::ui::editor_ui::resize_map(ui_state, spec);
            open = false;
        }
        if cancel_clicked {
            open = false;
        }

        crate::ui::editor_context::map_state_mut(ui_state).show_resize_map_dialog = open;
    }
}
