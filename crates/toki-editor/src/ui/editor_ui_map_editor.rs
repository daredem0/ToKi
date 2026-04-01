use super::{EditorUI, MapEditorState};
#[cfg(test)]
use crate::ui::undo_redo::EditorCommand;
use crate::ui::undo_redo::History;
use toki_core::assets::tilemap::TileMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum MapEditorTool {
    Drag,
    Brush,
    Fill,
    PickTile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEditorTileInfo {
    pub tile_x: u32,
    pub tile_y: u32,
    pub tile_name: String,
    pub solid: bool,
    pub trigger: bool,
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
        let Some(command) = self.history.undo_entry() else {
            return false;
        };
        if apply_map_editor_tilemap_snapshot(
            map_state,
            &command.map_name,
            command.is_draft,
            &command.before,
        ) {
            true
        } else {
            self.history.revert_failed_undo();
            false
        }
    }

    pub(crate) fn redo(&mut self, map_state: &mut MapEditorState) -> bool {
        let Some(command) = self.history.redo_entry() else {
            return false;
        };
        if apply_map_editor_tilemap_snapshot(
            map_state,
            &command.map_name,
            command.is_draft,
            &command.after,
        ) {
            true
        } else {
            self.history.revert_failed_redo();
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

pub(crate) fn sync_map_editor_selection(ui_state: &mut EditorUI, available_map_names: &[String]) {
    if has_unsaved_map_editor_changes(ui_state) {
        crate::ui::editor_context::map_state_mut(ui_state).map_load_requested = None;
        return;
    }

    if available_map_names.is_empty() {
        crate::ui::editor_context::map_state_mut(ui_state).active_map = None;
        crate::ui::editor_context::map_state_mut(ui_state).map_load_requested = None;
        return;
    }

    if ui_state
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
    if crate::ui::editor_context::map_state_mut(ui_state)
        .active_map
        .as_ref()
        != Some(&next_map)
    {
        crate::ui::editor_context::map_state_mut(ui_state).active_map = Some(next_map.clone());
        crate::ui::editor_context::map_state_mut(ui_state).map_load_requested = Some(next_map);
    }
}

pub(crate) fn begin_new_map_dialog(ui_state: &mut EditorUI) {
    crate::ui::editor_context::map_state_mut(ui_state).show_new_map_dialog = true;
    if crate::ui::editor_context::map_state_mut(ui_state)
        .new_map_name
        .trim()
        .is_empty()
    {
        crate::ui::editor_context::map_state_mut(ui_state).new_map_name = "new_map".to_string();
    }
    crate::ui::editor_context::map_state_mut(ui_state).new_map_width =
        crate::ui::editor_context::map_state_mut(ui_state)
            .new_map_width
            .max(1);
    crate::ui::editor_context::map_state_mut(ui_state).new_map_height =
        crate::ui::editor_context::map_state_mut(ui_state)
            .new_map_height
            .max(1);
}

pub(crate) fn submit_new_map_request(ui_state: &mut EditorUI) {
    let name = crate::ui::editor_context::map_state_mut(ui_state)
        .new_map_name
        .trim()
        .to_string();
    if name.is_empty() {
        return;
    }

    crate::ui::editor_context::map_state_mut(ui_state).new_map_requested = Some(NewMapRequest {
        name,
        width: crate::ui::editor_context::map_state_mut(ui_state)
            .new_map_width
            .max(1),
        height: crate::ui::editor_context::map_state_mut(ui_state)
            .new_map_height
            .max(1),
        tile_width: crate::ui::editor_context::map_state_mut(ui_state)
            .new_map_tile_width
            .max(1),
        tile_height: crate::ui::editor_context::map_state_mut(ui_state)
            .new_map_tile_height
            .max(1),
    });
    crate::ui::editor_context::map_state_mut(ui_state).show_new_map_dialog = false;
}

pub(crate) fn set_map_editor_draft(ui_state: &mut EditorUI, draft: MapEditorDraft) {
    crate::ui::editor_context::map_state_mut(ui_state).active_map = Some(draft.name.clone());
    crate::ui::editor_context::map_state_mut(ui_state).map_load_requested = None;
    crate::ui::editor_context::map_state_mut(ui_state).draft = Some(draft);
    crate::ui::editor_context::map_state_mut(ui_state).dirty = true;
    crate::ui::editor_context::map_state_mut(ui_state)
        .history
        .clear();
    crate::ui::editor_context::map_state_mut(ui_state).pending_tilemap_sync = None;
    crate::ui::editor_context::map_state_mut(ui_state).edit_before = None;
}

pub(crate) fn map_editor_selected_label(ui_state: &EditorUI) -> String {
    if let Some(draft) = &crate::ui::editor_context::map_state(ui_state).draft {
        return format!("{}*", draft.name);
    }

    crate::ui::editor_context::map_state(ui_state)
        .active_map
        .clone()
        .unwrap_or_else(|| "No map selected".to_string())
}

pub(crate) fn has_unsaved_map_editor_draft(ui_state: &EditorUI) -> bool {
    crate::ui::editor_context::map_state(ui_state)
        .draft
        .is_some()
}

pub(crate) fn has_unsaved_map_editor_changes(ui_state: &EditorUI) -> bool {
    crate::ui::editor_context::map_state(ui_state).dirty
        || crate::ui::editor_context::map_state(ui_state)
            .draft
            .is_some()
}

pub(crate) fn sync_map_editor_brush_selection(ui_state: &mut EditorUI, tile_names: &[String]) {
    if tile_names.is_empty() {
        crate::ui::editor_context::map_state_mut(ui_state).selected_tile = None;
        return;
    }

    if ui_state
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
    crate::ui::editor_context::map_state_mut(ui_state).selected_tile =
        Some(sorted_names[0].clone());
}

pub(crate) fn pick_map_editor_tile(ui_state: &mut EditorUI, tile_name: String) {
    crate::ui::editor_context::map_state_mut(ui_state).selected_tile = Some(tile_name);
    crate::ui::editor_context::map_state_mut(ui_state).tool = MapEditorTool::Brush;
}

pub(crate) fn mark_map_editor_dirty(ui_state: &mut EditorUI) {
    crate::ui::editor_context::map_state_mut(ui_state).dirty = true;
}

pub(crate) fn clear_map_editor_dirty(ui_state: &mut EditorUI) {
    crate::ui::editor_context::map_state_mut(ui_state).dirty = false;
}

pub(crate) fn finalize_saved_map_editor_draft(ui_state: &mut EditorUI, saved_name: String) {
    crate::ui::editor_context::map_state_mut(ui_state).draft = None;
    crate::ui::editor_context::map_state_mut(ui_state).dirty = false;
    crate::ui::editor_context::map_state_mut(ui_state).active_map = Some(saved_name.clone());
    crate::ui::editor_context::map_state_mut(ui_state).map_load_requested = Some(saved_name);
    crate::ui::editor_context::map_state_mut(ui_state).save_requested = false;
    crate::ui::editor_context::map_state_mut(ui_state)
        .history
        .clear();
    crate::ui::editor_context::map_state_mut(ui_state).pending_tilemap_sync = None;
    crate::ui::editor_context::map_state_mut(ui_state).edit_before = None;
}

pub(crate) fn finalize_saved_existing_map(ui_state: &mut EditorUI) {
    crate::ui::editor_context::map_state_mut(ui_state).dirty = false;
    crate::ui::editor_context::map_state_mut(ui_state).save_requested = false;
}

pub(crate) fn clear_map_editor_history(ui_state: &mut EditorUI) {
    crate::ui::editor_context::map_state_mut(ui_state)
        .history
        .clear();
    crate::ui::editor_context::map_state_mut(ui_state).pending_tilemap_sync = None;
    crate::ui::editor_context::map_state_mut(ui_state).edit_before = None;
}

pub(crate) fn begin_map_editor_edit(ui_state: &mut EditorUI, before: &TileMap) {
    if crate::ui::editor_context::map_state_mut(ui_state)
        .edit_before
        .is_none()
    {
        crate::ui::editor_context::map_state_mut(ui_state).edit_before = Some(before.clone());
    }
}

pub(crate) fn finish_map_editor_edit(ui_state: &mut EditorUI, after: &TileMap) -> bool {
    let Some(before) = crate::ui::editor_context::map_state_mut(ui_state)
        .edit_before
        .take()
    else {
        return false;
    };
    if before == *after {
        return false;
    }
    let map_name = ui_state
        .context::<crate::ui::editor_context::MapEditorContext>(super::CenterPanelTab::MapEditor)
        .expect("map editor context should exist")
        .map
        .active_map
        .clone()
        .unwrap_or_else(|| "map".to_string());
    let is_draft = crate::ui::editor_context::map_state_mut(ui_state)
        .draft
        .is_some();
    crate::ui::editor_context::map_state_mut(ui_state)
        .history
        .push(MapEditorEditCommand {
            map_name,
            is_draft,
            before,
            after: after.clone(),
        });
    crate::ui::editor_context::map_state_mut(ui_state).dirty = true;
    true
}

pub(crate) fn cancel_map_editor_edit(ui_state: &mut EditorUI) {
    crate::ui::editor_context::map_state_mut(ui_state).edit_before = None;
}

pub(crate) fn take_pending_map_editor_tilemap_sync(ui_state: &mut EditorUI) -> Option<TileMap> {
    crate::ui::editor_context::map_state_mut(ui_state)
        .pending_tilemap_sync
        .take()
}

impl EditorUI {
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
