use super::inspector::InspectorSystem;
use super::menus::MenuSystem;
use super::panels::PanelSystem;
use super::rule_graph::RuleGraph;
use super::undo_redo::{EditorCommand, IndexedEntity, UndoRedoHistory};
use crate::project::SceneGraphLayout;
use crate::scene::SceneViewport;
use std::collections::HashMap;
use std::path::PathBuf;
use toki_core::{
    assets::tilemap::TileMap,
    entity::{Entity, EntityId},
    rules::RuleSet,
    Scene,
};

#[derive(Debug, Clone)]
pub enum Selection {
    Scene(String),
    Map(String, String), // (scene_name, map_name)
    Entity(EntityId),
    StandaloneMap(String), // Map selected from Maps panel (not in scene context)
    EntityDefinition(String), // Entity definition from palette
    RuleGraphNode {
        scene_name: String,
        node_key: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum CenterPanelTab {
    SceneViewport,
    SceneGraph,
    SceneRules,
    MapEditor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RightPanelTab {
    Inspector,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
pub struct EntityMoveDragState {
    pub scene_name: String,
    pub entity: Entity,
    pub dragged_entities: Vec<Entity>,
    pub grab_offset: glam::Vec2, // Cursor world position offset from entity top-left at drag start
}

#[derive(Debug, Clone, Copy)]
pub struct MarqueeSelectionState {
    pub start_screen: egui::Pos2,
    pub current_screen: egui::Pos2,
}

#[derive(Debug, Clone)]
pub struct SceneRulesGraphCommandData {
    pub before_rule_set: RuleSet,
    pub after_rule_set: RuleSet,
    pub before_graph: Option<RuleGraph>,
    pub after_graph: RuleGraph,
    pub before_layout: Option<SceneGraphLayout>,
    pub zoom: f32,
    pub pan: [f32; 2],
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

    fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMapRequest {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

/// Manages the editor's UI state and rendering
pub struct EditorUI {
    // Scene management
    pub scenes: Vec<Scene>,
    pub selection: Option<Selection>,
    pub active_scene: Option<String>, // Name of currently active scene
    pub scene_content_changed: bool,  // Flag to signal that scene content changed

    // Legacy entity selection (keep for backward compatibility)
    pub selected_entity_id: Option<EntityId>,
    pub selected_entity_ids: Vec<EntityId>,

    // UI Panel visibility
    pub show_hierarchy: bool,
    pub show_inspector: bool,
    pub show_maps: bool,
    pub should_exit: bool,
    pub show_console: bool,
    pub create_test_entities: bool,

    // Project management flags
    pub new_project_requested: bool,
    pub new_top_down_project_requested: bool,
    pub open_project_requested: bool,
    pub browse_for_project_requested: bool,
    pub save_project_requested: bool,
    pub export_project_requested: bool,
    pub play_scene_requested: bool,
    pub init_config_requested: bool,
    pub window_title: Option<String>,
    pub background_task_running: bool,
    pub background_task_status: Option<String>,
    pub cancel_background_task_requested: bool,
    pub right_panel_tab: RightPanelTab,

    // Map loading request
    pub map_load_requested: Option<(String, String)>, // (scene_name, map_name)
    pub map_editor_active_map: Option<String>,
    pub map_editor_map_load_requested: Option<String>,
    pub map_editor_draft: Option<MapEditorDraft>,
    pub map_editor_dirty: bool,
    pub map_editor_selected_tile: Option<String>,
    pub map_editor_tool: MapEditorTool,
    pub map_editor_brush_size_tiles: u32,
    pub map_editor_brush_preview_image_path: Option<PathBuf>,
    pub map_editor_brush_preview_texture: Option<egui::TextureHandle>,
    pub map_editor_selected_tile_info: Option<MapEditorTileInfo>,
    pub map_editor_show_new_map_dialog: bool,
    pub map_editor_new_map_name: String,
    pub map_editor_new_map_width: u32,
    pub map_editor_new_map_height: u32,
    pub map_editor_new_map_requested: Option<NewMapRequest>,
    pub map_editor_save_requested: bool,
    pub map_editor_history: MapEditorHistory,
    pub map_editor_pending_tilemap_sync: Option<TileMap>,
    pub map_editor_edit_before: Option<TileMap>,

    // Asset validation
    pub validate_assets_requested: bool,

    // Entity placement system
    pub placement_mode: bool,
    pub placement_entity_definition: Option<String>,
    pub placement_preview_position: Option<glam::Vec2>, // World coordinates for preview
    pub placement_preview_cached_frame: Option<toki_core::sprite::SpriteFrame>, // Cached sprite frame for preview
    pub placement_preview_valid: Option<bool>, // Whether the current preview position is valid for placement
    pub entity_move_drag: Option<EntityMoveDragState>, // Active drag-move operation for existing scene entities
    pub marquee_selection: Option<MarqueeSelectionState>, // Active marquee-selection rectangle in viewport
    pub center_panel_tab: CenterPanelTab,                 // Active tab in center workspace
    pub graph_connect_from_node: Option<u64>,             // Scene graph connect source node
    pub graph_connect_to_node: Option<u64>,               // Scene graph connect target node
    pub graph_canvas_zoom: f32,                           // Scene graph canvas zoom factor
    pub graph_canvas_pan: [f32; 2], // Scene graph canvas pan offset (screen-space)
    pub graph_layouts_by_scene: HashMap<String, SceneGraphLayout>, // Persisted scene graph layouts loaded from project
    pub graph_layout_dirty: bool, // Graph layout changed and should be flushed into project metadata
    pub rule_graphs_by_scene: HashMap<String, RuleGraph>, // In-memory scene graph drafts (can contain detached nodes)
    pub command_history: UndoRedoHistory, // Undo/redo command history for scene mutations

    // Multi-entity inspector draft state
    pub multi_entity_render_layer_input: i64,
    pub multi_entity_delta_x_input: i32,
    pub multi_entity_delta_y_input: i32,
    pub multi_entity_inspector_selection_signature: Vec<EntityId>,
}

impl EditorUI {
    pub fn new() -> Self {
        Self {
            // Scene management
            scenes: vec![Scene::new("Main Scene".to_string())], // Start with default scene
            selection: None,
            active_scene: Some("Main Scene".to_string()), // Default scene starts active
            scene_content_changed: false,

            // Legacy fields (keep for backward compatibility)
            selected_entity_id: None,
            selected_entity_ids: Vec::new(),

            // UI Panel visibility
            show_hierarchy: true,
            show_inspector: true,
            show_maps: true,
            should_exit: false,
            show_console: true,
            create_test_entities: false,

            // Project management flags
            new_project_requested: false,
            new_top_down_project_requested: false,
            open_project_requested: false,
            browse_for_project_requested: false,
            save_project_requested: false,
            export_project_requested: false,
            play_scene_requested: false,
            init_config_requested: false,
            window_title: Some("No project open".to_string()),
            background_task_running: false,
            background_task_status: None,
            cancel_background_task_requested: false,
            right_panel_tab: RightPanelTab::Inspector,

            // Map loading request
            map_load_requested: None,
            map_editor_active_map: None,
            map_editor_map_load_requested: None,
            map_editor_draft: None,
            map_editor_dirty: false,
            map_editor_selected_tile: None,
            map_editor_tool: MapEditorTool::Drag,
            map_editor_brush_size_tiles: 1,
            map_editor_brush_preview_image_path: None,
            map_editor_brush_preview_texture: None,
            map_editor_selected_tile_info: None,
            map_editor_show_new_map_dialog: false,
            map_editor_new_map_name: "new_map".to_string(),
            map_editor_new_map_width: 32,
            map_editor_new_map_height: 32,
            map_editor_new_map_requested: None,
            map_editor_save_requested: false,
            map_editor_history: MapEditorHistory::default(),
            map_editor_pending_tilemap_sync: None,
            map_editor_edit_before: None,

            // Asset validation
            validate_assets_requested: false,

            // Entity placement system
            placement_mode: false,
            placement_entity_definition: None,
            placement_preview_position: None,
            placement_preview_cached_frame: None,
            placement_preview_valid: None,
            entity_move_drag: None,
            marquee_selection: None,
            center_panel_tab: CenterPanelTab::SceneViewport,
            graph_connect_from_node: None,
            graph_connect_to_node: None,
            graph_canvas_zoom: 1.0,
            graph_canvas_pan: [16.0, 16.0],
            graph_layouts_by_scene: HashMap::new(),
            graph_layout_dirty: false,
            rule_graphs_by_scene: HashMap::new(),
            command_history: UndoRedoHistory::default(),
            multi_entity_render_layer_input: 0,
            multi_entity_delta_x_input: 0,
            multi_entity_delta_y_input: 0,
            multi_entity_inspector_selection_signature: Vec::new(),
        }
    }

    // Scene management methods
    pub fn add_scene(&mut self, name: String) -> &mut Scene {
        self.scenes.push(Scene::new(name));
        self.scenes.last_mut().unwrap()
    }

    pub fn get_scene(&self, name: &str) -> Option<&Scene> {
        self.scenes.iter().find(|s| s.name == name)
    }

    pub fn load_scenes_from_project(&mut self, loaded_scenes: Vec<Scene>) {
        tracing::info!("Loading {} scenes into UI hierarchy", loaded_scenes.len());
        self.scenes = loaded_scenes;
        self.rule_graphs_by_scene.clear();
        self.command_history.clear();

        let current_active_missing = self
            .active_scene
            .as_ref()
            .is_none_or(|active| !self.scenes.iter().any(|scene| &scene.name == active));

        if !self.scenes.is_empty() && current_active_missing {
            self.active_scene = Some(self.scenes[0].name.clone());
            tracing::info!("Set '{}' as active scene", self.scenes[0].name);
        }
    }

    pub fn set_selection(&mut self, selection: Selection) {
        if let Selection::Entity(entity_id) = selection {
            self.selected_entity_id = Some(entity_id);
            self.selected_entity_ids = vec![entity_id];
            self.selection = Some(Selection::Entity(entity_id));
            return;
        }
        self.selected_entity_id = None;
        self.selected_entity_ids.clear();
        self.selection = Some(selection);
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
        self.selected_entity_id = None;
        self.selected_entity_ids.clear();
    }

    pub fn set_single_entity_selection(&mut self, entity_id: EntityId) {
        self.selected_entity_id = Some(entity_id);
        self.selected_entity_ids.clear();
        self.selected_entity_ids.push(entity_id);
        self.selection = Some(Selection::Entity(entity_id));
    }

    pub fn toggle_entity_selection(&mut self, entity_id: EntityId) {
        if let Some(index) = self
            .selected_entity_ids
            .iter()
            .position(|id| *id == entity_id)
        {
            self.selected_entity_ids.remove(index);
            if self.selected_entity_ids.is_empty() {
                self.clear_selection();
                return;
            }
            if self.selected_entity_id == Some(entity_id) {
                if let Some(last_selected) = self.selected_entity_ids.last().copied() {
                    self.selected_entity_id = Some(last_selected);
                    self.selection = Some(Selection::Entity(last_selected));
                }
            }
            return;
        }

        self.selected_entity_ids.push(entity_id);
        self.selected_entity_id = Some(entity_id);
        self.selection = Some(Selection::Entity(entity_id));
    }

    pub fn has_multi_entity_selection(&self) -> bool {
        self.selected_entity_ids.len() > 1
    }

    pub fn clear_entity_selection(&mut self) {
        self.clear_selection();
    }

    pub fn sync_map_editor_selection(&mut self, available_map_names: &[String]) {
        if self.has_unsaved_map_editor_changes() {
            self.map_editor_map_load_requested = None;
            return;
        }

        if available_map_names.is_empty() {
            self.map_editor_active_map = None;
            self.map_editor_map_load_requested = None;
            return;
        }

        if self
            .map_editor_active_map
            .as_ref()
            .is_some_and(|selected| available_map_names.iter().any(|name| name == selected))
        {
            return;
        }

        let mut sorted_names = available_map_names.to_vec();
        sorted_names.sort();
        let next_map = sorted_names[0].clone();
        if self.map_editor_active_map.as_ref() != Some(&next_map) {
            self.map_editor_active_map = Some(next_map.clone());
            self.map_editor_map_load_requested = Some(next_map);
        }
    }

    pub fn begin_new_map_dialog(&mut self) {
        self.map_editor_show_new_map_dialog = true;
        if self.map_editor_new_map_name.trim().is_empty() {
            self.map_editor_new_map_name = "new_map".to_string();
        }
        self.map_editor_new_map_width = self.map_editor_new_map_width.max(1);
        self.map_editor_new_map_height = self.map_editor_new_map_height.max(1);
    }

    pub fn submit_new_map_request(&mut self) {
        let name = self.map_editor_new_map_name.trim().to_string();
        if name.is_empty() {
            return;
        }

        self.map_editor_new_map_requested = Some(NewMapRequest {
            name,
            width: self.map_editor_new_map_width.max(1),
            height: self.map_editor_new_map_height.max(1),
        });
        self.map_editor_show_new_map_dialog = false;
    }

    pub fn set_map_editor_draft(&mut self, draft: MapEditorDraft) {
        self.map_editor_active_map = Some(draft.name.clone());
        self.map_editor_map_load_requested = None;
        self.map_editor_draft = Some(draft);
        self.map_editor_dirty = true;
        self.map_editor_history.clear();
        self.map_editor_pending_tilemap_sync = None;
        self.map_editor_edit_before = None;
    }

    pub fn map_editor_selected_label(&self) -> String {
        if let Some(draft) = &self.map_editor_draft {
            return format!("{}*", draft.name);
        }

        self.map_editor_active_map
            .clone()
            .unwrap_or_else(|| "No map selected".to_string())
    }

    pub fn has_unsaved_map_editor_draft(&self) -> bool {
        self.map_editor_draft.is_some()
    }

    pub fn has_unsaved_map_editor_changes(&self) -> bool {
        self.map_editor_dirty || self.map_editor_draft.is_some()
    }

    pub fn sync_map_editor_brush_selection(&mut self, tile_names: &[String]) {
        if tile_names.is_empty() {
            self.map_editor_selected_tile = None;
            return;
        }

        if self
            .map_editor_selected_tile
            .as_ref()
            .is_some_and(|selected| tile_names.iter().any(|name| name == selected))
        {
            return;
        }

        let mut sorted_names = tile_names.to_vec();
        sorted_names.sort();
        self.map_editor_selected_tile = Some(sorted_names[0].clone());
    }

    pub fn pick_map_editor_tile(&mut self, tile_name: String) {
        self.map_editor_selected_tile = Some(tile_name);
        self.map_editor_tool = MapEditorTool::Brush;
    }

    pub fn mark_map_editor_dirty(&mut self) {
        self.map_editor_dirty = true;
    }

    pub fn clear_map_editor_dirty(&mut self) {
        self.map_editor_dirty = false;
    }

    pub fn finalize_saved_map_editor_draft(&mut self, saved_name: String) {
        self.map_editor_draft = None;
        self.map_editor_dirty = false;
        self.map_editor_active_map = Some(saved_name.clone());
        self.map_editor_map_load_requested = Some(saved_name);
        self.map_editor_save_requested = false;
        self.map_editor_history.clear();
        self.map_editor_pending_tilemap_sync = None;
        self.map_editor_edit_before = None;
    }

    pub fn finalize_saved_existing_map(&mut self) {
        self.map_editor_dirty = false;
        self.map_editor_save_requested = false;
    }

    pub fn clear_map_editor_history(&mut self) {
        self.map_editor_history.clear();
        self.map_editor_pending_tilemap_sync = None;
        self.map_editor_edit_before = None;
    }

    pub fn begin_map_editor_edit(&mut self, before: &TileMap) {
        if self.map_editor_edit_before.is_none() {
            self.map_editor_edit_before = Some(before.clone());
        }
    }

    pub fn finish_map_editor_edit(&mut self, after: &TileMap) -> bool {
        let Some(before) = self.map_editor_edit_before.take() else {
            return false;
        };
        if before == *after {
            return false;
        }
        let map_name = self
            .map_editor_active_map
            .clone()
            .unwrap_or_else(|| "map".to_string());
        let is_draft = self.map_editor_draft.is_some();
        self.map_editor_history.push(MapEditorEditCommand {
            map_name,
            is_draft,
            before,
            after: after.clone(),
        });
        self.map_editor_dirty = true;
        true
    }

    pub fn cancel_map_editor_edit(&mut self) {
        self.map_editor_edit_before = None;
    }

    fn apply_map_editor_tilemap_snapshot(
        &mut self,
        map_name: &str,
        is_draft: bool,
        tilemap: &TileMap,
    ) -> bool {
        if self.map_editor_active_map.as_deref() != Some(map_name) {
            return false;
        }

        if is_draft {
            let Some(draft) = self.map_editor_draft.as_mut() else {
                return false;
            };
            if draft.name != map_name {
                return false;
            }
            draft.tilemap = tilemap.clone();
        } else if self.map_editor_draft.is_some() {
            return false;
        }

        self.map_editor_pending_tilemap_sync = Some(tilemap.clone());
        self.map_editor_dirty = true;
        true
    }

    pub fn take_pending_map_editor_tilemap_sync(&mut self) -> Option<TileMap> {
        self.map_editor_pending_tilemap_sync.take()
    }

    pub fn execute_command(&mut self, command: EditorCommand) -> bool {
        let mut history = std::mem::take(&mut self.command_history);
        let changed = history.execute(command, self);
        self.command_history = history;
        changed
    }

    pub fn undo(&mut self) -> bool {
        if self.center_panel_tab == CenterPanelTab::MapEditor && self.map_editor_history.can_undo()
        {
            let mut history = std::mem::take(&mut self.map_editor_history);
            let undone = history.undo(self);
            self.map_editor_history = history;
            return undone;
        }
        let mut history = std::mem::take(&mut self.command_history);
        let undone = history.undo(self);
        self.command_history = history;
        undone
    }

    pub fn redo(&mut self) -> bool {
        if self.center_panel_tab == CenterPanelTab::MapEditor && self.map_editor_history.can_redo()
        {
            let mut history = std::mem::take(&mut self.map_editor_history);
            let redone = history.redo(self);
            self.map_editor_history = history;
            return redone;
        }
        let mut history = std::mem::take(&mut self.command_history);
        let redone = history.redo(self);
        self.command_history = history;
        redone
    }

    pub fn can_undo(&self) -> bool {
        if self.center_panel_tab == CenterPanelTab::MapEditor {
            self.map_editor_history.can_undo()
        } else {
            self.command_history.can_undo()
        }
    }

    pub fn can_redo(&self) -> bool {
        if self.center_panel_tab == CenterPanelTab::MapEditor {
            self.map_editor_history.can_redo()
        } else {
            self.command_history.can_redo()
        }
    }

    // Entity placement mode management
    pub fn enter_placement_mode(&mut self, entity_definition: String) {
        self.placement_mode = true;
        self.placement_entity_definition = Some(entity_definition);
        tracing::info!(
            "Entered placement mode for entity: {}",
            self.placement_entity_definition.as_ref().unwrap()
        );
    }

    pub fn exit_placement_mode(&mut self) {
        if self.placement_mode {
            tracing::info!("Exited placement mode");
        }
        self.placement_mode = false;
        self.placement_entity_definition = None;
        self.placement_preview_position = None;
        self.placement_preview_cached_frame = None;
        self.placement_preview_valid = None;
        self.entity_move_drag = None;
        self.marquee_selection = None;
    }

    pub fn is_in_placement_mode(&self) -> bool {
        self.placement_mode
    }

    pub fn begin_entity_move_drag(&mut self, drag_state: EntityMoveDragState) {
        self.entity_move_drag = Some(drag_state);
    }

    pub fn is_entity_move_drag_active(&self) -> bool {
        self.entity_move_drag.is_some()
    }

    pub fn start_marquee_selection(&mut self, start: egui::Pos2) {
        self.marquee_selection = Some(MarqueeSelectionState {
            start_screen: start,
            current_screen: start,
        });
    }

    pub fn update_marquee_selection(&mut self, current: egui::Pos2) {
        if let Some(marquee) = self.marquee_selection.as_mut() {
            marquee.current_screen = current;
        }
    }

    pub fn finish_marquee_selection(&mut self) -> Option<MarqueeSelectionState> {
        self.marquee_selection.take()
    }

    pub fn is_marquee_selection_active(&self) -> bool {
        self.marquee_selection.is_some()
    }

    pub fn add_entity_to_selection(&mut self, entity_id: EntityId) {
        if !self.selected_entity_ids.contains(&entity_id) {
            self.selected_entity_ids.push(entity_id);
        }
        self.selected_entity_id = Some(entity_id);
        self.selection = Some(Selection::Entity(entity_id));
    }

    /// Render the entire UI
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        scene_viewport: Option<&mut SceneViewport>,
        map_editor_viewport: Option<&mut SceneViewport>,
        project: Option<&mut crate::project::Project>,
        available_map_names: Option<Vec<String>>,
        config: Option<&mut crate::config::EditorConfig>,
        log_capture: Option<&crate::logging::LogCapture>,
        renderer: Option<&mut egui_wgpu::Renderer>,
        busy_logo_texture: Option<&egui::TextureHandle>,
    ) {
        let config_readonly = config.as_deref();
        MenuSystem::render_top_menu(self, ctx, config_readonly, busy_logo_texture);

        // Render log panel first to claim full width at bottom
        if self.show_console {
            PanelSystem::render_log_panel(self, ctx, log_capture);
        }

        // Render hierarchy and inspector panels
        let game_state = scene_viewport
            .as_ref()
            .map(|v| v.scene_manager().game_state());

        if self.show_hierarchy {
            super::hierarchy::HierarchySystem::render_hierarchy_and_maps_combined_panel(
                self,
                ctx,
                game_state,
                config_readonly,
            );
        }

        if self.show_inspector {
            InspectorSystem::render_inspector_panel(
                self,
                ctx,
                game_state,
                project,
                config_readonly,
            );
        }

        // Render viewport last (mutable access)
        PanelSystem::render_viewport(
            self,
            ctx,
            scene_viewport,
            map_editor_viewport,
            available_map_names,
            config,
            renderer,
        );
    }

    /// Apply config settings to UI state
    pub fn apply_config(&mut self, config: &crate::config::EditorConfig) {
        self.show_hierarchy = config.editor_settings.panels.hierarchy_visible;
        self.show_inspector = config.editor_settings.panels.inspector_visible;
        self.show_console = config.editor_settings.panels.console_visible;
    }

    pub fn set_title(&mut self, title: &str) {
        self.window_title = Some(title.to_string());
    }

    pub fn load_graph_layouts_from_project(
        &mut self,
        graph_layouts: &HashMap<String, SceneGraphLayout>,
    ) {
        self.graph_layouts_by_scene = graph_layouts.clone();
        self.graph_layout_dirty = false;
    }

    pub fn load_rule_graph_drafts_from_project(&mut self, drafts: &HashMap<String, RuleGraph>) {
        self.rule_graphs_by_scene = drafts.clone();
    }

    pub fn export_graph_layouts_for_project(&self) -> HashMap<String, SceneGraphLayout> {
        self.graph_layouts_by_scene.clone()
    }

    pub fn export_rule_graph_drafts_for_project(&self) -> HashMap<String, RuleGraph> {
        self.rule_graphs_by_scene.clone()
    }

    pub fn is_graph_layout_dirty(&self) -> bool {
        self.graph_layout_dirty
    }

    pub fn clear_graph_layout_dirty(&mut self) {
        self.graph_layout_dirty = false;
    }

    pub fn graph_layout_position(&self, scene_name: &str, node_key: &str) -> Option<[f32; 2]> {
        self.graph_layouts_by_scene
            .get(scene_name)
            .and_then(|layout| layout.node_positions.get(node_key).copied())
    }

    pub fn graph_view_for_scene(&self, scene_name: &str) -> (f32, [f32; 2]) {
        if let Some(layout) = self.graph_layouts_by_scene.get(scene_name) {
            (layout.zoom, layout.pan)
        } else {
            (1.0, [16.0, 16.0])
        }
    }

    pub fn set_graph_view_for_scene(&mut self, scene_name: &str, zoom: f32, pan: [f32; 2]) {
        let layout = self
            .graph_layouts_by_scene
            .entry(scene_name.to_string())
            .or_default();
        if (layout.zoom - zoom).abs() > f32::EPSILON || layout.pan != pan {
            layout.zoom = zoom;
            layout.pan = pan;
            self.graph_layout_dirty = true;
        }
    }

    pub fn build_scene_graph_layout_snapshot(
        &self,
        scene_name: &str,
        graph: &RuleGraph,
        zoom: f32,
        pan: [f32; 2],
        base_layout: Option<SceneGraphLayout>,
    ) -> SceneGraphLayout {
        let mut layout = base_layout.unwrap_or_else(|| {
            self.graph_layouts_by_scene
                .get(scene_name)
                .cloned()
                .unwrap_or_default()
        });
        layout.node_positions.clear();
        for node in &graph.nodes {
            let Some(node_key) = graph.stable_node_key(node.id) else {
                continue;
            };
            layout.node_positions.insert(node_key, node.position);
        }
        layout.zoom = zoom;
        layout.pan = pan;
        layout
    }

    pub fn execute_scene_rules_graph_command(
        &mut self,
        scene_name: &str,
        data: SceneRulesGraphCommandData,
    ) -> bool {
        let after_layout = self.build_scene_graph_layout_snapshot(
            scene_name,
            &data.after_graph,
            data.zoom,
            data.pan,
            data.before_layout.clone(),
        );
        self.execute_command(EditorCommand::update_scene_rules_graph(
            scene_name.to_string(),
            data.before_rule_set,
            data.after_rule_set,
            data.before_graph,
            Some(data.after_graph),
            data.before_layout,
            Some(after_layout),
        ))
    }

    pub fn sync_rule_graph_with_rule_set(&mut self, scene_name: &str, rule_set: &RuleSet) {
        let needs_rebuild = match self.rule_graphs_by_scene.get(scene_name) {
            None => true,
            Some(graph) => match graph.to_rule_set() {
                Ok(graph_rules) => graph_rules != *rule_set,
                Err(_) => false,
            },
        };
        if needs_rebuild {
            self.rule_graphs_by_scene
                .insert(scene_name.to_string(), RuleGraph::from_rule_set(rule_set));
        }
    }

    pub fn rule_graph_for_scene(&self, scene_name: &str) -> Option<&RuleGraph> {
        self.rule_graphs_by_scene.get(scene_name)
    }

    pub fn set_rule_graph_for_scene(&mut self, scene_name: String, graph: RuleGraph) {
        self.rule_graphs_by_scene.insert(scene_name, graph);
    }

    pub fn render_hierarchy_and_maps_combined_panel(
        &mut self,
        ctx: &egui::Context,
        game_state: Option<&toki_core::GameState>,
        config: Option<&crate::config::EditorConfig>,
    ) {
        egui::SidePanel::left("hierarchy_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("📋 Scene Hierarchy");
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("hierarchy_scroll")
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                    // Collect actions to perform after UI iteration
                    let mut map_removals: Vec<(usize, usize)> = Vec::new();
                    let mut entity_removals: Vec<(String, u32)> = Vec::new(); // (scene_name, entity_id)
                    let mut selection_changes: Vec<Selection> = Vec::new();
                    let mut active_scene_change: Option<String> = None;

                    for (scene_index, scene) in self.scenes.iter().enumerate() {
                        let is_active_scene = self.active_scene.as_ref() == Some(&scene.name);
                        let scene_header_text = if is_active_scene {
                            format!("🎬 {} ⭐", scene.name) // Active scene gets a star
                        } else {
                            format!("🎬 {}", scene.name)
                        };

                        let scene_header_response = ui.collapsing(&scene_header_text, |ui| {

                            // Maps section within the scene - only show configured maps
                            if !scene.maps.is_empty() {
                                ui.label("Maps:");
                                ui.indent("scene_maps", |ui| {
                                    for (map_index, map_name) in scene.maps.iter().enumerate() {
                                        let is_selected = matches!(
                                            &self.selection,
                                            Some(Selection::Map(s, m)) if s == &scene.name && m == map_name
                                        );

                                        ui.horizontal(|ui| {
                                            let response = ui.selectable_label(is_selected, format!("🗺️ {}", map_name));
                                            if response.clicked() {
                                                selection_changes.push(Selection::Map(scene.name.clone(), map_name.clone()));
                                                tracing::info!("Selected map {} in scene {}", map_name, scene.name);
                                            }

                                            // Remove map button
                                            if ui.small_button("✕").clicked() {
                                                map_removals.push((scene_index, map_index));
                                                tracing::info!("Removed map {} from scene {}", map_name, scene.name);
                                            }
                                        });
                                    }
                                });
                                ui.add_space(5.0);
                            }

                            // Scene entities section (design-time entities in scene definition)
                            if !scene.entities.is_empty() {
                                ui.label("Scene Entities:");
                                ui.indent("scene_design_entities", |ui| {
                                    for entity in &scene.entities {
                                        let is_selected = matches!(
                                            &self.selection,
                                            Some(Selection::Entity(id)) if id == &entity.id
                                        );

                                        ui.horizontal(|ui| {
                                            let kind_label = entity
                                                .definition_name
                                                .clone()
                                                .or_else(|| {
                                                    if entity.category.is_empty() {
                                                        None
                                                    } else {
                                                        Some(entity.category.clone())
                                                    }
                                                })
                                                .unwrap_or_else(|| format!("{:?}", entity.entity_kind));
                                            let entity_display = if matches!(
                                                entity.effective_control_role(),
                                                toki_core::entity::ControlRole::PlayerCharacter
                                            ) {
                                                format!("👤 {} (Player Character, ID: {})", kind_label, entity.id)
                                            } else {
                                                format!("🧩 {} (ID: {})", kind_label, entity.id)
                                            };

                                            let response = ui.selectable_label(is_selected, entity_display);

                                            if response.clicked() {
                                                selection_changes.push(Selection::Entity(entity.id));
                                                tracing::info!("Selected scene entity ID: {}", entity.id);
                                            }

                                            // Show entity position
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(format!("({}, {})", entity.position.x, entity.position.y));
                                            });
                                        });

                                        // Right-click context menu for entity actions
                                        ui.horizontal(|ui| {
                                            ui.add_space(20.0); // Indent for context options
                                            if ui.small_button("🗑️").on_hover_text("Remove from scene").clicked() {
                                                // Add to removal list - will be processed after UI rendering
                                                entity_removals.push((scene.name.clone(), entity.id));
                                            }
                                        });
                                    }
                                });
                            }

                            // Runtime entities section (entities from game state)
                            ui.label("Runtime Entities:");
                            ui.indent("scene_runtime_entities", |ui| {
                                if let Some(game_state) = game_state {
                                    let entity_ids = game_state.entity_manager().active_entities();

                                    if entity_ids.is_empty() {
                                        ui.label("No runtime entities");
                                    } else {
                                        for entity_id in &entity_ids {
                                            if let Some(entity) = game_state.entity_manager().get_entity(*entity_id) {
                                                let is_selected = matches!(
                                                    &self.selection,
                                                    Some(Selection::Entity(id)) if id == entity_id
                                                );

                                                ui.horizontal(|ui| {
                                                    let response = ui.selectable_label(
                                                        is_selected,
                                                        format!("⚙️ Runtime Entity {}", entity_id)
                                                    );

                                                    if response.clicked() {
                                                        selection_changes.push(Selection::Entity(*entity_id));
                                                        self.selected_entity_id = Some(*entity_id);
                                                    }

                                                    // Show entity position
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        ui.label(format!("({}, {})", entity.position.x, entity.position.y));
                                                    });
                                                });
                                            }
                                        }
                                    }
                                } else {
                                    ui.label("No game state available");
                                }
                            });
                        });

                        // Scene selection (clicking on header)
                        if scene_header_response.header_response.clicked() {
                            selection_changes.push(Selection::Scene(scene.name.clone()));
                            tracing::info!("Selected scene: {}", scene.name);
                        }

                        // Right-click context menu for scene
                        scene_header_response.header_response.context_menu(|ui| {
                            let is_active = self.active_scene.as_ref() == Some(&scene.name);

                            ui.horizontal(|ui| {
                                if is_active {
                                    ui.label("✅ Active Scene");
                                } else if ui.button("🎯 Set as Active Scene").clicked() {
                                        active_scene_change = Some(scene.name.clone());
                                        tracing::info!("Setting {} as active scene", scene.name);
                                        ui.close();

                                }
                            });
                        });
                    }

                    // Process removals in reverse order to maintain correct indices
                    map_removals.sort_by(|a, b| b.1.cmp(&a.1));
                    for (scene_index, map_index) in map_removals {
                        if let Some(scene) = self.scenes.get_mut(scene_index) {
                            if map_index < scene.maps.len() {
                                let removed_map = scene.maps.remove(map_index);
                                // Clear selection if it was the removed map
                                if matches!(&self.selection, Some(Selection::Map(s, m)) if s == &scene.name && m == &removed_map) {
                                    self.clear_selection();
                                }
                            }
                        }
                    }

                    // Process entity removals
                    for (scene_name, entity_id) in entity_removals {
                        let Some(scene_index) =
                            self.scenes.iter().position(|scene| scene.name == scene_name)
                        else {
                            continue;
                        };
                        let Some((index, entity)) = self.scenes[scene_index]
                            .entities
                            .iter()
                            .enumerate()
                            .find(|(_, entity)| entity.id == entity_id)
                            .map(|(index, entity)| (index, entity.clone()))
                        else {
                            continue;
                        };

                        let removed = self.execute_command(EditorCommand::remove_entities(
                            scene_name.clone(),
                            vec![IndexedEntity { index, entity }],
                        ));
                        if removed {
                            tracing::info!("Removed entity {} from scene {}", entity_id, scene_name);

                            // Clear selection if it was the removed entity
                            if matches!(&self.selection, Some(Selection::Entity(id)) if id == &entity_id) {
                                self.clear_selection();
                            }
                        }
                    }

                    // Apply selection changes (only apply the last one)
                    if let Some(selection) = selection_changes.last() {
                        self.set_selection(selection.clone());
                    }

                    // Apply active scene change
                    if let Some(new_active_scene) = active_scene_change {
                        self.active_scene = Some(new_active_scene);
                    }

                    ui.separator();

                    if ui.button("+ Add Scene").clicked() {
                        let new_scene_name = format!("Scene {}", self.scenes.len() + 1);
                        self.add_scene(new_scene_name.clone());
                        tracing::info!("Created new scene: {}", new_scene_name);
                    }

                    if self.show_maps {
                        ui.add_space(10.0);
                        ui.heading("🗺️ Maps");
                        ui.separator();

                        if let Some(config) = config {
                            if let Some(project_path) = config.current_project_path() {
                                let tilemaps_path = project_path.join("assets").join("tilemaps");

                                if tilemaps_path.exists() {
                                    if let Ok(entries) = std::fs::read_dir(&tilemaps_path) {
                                        let mut map_selections: Vec<String> = Vec::new();
                                        let mut scene_map_additions: Vec<(String, String)> = Vec::new();

                                        for entry in entries.flatten() {
                                            if let Some(name) = entry.file_name().to_str() {
                                                if name.ends_with(".json") {
                                                    let map_name =
                                                        name.trim_end_matches(".json").to_string();

                                                    let is_selected = matches!(
                                                        &self.selection,
                                                        Some(Selection::StandaloneMap(name)) if name == &map_name
                                                    );

                                                    let response =
                                                        ui.selectable_label(is_selected, &map_name);

                                                    if response.clicked() {
                                                        tracing::info!("Map selected: {}", map_name);
                                                        map_selections.push(map_name.clone());
                                                    }

                                                    response.context_menu(|ui| {
                                                        ui.label("Add to Scene:");
                                                        ui.separator();

                                                        let scene_names: Vec<(String, bool)> = self
                                                            .scenes
                                                            .iter()
                                                            .map(|s| {
                                                                (
                                                                    s.name.clone(),
                                                                    s.maps.contains(&map_name),
                                                                )
                                                            })
                                                            .collect();

                                                        for (scene_name, already_added) in
                                                            scene_names
                                                        {
                                                            if !already_added {
                                                                if ui.button(&scene_name).clicked() {
                                                                    scene_map_additions.push((
                                                                        scene_name.clone(),
                                                                        map_name.clone(),
                                                                    ));
                                                                    ui.close();
                                                                }
                                                            } else {
                                                                ui.add_enabled(
                                                                    false,
                                                                    egui::Button::new(format!(
                                                                        "{} (already added)",
                                                                        scene_name
                                                                    )),
                                                                );
                                                            }
                                                        }

                                                        if self.scenes.is_empty() {
                                                            ui.label("No scenes available");
                                                        }
                                                    });
                                                }
                                            }
                                        }

                                        for map_name in map_selections {
                                            self.set_selection(Selection::StandaloneMap(map_name));
                                        }

                                        for (scene_name, map_name) in scene_map_additions {
                                            if let Some(target_scene) =
                                                self.scenes.iter_mut().find(|s| s.name == scene_name)
                                            {
                                                target_scene.maps.push(map_name.clone());
                                                tracing::info!(
                                                    "Added map '{}' to scene '{}'",
                                                    map_name,
                                                    scene_name
                                                );
                                                self.scene_content_changed = true;
                                            }
                                        }
                                    } else {
                                        tracing::warn!("Could not read tilemaps directory");
                                    }
                                }
                            }
                        }
                    }

                    ui.add_space(10.0);
                    ui.heading("🧙 Entities");
                    ui.separator();

                    if let Some(config) = config {
                        if let Some(project_path) = config.current_project_path() {
                            let (selected_entity, entity_additions, placement_request) =
                                super::hierarchy::HierarchySystem::render_entity_palette(
                                    ui,
                                    project_path,
                                    &self.selection,
                                    &self.scenes,
                                );

                            if let Some(selected_entity) = selected_entity {
                                self.set_selection(Selection::EntityDefinition(selected_entity));
                            }

                            if let Some(entity_definition) = placement_request {
                                self.enter_placement_mode(entity_definition);
                            }

                            for (scene_name, entity_name) in entity_additions {
                                if let Some(project_path) = config.current_project_path() {
                                    let entity_file = project_path
                                        .join("entities")
                                        .join(format!("{}.json", entity_name));

                                    if entity_file.exists() {
                                        match std::fs::read_to_string(&entity_file) {
                                            Ok(content) => {
                                                match serde_json::from_str::<toki_core::entity::EntityDefinition>(&content) {
                                                    Ok(entity_def) => {
                                                        let Some(scene_index) = self
                                                            .scenes
                                                            .iter()
                                                            .position(|scene| scene.name == scene_name)
                                                        else {
                                                            continue;
                                                        };

                                                        let new_id = self.scenes[scene_index]
                                                            .entities
                                                            .iter()
                                                            .map(|entity| entity.id)
                                                            .max()
                                                            .unwrap_or(0)
                                                            + 1;

                                                        let default_position =
                                                            glam::IVec2::new(100, 100);

                                                        match entity_def.create_entity(
                                                            default_position,
                                                            new_id,
                                                        ) {
                                                            Ok(entity) => {
                                                                if self.execute_command(
                                                                    EditorCommand::add_entity(
                                                                        scene_name.clone(),
                                                                        entity,
                                                                    ),
                                                                ) {
                                                                    tracing::info!(
                                                                        "Successfully added entity '{}' (ID: {}) to scene '{}' at position ({}, {})",
                                                                        entity_name,
                                                                        new_id,
                                                                        scene_name,
                                                                        default_position.x,
                                                                        default_position.y
                                                                    );
                                                                }
                                                            }
                                                            Err(e) => {
                                                                tracing::error!(
                                                                    "Failed to create entity '{}': {}",
                                                                    entity_name,
                                                                    e
                                                                );
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        tracing::error!(
                                                            "Failed to parse entity definition '{}': {}",
                                                            entity_name,
                                                            e
                                                        );
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                tracing::error!(
                                                    "Failed to read entity file '{}': {}",
                                                    entity_name,
                                                    e
                                                );
                                            }
                                        }
                                    } else {
                                        tracing::error!(
                                            "Entity definition file not found: {:?}",
                                            entity_file
                                        );
                                    }
                                } else {
                                    tracing::error!(
                                        "No project path available for entity creation"
                                    );
                                }
                            }
                        } else {
                            ui.label("No project loaded for Entity palette");
                        }
                    } else {
                        ui.label("No project configuration available for Entity palette");
                    }
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use glam::{IVec2, UVec2};
    use toki_core::entity::{EntityAttributes, EntityKind};
    use toki_core::rules::{
        Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleTrigger,
    };

    use super::{EditorUI, MapEditorDraft};
    use crate::ui::rule_graph::RuleGraph;
    use crate::ui::undo_redo::EditorCommand;

    fn sample_entity(id: u32, position: IVec2) -> toki_core::entity::Entity {
        toki_core::entity::Entity {
            id,
            position,
            size: UVec2::new(16, 16),
            entity_kind: EntityKind::Npc,
            category: "creature".to_string(),
            definition_name: Some("npc".to_string()),
            control_role: toki_core::entity::ControlRole::None,
            audio: toki_core::entity::EntityAudioSettings::default(),
            attributes: EntityAttributes::default(),
            collision_box: None,
        }
    }

    #[test]
    fn sync_rule_graph_with_rule_set_preserves_unserializable_existing_draft() {
        let mut ui = EditorUI::new();
        let rule_set = RuleSet {
            rules: vec![Rule {
                id: "rule_1".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnUpdate,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::PlaySound {
                    channel: RuleSoundChannel::Movement,
                    sound_id: "sfx".to_string(),
                }],
            }],
        };
        let mut graph = RuleGraph::from_rule_set(&rule_set);
        let trigger_id = graph.chains[0].trigger_node_id;
        let detached_target = graph
            .add_condition_node(RuleCondition::KeyHeld {
                key: toki_core::rules::RuleKey::Left,
            })
            .expect("detached target should be created");
        graph
            .connect_nodes(trigger_id, detached_target)
            .expect("branching connect should succeed");
        assert!(
            graph.to_rule_set().is_err(),
            "graph should be intentionally non-serializable due to branching"
        );

        ui.set_rule_graph_for_scene("Main Scene".to_string(), graph.clone());
        ui.sync_rule_graph_with_rule_set("Main Scene", &rule_set);

        let persisted_graph = ui
            .rule_graph_for_scene("Main Scene")
            .expect("graph draft should still exist");
        assert!(
            persisted_graph
                .edges
                .iter()
                .any(|edge| edge.from == trigger_id && edge.to == detached_target),
            "branching edge should be preserved instead of rebuilding from RuleSet"
        );
    }

    #[test]
    fn add_entity_to_selection_preserves_existing_and_avoids_duplicates() {
        let mut ui = EditorUI::new();

        ui.add_entity_to_selection(1);
        ui.add_entity_to_selection(2);
        ui.add_entity_to_selection(1);

        assert_eq!(ui.selected_entity_ids, vec![1, 2]);
        assert_eq!(ui.selected_entity_id, Some(1));
    }

    #[test]
    fn marquee_selection_lifecycle_tracks_start_update_and_finish() {
        let mut ui = EditorUI::new();
        assert!(!ui.is_marquee_selection_active());

        ui.start_marquee_selection(egui::pos2(10.0, 20.0));
        ui.update_marquee_selection(egui::pos2(30.0, 40.0));

        let marquee = ui
            .finish_marquee_selection()
            .expect("marquee should be active");
        assert_eq!(marquee.start_screen, egui::pos2(10.0, 20.0));
        assert_eq!(marquee.current_screen, egui::pos2(30.0, 40.0));
        assert!(!ui.is_marquee_selection_active());
    }

    #[test]
    fn execute_command_undo_and_redo_round_trip_entity_creation() {
        let mut ui = EditorUI::new();
        let command = EditorCommand::add_entity("Main Scene", sample_entity(11, IVec2::new(8, 9)));

        assert!(ui.execute_command(command));
        assert!(ui.can_undo());
        assert_eq!(
            ui.scenes
                .iter()
                .find(|scene| scene.name == "Main Scene")
                .expect("main scene should exist")
                .entities
                .len(),
            1
        );

        assert!(ui.undo());
        assert!(ui.can_redo());
        assert!(ui
            .scenes
            .iter()
            .find(|scene| scene.name == "Main Scene")
            .expect("main scene should exist")
            .entities
            .is_empty());

        assert!(ui.redo());
        assert_eq!(
            ui.scenes
                .iter()
                .find(|scene| scene.name == "Main Scene")
                .expect("main scene should exist")
                .entities
                .len(),
            1
        );
    }

    #[test]
    fn load_scenes_from_project_clears_undo_redo_history() {
        let mut ui = EditorUI::new();
        assert!(ui.execute_command(EditorCommand::add_entity(
            "Main Scene",
            sample_entity(1, IVec2::new(0, 0))
        )));
        assert!(ui.can_undo());

        ui.load_scenes_from_project(vec![toki_core::Scene::new("Imported".to_string())]);

        assert!(!ui.can_undo());
        assert!(!ui.can_redo());
    }

    #[test]
    fn load_scenes_from_project_replaces_missing_active_scene_with_first_loaded_scene() {
        let mut ui = EditorUI::new();
        ui.active_scene = Some("Missing".to_string());

        ui.load_scenes_from_project(vec![toki_core::Scene::new("main".to_string())]);

        assert_eq!(ui.active_scene.as_deref(), Some("main"));
    }

    #[test]
    fn sync_map_editor_selection_picks_sorted_first_map_and_requests_load() {
        let mut ui = EditorUI::new();
        let maps = vec![
            "zeta".to_string(),
            "alpha".to_string(),
            "middle".to_string(),
        ];

        ui.sync_map_editor_selection(&maps);

        assert_eq!(ui.map_editor_active_map.as_deref(), Some("alpha"));
        assert_eq!(ui.map_editor_map_load_requested.as_deref(), Some("alpha"));
    }

    #[test]
    fn sync_map_editor_selection_preserves_existing_valid_choice() {
        let mut ui = EditorUI::new();
        ui.map_editor_active_map = Some("middle".to_string());
        let maps = vec![
            "zeta".to_string(),
            "alpha".to_string(),
            "middle".to_string(),
        ];

        ui.sync_map_editor_selection(&maps);

        assert_eq!(ui.map_editor_active_map.as_deref(), Some("middle"));
        assert!(ui.map_editor_map_load_requested.is_none());
    }

    #[test]
    fn sync_map_editor_selection_preserves_unsaved_draft() {
        let mut ui = EditorUI::new();
        ui.set_map_editor_draft(MapEditorDraft {
            name: "draft_map".to_string(),
            tilemap: toki_core::assets::tilemap::TileMap {
                size: glam::UVec2::new(2, 2),
                tile_size: glam::UVec2::new(8, 8),
                atlas: std::path::PathBuf::from("terrain.json"),
                tiles: vec!["grass".to_string(); 4],
            },
        });

        ui.sync_map_editor_selection(&["alpha".to_string(), "zeta".to_string()]);

        assert_eq!(ui.map_editor_active_map.as_deref(), Some("draft_map"));
        assert!(ui.map_editor_map_load_requested.is_none());
        assert!(ui.has_unsaved_map_editor_draft());
    }

    #[test]
    fn finalize_saved_map_editor_draft_requests_reload_from_disk() {
        let mut ui = EditorUI::new();
        ui.set_map_editor_draft(MapEditorDraft {
            name: "draft_map".to_string(),
            tilemap: toki_core::assets::tilemap::TileMap {
                size: glam::UVec2::new(2, 2),
                tile_size: glam::UVec2::new(8, 8),
                atlas: std::path::PathBuf::from("terrain.json"),
                tiles: vec!["grass".to_string(); 4],
            },
        });

        ui.finalize_saved_map_editor_draft("draft_map".to_string());

        assert!(!ui.has_unsaved_map_editor_draft());
        assert!(!ui.has_unsaved_map_editor_changes());
        assert_eq!(ui.map_editor_active_map.as_deref(), Some("draft_map"));
        assert_eq!(
            ui.map_editor_map_load_requested.as_deref(),
            Some("draft_map")
        );
    }

    #[test]
    fn sync_map_editor_selection_preserves_dirty_loaded_map() {
        let mut ui = EditorUI::new();
        ui.map_editor_active_map = Some("middle".to_string());
        ui.mark_map_editor_dirty();

        ui.sync_map_editor_selection(&["alpha".to_string(), "middle".to_string()]);

        assert_eq!(ui.map_editor_active_map.as_deref(), Some("middle"));
        assert!(ui.map_editor_map_load_requested.is_none());
    }

    #[test]
    fn sync_map_editor_brush_selection_picks_first_sorted_tile() {
        let mut ui = EditorUI::new();

        ui.sync_map_editor_brush_selection(&[
            "water".to_string(),
            "grass".to_string(),
            "bush".to_string(),
        ]);

        assert_eq!(ui.map_editor_selected_tile.as_deref(), Some("bush"));
    }

    #[test]
    fn map_editor_defaults_to_drag_tool() {
        let ui = EditorUI::new();
        assert_eq!(ui.map_editor_tool, super::MapEditorTool::Drag);
        assert_eq!(ui.map_editor_brush_size_tiles, 1);
        assert!(ui.map_editor_selected_tile_info.is_none());
    }

    #[test]
    fn pick_map_editor_tile_sets_selected_tile_and_switches_back_to_brush() {
        let mut ui = EditorUI::new();
        ui.map_editor_tool = super::MapEditorTool::PickTile;

        ui.pick_map_editor_tile("water".to_string());

        assert_eq!(ui.map_editor_selected_tile.as_deref(), Some("water"));
        assert_eq!(ui.map_editor_tool, super::MapEditorTool::Brush);
    }

    #[test]
    fn map_editor_undo_and_redo_round_trip_a_draft_edit() {
        let mut ui = EditorUI::new();
        ui.center_panel_tab = super::CenterPanelTab::MapEditor;
        ui.set_map_editor_draft(MapEditorDraft {
            name: "draft_map".to_string(),
            tilemap: toki_core::assets::tilemap::TileMap {
                size: glam::UVec2::new(2, 2),
                tile_size: glam::UVec2::new(8, 8),
                atlas: std::path::PathBuf::from("terrain.json"),
                tiles: vec!["grass".to_string(); 4],
            },
        });

        let before = ui
            .map_editor_draft
            .as_ref()
            .expect("draft should exist")
            .tilemap
            .clone();
        let mut after = before.clone();
        after.tiles[0] = "water".to_string();

        ui.begin_map_editor_edit(&before);
        assert!(ui.finish_map_editor_edit(&after));
        assert!(ui.can_undo());

        assert!(ui.undo());
        let undone = ui
            .take_pending_map_editor_tilemap_sync()
            .expect("undo should queue a tilemap sync");
        assert_eq!(undone.tiles[0], "grass");

        assert!(ui.redo());
        let redone = ui
            .take_pending_map_editor_tilemap_sync()
            .expect("redo should queue a tilemap sync");
        assert_eq!(redone.tiles[0], "water");
    }

    #[test]
    fn map_editor_can_undo_prefers_map_history_when_map_editor_tab_is_active() {
        let mut ui = EditorUI::new();
        assert!(ui.execute_command(EditorCommand::add_entity(
            "Main Scene",
            sample_entity(1, IVec2::new(0, 0))
        )));
        ui.center_panel_tab = super::CenterPanelTab::MapEditor;
        assert!(!ui.can_undo());

        ui.set_map_editor_draft(MapEditorDraft {
            name: "draft_map".to_string(),
            tilemap: toki_core::assets::tilemap::TileMap {
                size: glam::UVec2::new(1, 1),
                tile_size: glam::UVec2::new(8, 8),
                atlas: std::path::PathBuf::from("terrain.json"),
                tiles: vec!["grass".to_string()],
            },
        });
        let before = ui.map_editor_draft.as_ref().unwrap().tilemap.clone();
        let mut after = before.clone();
        after.tiles[0] = "water".to_string();
        ui.begin_map_editor_edit(&before);
        assert!(ui.finish_map_editor_edit(&after));

        assert!(ui.can_undo());
        assert!(ui.undo());
        assert!(ui.take_pending_map_editor_tilemap_sync().is_some());
    }
}
