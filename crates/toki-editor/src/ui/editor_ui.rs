use super::inspector::InspectorSystem;
use super::menus::MenuSystem;
use super::panels::PanelSystem;
use super::rule_graph::RuleGraph;
use super::undo_redo::UndoRedoHistory;
use crate::project::ProjectTemplateKind;
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
    scene::SceneAnchorKind,
    Scene,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    Scene(String),
    ScenePlayerEntry(String),
    SceneAnchor {
        scene_name: String,
        anchor_id: String,
    },
    Map(String, String), // (scene_name, map_name)
    Entity(EntityId),
    StandaloneMap(String), // Map selected from Maps panel (not in scene context)
    EntityDefinition(String), // Entity definition from palette
    MenuScreen(String),
    MenuDialog(String),
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

#[derive(Debug, Clone)]
pub struct SceneAnchorMoveDragState {
    pub scene_name: String,
    pub anchor: toki_core::scene::SceneAnchor,
    pub grab_offset: glam::Vec2,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewProjectRequest {
    pub template: ProjectTemplateKind,
    pub parent_path: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntitySelectionState {
    primary: Option<EntityId>,
    ids: Vec<EntityId>,
}

/// UI panel visibility and editor lifecycle flags
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UIVisibilityState {
    pub show_hierarchy: bool,
    pub show_inspector: bool,
    pub show_maps: bool,
    pub show_runtime_entities: bool,
    pub should_exit: bool,
    pub show_console: bool,
    pub create_test_entities: bool,
}

impl Default for UIVisibilityState {
    fn default() -> Self {
        Self {
            show_hierarchy: true,
            show_inspector: true,
            show_maps: true,
            show_runtime_entities: false,
            should_exit: false,
            show_console: true,
            create_test_entities: false,
        }
    }
}

/// Project management state: project lifecycle, dialogs, and background tasks
#[derive(Debug, Clone)]
pub struct ProjectEditorState {
    // Project request flags
    pub new_project_requested: bool,
    pub new_top_down_project_requested: bool,
    pub open_project_requested: bool,
    pub browse_for_project_requested: bool,
    pub save_project_requested: bool,
    pub export_project_requested: bool,
    pub play_scene_requested: bool,
    pub init_config_requested: bool,
    pub validate_assets_requested: bool,

    // New project dialog state
    pub show_new_project_dialog: bool,
    pub new_project_template: ProjectTemplateKind,
    pub new_project_parent_directory: Option<PathBuf>,
    pub new_project_name: String,
    pub new_project_submit_requested: Option<NewProjectRequest>,

    // Background task state
    pub background_task_running: bool,
    pub background_task_status: Option<String>,
    pub cancel_background_task_requested: bool,

    // Window state
    pub window_title: Option<String>,
    pub pending_confirmation: Option<EditorConfirmation>,
}

impl Default for ProjectEditorState {
    fn default() -> Self {
        Self {
            new_project_requested: false,
            new_top_down_project_requested: false,
            open_project_requested: false,
            browse_for_project_requested: false,
            save_project_requested: false,
            export_project_requested: false,
            play_scene_requested: false,
            init_config_requested: false,
            validate_assets_requested: false,
            show_new_project_dialog: false,
            new_project_template: ProjectTemplateKind::Empty,
            new_project_parent_directory: None,
            new_project_name: "NewProject".to_string(),
            new_project_submit_requested: None,
            background_task_running: false,
            background_task_status: None,
            cancel_background_task_requested: false,
            window_title: Some("No project open".to_string()),
            pending_confirmation: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorConfirmation {
    DeleteScene { scene_name: String },
}

impl ProjectEditorState {
    pub fn begin_new_project_dialog(
        &mut self,
        template: ProjectTemplateKind,
        suggested_parent_directory: Option<PathBuf>,
        suggested_name: String,
    ) {
        self.show_new_project_dialog = true;
        self.new_project_template = template;
        self.new_project_parent_directory = suggested_parent_directory;
        if !suggested_name.trim().is_empty() {
            self.new_project_name = suggested_name;
        }
    }

    pub fn submit_new_project_request(&mut self) {
        let Some(parent_path) = self.new_project_parent_directory.clone() else {
            return;
        };
        let name = self.new_project_name.trim().to_string();
        if name.is_empty() {
            return;
        }

        self.new_project_submit_requested = Some(NewProjectRequest {
            template: self.new_project_template,
            parent_path,
            name,
        });
        self.show_new_project_dialog = false;
    }

    /// Sets the window title
    pub fn set_window_title(&mut self, title: &str) {
        self.window_title = Some(title.to_string());
    }
}

/// Entity placement and drag interaction state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneAnchorPlacementDraft {
    pub kind: SceneAnchorKind,
    pub suggested_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementKind {
    EntityDefinition(String),
    SceneAnchor(SceneAnchorPlacementDraft),
}

#[derive(Debug, Clone, Default)]
pub struct PlacementState {
    pub kind: Option<PlacementKind>,
    pub preview_position: Option<glam::Vec2>,
    pub preview_cached_frame: Option<PlacementPreviewVisual>,
    pub preview_valid: Option<bool>,
    pub entity_move_drag: Option<EntityMoveDragState>,
    pub scene_anchor_move_drag: Option<SceneAnchorMoveDragState>,
    pub marquee_selection: Option<MarqueeSelectionState>,
}

impl PlacementState {
    pub fn enter_placement_mode(&mut self, entity_definition: String) {
        self.kind = Some(PlacementKind::EntityDefinition(entity_definition));
        tracing::info!(
            "Entered placement mode for entity: {:?}",
            self.entity_definition()
        );
    }

    pub fn enter_scene_anchor_placement_mode(&mut self, draft: SceneAnchorPlacementDraft) {
        tracing::info!(
            "Entered placement mode for scene anchor '{}' ({:?})",
            draft.suggested_id,
            draft.kind
        );
        self.kind = Some(PlacementKind::SceneAnchor(draft));
    }

    pub fn exit_placement_mode(&mut self) {
        if self.kind.is_some() {
            tracing::info!("Exiting placement mode");
        }
        self.kind = None;
        self.preview_position = None;
        self.preview_cached_frame = None;
        self.preview_valid = None;
        self.entity_move_drag = None;
        self.scene_anchor_move_drag = None;
        self.marquee_selection = None;
    }

    pub fn is_in_placement_mode(&self) -> bool {
        self.kind.is_some()
    }

    pub fn entity_definition(&self) -> Option<&str> {
        match &self.kind {
            Some(PlacementKind::EntityDefinition(name)) => Some(name.as_str()),
            _ => None,
        }
    }

    pub fn scene_anchor_draft(&self) -> Option<&SceneAnchorPlacementDraft> {
        match &self.kind {
            Some(PlacementKind::SceneAnchor(draft)) => Some(draft),
            _ => None,
        }
    }

    pub fn begin_entity_move_drag(&mut self, drag_state: EntityMoveDragState) {
        self.entity_move_drag = Some(drag_state);
    }

    pub fn is_entity_move_drag_active(&self) -> bool {
        self.entity_move_drag.is_some()
    }

    pub fn begin_scene_anchor_move_drag(&mut self, drag_state: SceneAnchorMoveDragState) {
        self.scene_anchor_move_drag = Some(drag_state);
    }

    pub fn is_scene_anchor_move_drag_active(&self) -> bool {
        self.scene_anchor_move_drag.is_some()
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
}

/// Scene graph editor state: connection mode, view state, and persistent layouts
#[derive(Debug, Clone)]
pub struct GraphEditorState {
    pub connect_from_node: Option<u64>,
    pub connect_to_node: Option<u64>,
    pub canvas_zoom: f32,
    pub canvas_pan: [f32; 2],
    pub layouts_by_scene: HashMap<String, SceneGraphLayout>,
    pub layout_dirty: bool,
    pub rule_graphs_by_scene: HashMap<String, RuleGraph>,
}

impl Default for GraphEditorState {
    fn default() -> Self {
        Self {
            connect_from_node: None,
            connect_to_node: None,
            canvas_zoom: 1.0,
            canvas_pan: [16.0, 16.0],
            layouts_by_scene: HashMap::new(),
            layout_dirty: false,
            rule_graphs_by_scene: HashMap::new(),
        }
    }
}

/// Request to load a map from a specific scene
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapLoadRequest {
    pub scene_name: String,
    pub map_name: String,
}

/// Map editor state: tilemap editing tools, selection, drafts, and history
pub struct MapEditorState {
    pub load_requested: Option<MapLoadRequest>,
    pub active_map: Option<String>,
    pub map_load_requested: Option<String>,
    pub draft: Option<MapEditorDraft>,
    pub dirty: bool,
    pub selected_tile: Option<String>,
    pub selected_object_sheet: Option<String>,
    pub selected_object_name: Option<String>,
    pub tool: MapEditorTool,
    pub brush_size_tiles: u32,
    pub brush_preview_image_path: Option<PathBuf>,
    pub brush_preview_texture: Option<egui::TextureHandle>,
    pub selected_tile_info: Option<MapEditorTileInfo>,
    pub selected_object_info: Option<MapEditorObjectInfo>,
    pub show_new_map_dialog: bool,
    pub new_map_name: String,
    pub new_map_width: u32,
    pub new_map_height: u32,
    pub new_map_tile_width: u32,
    pub new_map_tile_height: u32,
    pub new_map_requested: Option<NewMapRequest>,
    pub save_requested: bool,
    pub history: MapEditorHistory,
    pub pending_tilemap_sync: Option<TileMap>,
    pub edit_before: Option<TileMap>,
    pub object_move_drag: Option<MapObjectMoveDragState>,
    pub object_edit_requested: Option<MapEditorObjectPropertyEditRequest>,
}

impl Default for MapEditorState {
    fn default() -> Self {
        Self {
            load_requested: None,
            active_map: None,
            map_load_requested: None,
            draft: None,
            dirty: false,
            selected_tile: None,
            selected_object_sheet: None,
            selected_object_name: None,
            tool: MapEditorTool::Drag,
            brush_size_tiles: 1,
            brush_preview_image_path: None,
            brush_preview_texture: None,
            selected_tile_info: None,
            selected_object_info: None,
            show_new_map_dialog: false,
            new_map_name: "new_map".to_string(),
            new_map_width: 32,
            new_map_height: 32,
            new_map_tile_width: 16,
            new_map_tile_height: 16,
            new_map_requested: None,
            save_requested: false,
            history: MapEditorHistory::default(),
            pending_tilemap_sync: None,
            edit_before: None,
            object_move_drag: None,
            object_edit_requested: None,
        }
    }
}

/// Manages the editor's UI state and rendering
pub struct EditorUI {
    // Scene management
    pub scenes: Vec<Scene>,
    pub selection: Option<Selection>,
    pub active_scene: Option<String>, // Name of currently active scene
    pub scene_content_changed: bool,  // Flag to signal that scene content changed

    // Entity selection
    entity_selection: EntitySelectionState,

    // UI Panel visibility
    pub visibility: UIVisibilityState,

    // Project management
    pub project: ProjectEditorState,

    pub right_panel_tab: RightPanelTab,

    // Map editor state
    pub map: MapEditorState,

    // Entity placement system
    pub placement: PlacementState,

    pub center_panel_tab: CenterPanelTab, // Active tab in center workspace

    // Scene graph editor state
    pub graph: GraphEditorState,

    pub command_history: UndoRedoHistory, // Undo/redo command history for scene mutations

    // Multi-entity inspector draft state
    pub multi_entity_render_layer_input: i64,
    pub multi_entity_delta_x_input: i32,
    pub multi_entity_delta_y_input: i32,
    pub multi_entity_inspector_selection_signature: Vec<EntityId>,
    pub menu_preview_font_families: Vec<String>,
}

impl EditorUI {
    pub fn new() -> Self {
        Self {
            // Scene management
            scenes: vec![Scene::new("Main Scene".to_string())], // Start with default scene
            selection: None,
            active_scene: Some("Main Scene".to_string()), // Default scene starts active
            scene_content_changed: false,

            // Entity selection
            entity_selection: EntitySelectionState::default(),

            // UI Panel visibility
            visibility: UIVisibilityState::default(),

            // Project management
            project: ProjectEditorState::default(),

            right_panel_tab: RightPanelTab::Inspector,

            // Map editor state
            map: MapEditorState::default(),

            // Entity placement system
            placement: PlacementState::default(),

            center_panel_tab: CenterPanelTab::SceneViewport,

            // Scene graph editor state
            graph: GraphEditorState::default(),

            command_history: UndoRedoHistory::default(),
            multi_entity_render_layer_input: 0,
            multi_entity_delta_x_input: 0,
            multi_entity_delta_y_input: 0,
            multi_entity_inspector_selection_signature: Vec::new(),
            menu_preview_font_families: vec![
                "Sans".to_string(),
                "Serif".to_string(),
                "Mono".to_string(),
            ],
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
        self.graph.rule_graphs_by_scene.clear();
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
            self.entity_selection.primary = Some(entity_id);
            self.entity_selection.ids = vec![entity_id];
            self.selection = Some(Selection::Entity(entity_id));
            return;
        }
        self.clear_entity_selection_state();
        self.selection = Some(selection);
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
        self.entity_selection = EntitySelectionState::default();
    }

    pub fn begin_new_project_dialog(
        &mut self,
        template: ProjectTemplateKind,
        suggested_parent_directory: Option<PathBuf>,
        suggested_name: String,
    ) {
        self.project
            .begin_new_project_dialog(template, suggested_parent_directory, suggested_name);
    }

    pub fn submit_new_project_request(&mut self) {
        self.project.submit_new_project_request();
    }

    pub fn set_single_entity_selection(&mut self, entity_id: EntityId) {
        self.entity_selection.primary = Some(entity_id);
        self.entity_selection.ids.clear();
        self.entity_selection.ids.push(entity_id);
        self.selection = Some(Selection::Entity(entity_id));
    }

    pub fn toggle_entity_selection(&mut self, entity_id: EntityId) {
        if let Some(index) = self
            .entity_selection
            .ids
            .iter()
            .position(|id| *id == entity_id)
        {
            self.entity_selection.ids.remove(index);
            if self.entity_selection.ids.is_empty() {
                self.clear_selection();
                return;
            }
            if self.entity_selection.primary == Some(entity_id) {
                if let Some(last_selected) = self.entity_selection.ids.last().copied() {
                    self.entity_selection.primary = Some(last_selected);
                    self.selection = Some(Selection::Entity(last_selected));
                }
            }
            return;
        }

        self.entity_selection.ids.push(entity_id);
        self.entity_selection.primary = Some(entity_id);
        self.selection = Some(Selection::Entity(entity_id));
    }

    pub fn has_multi_entity_selection(&self) -> bool {
        self.entity_selection.ids.len() > 1
    }

    pub fn clear_entity_selection(&mut self) {
        self.clear_selection();
    }

    pub(crate) fn clear_entity_selection_state(&mut self) {
        self.entity_selection = EntitySelectionState::default();
    }

    pub fn enter_placement_mode(&mut self, entity_definition: String) {
        self.placement.enter_placement_mode(entity_definition);
    }

    pub fn enter_scene_anchor_placement_mode(&mut self, draft: SceneAnchorPlacementDraft) {
        self.placement.enter_scene_anchor_placement_mode(draft);
    }

    pub fn exit_placement_mode(&mut self) {
        self.placement.exit_placement_mode();
    }

    pub fn is_in_placement_mode(&self) -> bool {
        self.placement.is_in_placement_mode()
    }

    pub fn begin_entity_move_drag(&mut self, drag_state: EntityMoveDragState) {
        self.placement.begin_entity_move_drag(drag_state);
    }

    pub fn is_entity_move_drag_active(&self) -> bool {
        self.placement.is_entity_move_drag_active()
    }

    pub fn begin_scene_anchor_move_drag(&mut self, drag_state: SceneAnchorMoveDragState) {
        self.placement.begin_scene_anchor_move_drag(drag_state);
    }

    pub fn is_scene_anchor_move_drag_active(&self) -> bool {
        self.placement.is_scene_anchor_move_drag_active()
    }

    pub fn start_marquee_selection(&mut self, start: egui::Pos2) {
        self.placement.start_marquee_selection(start);
    }

    pub fn update_marquee_selection(&mut self, current: egui::Pos2) {
        self.placement.update_marquee_selection(current);
    }

    pub fn finish_marquee_selection(&mut self) -> Option<MarqueeSelectionState> {
        self.placement.finish_marquee_selection()
    }

    pub fn is_marquee_selection_active(&self) -> bool {
        self.placement.is_marquee_selection_active()
    }

    pub fn add_entity_to_selection(&mut self, entity_id: EntityId) {
        if !self.entity_selection.ids.contains(&entity_id) {
            self.entity_selection.ids.push(entity_id);
        }
        self.entity_selection.primary = Some(entity_id);
        self.selection = Some(Selection::Entity(entity_id));
    }

    #[cfg(test)]
    pub fn selected_entity_id(&self) -> Option<EntityId> {
        self.entity_selection.primary
    }

    pub fn selected_entity_ids(&self) -> &[EntityId] {
        &self.entity_selection.ids
    }

    pub fn selected_entity_ids_vec(&self) -> Vec<EntityId> {
        self.entity_selection.ids.clone()
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
        MenuSystem::render_top_menu(
            self,
            ctx,
            project.as_deref_mut(),
            config_readonly,
            busy_logo_texture,
        );

        // Render log panel first to claim full width at bottom
        if self.visibility.show_console {
            PanelSystem::render_log_panel(self, ctx, log_capture);
        }

        // Render hierarchy and inspector panels
        let game_state = scene_viewport.as_ref().map(|v| v.game_state());

        if self.visibility.show_hierarchy {
            self.render_hierarchy_and_maps_combined_panel(ctx, game_state, config_readonly);
        }

        if self.visibility.show_inspector {
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
        self.visibility.show_hierarchy = config.editor_settings.panels.hierarchy_visible;
        self.visibility.show_inspector = config.editor_settings.panels.inspector_visible;
        self.visibility.show_console = config.editor_settings.panels.console_visible;
    }

    pub fn set_title(&mut self, title: &str) {
        self.project.set_window_title(title);
    }
}

#[cfg(test)]
#[path = "editor_ui_tests.rs"]
mod tests;
