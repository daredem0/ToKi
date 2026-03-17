use super::inspector::InspectorSystem;
use super::menus::MenuSystem;
use super::panels::PanelSystem;
use super::rule_graph::RuleGraph;
use super::undo_redo::UndoRedoHistory;
use crate::project::SceneGraphLayout;
use crate::scene::SceneViewport;

#[path = "editor_ui_asset_palette.rs"]
mod editor_ui_asset_palette;
#[path = "editor_ui_graph.rs"]
mod editor_ui_graph;
#[path = "editor_ui_hierarchy_panel.rs"]
mod editor_ui_hierarchy_panel;
#[path = "editor_ui_map_editor.rs"]
mod editor_ui_map_editor;
#[path = "editor_ui_menu_editor.rs"]
mod editor_ui_menu_editor;
#[path = "editor_ui_scene_tree.rs"]
mod editor_ui_scene_tree;

pub(crate) use editor_ui_graph::SceneRulesGraphCommandData;
pub(crate) use editor_ui_map_editor::{
    MapEditorDraft, MapEditorHistory, MapEditorObjectInfo, MapEditorObjectPropertyEditRequest,
    MapEditorTileInfo, MapEditorTool, MapObjectMoveDragState, NewMapRequest,
};
use std::collections::HashMap;
use std::path::PathBuf;
use toki_core::{
    assets::tilemap::TileMap,
    entity::{Entity, EntityId},
    Scene,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    Scene(String),
    Map(String, String), // (scene_name, map_name)
    Entity(EntityId),
    StandaloneMap(String), // Map selected from Maps panel (not in scene context)
    EntityDefinition(String), // Entity definition from palette
    MenuScreen(String),
    MenuEntry {
        screen_id: String,
        item_index: usize,
    },
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
    MenuEditor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RightPanelTab {
    Inspector,
    Project,
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
pub struct PlacementPreviewVisual {
    pub frame: toki_core::sprite::SpriteFrame,
    pub texture_path: Option<PathBuf>,
    pub size: glam::UVec2,
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
    pub show_runtime_entities: bool,
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
    pub map_editor_selected_object_sheet: Option<String>,
    pub map_editor_selected_object_name: Option<String>,
    pub map_editor_tool: MapEditorTool,
    pub map_editor_brush_size_tiles: u32,
    pub map_editor_brush_preview_image_path: Option<PathBuf>,
    pub map_editor_brush_preview_texture: Option<egui::TextureHandle>,
    pub map_editor_selected_tile_info: Option<MapEditorTileInfo>,
    pub map_editor_selected_object_info: Option<MapEditorObjectInfo>,
    pub map_editor_show_new_map_dialog: bool,
    pub map_editor_new_map_name: String,
    pub map_editor_new_map_width: u32,
    pub map_editor_new_map_height: u32,
    pub map_editor_new_map_requested: Option<NewMapRequest>,
    pub map_editor_save_requested: bool,
    pub map_editor_history: MapEditorHistory,
    pub map_editor_pending_tilemap_sync: Option<TileMap>,
    pub map_editor_edit_before: Option<TileMap>,
    pub map_object_move_drag: Option<MapObjectMoveDragState>,
    pub map_editor_object_edit_requested: Option<MapEditorObjectPropertyEditRequest>,

    // Asset validation
    pub validate_assets_requested: bool,

    // Entity placement system
    pub placement_mode: bool,
    pub placement_entity_definition: Option<String>,
    pub placement_preview_position: Option<glam::Vec2>, // World coordinates for preview
    pub placement_preview_cached_frame: Option<PlacementPreviewVisual>, // Cached preview visual for placement
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
            show_runtime_entities: false,
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
            map_editor_selected_object_sheet: None,
            map_editor_selected_object_name: None,
            map_editor_tool: MapEditorTool::Drag,
            map_editor_brush_size_tiles: 1,
            map_editor_brush_preview_image_path: None,
            map_editor_brush_preview_texture: None,
            map_editor_selected_tile_info: None,
            map_editor_selected_object_info: None,
            map_editor_show_new_map_dialog: false,
            map_editor_new_map_name: "new_map".to_string(),
            map_editor_new_map_width: 32,
            map_editor_new_map_height: 32,
            map_editor_new_map_requested: None,
            map_editor_save_requested: false,
            map_editor_history: MapEditorHistory::default(),
            map_editor_pending_tilemap_sync: None,
            map_editor_edit_before: None,
            map_object_move_drag: None,
            map_editor_object_edit_requested: None,

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
        mut project: Option<&mut crate::project::Project>,
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
            self.render_hierarchy_and_maps_combined_panel(ctx, game_state, config_readonly);
        }

        if self.show_inspector {
            InspectorSystem::render_inspector_panel(
                self,
                ctx,
                game_state,
                project.as_deref_mut(),
                config_readonly,
            );
        }

        if self.center_panel_tab == CenterPanelTab::MenuEditor {
            self.sync_menu_editor_selection(project.as_deref());
        }

        // Render viewport last (mutable access)
        PanelSystem::render_viewport(
            self,
            ctx,
            scene_viewport,
            map_editor_viewport,
            project,
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
}

#[cfg(test)]
#[path = "editor_ui_tests.rs"]
mod tests;
