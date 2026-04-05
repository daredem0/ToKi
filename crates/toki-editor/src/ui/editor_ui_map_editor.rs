use super::{EditorUI, MapEditorState};
use crate::config::EditorConfig;
#[cfg(test)]
use crate::ui::undo_redo::EditorCommand;
use crate::ui::undo_redo::History;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use toki_core::assets::atlas::{AtlasMeta, ImportedAutoTile};
use toki_core::assets::tilemap::{TileLayer, TileMap};
use toki_core::assets::tileset::{TileSetAtlasSource, TileSetEntryKind, TileSetMeta};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MapEditorBrushKind {
    Tile,
    AutoTileGroup,
    AnimatedTile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MapEditorBrushEntry {
    pub id: String,
    pub kind: MapEditorBrushKind,
    pub display_label: String,
    pub preview_tile_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedMapEditorBrushSource {
    pub brush_entries: Vec<MapEditorBrushEntry>,
    pub tileset: TileSetMeta,
    pub atlases: HashMap<String, TileSetAtlasSource>,
}

#[allow(dead_code)]
fn imported_auto_tile_for_group<'a>(
    imported_auto_tiles: &'a [ImportedAutoTile],
    group_name: &str,
) -> Option<&'a ImportedAutoTile> {
    imported_auto_tiles
        .iter()
        .find(|import| import.group_name == group_name)
}

#[allow(dead_code)]
fn imported_auto_tile_display_name(imported: &ImportedAutoTile) -> Option<String> {
    imported
        .source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::trim)
        .filter(|stem| !stem.is_empty())
        .map(ToOwned::to_owned)
}

fn auto_tile_preview_tile_id(atlas: &AtlasMeta, group_name: &str) -> Option<String> {
    let group = atlas.get_auto_tile_group(group_name)?;
    if let Some(preview_tile) = group
        .preview_tile
        .as_ref()
        .filter(|tile| atlas.tiles.contains_key(*tile))
    {
        return Some(preview_tile.clone());
    }

    let mut variants = group.variants.iter().collect::<Vec<_>>();
    variants.sort_by_key(|(mask, _)| **mask);
    variants.into_iter().find_map(|(_, tile_name)| {
        atlas
            .tiles
            .contains_key(tile_name)
            .then(|| tile_name.clone())
    })
}

fn animated_tile_preview_tile_id(atlas: &AtlasMeta, tile_name: &str) -> Option<String> {
    atlas.get_animated_tile(tile_name).and_then(|animated| {
        animated
            .frames
            .iter()
            .find(|frame| atlas.tiles.contains_key(frame.as_str()))
            .cloned()
    })
}

#[allow(dead_code)]
fn brush_entry_for_tile(atlas: &AtlasMeta, tile_name: &str) -> MapEditorBrushEntry {
    if atlas.is_auto_tile_group(tile_name) {
        let display_name = imported_auto_tile_for_group(&atlas.imported_auto_tiles, tile_name)
            .and_then(imported_auto_tile_display_name)
            .unwrap_or_else(|| tile_name.to_string());
        return MapEditorBrushEntry {
            id: tile_name.to_string(),
            kind: MapEditorBrushKind::AutoTileGroup,
            display_label: format!("[A] {display_name}"),
            preview_tile_id: auto_tile_preview_tile_id(atlas, tile_name),
        };
    }

    if atlas.is_animated_tile(tile_name) {
        return MapEditorBrushEntry {
            id: tile_name.to_string(),
            kind: MapEditorBrushKind::AnimatedTile,
            display_label: format!("[~] {tile_name}"),
            preview_tile_id: animated_tile_preview_tile_id(atlas, tile_name),
        };
    }

    MapEditorBrushEntry {
        id: tile_name.to_string(),
        kind: MapEditorBrushKind::Tile,
        display_label: tile_name.to_string(),
        preview_tile_id: Some(tile_name.to_string()),
    }
}

#[allow(dead_code)]
pub(crate) fn build_map_editor_brush_entries(atlas: &AtlasMeta) -> Vec<MapEditorBrushEntry> {
    let mut brush_ids = atlas.tiles.keys().cloned().collect::<Vec<_>>();
    for group_name in atlas.auto_tile_groups.keys() {
        if !brush_ids.contains(group_name) {
            brush_ids.push(group_name.clone());
        }
    }
    for anim_name in atlas.animated_tiles.keys() {
        if !brush_ids.contains(anim_name) {
            brush_ids.push(anim_name.clone());
        }
    }
    brush_ids.sort();
    brush_ids
        .into_iter()
        .map(|tile_name| brush_entry_for_tile(atlas, &tile_name))
        .collect()
}

pub(crate) fn selected_map_editor_brush_entry<'a>(
    brush_entries: &'a [MapEditorBrushEntry],
    selected_tile: Option<&str>,
) -> Option<&'a MapEditorBrushEntry> {
    let selected_tile = selected_tile?;
    brush_entries.iter().find(|entry| entry.id == selected_tile)
}

pub(crate) fn map_editor_brush_entry_atlas_name(entry_id: &str) -> Option<&str> {
    entry_id.split('/').next().filter(|segment| !segment.is_empty())
}

fn build_map_editor_brush_entries_from_tileset(
    tileset: &TileSetMeta,
    atlases: &HashMap<String, TileSetAtlasSource>,
) -> Vec<MapEditorBrushEntry> {
    let mut entries = tileset
        .entries
        .iter()
        .filter_map(|(entry_id, entry)| {
            let atlas = atlases
                .get(&entry.atlas_name)
                .or_else(|| atlases.get(toki_core::project_assets::normalize_asset_name(&entry.atlas_name)))?;
            let (kind, prefix, preview_tile_id) = match entry.kind {
                TileSetEntryKind::Tile => (
                    MapEditorBrushKind::Tile,
                    "",
                    Some(entry.source_name.clone()),
                ),
                TileSetEntryKind::AutoTileGroup => (
                    MapEditorBrushKind::AutoTileGroup,
                    "[A] ",
                    auto_tile_preview_tile_id(&atlas.meta, &entry.source_name),
                ),
                TileSetEntryKind::AnimatedTile => (
                    MapEditorBrushKind::AnimatedTile,
                    "[~] ",
                    animated_tile_preview_tile_id(&atlas.meta, &entry.source_name),
                ),
            };
            Some(MapEditorBrushEntry {
                id: entry_id.clone(),
                kind,
                display_label: format!(
                    "{prefix}{}",
                    entry.display_name.clone().unwrap_or_else(|| entry.source_name.clone())
                ),
                preview_tile_id,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.display_label.cmp(&right.display_label).then(left.id.cmp(&right.id)));
    entries
}

pub(crate) fn resolve_map_editor_tileset_path(project_path: &Path, tilemap: &TileMap) -> Option<PathBuf> {
    toki_core::project_assets::resolve_tilemap_tileset_path(
        project_path,
        &project_path.join("assets").join("tilemaps").join("__editor__.json"),
        tilemap,
    )
}

#[allow(dead_code)]
pub(crate) fn load_map_editor_tileset_meta(
    ui_state: &EditorUI,
    config: Option<&EditorConfig>,
    tilemap: &TileMap,
) -> Option<TileSetMeta> {
    load_map_editor_tileset_meta_from_project_path(
        ui_state,
        config?.current_project_path()?,
        tilemap,
    )
}

pub(crate) fn load_map_editor_tileset_meta_from_project_path(
    ui_state: &EditorUI,
    project_path: &Path,
    tilemap: &TileMap,
) -> Option<TileSetMeta> {
    if let Some(tileset) = crate::ui::editor_context::map_state(ui_state)
        .modified_tileset
        .clone()
    {
        return Some(tileset);
    }
    let tileset_path = resolve_map_editor_tileset_path(project_path, tilemap)?;
    TileSetMeta::load_from_file(&tileset_path).ok()
}

pub(crate) fn load_map_editor_brush_source(
    ui_state: &EditorUI,
    config: Option<&EditorConfig>,
) -> Option<LoadedMapEditorBrushSource> {
    let project_path = config?.current_project_path()?;
    let tilemap = if let Some(draft) = &crate::ui::editor_context::map_state(ui_state).draft {
        draft.tilemap.clone()
    } else {
        let active_map = crate::ui::editor_context::map_state(ui_state)
            .active_map
            .as_ref()?;
        toki_core::assets::tilemap::TileMap::load_from_file(
            project_path
                .join("assets")
                .join("tilemaps")
                .join(format!("{}.json", active_map)),
        )
        .ok()?
    };
    load_map_editor_brush_source_for_tilemap(ui_state, project_path, &tilemap)
}

pub(crate) fn load_map_editor_brush_source_for_tilemap(
    ui_state: &EditorUI,
    project_path: &Path,
    tilemap: &TileMap,
) -> Option<LoadedMapEditorBrushSource> {
    let tileset = load_map_editor_tileset_meta_from_project_path(ui_state, project_path, tilemap)?;
    let tileset_path = resolve_map_editor_tileset_path(project_path, tilemap)?;
    let mut atlases = HashMap::new();
    for atlas_path in toki_core::project_assets::resolve_tileset_atlas_paths(
        project_path,
        &tileset_path,
        &tileset,
    ) {
        let atlas = AtlasMeta::load_from_file(&atlas_path).ok()?;
        let atlas_name = atlas_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(toki_core::project_assets::normalize_asset_name)
            .unwrap_or_default()
            .to_string();
        atlases.insert(
            atlas_name.clone(),
            TileSetAtlasSource {
                name: atlas_name,
                path: atlas_path,
                meta: atlas,
            },
        );
    }
    let brush_entries = build_map_editor_brush_entries_from_tileset(&tileset, &atlases);
    Some(LoadedMapEditorBrushSource {
        brush_entries,
        tileset,
        atlases,
    })
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
    let state = crate::ui::editor_context::map_state(ui_state);
    state.draft.is_some() && state.dirty
}

pub(crate) fn has_unsaved_map_editor_changes(ui_state: &EditorUI) -> bool {
    crate::ui::editor_context::map_state(ui_state).dirty
}

pub(crate) fn sync_map_editor_brush_selection(
    ui_state: &mut EditorUI,
    brush_entries: &[MapEditorBrushEntry],
) {
    if brush_entries.is_empty() {
        crate::ui::editor_context::map_state_mut(ui_state).selected_tile = None;
        return;
    }

    if ui_state
        .context::<crate::ui::editor_context::MapEditorContext>(super::CenterPanelTab::MapEditor)
        .expect("map editor context should exist")
        .map
        .selected_tile
        .as_ref()
        .is_some_and(|selected| brush_entries.iter().any(|entry| entry.id == *selected))
    {
        return;
    }

    crate::ui::editor_context::map_state_mut(ui_state).selected_tile =
        Some(brush_entries[0].id.clone());
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

// --- Layer operations ---

pub(crate) fn add_layer_to_map(ui_state: &mut EditorUI, layer_name: &str) {
    let map_state = crate::ui::editor_context::map_state_mut(ui_state);
    let Some(draft) = &mut map_state.draft else {
        return;
    };
    let before = draft.tilemap.clone();
    let tile_count = (draft.tilemap.size.x * draft.tilemap.size.y) as usize;
    let new_layer = TileLayer::new_empty(layer_name, tile_count);
    let insert_at = (map_state.active_layer + 1).min(draft.tilemap.layers.len());
    draft.tilemap.layers.insert(insert_at, new_layer);
    map_state.active_layer = insert_at;
    record_layer_edit(ui_state, before);
}

pub(crate) fn remove_layer_from_map(ui_state: &mut EditorUI, layer_index: usize) {
    let map_state = crate::ui::editor_context::map_state_mut(ui_state);
    let Some(draft) = &mut map_state.draft else {
        return;
    };
    if draft.tilemap.layers.len() <= 1 || layer_index >= draft.tilemap.layers.len() {
        return;
    }
    let before = draft.tilemap.clone();
    draft.tilemap.layers.remove(layer_index);
    map_state.active_layer = map_state
        .active_layer
        .min(draft.tilemap.layers.len().saturating_sub(1));
    record_layer_edit(ui_state, before);
}

pub(crate) fn move_layer(ui_state: &mut EditorUI, from: usize, to: usize) {
    let map_state = crate::ui::editor_context::map_state_mut(ui_state);
    let Some(draft) = &mut map_state.draft else {
        return;
    };
    let len = draft.tilemap.layers.len();
    if from >= len || to >= len || from == to {
        return;
    }
    let before = draft.tilemap.clone();
    let layer = draft.tilemap.layers.remove(from);
    draft.tilemap.layers.insert(to, layer);
    map_state.active_layer = to;
    record_layer_edit(ui_state, before);
}

#[allow(dead_code)]
pub(crate) fn rename_layer(ui_state: &mut EditorUI, layer_index: usize, new_name: &str) {
    let map_state = crate::ui::editor_context::map_state_mut(ui_state);
    let Some(draft) = &mut map_state.draft else {
        return;
    };
    let Some(layer) = draft.tilemap.layers.get_mut(layer_index) else {
        return;
    };
    if layer.name == new_name {
        return;
    }
    let before = draft.tilemap.clone();
    draft.tilemap.layers[layer_index].name = new_name.to_string();
    record_layer_edit(ui_state, before);
}

pub(crate) fn toggle_layer_visibility(ui_state: &mut EditorUI, layer_index: usize) {
    let map_state = crate::ui::editor_context::map_state_mut(ui_state);
    let Some(draft) = &mut map_state.draft else {
        return;
    };
    let Some(layer) = draft.tilemap.layers.get_mut(layer_index) else {
        return;
    };
    layer.visible = !layer.visible;
    map_state.pending_tilemap_sync = Some(draft.tilemap.clone());
    map_state.dirty = true;
}

pub(crate) fn toggle_layer_above_entities(ui_state: &mut EditorUI, layer_index: usize) {
    let map_state = crate::ui::editor_context::map_state_mut(ui_state);
    let Some(draft) = &mut map_state.draft else {
        return;
    };
    if layer_index >= draft.tilemap.layers.len() {
        return;
    }
    let before = draft.tilemap.clone();
    draft.tilemap.layers[layer_index].above_entities =
        !draft.tilemap.layers[layer_index].above_entities;
    record_layer_edit(ui_state, before);
}

pub(crate) fn set_active_layer(ui_state: &mut EditorUI, layer_index: usize) {
    let map_state = crate::ui::editor_context::map_state_mut(ui_state);
    let max = map_state
        .draft
        .as_ref()
        .map(|d| d.tilemap.layers.len().saturating_sub(1))
        .unwrap_or(0);
    map_state.active_layer = layer_index.min(max);
}

fn record_layer_edit(ui_state: &mut EditorUI, before: TileMap) {
    let map_state = crate::ui::editor_context::map_state_mut(ui_state);
    let Some(draft) = &map_state.draft else {
        return;
    };
    let after = draft.tilemap.clone();
    let map_name = map_state
        .active_map
        .clone()
        .unwrap_or_else(|| "map".to_string());
    let is_draft = true;
    map_state.history.push(MapEditorEditCommand {
        map_name,
        is_draft,
        before,
        after: after.clone(),
    });
    map_state.pending_tilemap_sync = Some(after);
    map_state.dirty = true;
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
