use super::{CenterPanelTab, EditorUI};
use crate::ui::undo_redo::EditorCommand;
use std::path::PathBuf;
use toki_core::assets::tilemap::{MapObjectInstance, TileMap};

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
    undo_stack: Vec<MapEditorEditCommand>,
    redo_stack: Vec<MapEditorEditCommand>,
}

impl MapEditorHistory {
    fn push(&mut self, command: MapEditorEditCommand) {
        self.undo_stack.push(command);
        self.redo_stack.clear();
    }

    fn undo(&mut self, ui_state: &mut EditorUI) -> bool {
        let Some(command) = self.undo_stack.pop() else {
            return false;
        };
        if ui_state.apply_map_editor_tilemap_snapshot(
            &command.map_name,
            command.is_draft,
            &command.before,
        ) {
            self.redo_stack.push(command);
            true
        } else {
            self.undo_stack.push(command);
            false
        }
    }

    fn redo(&mut self, ui_state: &mut EditorUI) -> bool {
        let Some(command) = self.redo_stack.pop() else {
            return false;
        };
        if ui_state.apply_map_editor_tilemap_snapshot(
            &command.map_name,
            command.is_draft,
            &command.after,
        ) {
            self.undo_stack.push(command);
            true
        } else {
            self.redo_stack.push(command);
            false
        }
    }

    fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
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
            self.map.map_load_requested = None;
            return;
        }

        if available_map_names.is_empty() {
            self.map.active_map = None;
            self.map.map_load_requested = None;
            return;
        }

        if self
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
        if self.map.active_map.as_ref() != Some(&next_map) {
            self.map.active_map = Some(next_map.clone());
            self.map.map_load_requested = Some(next_map);
        }
    }

    pub fn begin_new_map_dialog(&mut self) {
        self.map.show_new_map_dialog = true;
        if self.map.new_map_name.trim().is_empty() {
            self.map.new_map_name = "new_map".to_string();
        }
        self.map.new_map_width = self.map.new_map_width.max(1);
        self.map.new_map_height = self.map.new_map_height.max(1);
    }

    pub fn submit_new_map_request(&mut self) {
        let name = self.map.new_map_name.trim().to_string();
        if name.is_empty() {
            return;
        }

        self.map.new_map_requested = Some(NewMapRequest {
            name,
            width: self.map.new_map_width.max(1),
            height: self.map.new_map_height.max(1),
            tile_width: self.map.new_map_tile_width.max(1),
            tile_height: self.map.new_map_tile_height.max(1),
        });
        self.map.show_new_map_dialog = false;
    }

    pub fn set_map_editor_draft(&mut self, draft: MapEditorDraft) {
        self.map.active_map = Some(draft.name.clone());
        self.map.map_load_requested = None;
        self.map.draft = Some(draft);
        self.map.dirty = true;
        self.map.history.clear();
        self.map.pending_tilemap_sync = None;
        self.map.edit_before = None;
        self.map.selected_object_info = None;
        self.map.object_edit_requested = None;
        self.map.object_move_drag = None;
    }

    pub fn map_editor_selected_label(&self) -> String {
        if let Some(draft) = &self.map.draft {
            return format!("{}*", draft.name);
        }

        self.map
            .active_map
            .clone()
            .unwrap_or_else(|| "No map selected".to_string())
    }

    pub fn has_unsaved_map_editor_draft(&self) -> bool {
        self.map.draft.is_some()
    }

    pub fn has_unsaved_map_editor_changes(&self) -> bool {
        self.map.dirty || self.map.draft.is_some()
    }

    pub fn sync_map_editor_brush_selection(&mut self, tile_names: &[String]) {
        if tile_names.is_empty() {
            self.map.selected_tile = None;
            return;
        }

        if self
            .map
            .selected_tile
            .as_ref()
            .is_some_and(|selected| tile_names.iter().any(|name| name == selected))
        {
            return;
        }

        let mut sorted_names = tile_names.to_vec();
        sorted_names.sort();
        self.map.selected_tile = Some(sorted_names[0].clone());
    }

    pub fn sync_map_editor_object_sheet_selection(&mut self, sheet_names: &[String]) {
        if sheet_names.is_empty() {
            self.map.selected_object_sheet = None;
            return;
        }

        if self
            .map
            .selected_object_sheet
            .as_ref()
            .is_some_and(|selected| sheet_names.iter().any(|name| name == selected))
        {
            return;
        }

        let mut sorted_names = sheet_names.to_vec();
        sorted_names.sort();
        self.map.selected_object_sheet = Some(sorted_names[0].clone());
    }

    pub fn sync_map_editor_object_selection(&mut self, object_names: &[String]) {
        if object_names.is_empty() {
            self.map.selected_object_name = None;
            return;
        }

        if self
            .map
            .selected_object_name
            .as_ref()
            .is_some_and(|selected| object_names.iter().any(|name| name == selected))
        {
            return;
        }

        let mut sorted_names = object_names.to_vec();
        sorted_names.sort();
        self.map.selected_object_name = Some(sorted_names[0].clone());
    }

    pub fn pick_map_editor_tile(&mut self, tile_name: String) {
        self.map.selected_tile = Some(tile_name);
        self.map.tool = MapEditorTool::Brush;
    }

    pub fn mark_map_editor_dirty(&mut self) {
        self.map.dirty = true;
    }

    pub fn clear_map_editor_dirty(&mut self) {
        self.map.dirty = false;
    }

    pub fn finalize_saved_map_editor_draft(&mut self, saved_name: String) {
        self.map.draft = None;
        self.map.dirty = false;
        self.map.active_map = Some(saved_name.clone());
        self.map.map_load_requested = Some(saved_name);
        self.map.save_requested = false;
        self.map.history.clear();
        self.map.pending_tilemap_sync = None;
        self.map.edit_before = None;
    }

    pub fn finalize_saved_existing_map(&mut self) {
        self.map.dirty = false;
        self.map.save_requested = false;
    }

    pub fn clear_map_editor_history(&mut self) {
        self.map.history.clear();
        self.map.pending_tilemap_sync = None;
        self.map.edit_before = None;
    }

    pub fn select_map_editor_object(&mut self, index: usize, object: &MapObjectInstance) {
        self.map.selected_object_info = Some(MapEditorObjectInfo {
            index,
            sheet: object.sheet.clone(),
            object_name: object.object_name.clone(),
            position: object.position,
            size_px: object.size_px,
            visible: object.visible,
            solid: object.solid,
        });
        self.map.selected_tile_info = None;
    }

    pub fn clear_map_editor_object_selection(&mut self) {
        self.map.selected_object_info = None;
        self.map.object_move_drag = None;
        self.map.object_edit_requested = None;
    }

    pub fn sync_selected_map_editor_object_from_tilemap(&mut self, tilemap: &TileMap) {
        let Some(selected) = self.map.selected_object_info.as_mut() else {
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
        selected.visible = object.visible;
        selected.solid = object.solid;
    }

    pub fn begin_map_object_move_drag(&mut self, object_index: usize, grab_offset: glam::Vec2) {
        self.map.object_move_drag = Some(MapObjectMoveDragState {
            object_index,
            grab_offset,
        });
    }

    pub fn is_map_object_move_drag_active(&self) -> bool {
        self.map.object_move_drag.is_some()
    }

    pub fn finish_map_object_move_drag(&mut self) {
        self.map.object_move_drag = None;
    }

    pub fn queue_map_editor_object_property_edit(
        &mut self,
        object_index: usize,
        visible: bool,
        solid: bool,
    ) {
        self.map.object_edit_requested = Some(MapEditorObjectPropertyEditRequest {
            object_index,
            visible,
            solid,
        });
        if let Some(selected) = self.map.selected_object_info.as_mut() {
            if selected.index == object_index {
                selected.visible = visible;
                selected.solid = solid;
            }
        }
    }

    pub fn take_map_editor_object_property_edit_request(
        &mut self,
    ) -> Option<MapEditorObjectPropertyEditRequest> {
        self.map.object_edit_requested.take()
    }

    pub fn begin_map_editor_edit(&mut self, before: &TileMap) {
        if self.map.edit_before.is_none() {
            self.map.edit_before = Some(before.clone());
        }
    }

    pub fn finish_map_editor_edit(&mut self, after: &TileMap) -> bool {
        let Some(before) = self.map.edit_before.take() else {
            return false;
        };
        if before == *after {
            return false;
        }
        let map_name = self
            .map
            .active_map
            .clone()
            .unwrap_or_else(|| "map".to_string());
        let is_draft = self.map.draft.is_some();
        self.map.history.push(MapEditorEditCommand {
            map_name,
            is_draft,
            before,
            after: after.clone(),
        });
        self.map.dirty = true;
        true
    }

    pub fn cancel_map_editor_edit(&mut self) {
        self.map.edit_before = None;
    }

    fn apply_map_editor_tilemap_snapshot(
        &mut self,
        map_name: &str,
        is_draft: bool,
        tilemap: &TileMap,
    ) -> bool {
        if self.map.active_map.as_deref() != Some(map_name) {
            return false;
        }

        if is_draft {
            let Some(draft) = self.map.draft.as_mut() else {
                return false;
            };
            if draft.name != map_name {
                return false;
            }
            draft.tilemap = tilemap.clone();
        } else if self.map.draft.is_some() {
            return false;
        }

        self.map.pending_tilemap_sync = Some(tilemap.clone());
        self.map.dirty = true;
        true
    }

    pub fn take_pending_map_editor_tilemap_sync(&mut self) -> Option<TileMap> {
        self.map.pending_tilemap_sync.take()
    }

    pub fn execute_command(&mut self, command: EditorCommand) -> bool {
        let mut history = std::mem::take(&mut self.command_history);
        let changed = history.execute(command, self, None);
        self.command_history = history;
        changed
    }

    pub fn execute_command_with_project(
        &mut self,
        project: &mut crate::project::Project,
        command: EditorCommand,
    ) -> bool {
        let mut history = std::mem::take(&mut self.command_history);
        let changed = history.execute(command, self, Some(project));
        self.command_history = history;
        changed
    }

    pub fn undo(&mut self) -> bool {
        if self.center_panel_tab == CenterPanelTab::MapEditor && self.map.history.can_undo() {
            let mut history = std::mem::take(&mut self.map.history);
            let undone = history.undo(self);
            self.map.history = history;
            return undone;
        }
        let mut history = std::mem::take(&mut self.command_history);
        let undone = history.undo(self, None);
        self.command_history = history;
        undone
    }

    pub fn undo_with_project(&mut self, project: &mut crate::project::Project) -> bool {
        if self.center_panel_tab == CenterPanelTab::MapEditor && self.map.history.can_undo() {
            let mut history = std::mem::take(&mut self.map.history);
            let undone = history.undo(self);
            self.map.history = history;
            return undone;
        }
        let mut history = std::mem::take(&mut self.command_history);
        let undone = history.undo(self, Some(project));
        self.command_history = history;
        undone
    }

    pub fn redo(&mut self) -> bool {
        if self.center_panel_tab == CenterPanelTab::MapEditor && self.map.history.can_redo() {
            let mut history = std::mem::take(&mut self.map.history);
            let redone = history.redo(self);
            self.map.history = history;
            return redone;
        }
        let mut history = std::mem::take(&mut self.command_history);
        let redone = history.redo(self, None);
        self.command_history = history;
        redone
    }

    pub fn redo_with_project(&mut self, project: &mut crate::project::Project) -> bool {
        if self.center_panel_tab == CenterPanelTab::MapEditor && self.map.history.can_redo() {
            let mut history = std::mem::take(&mut self.map.history);
            let redone = history.redo(self);
            self.map.history = history;
            return redone;
        }
        let mut history = std::mem::take(&mut self.command_history);
        let redone = history.redo(self, Some(project));
        self.command_history = history;
        redone
    }

    pub fn can_undo(&self) -> bool {
        if self.center_panel_tab == CenterPanelTab::MapEditor {
            self.map.history.can_undo()
        } else {
            self.command_history.can_undo()
        }
    }

    pub fn can_redo(&self) -> bool {
        if self.center_panel_tab == CenterPanelTab::MapEditor {
            self.map.history.can_redo()
        } else {
            self.command_history.can_redo()
        }
    }
}
