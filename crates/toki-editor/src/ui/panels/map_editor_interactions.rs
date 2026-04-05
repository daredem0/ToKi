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
        if !rect.contains(pointer_pos) {
            return false;
        }

        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let Some(tilemap) = viewport.tilemap_mut() else {
            return false;
        };
        if ui.input(|input| input.pointer.primary_pressed()) {
            crate::ui::editor_ui::begin_map_editor_edit(ui_state, tilemap);
        }
        let Some(tile_pos) = MapPaintInteraction::tile_position_at_world(tilemap, world_pos) else {
            return false;
        };

        let map_state = crate::ui::editor_context::map_state(ui_state);
        let active_layer = map_state.active_layer;
        let solid_stamp = map_state.brush_stamp_solid;
        let mut changed = MapPaintInteraction::paint_brush(
            tilemap,
            active_layer,
            tile_pos,
            selected_tile,
            brush_size_tiles,
        );
        if let Some(solid) = solid_stamp {
            changed |= Self::stamp_solid_in_footprint(
                tilemap,
                active_layer,
                tile_pos,
                brush_size_tiles,
                solid,
            );
        }
        if changed {
            viewport.mark_dirty();
            return true;
        }

        false
    }

    fn stamp_solid_in_footprint(
        tilemap: &mut TileMap,
        layer: usize,
        center: glam::UVec2,
        brush_size: u32,
        solid: bool,
    ) -> bool {
        let Some((start, end)) =
            MapPaintInteraction::brush_footprint_bounds(tilemap, center, brush_size)
        else {
            return false;
        };
        let mut changed = false;
        for y in start.y..end.y {
            for x in start.x..end.x {
                changed |= MapPaintInteraction::stamp_collision_override(
                    tilemap,
                    layer,
                    glam::UVec2::new(x, y),
                    solid,
                );
            }
        }
        changed
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
        crate::ui::editor_ui::begin_map_editor_edit(ui_state, tilemap);

        let active_layer = crate::ui::editor_context::map_state(ui_state).active_layer;
        if MapPaintInteraction::fill_all(tilemap, active_layer, selected_tile) {
            crate::ui::editor_ui::finish_map_editor_edit(ui_state, tilemap);
            viewport.mark_dirty();
            return true;
        }

        crate::ui::editor_ui::cancel_map_editor_edit(ui_state);
        false
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
        if !rect.contains(pointer_pos) {
            return Some(None);
        }
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
        if !rect.contains(pointer_pos) {
            return None;
        }
        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let tilemap = viewport.tilemap()?;
        let tile_pos = MapPaintInteraction::tile_position_at_world(tilemap, world_pos)?;
        tilemap
            .get_tile_name(tile_pos.x, tile_pos.y)
            .ok()
            .map(ToString::to_string)
    }
}
