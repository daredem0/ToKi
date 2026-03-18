use super::*;
use crate::ui::editor_ui::MapEditorTool;
use crate::ui::EditorUI;

impl PanelSystem {
    pub(super) fn render_map_editor(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
        map_editor_viewport: Option<&mut SceneViewport>,
        available_map_names: Option<Vec<String>>,
        mut config: Option<&mut EditorConfig>,
        renderer: Option<&mut egui_wgpu::Renderer>,
    ) {
        if let Some(names) = &available_map_names {
            ui_state.sync_map_editor_selection(names);
        } else {
            ui_state.sync_map_editor_selection(&[]);
        }

        let project_path = config
            .as_deref()
            .and_then(|cfg| cfg.current_project_path())
            .cloned();
        let available_tiles = project_path
            .as_deref()
            .and_then(|path| {
                map_editor_viewport
                    .as_ref()
                    .and_then(|viewport| viewport.scene_manager().tilemap())
                    .and_then(|tilemap| Self::load_map_editor_tile_names(path, tilemap).ok())
            })
            .unwrap_or_default();
        ui_state.sync_map_editor_brush_selection(&available_tiles);

        ui.horizontal(|ui| {
            ui.heading("Map Editor");
            ui.separator();
            if ui.button("New Map").clicked() {
                ui_state.begin_new_map_dialog();
            }
            if ui
                .add_enabled(
                    ui_state.has_unsaved_map_editor_changes(),
                    egui::Button::new("Save Map"),
                )
                .clicked()
            {
                ui_state.map.save_requested = true;
            }
            ui.separator();
            ui.label("Map:");

            let selected_label = ui_state.map_editor_selected_label();
            egui::ComboBox::from_id_salt("map_editor_map_selector")
                .selected_text(selected_label)
                .show_ui(ui, |ui| {
                    if let Some(map_names) = &available_map_names {
                        if ui_state.has_unsaved_map_editor_changes() {
                            ui.label("Save the current draft before switching maps.");
                            return;
                        }
                        for map_name in map_names {
                            let is_selected = ui_state.map.active_map.as_deref()
                                == Some(map_name.as_str());
                            if ui.selectable_label(is_selected, map_name).clicked() && !is_selected
                            {
                                ui_state.map.active_map = Some(map_name.clone());
                                ui_state.map.map_load_requested = Some(map_name.clone());
                            }
                        }
                    }
                });

            if ui_state.has_unsaved_map_editor_draft() {
                ui.label("Unsaved draft");
            } else if ui_state.map.dirty {
                ui.label("Unsaved changes");
            } else if let Some(active_map) = ui_state.map.active_map.as_deref() {
                ui.label(format!("Editing asset: {}", active_map));
            }
        });
        ui.horizontal(|ui| {
            ui.label("Tool:");
            ui.label(match ui_state.map.tool {
                MapEditorTool::Drag => "Drag",
                MapEditorTool::Brush => "Brush",
                MapEditorTool::Fill => "Fill",
                MapEditorTool::PickTile => "Pick Tile",
                MapEditorTool::PlaceObject => "Place Object",
                MapEditorTool::DeleteObject => "Delete",
            });
        });
        ui.separator();

        if ui_state.map.show_new_map_dialog {
            let mut open = ui_state.map.show_new_map_dialog;
            let mut create_clicked = false;
            let mut cancel_clicked = false;
            egui::Window::new("New Map")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut ui_state.map.new_map_name);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Width");
                        ui.add(
                            egui::DragValue::new(&mut ui_state.map.new_map_width)
                                .range(1..=512)
                                .speed(1),
                        );
                        ui.label("Height");
                        ui.add(
                            egui::DragValue::new(&mut ui_state.map.new_map_height)
                                .range(1..=512)
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
                ui_state.submit_new_map_request();
                open = false;
            }
            if cancel_clicked {
                open = false;
            }
            ui_state.map.show_new_map_dialog = open;
        }

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
        if let Some(tilemap) = viewport.scene_manager().tilemap() {
            ui_state.sync_selected_map_editor_object_from_tilemap(tilemap);
        }
        if Self::apply_pending_map_editor_object_edit(ui_state, viewport) {
            viewport.mark_dirty();
        }

        let (rect, response) =
            ui.allocate_exact_size(available_size, egui::Sense::click_and_drag());

        match ui_state.map.tool {
            MapEditorTool::Drag => {
                ui_state.cancel_map_editor_edit();
                if response.drag_started() {
                    if let Some(drag_start_pos) = response.interact_pointer_pos() {
                        Self::handle_map_editor_object_drag_start(
                            ui_state,
                            viewport,
                            drag_start_pos,
                            rect,
                        );
                    }
                }

                if ui_state.is_map_object_move_drag_active() {
                    Self::handle_map_editor_object_drag_update(ui, ui_state, viewport, rect);
                    if response.drag_stopped()
                        && Self::handle_map_editor_object_drag_release(ui_state, viewport)
                    {
                        ui_state.mark_map_editor_dirty();
                    }
                    viewport.stop_camera_drag();
                } else {
                    Self::handle_map_editor_primary_drag(viewport, &response, config.as_deref());
                }
            }
            MapEditorTool::Brush => {
                Self::handle_map_editor_secondary_drag(ui, viewport, &response, config.as_deref());
            }
            MapEditorTool::Fill => {
                ui_state.cancel_map_editor_edit();
                Self::handle_map_editor_secondary_drag(ui, viewport, &response, config.as_deref());
            }
            MapEditorTool::PickTile => {
                ui_state.cancel_map_editor_edit();
                Self::handle_map_editor_secondary_drag(ui, viewport, &response, config.as_deref());
            }
            MapEditorTool::PlaceObject => {
                ui_state.cancel_map_editor_edit();
                Self::handle_map_editor_secondary_drag(ui, viewport, &response, config.as_deref());
            }
            MapEditorTool::DeleteObject => {
                ui_state.cancel_map_editor_edit();
                Self::handle_map_editor_secondary_drag(ui, viewport, &response, config.as_deref());
            }
        }

        viewport.render(ui, rect, project_path.as_deref(), renderer);
        if let Some(cfg) = config.as_deref() {
            Self::paint_viewport_grid_overlay(ui, rect, viewport, cfg);
        }
        if let Some(project_path) = project_path.as_deref() {
            Self::paint_map_editor_brush_preview(ui, ui_state, viewport, rect, project_path);
            Self::paint_map_editor_object_preview(ui, ui_state, viewport, rect, project_path);
        }

        match ui_state.map.tool {
            MapEditorTool::Drag => {
                if response.clicked() {
                    if let Some(selected_object_index) =
                        Self::handle_map_editor_object_select(ui, viewport, &response, rect)
                    {
                        if let Some(tilemap) = viewport.scene_manager().tilemap() {
                            if let Some(object) = tilemap.objects.get(selected_object_index) {
                                ui_state.select_map_editor_object(selected_object_index, object);
                            }
                        }
                    } else if let Some(project_path) = project_path.as_deref() {
                        ui_state.clear_map_editor_object_selection();
                        if let Some(tile_info) = Self::handle_map_editor_tile_inspect(
                            ui,
                            viewport,
                            &response,
                            rect,
                            project_path,
                        ) {
                            ui_state.map.selected_tile_info = tile_info;
                        }
                    } else {
                        ui_state.clear_map_editor_object_selection();
                    }
                }
            }
            MapEditorTool::Brush => {
                let primary_down = ui.input(|input| input.pointer.primary_down());
                if !primary_down {
                    if let Some(tilemap) = viewport.scene_manager().tilemap() {
                        ui_state.finish_map_editor_edit(tilemap);
                    } else {
                        ui_state.cancel_map_editor_edit();
                    }
                }
                if let Some(selected_tile) = ui_state.map.selected_tile.clone() {
                    if Self::handle_map_editor_brush_paint(
                        ui,
                        ui_state,
                        viewport,
                        &response,
                        rect,
                        &selected_tile,
                        ui_state.map.brush_size_tiles,
                    ) {
                        ui_state.mark_map_editor_dirty();
                    }
                }
            }
            MapEditorTool::Fill => {
                if let Some(selected_tile) = ui_state.map.selected_tile.clone() {
                    if Self::handle_map_editor_fill_paint(
                        ui,
                        ui_state,
                        viewport,
                        &response,
                        &selected_tile,
                    ) {
                        ui_state.mark_map_editor_dirty();
                    }
                }
            }
            MapEditorTool::PickTile => {
                if let Some(tile_name) =
                    Self::handle_map_editor_tile_pick(ui, viewport, &response, rect)
                {
                    ui_state.pick_map_editor_tile(tile_name);
                }
            }
            MapEditorTool::PlaceObject => {
                if Self::handle_map_editor_object_place(
                    ui,
                    ui_state,
                    viewport,
                    &response,
                    rect,
                    project_path.as_deref(),
                ) {
                    ui_state.mark_map_editor_dirty();
                }
            }
            MapEditorTool::DeleteObject => {
                if Self::handle_map_editor_object_delete(ui, ui_state, viewport, &response, rect) {
                    ui_state.mark_map_editor_dirty();
                }
            }
        }
    }
}
