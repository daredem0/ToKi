use super::{EditorUI, MapEditorState};
#[cfg(test)]
use crate::ui::undo_redo::EditorCommand;
use crate::ui::undo_redo::History;
use std::path::PathBuf;
use toki_core::assets::tilemap::{MapObjectInstance, TileMap};
use toki_core::entity::EntityGrounding;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MapEditorTool {
    Drag,
    Brush,
    Fill,
    PickTile,
    PlaceObject,
    DeleteObject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEditorTileInfo {
    pub tile_x: u32,
    pub tile_y: u32,
    pub tile_name: String,
    pub solid: bool,
    pub trigger: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEditorObjectInfo {
    pub index: usize,
    pub sheet: PathBuf,
    pub object_name: String,
    pub position: glam::UVec2,
    pub size_px: glam::UVec2,
    pub grounding: EntityGrounding,
    pub visible: bool,
    pub solid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapObjectMoveDragState {
    pub object_index: usize,
    pub grab_offset: glam::Vec2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEditorObjectPropertyEditRequest {
    pub object_index: usize,
    pub grounding: EntityGrounding,
    pub visible: bool,
    pub solid: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapEditorDraft {
    pub name: String,
    pub tilemap: TileMap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapEditorEditCommand {
    pub map_name: String,
    pub is_draft: bool,
    pub before: TileMap,
    pub after: TileMap,
}

#[derive(Debug, Clone, Default)]
pub struct MapEditorHistory {
    history: History<MapEditorEditCommand>,
}

impl MapEditorHistory {
    pub(crate) fn push(&mut self, command: MapEditorEditCommand) {
        self.history.push(command);
    }

    pub(crate) fn undo(&mut self, map_state: &mut MapEditorState) -> bool {
        let Some(command) = self.history.take_undo() else {
            return false;
        };
        if apply_map_editor_tilemap_snapshot(map_state, &command.map_name, command.is_draft, &command.before) {
            self.history.restore_redo(command);
            true
        } else {
            self.history.restore_undo(command);
            false
        }
    }

    pub(crate) fn redo(&mut self, map_state: &mut MapEditorState) -> bool {
        let Some(command) = self.history.take_redo() else {
            return false;
        };
        if apply_map_editor_tilemap_snapshot(map_state, &command.map_name, command.is_draft, &command.after) {
            self.history.restore_undo(command);
            true
        } else {
            self.history.restore_redo(command);
            false
        }
    }

    pub(crate) fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub(crate) fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub(crate) fn clear(&mut self) {
        self.history.clear();
    }
}

fn apply_map_editor_tilemap_snapshot(
    map_state: &mut MapEditorState,
    map_name: &str,
    is_draft: bool,
    tilemap: &TileMap,
) -> bool {
    if map_state.active_map.as_deref() != Some(map_name) {
        return false;
    }

    if is_draft {
        let Some(draft) = map_state.draft.as_mut() else {
            return false;
        };
        if draft.name != map_name {
            return false;
        }
        draft.tilemap = tilemap.clone();
    } else if map_state.draft.is_some() {
        return false;
    }

    map_state.pending_tilemap_sync = Some(tilemap.clone());
    map_state.dirty = true;
    true
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMapRequest {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
}

impl EditorUI {
    pub fn sync_map_editor_selection(&mut self, available_map_names: &[String]) {
        if self.has_unsaved_map_editor_changes() {
            crate::ui::editor_context::map_state_mut(self).map_load_requested = None;
            return;
        }

        if available_map_names.is_empty() {
            crate::ui::editor_context::map_state_mut(self).active_map = None;
            crate::ui::editor_context::map_state_mut(self).map_load_requested = None;
            return;
        }

        if self
            .context::<crate::ui::editor_context::MapEditorContext>(super::CenterPanelTab::MapEditor)
            .expect("map editor context should exist")
            .map
            .active_map
            .as_ref()
            .is_some_and(|selected| available_map_names.iter().any(|name| name == selected))
        {
            return;
        }

        let mut sorted_names = available_map_names.to_vec();
        sorted_names.sort();
        let next_map = sorted_names[0].clone();
        if crate::ui::editor_context::map_state_mut(self).active_map.as_ref() != Some(&next_map) {
            crate::ui::editor_context::map_state_mut(self).active_map = Some(next_map.clone());
            crate::ui::editor_context::map_state_mut(self).map_load_requested = Some(next_map);
        }
    }

    pub fn begin_new_map_dialog(&mut self) {
        crate::ui::editor_context::map_state_mut(self).show_new_map_dialog = true;
        if crate::ui::editor_context::map_state_mut(self).new_map_name.trim().is_empty() {
            crate::ui::editor_context::map_state_mut(self).new_map_name = "new_map".to_string();
        }
        crate::ui::editor_context::map_state_mut(self).new_map_width = crate::ui::editor_context::map_state_mut(self).new_map_width.max(1);
        crate::ui::editor_context::map_state_mut(self).new_map_height = crate::ui::editor_context::map_state_mut(self).new_map_height.max(1);
    }

    pub fn submit_new_map_request(&mut self) {
        let name = crate::ui::editor_context::map_state_mut(self).new_map_name.trim().to_string();
        if name.is_empty() {
            return;
        }

        crate::ui::editor_context::map_state_mut(self).new_map_requested = Some(NewMapRequest {
            name,
            width: crate::ui::editor_context::map_state_mut(self).new_map_width.max(1),
            height: crate::ui::editor_context::map_state_mut(self).new_map_height.max(1),
            tile_width: crate::ui::editor_context::map_state_mut(self).new_map_tile_width.max(1),
            tile_height: crate::ui::editor_context::map_state_mut(self).new_map_tile_height.max(1),
        });
        crate::ui::editor_context::map_state_mut(self).show_new_map_dialog = false;
    }

    pub fn set_map_editor_draft(&mut self, draft: MapEditorDraft) {
        crate::ui::editor_context::map_state_mut(self).active_map = Some(draft.name.clone());
        crate::ui::editor_context::map_state_mut(self).map_load_requested = None;
        crate::ui::editor_context::map_state_mut(self).draft = Some(draft);
        crate::ui::editor_context::map_state_mut(self).dirty = true;
        crate::ui::editor_context::map_state_mut(self).history.clear();
        crate::ui::editor_context::map_state_mut(self).pending_tilemap_sync = None;
        crate::ui::editor_context::map_state_mut(self).edit_before = None;
        crate::ui::editor_context::map_state_mut(self).selected_object_info = None;
        crate::ui::editor_context::map_state_mut(self).object_edit_requested = None;
        crate::ui::editor_context::map_state_mut(self).object_move_drag = None;
    }

    pub fn map_editor_selected_label(&self) -> String {
        if let Some(draft) = &crate::ui::editor_context::map_state(self).draft {
            return format!("{}*", draft.name);
        }

        crate::ui::editor_context::map_state(self)
            .active_map
            .clone()
            .unwrap_or_else(|| "No map selected".to_string())
    }

    pub fn has_unsaved_map_editor_draft(&self) -> bool {
        crate::ui::editor_context::map_state(self).draft.is_some()
    }

    pub fn has_unsaved_map_editor_changes(&self) -> bool {
        crate::ui::editor_context::map_state(self).dirty || crate::ui::editor_context::map_state(self).draft.is_some()
    }

    pub fn sync_map_editor_brush_selection(&mut self, tile_names: &[String]) {
        if tile_names.is_empty() {
            crate::ui::editor_context::map_state_mut(self).selected_tile = None;
            return;
        }

        if self
            .context::<crate::ui::editor_context::MapEditorContext>(super::CenterPanelTab::MapEditor)
            .expect("map editor context should exist")
            .map
            .selected_tile
            .as_ref()
            .is_some_and(|selected| tile_names.iter().any(|name| name == selected))
        {
            return;
        }

        let mut sorted_names = tile_names.to_vec();
        sorted_names.sort();
        crate::ui::editor_context::map_state_mut(self).selected_tile = Some(sorted_names[0].clone());
    }

    pub fn sync_map_editor_object_sheet_selection(&mut self, sheet_names: &[String]) {
        if sheet_names.is_empty() {
            crate::ui::editor_context::map_state_mut(self).selected_object_sheet = None;
            return;
        }

        if self
            .context::<crate::ui::editor_context::MapEditorContext>(super::CenterPanelTab::MapEditor)
            .expect("map editor context should exist")
            .map
            .selected_object_sheet
            .as_ref()
            .is_some_and(|selected| sheet_names.iter().any(|name| name == selected))
        {
            return;
        }

        let mut sorted_names = sheet_names.to_vec();
        sorted_names.sort();
        crate::ui::editor_context::map_state_mut(self).selected_object_sheet = Some(sorted_names[0].clone());
    }

    pub fn sync_map_editor_object_selection(&mut self, object_names: &[String]) {
        if object_names.is_empty() {
            crate::ui::editor_context::map_state_mut(self).selected_object_name = None;
            return;
        }

        if self
            .context::<crate::ui::editor_context::MapEditorContext>(super::CenterPanelTab::MapEditor)
            .expect("map editor context should exist")
            .map
            .selected_object_name
            .as_ref()
            .is_some_and(|selected| object_names.iter().any(|name| name == selected))
        {
            return;
        }

        let mut sorted_names = object_names.to_vec();
        sorted_names.sort();
        crate::ui::editor_context::map_state_mut(self).selected_object_name = Some(sorted_names[0].clone());
    }

    pub fn pick_map_editor_tile(&mut self, tile_name: String) {
        crate::ui::editor_context::map_state_mut(self).selected_tile = Some(tile_name);
        crate::ui::editor_context::map_state_mut(self).tool = MapEditorTool::Brush;
    }

    pub fn mark_map_editor_dirty(&mut self) {
        crate::ui::editor_context::map_state_mut(self).dirty = true;
    }

    pub fn clear_map_editor_dirty(&mut self) {
        crate::ui::editor_context::map_state_mut(self).dirty = false;
    }

    pub fn finalize_saved_map_editor_draft(&mut self, saved_name: String) {
        crate::ui::editor_context::map_state_mut(self).draft = None;
        crate::ui::editor_context::map_state_mut(self).dirty = false;
        crate::ui::editor_context::map_state_mut(self).active_map = Some(saved_name.clone());
        crate::ui::editor_context::map_state_mut(self).map_load_requested = Some(saved_name);
        crate::ui::editor_context::map_state_mut(self).save_requested = false;
        crate::ui::editor_context::map_state_mut(self).history.clear();
        crate::ui::editor_context::map_state_mut(self).pending_tilemap_sync = None;
        crate::ui::editor_context::map_state_mut(self).edit_before = None;
    }

    pub fn finalize_saved_existing_map(&mut self) {
        crate::ui::editor_context::map_state_mut(self).dirty = false;
        crate::ui::editor_context::map_state_mut(self).save_requested = false;
    }

    pub fn clear_map_editor_history(&mut self) {
        crate::ui::editor_context::map_state_mut(self).history.clear();
        crate::ui::editor_context::map_state_mut(self).pending_tilemap_sync = None;
        crate::ui::editor_context::map_state_mut(self).edit_before = None;
    }

    pub fn select_map_editor_object(&mut self, index: usize, object: &MapObjectInstance) {
        crate::ui::editor_context::map_state_mut(self).selected_object_info = Some(MapEditorObjectInfo {
            index,
            sheet: object.sheet.clone(),
            object_name: object.object_name.clone(),
            position: object.position,
            size_px: object.size_px,
            grounding: object.grounding.clone(),
            visible: object.visible,
            solid: object.solid,
        });
        crate::ui::editor_context::map_state_mut(self).selected_tile_info = None;
    }

    pub fn clear_map_editor_object_selection(&mut self) {
        crate::ui::editor_context::map_state_mut(self).selected_object_info = None;
        crate::ui::editor_context::map_state_mut(self).object_move_drag = None;
        crate::ui::editor_context::map_state_mut(self).object_edit_requested = None;
    }

    pub fn sync_selected_map_editor_object_from_tilemap(&mut self, tilemap: &TileMap) {
        let Some(selected) = crate::ui::editor_context::map_state_mut(self).selected_object_info.as_mut() else {
            return;
        };
        let Some(object) = tilemap.objects.get(selected.index) else {
            self.clear_map_editor_object_selection();
            return;
        };
        selected.sheet = object.sheet.clone();
        selected.object_name = object.object_name.clone();
        selected.position = object.position;
        selected.size_px = object.size_px;
        selected.grounding = object.grounding.clone();
        selected.visible = object.visible;
        selected.solid = object.solid;
    }

    pub fn begin_map_object_move_drag(&mut self, object_index: usize, grab_offset: glam::Vec2) {
        crate::ui::editor_context::map_state_mut(self).object_move_drag = Some(MapObjectMoveDragState {
            object_index,
            grab_offset,
        });
    }

    pub fn is_map_object_move_drag_active(&self) -> bool {
        crate::ui::editor_context::map_state(self).object_move_drag.is_some()
    }

    pub fn finish_map_object_move_drag(&mut self) {
        crate::ui::editor_context::map_state_mut(self).object_move_drag = None;
    }

    pub fn queue_map_editor_object_property_edit(
        &mut self,
        object_index: usize,
        grounding: EntityGrounding,
        visible: bool,
        solid: bool,
    ) {
        crate::ui::editor_context::map_state_mut(self).object_edit_requested = Some(MapEditorObjectPropertyEditRequest {
            object_index,
            grounding: grounding.clone(),
            visible,
            solid,
        });
        if let Some(selected) = crate::ui::editor_context::map_state_mut(self).selected_object_info.as_mut() {
            if selected.index == object_index {
                selected.grounding = grounding;
                selected.visible = visible;
                selected.solid = solid;
            }
        }
    }

    pub fn take_map_editor_object_property_edit_request(
        &mut self,
    ) -> Option<MapEditorObjectPropertyEditRequest> {
        crate::ui::editor_context::map_state_mut(self).object_edit_requested.take()
    }

    pub fn begin_map_editor_edit(&mut self, before: &TileMap) {
        if crate::ui::editor_context::map_state_mut(self).edit_before.is_none() {
            crate::ui::editor_context::map_state_mut(self).edit_before = Some(before.clone());
        }
    }

    pub fn finish_map_editor_edit(&mut self, after: &TileMap) -> bool {
        let Some(before) = crate::ui::editor_context::map_state_mut(self).edit_before.take() else {
            return false;
        };
        if before == *after {
            return false;
        }
        let map_name = self
            .context::<crate::ui::editor_context::MapEditorContext>(super::CenterPanelTab::MapEditor)
            .expect("map editor context should exist")
            .map
            .active_map
            .clone()
            .unwrap_or_else(|| "map".to_string());
        let is_draft = crate::ui::editor_context::map_state_mut(self).draft.is_some();
        crate::ui::editor_context::map_state_mut(self).history.push(MapEditorEditCommand {
            map_name,
            is_draft,
            before,
            after: after.clone(),
        });
        crate::ui::editor_context::map_state_mut(self).dirty = true;
        true
    }

    pub fn cancel_map_editor_edit(&mut self) {
        crate::ui::editor_context::map_state_mut(self).edit_before = None;
    }

    pub fn take_pending_map_editor_tilemap_sync(&mut self) -> Option<TileMap> {
        crate::ui::editor_context::map_state_mut(self).pending_tilemap_sync.take()
    }

    #[cfg(test)]
    pub fn execute_command(&mut self, command: EditorCommand) -> bool {
        crate::editor_services::commands::execute(self, command)
    }

    #[cfg(test)]
    pub fn undo(&mut self) -> bool {
        crate::editor_services::commands::undo(self)
    }

    #[cfg(test)]
    pub fn undo_with_project(&mut self, project: &mut crate::project::Project) -> bool {
        crate::editor_services::commands::undo_with_project(self, project)
    }

    #[cfg(test)]
    pub fn redo(&mut self) -> bool {
        crate::editor_services::commands::redo(self)
    }

    #[cfg(test)]
    pub fn redo_with_project(&mut self, project: &mut crate::project::Project) -> bool {
        crate::editor_services::commands::redo_with_project(self, project)
    }

    #[cfg(test)]
    pub fn can_undo(&self) -> bool {
        crate::editor_services::commands::can_undo(self)
    }

    #[cfg(test)]
    pub fn can_redo(&self) -> bool {
        crate::editor_services::commands::can_redo(self)
    }
}
