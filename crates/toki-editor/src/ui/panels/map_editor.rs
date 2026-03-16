use super::*;
use crate::ui::editor_ui::{MapEditorTileInfo, MapEditorTool};
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
                ui_state.map_editor_save_requested = true;
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
                            let is_selected = ui_state.map_editor_active_map.as_deref()
                                == Some(map_name.as_str());
                            if ui.selectable_label(is_selected, map_name).clicked() && !is_selected
                            {
                                ui_state.map_editor_active_map = Some(map_name.clone());
                                ui_state.map_editor_map_load_requested = Some(map_name.clone());
                            }
                        }
                    }
                });

            if ui_state.has_unsaved_map_editor_draft() {
                ui.label("Unsaved draft");
            } else if ui_state.map_editor_dirty {
                ui.label("Unsaved changes");
            } else if let Some(active_map) = ui_state.map_editor_active_map.as_deref() {
                ui.label(format!("Editing asset: {}", active_map));
            }
        });
        ui.horizontal(|ui| {
            ui.label("Tool:");
            ui.label(match ui_state.map_editor_tool {
                MapEditorTool::Drag => "Drag",
                MapEditorTool::Brush => "Brush",
                MapEditorTool::Fill => "Fill",
                MapEditorTool::PickTile => "Pick Tile",
                MapEditorTool::PlaceObject => "Place Object",
                MapEditorTool::DeleteObject => "Delete",
            });
        });
        ui.separator();

        if ui_state.map_editor_show_new_map_dialog {
            let mut open = ui_state.map_editor_show_new_map_dialog;
            let mut create_clicked = false;
            let mut cancel_clicked = false;
            egui::Window::new("New Map")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut ui_state.map_editor_new_map_name);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Width");
                        ui.add(
                            egui::DragValue::new(&mut ui_state.map_editor_new_map_width)
                                .range(1..=512)
                                .speed(1),
                        );
                        ui.label("Height");
                        ui.add(
                            egui::DragValue::new(&mut ui_state.map_editor_new_map_height)
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
            ui_state.map_editor_show_new_map_dialog = open;
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

        match ui_state.map_editor_tool {
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

        match ui_state.map_editor_tool {
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
                            ui_state.map_editor_selected_tile_info = tile_info;
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
                if let Some(selected_tile) = ui_state.map_editor_selected_tile.clone() {
                    if Self::handle_map_editor_brush_paint(
                        ui,
                        ui_state,
                        viewport,
                        &response,
                        rect,
                        &selected_tile,
                        ui_state.map_editor_brush_size_tiles,
                    ) {
                        ui_state.mark_map_editor_dirty();
                    }
                }
            }
            MapEditorTool::Fill => {
                if let Some(selected_tile) = ui_state.map_editor_selected_tile.clone() {
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

    pub(super) fn handle_map_editor_primary_drag(
        viewport: &mut SceneViewport,
        response: &egui::Response,
        config: Option<&EditorConfig>,
    ) {
        CameraInteraction::handle_drag(viewport, response, config);
    }

    pub(super) fn apply_pending_map_editor_object_edit(
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
    ) -> bool {
        let Some(edit) = ui_state.take_map_editor_object_property_edit_request() else {
            return false;
        };
        let Some(tilemap) = viewport.scene_manager_mut().tilemap_mut() else {
            return false;
        };
        let Some(object) = tilemap.objects.get(edit.object_index) else {
            return false;
        };
        if object.visible == edit.visible && object.solid == edit.solid {
            return false;
        }

        ui_state.begin_map_editor_edit(tilemap);
        let Some(object) = tilemap.objects.get_mut(edit.object_index) else {
            ui_state.cancel_map_editor_edit();
            return false;
        };
        object.visible = edit.visible;
        object.solid = edit.solid;
        ui_state.finish_map_editor_edit(tilemap);
        true
    }

    pub(super) fn handle_map_editor_secondary_drag(
        ui: &egui::Ui,
        viewport: &mut SceneViewport,
        response: &egui::Response,
        config: Option<&EditorConfig>,
    ) {
        let pan_speed = config
            .map(|c| c.editor_settings.camera.pan_speed)
            .unwrap_or(1.0);

        if response.hovered() && ui.input(|input| input.pointer.secondary_pressed()) {
            if let Some(start_pos) = ui.input(|input| input.pointer.interact_pos()) {
                viewport.start_camera_drag(glam::Vec2::new(start_pos.x, start_pos.y));
            }
        } else if response.hovered() && ui.input(|input| input.pointer.secondary_down()) {
            if let Some(drag_pos) = ui.input(|input| input.pointer.interact_pos()) {
                viewport.update_camera_drag(glam::Vec2::new(drag_pos.x, drag_pos.y), pan_speed);
            }
        } else if ui.input(|input| input.pointer.secondary_released()) {
            viewport.stop_camera_drag();
        }
    }

    pub(super) fn handle_map_editor_brush_paint(
        ui: &egui::Ui,
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
        response: &egui::Response,
        rect: egui::Rect,
        selected_tile: &str,
        brush_size_tiles: u32,
    ) -> bool {
        let wants_paint = response.hovered()
            && ui.input(|input| input.pointer.primary_down() || input.pointer.primary_pressed());
        if !wants_paint {
            return false;
        }

        let Some(pointer_pos) = ui.input(|input| input.pointer.interact_pos()) else {
            return false;
        };

        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let Some(tilemap) = viewport.scene_manager_mut().tilemap_mut() else {
            return false;
        };
        if ui.input(|input| input.pointer.primary_pressed()) {
            ui_state.begin_map_editor_edit(tilemap);
        }
        let Some(tile_pos) = MapPaintInteraction::tile_position_at_world(tilemap, world_pos) else {
            return false;
        };

        if MapPaintInteraction::paint_brush(tilemap, tile_pos, selected_tile, brush_size_tiles) {
            viewport.mark_dirty();
            return true;
        }

        false
    }

    pub(super) fn handle_map_editor_fill_paint(
        ui: &egui::Ui,
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
        response: &egui::Response,
        selected_tile: &str,
    ) -> bool {
        let wants_fill = response.hovered() && ui.input(|input| input.pointer.primary_clicked());
        if !wants_fill {
            return false;
        }

        let Some(tilemap) = viewport.scene_manager_mut().tilemap_mut() else {
            return false;
        };
        ui_state.begin_map_editor_edit(tilemap);

        if MapPaintInteraction::fill_all(tilemap, selected_tile) {
            ui_state.finish_map_editor_edit(tilemap);
            viewport.mark_dirty();
            return true;
        }

        ui_state.cancel_map_editor_edit();
        false
    }

    pub(super) fn paint_map_editor_brush_preview(
        ui: &egui::Ui,
        ui_state: &mut EditorUI,
        viewport: &SceneViewport,
        rect: egui::Rect,
        project_path: &std::path::Path,
    ) {
        if ui_state.map_editor_tool != MapEditorTool::Brush {
            return;
        }
        let Some(selected_tile) = ui_state.map_editor_selected_tile.clone() else {
            return;
        };
        let Some(pointer_pos) = ui.input(|input| input.pointer.hover_pos()) else {
            return;
        };
        if !rect.contains(pointer_pos) {
            return;
        }
        let Some(tilemap) = viewport.scene_manager().tilemap() else {
            return;
        };
        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let Some(center_tile) = MapPaintInteraction::tile_position_at_world(tilemap, world_pos)
        else {
            return;
        };
        let Some((start_tile, end_tile)) = MapPaintInteraction::brush_footprint_bounds(
            tilemap,
            center_tile,
            ui_state.map_editor_brush_size_tiles,
        ) else {
            return;
        };
        let Some((atlas, texture_path)) =
            Self::load_map_editor_preview_assets(project_path, tilemap).ok()
        else {
            return;
        };
        let Some(texture) =
            Self::ensure_map_editor_brush_preview_texture(ui_state, ui.ctx(), &texture_path)
        else {
            return;
        };
        let Some(texture_size) = atlas.image_size() else {
            return;
        };
        let Some(tile_rect_px) = atlas.get_tile_rect(&selected_tile) else {
            return;
        };
        let uv_rect = egui::Rect::from_min_max(
            egui::pos2(
                tile_rect_px[0] as f32 / texture_size.x as f32,
                tile_rect_px[1] as f32 / texture_size.y as f32,
            ),
            egui::pos2(
                (tile_rect_px[0] + tile_rect_px[2]) as f32 / texture_size.x as f32,
                (tile_rect_px[1] + tile_rect_px[3]) as f32 / texture_size.y as f32,
            ),
        );
        let (viewport_width, viewport_height) = viewport.viewport_size();
        let display_rect = Self::compute_viewport_display_rect(
            rect,
            (viewport_width, viewport_height),
            viewport.sizing_mode() == crate::scene::viewport::ViewportSizingMode::Responsive,
        );
        let (camera_position, camera_scale) = viewport.camera_state();
        let painter = ui.painter().with_clip_rect(display_rect);
        let preview_tint = egui::Color32::from_white_alpha(170);
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(150));

        for tile_y in start_tile.y..end_tile.y {
            for tile_x in start_tile.x..end_tile.x {
                let Some(tile_screen_rect) = Self::map_editor_tile_screen_rect(
                    display_rect,
                    (viewport_width, viewport_height),
                    camera_position,
                    camera_scale,
                    tilemap.tile_size,
                    glam::UVec2::new(tile_x, tile_y),
                ) else {
                    continue;
                };
                painter.image(texture.id(), tile_screen_rect, uv_rect, preview_tint);
                painter.rect_stroke(tile_screen_rect, 0.0, stroke, egui::StrokeKind::Inside);
            }
        }
    }

    pub(super) fn handle_map_editor_object_place(
        ui: &egui::Ui,
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
        response: &egui::Response,
        rect: egui::Rect,
        project_path: Option<&std::path::Path>,
    ) -> bool {
        let clicked = response.hovered() && ui.input(|input| input.pointer.primary_clicked());
        if !clicked {
            return false;
        }

        let Some(object_sheet_name) = ui_state.map_editor_selected_object_sheet.clone() else {
            return false;
        };
        let Some(object_name) = ui_state.map_editor_selected_object_name.clone() else {
            return false;
        };
        let Some(pointer_pos) = ui.input(|input| input.pointer.interact_pos()) else {
            return false;
        };
        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let Some(tilemap) = viewport.scene_manager_mut().tilemap_mut() else {
            return false;
        };
        let Some(world_anchor) = MapObjectInteraction::object_anchor_at_world(tilemap, world_pos)
        else {
            return false;
        };
        let Some(project_path) = project_path else {
            return false;
        };
        let Some((object_sheet, _texture_path)) =
            Self::load_map_editor_object_preview_assets(project_path, &object_sheet_name).ok()
        else {
            return false;
        };
        let Some(object_info) = object_sheet.objects.get(&object_name) else {
            return false;
        };
        let object_sheet_file = if object_sheet_name.ends_with(".json") {
            object_sheet_name
        } else {
            format!("{}.json", object_sheet_name)
        };

        ui_state.begin_map_editor_edit(tilemap);
        if MapObjectInteraction::place_object(
            tilemap,
            world_anchor,
            &object_sheet_file,
            &object_name,
            glam::UVec2::new(
                object_info.size_tiles.x * object_sheet.tile_size.x,
                object_info.size_tiles.y * object_sheet.tile_size.y,
            ),
        ) {
            ui_state.finish_map_editor_edit(tilemap);
            viewport.mark_dirty();
            return true;
        }

        ui_state.cancel_map_editor_edit();
        false
    }

    pub(super) fn handle_map_editor_object_delete(
        ui: &egui::Ui,
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
        response: &egui::Response,
        rect: egui::Rect,
    ) -> bool {
        let clicked = response.hovered() && ui.input(|input| input.pointer.primary_clicked());
        if !clicked {
            return false;
        }

        let Some(pointer_pos) = ui.input(|input| input.pointer.interact_pos()) else {
            return false;
        };
        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let Some(tilemap) = viewport.scene_manager_mut().tilemap_mut() else {
            return false;
        };
        let Some(object_index) = MapObjectInteraction::object_index_at_world(tilemap, world_pos)
        else {
            return false;
        };

        ui_state.begin_map_editor_edit(tilemap);
        if MapObjectInteraction::delete_object(tilemap, object_index) {
            ui_state.finish_map_editor_edit(tilemap);
            ui_state.clear_map_editor_object_selection();
            viewport.mark_dirty();
            return true;
        }

        ui_state.cancel_map_editor_edit();
        false
    }

    pub(super) fn handle_map_editor_object_select(
        ui: &egui::Ui,
        viewport: &SceneViewport,
        response: &egui::Response,
        rect: egui::Rect,
    ) -> Option<usize> {
        let clicked = response.hovered() && ui.input(|input| input.pointer.primary_clicked());
        if !clicked {
            return None;
        }

        let pointer_pos = ui.input(|input| input.pointer.interact_pos())?;
        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let tilemap = viewport.scene_manager().tilemap()?;
        MapObjectInteraction::object_index_at_world(tilemap, world_pos)
    }

    pub(super) fn handle_map_editor_object_drag_start(
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
        drag_start_pos: egui::Pos2,
        rect: egui::Rect,
    ) {
        if ui_state.is_map_object_move_drag_active() {
            return;
        }

        let world_pos = viewport.screen_to_world_pos_raw(drag_start_pos, rect);
        let Some(tilemap) = viewport.scene_manager().tilemap() else {
            return;
        };
        let Some(object_index) = MapObjectInteraction::object_index_at_world(tilemap, world_pos)
        else {
            return;
        };
        let Some(object) = tilemap.objects.get(object_index) else {
            return;
        };

        ui_state.begin_map_editor_edit(tilemap);
        ui_state.select_map_editor_object(object_index, object);
        ui_state.begin_map_object_move_drag(object_index, world_pos - object.position.as_vec2());
    }

    pub(super) fn handle_map_editor_object_drag_update(
        ui: &egui::Ui,
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
        rect: egui::Rect,
    ) {
        let Some(drag_state) = ui_state.map_object_move_drag else {
            return;
        };
        let Some(pointer_pos) = ui.input(|input| input.pointer.interact_pos()) else {
            return;
        };
        let world_pos =
            viewport.screen_to_world_pos_raw(pointer_pos, rect) - drag_state.grab_offset;
        let Some(tilemap) = viewport.scene_manager_mut().tilemap_mut() else {
            return;
        };
        let Some(world_anchor) = MapObjectInteraction::object_anchor_at_world(tilemap, world_pos)
        else {
            return;
        };
        if MapObjectInteraction::move_object(tilemap, drag_state.object_index, world_anchor) {
            if let Some(object) = tilemap.objects.get(drag_state.object_index) {
                ui_state.select_map_editor_object(drag_state.object_index, object);
            }
            viewport.mark_dirty();
        }
    }

    pub(super) fn handle_map_editor_object_drag_release(
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
    ) -> bool {
        ui_state.finish_map_object_move_drag();
        let Some(tilemap) = viewport.scene_manager().tilemap() else {
            ui_state.cancel_map_editor_edit();
            return false;
        };
        let changed = ui_state.finish_map_editor_edit(tilemap);
        if !changed {
            ui_state.cancel_map_editor_edit();
        }
        viewport.mark_dirty();
        changed
    }

    pub(super) fn paint_map_editor_object_preview(
        ui: &egui::Ui,
        ui_state: &mut EditorUI,
        viewport: &SceneViewport,
        rect: egui::Rect,
        project_path: &std::path::Path,
    ) {
        if ui_state.map_editor_tool != MapEditorTool::PlaceObject {
            return;
        }
        let Some(object_sheet_name) = ui_state.map_editor_selected_object_sheet.clone() else {
            return;
        };
        let Some(object_name) = ui_state.map_editor_selected_object_name.clone() else {
            return;
        };
        let Some(pointer_pos) = ui.input(|input| input.pointer.hover_pos()) else {
            return;
        };
        if !rect.contains(pointer_pos) {
            return;
        }
        let Some(tilemap) = viewport.scene_manager().tilemap() else {
            return;
        };
        let Some((object_sheet, texture_path)) =
            Self::load_map_editor_object_preview_assets(project_path, &object_sheet_name).ok()
        else {
            return;
        };
        let Some(object_info) = object_sheet.objects.get(&object_name) else {
            return;
        };
        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let Some(world_anchor) = MapObjectInteraction::object_anchor_at_world(tilemap, world_pos)
        else {
            return;
        };
        let Some(texture) =
            Self::ensure_map_editor_preview_texture(ui_state, ui.ctx(), &texture_path)
        else {
            return;
        };
        let Some(texture_size) = object_sheet.image_size() else {
            return;
        };
        let Some(rect_px) = object_sheet.get_object_rect(&object_name) else {
            return;
        };
        let uv_rect = egui::Rect::from_min_max(
            egui::pos2(
                rect_px[0] as f32 / texture_size.x as f32,
                rect_px[1] as f32 / texture_size.y as f32,
            ),
            egui::pos2(
                (rect_px[0] + rect_px[2]) as f32 / texture_size.x as f32,
                (rect_px[1] + rect_px[3]) as f32 / texture_size.y as f32,
            ),
        );
        let (viewport_width, viewport_height) = viewport.viewport_size();
        let display_rect = Self::compute_viewport_display_rect(
            rect,
            (viewport_width, viewport_height),
            viewport.sizing_mode() == crate::scene::viewport::ViewportSizingMode::Responsive,
        );
        let (camera_position, camera_scale) = viewport.camera_state();
        let Some(object_screen_rect) = Self::world_rect_to_screen_rect(
            display_rect,
            camera_position,
            camera_scale,
            world_anchor,
            glam::UVec2::new(
                object_info.size_tiles.x * object_sheet.tile_size.x,
                object_info.size_tiles.y * object_sheet.tile_size.y,
            ),
        ) else {
            return;
        };
        let painter = ui.painter().with_clip_rect(display_rect);
        painter.image(
            texture.id(),
            object_screen_rect,
            uv_rect,
            egui::Color32::from_white_alpha(180),
        );
        painter.rect_stroke(
            object_screen_rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_white_alpha(180)),
            egui::StrokeKind::Outside,
        );
    }

    pub(super) fn load_map_editor_object_preview_assets(
        project_path: &std::path::Path,
        object_sheet_name: &str,
    ) -> anyhow::Result<(ObjectSheetMeta, std::path::PathBuf)> {
        let sheet_file = if object_sheet_name.ends_with(".json") {
            object_sheet_name.to_string()
        } else {
            format!("{}.json", object_sheet_name)
        };
        let object_sheet_path = project_path.join("assets").join("sprites").join(sheet_file);
        let object_sheet = ObjectSheetMeta::load_from_file(&object_sheet_path)
            .map_err(|error| anyhow::anyhow!("failed to load object sheet: {}", error))?;
        let texture_path = object_sheet_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("object sheet path has no parent"))?
            .join(&object_sheet.image);
        Ok((object_sheet, texture_path))
    }

    pub(super) fn ensure_map_editor_preview_texture(
        ui_state: &mut EditorUI,
        ctx: &egui::Context,
        texture_path: &std::path::Path,
    ) -> Option<egui::TextureHandle> {
        if ui_state.map_editor_brush_preview_image_path.as_deref() == Some(texture_path)
            && ui_state.map_editor_brush_preview_texture.is_some()
        {
            return ui_state.map_editor_brush_preview_texture.clone();
        }

        let decoded = toki_core::graphics::image::load_image_rgba8(texture_path).ok()?;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.data,
        );
        let key = format!("map_editor_preview:{}", texture_path.display());
        let texture = ctx.load_texture(key, color_image, egui::TextureOptions::NEAREST);
        ui_state.map_editor_brush_preview_image_path = Some(texture_path.to_path_buf());
        ui_state.map_editor_brush_preview_texture = Some(texture.clone());
        Some(texture)
    }

    pub(super) fn world_rect_to_screen_rect(
        display_rect: egui::Rect,
        camera_position: glam::IVec2,
        camera_scale: f32,
        world_top_left: glam::UVec2,
        world_size: glam::UVec2,
    ) -> Option<egui::Rect> {
        if camera_scale <= 0.0 {
            return None;
        }

        let screen_min_x = display_rect.min.x
            + (world_top_left.x as f32 - camera_position.x as f32) / camera_scale;
        let screen_min_y = display_rect.min.y
            + (world_top_left.y as f32 - camera_position.y as f32) / camera_scale;
        let screen_size = egui::vec2(
            world_size.x as f32 / camera_scale,
            world_size.y as f32 / camera_scale,
        );
        Some(egui::Rect::from_min_size(
            egui::pos2(screen_min_x, screen_min_y),
            screen_size,
        ))
    }

    pub(super) fn handle_map_editor_tile_inspect(
        ui: &egui::Ui,
        viewport: &mut SceneViewport,
        response: &egui::Response,
        rect: egui::Rect,
        project_path: &std::path::Path,
    ) -> Option<Option<MapEditorTileInfo>> {
        let clicked = response.hovered() && ui.input(|input| input.pointer.primary_clicked());
        if !clicked {
            return None;
        }

        let Some(pointer_pos) = ui.input(|input| input.pointer.interact_pos()) else {
            return Some(None);
        };
        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let Some(tilemap) = viewport.scene_manager().tilemap() else {
            return Some(None);
        };
        let Some(tile_pos) = MapPaintInteraction::tile_position_at_world(tilemap, world_pos) else {
            return Some(None);
        };
        let Some(tile_name) = tilemap
            .get_tile_name(tile_pos.x, tile_pos.y)
            .ok()
            .map(ToString::to_string)
        else {
            return Some(None);
        };
        let Some(atlas) = Self::load_map_editor_atlas(project_path, tilemap).ok() else {
            return Some(None);
        };
        let Some(properties) = atlas.get_tile_properties(&tile_name) else {
            return Some(None);
        };

        Some(Some(MapEditorTileInfo {
            tile_x: tile_pos.x,
            tile_y: tile_pos.y,
            tile_name,
            solid: properties.solid,
            trigger: properties.trigger,
        }))
    }

    pub(super) fn handle_map_editor_tile_pick(
        ui: &egui::Ui,
        viewport: &SceneViewport,
        response: &egui::Response,
        rect: egui::Rect,
    ) -> Option<String> {
        let clicked = response.hovered() && ui.input(|input| input.pointer.primary_clicked());
        if !clicked {
            return None;
        }

        let pointer_pos = ui.input(|input| input.pointer.interact_pos())?;
        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let tilemap = viewport.scene_manager().tilemap()?;
        let tile_pos = MapPaintInteraction::tile_position_at_world(tilemap, world_pos)?;
        tilemap
            .get_tile_name(tile_pos.x, tile_pos.y)
            .ok()
            .map(ToString::to_string)
    }

    pub(super) fn load_map_editor_tile_names(
        project_path: &std::path::Path,
        tilemap: &TileMap,
    ) -> anyhow::Result<Vec<String>> {
        let atlas = Self::load_map_editor_atlas(project_path, tilemap)?;
        let mut tile_names = atlas.tiles.keys().cloned().collect::<Vec<_>>();
        tile_names.sort();
        Ok(tile_names)
    }

    pub(super) fn load_map_editor_atlas(
        project_path: &std::path::Path,
        tilemap: &TileMap,
    ) -> anyhow::Result<AtlasMeta> {
        let atlas_path = {
            let tilemaps_path = project_path
                .join("assets")
                .join("tilemaps")
                .join(&tilemap.atlas);
            if tilemaps_path.exists() {
                tilemaps_path
            } else {
                project_path
                    .join("assets")
                    .join("sprites")
                    .join(&tilemap.atlas)
            }
        };
        AtlasMeta::load_from_file(&atlas_path)
            .map_err(|e| anyhow::anyhow!("Failed to load atlas '{}': {}", atlas_path.display(), e))
    }

    pub(super) fn load_map_editor_preview_assets(
        project_path: &std::path::Path,
        tilemap: &TileMap,
    ) -> anyhow::Result<(AtlasMeta, std::path::PathBuf)> {
        let atlas_path = {
            let tilemaps_path = project_path
                .join("assets")
                .join("tilemaps")
                .join(&tilemap.atlas);
            if tilemaps_path.exists() {
                tilemaps_path
            } else {
                project_path
                    .join("assets")
                    .join("sprites")
                    .join(&tilemap.atlas)
            }
        };
        let atlas = AtlasMeta::load_from_file(&atlas_path).map_err(|e| {
            anyhow::anyhow!("Failed to load atlas '{}': {}", atlas_path.display(), e)
        })?;
        let texture_path = atlas_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Atlas path '{}' has no parent", atlas_path.display()))?
            .join(&atlas.image);
        Ok((atlas, texture_path))
    }

    pub(super) fn ensure_map_editor_brush_preview_texture(
        ui_state: &mut EditorUI,
        ctx: &egui::Context,
        texture_path: &std::path::Path,
    ) -> Option<egui::TextureHandle> {
        if ui_state.map_editor_brush_preview_image_path.as_deref() == Some(texture_path)
            && ui_state.map_editor_brush_preview_texture.is_some()
        {
            return ui_state.map_editor_brush_preview_texture.clone();
        }

        let decoded = toki_core::graphics::image::load_image_rgba8(texture_path).ok()?;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.data,
        );
        let key = format!("map_editor_brush_preview:{}", texture_path.display());
        let texture = ctx.load_texture(key, color_image, egui::TextureOptions::NEAREST);
        ui_state.map_editor_brush_preview_image_path = Some(texture_path.to_path_buf());
        ui_state.map_editor_brush_preview_texture = Some(texture.clone());
        Some(texture)
    }

    pub(super) fn map_editor_tile_screen_rect(
        display_rect: egui::Rect,
        viewport_size: (u32, u32),
        camera_position: glam::IVec2,
        camera_scale: f32,
        tile_size: glam::UVec2,
        tile_pos: glam::UVec2,
    ) -> Option<egui::Rect> {
        let world_span_x = viewport_size.0 as f32 * camera_scale;
        let world_span_y = viewport_size.1 as f32 * camera_scale;
        if world_span_x <= 0.0 || world_span_y <= 0.0 {
            return None;
        }

        let world_min_x = camera_position.x as f32;
        let world_min_y = camera_position.y as f32;
        let world_left = tile_pos.x as f32 * tile_size.x as f32;
        let world_top = tile_pos.y as f32 * tile_size.y as f32;
        let world_right = world_left + tile_size.x as f32;
        let world_bottom = world_top + tile_size.y as f32;

        let left_t = (world_left - world_min_x) / world_span_x;
        let top_t = (world_top - world_min_y) / world_span_y;
        let right_t = (world_right - world_min_x) / world_span_x;
        let bottom_t = (world_bottom - world_min_y) / world_span_y;

        Some(egui::Rect::from_min_max(
            egui::pos2(
                egui::lerp(display_rect.left()..=display_rect.right(), left_t),
                egui::lerp(display_rect.top()..=display_rect.bottom(), top_t),
            ),
            egui::pos2(
                egui::lerp(display_rect.left()..=display_rect.right(), right_t),
                egui::lerp(display_rect.top()..=display_rect.bottom(), bottom_t),
            ),
        ))
    }
}
