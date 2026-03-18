use super::*;
use crate::ui::editor_ui::MapEditorTileInfo;
use crate::ui::EditorUI;

impl PanelSystem {
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
        let Some(tilemap) = viewport.tilemap_mut() else {
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
        let Some(tilemap) = viewport.tilemap_mut() else {
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

        let Some(tilemap) = viewport.tilemap_mut() else {
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

        let Some(object_sheet_name) = ui_state.map.selected_object_sheet.clone() else {
            return false;
        };
        let Some(object_name) = ui_state.map.selected_object_name.clone() else {
            return false;
        };
        let Some(pointer_pos) = ui.input(|input| input.pointer.interact_pos()) else {
            return false;
        };
        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let Some(tilemap) = viewport.tilemap_mut() else {
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
        let Some(tilemap) = viewport.tilemap_mut() else {
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
        let tilemap = viewport.tilemap()?;
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
        let Some(tilemap) = viewport.tilemap() else {
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
        let Some(drag_state) = ui_state.map.object_move_drag else {
            return;
        };
        let Some(pointer_pos) = ui.input(|input| input.pointer.interact_pos()) else {
            return;
        };
        let world_pos =
            viewport.screen_to_world_pos_raw(pointer_pos, rect) - drag_state.grab_offset;
        let Some(tilemap) = viewport.tilemap_mut() else {
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
        let Some(tilemap) = viewport.tilemap() else {
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
        let Some(tilemap) = viewport.tilemap() else {
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
        let tilemap = viewport.tilemap()?;
        let tile_pos = MapPaintInteraction::tile_position_at_world(tilemap, world_pos)?;
        tilemap
            .get_tile_name(tile_pos.x, tile_pos.y)
            .ok()
            .map(ToString::to_string)
    }
}
