use super::graph_canvas::GraphCanvasState;
use super::inspector::InspectorSystem;
use super::menus::MenuSystem;
use super::panels::PanelSystem;
use super::rule_graph::RuleGraph;
use super::undo_redo::UndoRedoHistory;
use crate::editor_tab_strip::EditorTabStripState;
use crate::editor_types::PlacementPreviewVisual;
use crate::project::SceneGraphLayout;
use crate::project::{Project, ProjectSettingsDraft, ProjectTemplateKind};
use crate::scene::SceneViewport;
use crate::ui::editor_context::{
    default_active_context, default_parked_contexts, null_context, AnimationEditorContext,
    CenterPanelHost, DialogEditorContext, EditorContext, EntityEditorContext, RuleGraphContext,
    SceneViewportContext, SpriteEditorContext, UiEditorContext,
};
use toki_core::palette::{builtin_palettes, Palette4};

#[path = "editor_ui_animation_authoring.rs"]
mod editor_ui_animation_authoring;
#[path = "editor_ui_animation_editor.rs"]
mod editor_ui_animation_editor;
#[path = "editor_ui_asset_palette.rs"]
mod editor_ui_asset_palette;
#[path = "editor_ui_dialog_editor.rs"]
mod editor_ui_dialog_editor;
#[path = "editor_ui_entity_editor.rs"]
mod editor_ui_entity_editor;
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
#[path = "editor_ui_sprite_editor.rs"]
mod editor_ui_sprite_editor;
#[path = "editor_ui_ui_editor.rs"]
mod editor_ui_ui_editor;

pub(crate) use editor_ui_animation_authoring::AnimationAuthoringState;
pub(crate) use editor_ui_animation_editor::AnimationEditorState;
pub(crate) use editor_ui_dialog_editor::{
    clear_dialog_graph_layout_dirty, export_dialog_graph_layouts_for_project,
    is_dialog_graph_layout_dirty, load_dialog_graph_layouts_from_project, sync_dialog_registry,
    DialogEditorState,
};
pub(crate) use editor_ui_entity_editor::{
    create_default_definition, EntityCategory, EntityEditState, EntitySummary,
};
pub(crate) use editor_ui_graph::{
    clear_graph_layout_dirty, execute_scene_rules_graph_command, export_graph_layouts_for_project,
    export_rule_graph_drafts_for_project, graph_layout_position, graph_view_for_scene,
    is_graph_layout_dirty, load_graph_layouts_from_project, load_rule_graph_drafts_from_project,
    rule_graph_for_scene, set_graph_view_for_scene, set_rule_graph_for_scene,
    sync_rule_graph_with_rule_set, SceneRulesGraphCommandData,
};
#[allow(unused_imports)]
pub(crate) use editor_ui_map_editor::{
    add_layer_to_map, begin_map_editor_edit, begin_new_map_dialog, cancel_map_editor_edit,
    clear_map_editor_dirty, clear_map_editor_history, finalize_saved_existing_map,
    finalize_saved_map_editor_draft, finish_map_editor_edit, has_unsaved_map_editor_changes,
    has_unsaved_map_editor_draft, map_editor_selected_label, mark_map_editor_dirty, move_layer,
    pick_map_editor_tile, remove_layer_from_map, rename_layer, set_active_layer,
    set_map_editor_draft, submit_new_map_request, sync_map_editor_brush_selection,
    sync_map_editor_selection, take_pending_map_editor_tilemap_sync, toggle_layer_above_entities,
    toggle_layer_visibility, MapEditorDraft, MapEditorHistory, MapEditorTileInfo, MapEditorTool,
    NewMapRequest,
};
pub(crate) use editor_ui_menu_editor::{
    select_menu_dialog, select_menu_entry, select_menu_screen, selected_menu_dialog_id,
    selected_menu_screen_id, sync_menu_editor_selection,
};
pub(crate) use editor_ui_sprite_editor::{
    begin_new_sprite_canvas_dialog, cancel_new_sprite_canvas_dialog, CanvasSide, DualCanvasLayout,
    PixelColor, ResizeAnchor, SelectionMask, SpriteAssetKind, SpriteCanvas, SpriteCanvasViewport,
    SpriteEditorState, SpriteEditorTool, SpriteSelection,
};
pub(crate) use editor_ui_ui_editor::{
    clear_ui_layout_view_dirty, export_ui_layout_views_for_project, is_ui_layout_view_dirty,
    load_ui_layout_views_from_project, sync_ui_layout_registry, UiCanvasInteraction, UiEditorState,
};
use std::collections::{BTreeMap, HashMap};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum CenterPanelTab {
    SceneViewport,
    SceneGraph,
    MapEditor,
    MenuEditor,
    DialogEditor,
    UiEditor,
    SpriteEditor,
    AnimationEditor,
    EntityEditor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RightPanelTab {
    Inspector,
    Project,
    Toolbox,
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

#[derive(Debug, Clone)]
pub struct WorkspaceUiState {
    pub right_panel_tab: RightPanelTab,
    pub center_panel_tab: CenterPanelTab,
    pub center_panel_tab_strip: EditorTabStripState,
}

impl Default for WorkspaceUiState {
    fn default() -> Self {
        Self {
            right_panel_tab: RightPanelTab::Inspector,
            center_panel_tab: CenterPanelTab::SceneViewport,
            center_panel_tab_strip: EditorTabStripState::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MultiEntityInspectorState {
    pub render_layer_input: i64,
    pub delta_x_input: i32,
    pub delta_y_input: i32,
    pub selection_signature: Vec<EntityId>,
}

#[derive(Debug, Clone, Default)]
pub struct ViewportCursorState {
    pub world_position: Option<glam::IVec2>,
    pub show_tiles: bool,
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
    // Pending project request
    pub pending_request: Option<ProjectRequest>,

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
    pub available_palettes: BTreeMap<String, Palette4>,
    pub available_dialog_outcomes: BTreeMap<String, Vec<String>>,
    pub indexed_palette_override: Option<String>,
}

impl Default for ProjectEditorState {
    fn default() -> Self {
        Self {
            pending_request: None,
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
            available_palettes: builtin_palettes(),
            available_dialog_outcomes: BTreeMap::new(),
            indexed_palette_override: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorConfirmation {
    DeleteScene { scene_name: String },
    ExitEditor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectRequest {
    NewProject,
    NewTopDownProject,
    OpenProject,
    BrowseForProject,
    ReloadProjectAssets,
    SaveProject,
    ExportProject,
    PlayScene,
    InitConfig,
    ValidateAssets,
}

impl ProjectEditorState {
    pub fn request(&mut self, request: ProjectRequest) {
        self.pending_request = Some(request);
    }

    pub fn take_request(&mut self, request: ProjectRequest) -> bool {
        if self.pending_request == Some(request) {
            self.pending_request = None;
            true
        } else {
            false
        }
    }

    pub fn set_available_palettes(&mut self, project_palettes: &BTreeMap<String, Palette4>) {
        let mut palettes = builtin_palettes();
        palettes.extend(
            project_palettes
                .iter()
                .map(|(id, palette)| (id.clone(), *palette)),
        );
        self.available_palettes = palettes;
    }

    pub fn set_available_dialogs(&mut self, dialogs: &BTreeMap<String, Vec<String>>) {
        self.available_dialog_outcomes = dialogs.clone();
    }

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
pub struct DecorationPlacementDraft {
    pub sheet: String,
    pub object_name: String,
    pub size_px: glam::UVec2,
    pub grounding: toki_core::entity::EntityGrounding,
    pub visible: bool,
    pub solid: bool,
}

/// Which kind-category tab is active in the Toolbox panel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ToolboxTab {
    Creatures,
    Humans,
    Items,
    #[default]
    Decorations,
}

impl ToolboxTab {
    pub const ALL: &[Self] = &[
        Self::Creatures,
        Self::Humans,
        Self::Items,
        Self::Decorations,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Creatures => "Creatures",
            Self::Humans => "Humans",
            Self::Items => "Items",
            Self::Decorations => "Decorations",
        }
    }
}

#[derive(Default)]
pub struct SceneToolboxState {
    pub selected_tab: ToolboxTab,
    pub selected_object_sheet: Option<String>,
    pub selected_object_name: Option<String>,
    pub preview_image_path: Option<PathBuf>,
    pub preview_texture: Option<egui::TextureHandle>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementKind {
    EntityDefinition(String),
    Decoration(DecorationPlacementDraft),
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
        self.preview_position = None;
        self.preview_cached_frame = None;
        self.preview_valid = None;
    }

    pub fn enter_decoration_placement_mode(&mut self, draft: DecorationPlacementDraft) {
        tracing::info!(
            "Entered placement mode for decoration '{}:{}'",
            draft.sheet,
            draft.object_name
        );
        self.kind = Some(PlacementKind::Decoration(draft));
        self.preview_position = None;
        self.preview_cached_frame = None;
        self.preview_valid = None;
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

    pub fn decoration_draft(&self) -> Option<&DecorationPlacementDraft> {
        match &self.kind {
            Some(PlacementKind::Decoration(draft)) => Some(draft),
            _ => None,
        }
    }

    pub fn scene_anchor_draft(&self) -> Option<&SceneAnchorPlacementDraft> {
        match &self.kind {
            Some(PlacementKind::SceneAnchor(draft)) => Some(draft),
            _ => None,
        }
    }

    pub fn mode_label(&self) -> Option<String> {
        match &self.kind {
            Some(PlacementKind::EntityDefinition(name)) => Some(format!("Entity: {name}")),
            Some(PlacementKind::Decoration(draft)) => {
                Some(format!("Object: {}/{}", draft.sheet, draft.object_name))
            }
            Some(PlacementKind::SceneAnchor(draft)) => {
                Some(format!("Anchor: {}", draft.suggested_id))
            }
            None => None,
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

/// Which sub-view is active inside the Scene Editor tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SceneEditorSubView {
    #[default]
    Graph,
    Rules,
}

/// Scene graph editor state: connection mode, view state, and persistent layouts
#[derive(Debug, Clone, Default)]
pub struct GraphEditorState {
    pub connect_from_node: Option<u64>,
    pub connect_to_node: Option<u64>,
    pub canvas_state: GraphCanvasState,
    pub sub_view: SceneEditorSubView,
    pub layouts_by_scene: HashMap<String, SceneGraphLayout>,
    pub layout_dirty: bool,
    pub rule_graphs_by_scene: HashMap<String, RuleGraph>,
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
    pub tool: MapEditorTool,
    pub brush_size_tiles: u32,
    pub brush_preview_image_path: Option<PathBuf>,
    pub brush_preview_texture: Option<egui::TextureHandle>,
    pub selected_tile_info: Option<MapEditorTileInfo>,
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
    pub active_layer: usize,
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
            tool: MapEditorTool::Drag,
            brush_size_tiles: 1,
            brush_preview_image_path: None,
            brush_preview_texture: None,
            selected_tile_info: None,
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
            active_layer: 0,
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
    pub project_settings_draft: Option<(PathBuf, ProjectSettingsDraft)>,

    pub workspace: WorkspaceUiState,
    active_tab: CenterPanelTab,
    active_context: Box<dyn EditorContext>,
    parked_contexts: HashMap<CenterPanelTab, Box<dyn EditorContext>>,

    pub command_history: UndoRedoHistory, // Undo/redo command history for scene mutations

    // Multi-entity inspector draft state
    pub multi_entity: MultiEntityInspectorState,
    pub menu_preview_font_families: Vec<String>,
}

pub struct EditorRenderContext<'a> {
    pub scene_viewport: Option<&'a mut SceneViewport>,
    pub map_editor_viewport: Option<&'a mut SceneViewport>,
    pub project: Option<&'a mut crate::project::Project>,
    pub project_assets: Option<&'a mut crate::project::ProjectAssets>,
    pub available_map_names: Option<Vec<String>>,
    pub config: Option<&'a mut crate::config::EditorConfig>,
    pub log_capture: Option<&'a crate::logging::LogCapture>,
    pub renderer: Option<&'a mut egui_wgpu::Renderer>,
    pub busy_logo_texture: Option<&'a egui::TextureHandle>,
}

impl EditorUI {
    pub fn active_tab(&self) -> CenterPanelTab {
        self.active_tab
    }

    pub fn set_active_tab(&mut self, tab: CenterPanelTab) {
        if self.active_tab == tab {
            return;
        }

        let mut old_context = std::mem::replace(&mut self.active_context, null_context());
        old_context.on_deactivate(self);
        self.parked_contexts.insert(self.active_tab, old_context);

        let mut new_context = self
            .parked_contexts
            .remove(&tab)
            .unwrap_or_else(|| default_active_context(tab));
        self.active_tab = tab;
        self.workspace.center_panel_tab = tab;
        new_context.on_activate(self);
        self.active_context = new_context;
    }

    pub(crate) fn context<T: 'static>(&self, tab: CenterPanelTab) -> Option<&T> {
        if self.active_tab == tab {
            self.active_context
                .as_any()
                .downcast_ref::<T>()
                .or_else(|| {
                    self.parked_contexts
                        .get(&tab)
                        .and_then(|context| context.as_any().downcast_ref::<T>())
                })
        } else {
            match self.parked_contexts.get(&tab) {
                Some(context) => context.as_any().downcast_ref::<T>(),
                None => None,
            }
        }
    }

    pub(crate) fn context_mut<T: 'static>(&mut self, tab: CenterPanelTab) -> Option<&mut T> {
        if self.active_tab == tab {
            self.active_context
                .as_any_mut()
                .downcast_mut::<T>()
                .or_else(|| {
                    self.parked_contexts
                        .get_mut(&tab)
                        .and_then(|context| context.as_any_mut().downcast_mut::<T>())
                })
        } else {
            match self.parked_contexts.get_mut(&tab) {
                Some(context) => context.as_any_mut().downcast_mut::<T>(),
                None => None,
            }
        }
    }

    pub(crate) fn with_active_context<R>(
        &mut self,
        f: impl FnOnce(&mut dyn EditorContext, &mut EditorUI) -> R,
    ) -> R {
        let active_tab = self.active_tab;
        let active_context = std::mem::replace(&mut self.active_context, null_context());
        let replaced = self.parked_contexts.insert(active_tab, active_context);
        debug_assert!(
            replaced.is_none(),
            "active tab context should not already be parked"
        );

        let context_ptr = self
            .parked_contexts
            .get_mut(&active_tab)
            .expect("active tab context should be temporarily parked")
            .as_mut() as *mut dyn EditorContext;

        // SAFETY: `context_ptr` points to the boxed context we just inserted into
        // `parked_contexts` under `active_tab`. That box remains in the map for the
        // entire duration of `f`, and we do not mutate the map entry itself before
        // removing it afterwards. This lets the callback access the same active
        // context both directly and through `EditorUI` context lookups.
        let result = unsafe { f(&mut *context_ptr, self) };

        let active_context = self
            .parked_contexts
            .remove(&active_tab)
            .expect("active tab context should be restored after callback");
        self.active_context = active_context;
        result
    }

    pub(crate) fn active_context_ref(&self) -> &dyn EditorContext {
        self.active_context.as_ref()
    }

    pub(crate) fn scene_viewport_context(&self) -> &SceneViewportContext {
        self.context::<SceneViewportContext>(CenterPanelTab::SceneViewport)
            .expect("scene viewport context should always exist")
    }

    pub(crate) fn scene_viewport_context_mut(&mut self) -> &mut SceneViewportContext {
        self.context_mut::<SceneViewportContext>(CenterPanelTab::SceneViewport)
            .expect("scene viewport context should always exist")
    }

    fn existing_rule_graph_context_tab(&self) -> Option<CenterPanelTab> {
        let is_active_rule_graph = self.active_tab == CenterPanelTab::SceneGraph
            && self.active_context.as_any().is::<RuleGraphContext>();
        let has_stored_rule_graph = self
            .context::<RuleGraphContext>(CenterPanelTab::SceneGraph)
            .is_some();
        if is_active_rule_graph || has_stored_rule_graph {
            Some(CenterPanelTab::SceneGraph)
        } else {
            None
        }
    }

    fn preferred_rule_graph_context_tab(&self) -> CenterPanelTab {
        if self.active_tab == CenterPanelTab::SceneGraph {
            CenterPanelTab::SceneGraph
        } else {
            self.existing_rule_graph_context_tab()
                .unwrap_or(CenterPanelTab::SceneGraph)
        }
    }

    pub(crate) fn rule_graph_context_tab(&self) -> CenterPanelTab {
        self.preferred_rule_graph_context_tab()
    }

    fn ensure_rule_graph_context(&mut self) -> CenterPanelTab {
        if let Some(tab) = self.existing_rule_graph_context_tab() {
            return if self.active_tab == CenterPanelTab::SceneGraph {
                self.active_tab
            } else {
                tab
            };
        }

        let tab = CenterPanelTab::SceneGraph;
        self.parked_contexts
            .entry(tab)
            .or_insert_with(|| default_active_context(tab));
        tab
    }

    #[cfg(test)]
    pub(crate) fn rule_graph_context(&self) -> &RuleGraphContext {
        self.context::<RuleGraphContext>(self.rule_graph_context_tab())
            .expect("rule graph context should always exist")
    }

    pub(crate) fn rule_graph_context_mut(&mut self) -> &mut RuleGraphContext {
        let tab = self.ensure_rule_graph_context();
        self.context_mut::<RuleGraphContext>(tab)
            .expect("rule graph context should always exist")
    }

    pub(crate) fn dialog_editor_context(&self) -> &DialogEditorContext {
        self.context::<DialogEditorContext>(CenterPanelTab::DialogEditor)
            .expect("dialog editor context should always exist")
    }

    pub(crate) fn dialog_editor_context_mut(&mut self) -> &mut DialogEditorContext {
        self.context_mut::<DialogEditorContext>(CenterPanelTab::DialogEditor)
            .expect("dialog editor context should always exist")
    }

    pub(crate) fn ui_editor_context(&self) -> &UiEditorContext {
        self.context::<UiEditorContext>(CenterPanelTab::UiEditor)
            .expect("ui editor context should always exist")
    }

    pub(crate) fn ui_editor_context_mut(&mut self) -> &mut UiEditorContext {
        self.context_mut::<UiEditorContext>(CenterPanelTab::UiEditor)
            .expect("ui editor context should always exist")
    }

    pub(crate) fn sprite_editor_context(&self) -> &SpriteEditorContext {
        self.context::<SpriteEditorContext>(CenterPanelTab::SpriteEditor)
            .expect("sprite editor context should always exist")
    }

    pub(crate) fn sprite_editor_context_mut(&mut self) -> &mut SpriteEditorContext {
        self.context_mut::<SpriteEditorContext>(CenterPanelTab::SpriteEditor)
            .expect("sprite editor context should always exist")
    }

    pub(crate) fn animation_editor_context(&self) -> &AnimationEditorContext {
        self.context::<AnimationEditorContext>(CenterPanelTab::AnimationEditor)
            .expect("animation editor context should always exist")
    }

    pub(crate) fn animation_editor_context_mut(&mut self) -> &mut AnimationEditorContext {
        self.context_mut::<AnimationEditorContext>(CenterPanelTab::AnimationEditor)
            .expect("animation editor context should always exist")
    }

    pub(crate) fn entity_editor_context(&self) -> &EntityEditorContext {
        self.context::<EntityEditorContext>(CenterPanelTab::EntityEditor)
            .expect("entity editor context should always exist")
    }

    pub(crate) fn entity_editor_context_mut(&mut self) -> &mut EntityEditorContext {
        self.context_mut::<EntityEditorContext>(CenterPanelTab::EntityEditor)
            .expect("entity editor context should always exist")
    }

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
            project_settings_draft: None,

            workspace: WorkspaceUiState::default(),
            active_tab: CenterPanelTab::SceneViewport,
            active_context: default_active_context(CenterPanelTab::SceneViewport),
            parked_contexts: default_parked_contexts(CenterPanelTab::SceneViewport),

            command_history: UndoRedoHistory::default(),
            multi_entity: MultiEntityInspectorState::default(),
            menu_preview_font_families: vec![
                "Sans".to_string(),
                "Serif".to_string(),
                "Mono".to_string(),
            ],
        }
    }

    pub(crate) fn project_settings_draft_for(
        &mut self,
        project: &Project,
    ) -> &mut ProjectSettingsDraft {
        let needs_refresh = self
            .project_settings_draft
            .as_ref()
            .is_none_or(|(path, _)| path != &project.path);
        if needs_refresh {
            self.project_settings_draft = Some((
                project.path.clone(),
                ProjectSettingsDraft::from_project(project),
            ));
        }
        &mut self
            .project_settings_draft
            .as_mut()
            .expect("project settings draft should exist")
            .1
    }

    // Scene management methods
    pub fn add_scene(&mut self, name: String) -> &mut Scene {
        self.scenes.push(Scene::new(name));
        self.scenes
            .last_mut()
            .expect("scene list should contain the newly added scene")
    }

    pub fn get_scene(&self, name: &str) -> Option<&Scene> {
        self.scenes.iter().find(|s| s.name == name)
    }

    pub fn load_scenes_from_project(&mut self, loaded_scenes: Vec<Scene>) {
        tracing::info!("Loading {} scenes into UI hierarchy", loaded_scenes.len());
        self.scenes = loaded_scenes;
        crate::ui::editor_context::graph_state_mut(self)
            .rule_graphs_by_scene
            .clear();
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
    pub fn render(&mut self, ctx: &egui::Context, render_ctx: EditorRenderContext<'_>) {
        let EditorRenderContext {
            scene_viewport,
            map_editor_viewport,
            project,
            project_assets,
            available_map_names,
            config,
            log_capture,
            renderer,
            busy_logo_texture,
        } = render_ctx;
        let mut context_host = CenterPanelHost {
            scene_viewport,
            map_editor_viewport,
            project,
            project_assets,
            available_map_names,
            config,
            config_readonly: None,
            renderer,
        };
        let config_readonly = context_host.config.as_deref();
        MenuSystem::render_top_menu(
            self,
            ctx,
            context_host.project.as_deref_mut(),
            config_readonly,
            busy_logo_texture,
        );

        // Render log panel first to claim full width at bottom
        if self.visibility.show_console {
            PanelSystem::render_log_panel(self, ctx, log_capture);
        }

        // Render hierarchy and inspector panels
        let game_state = context_host.scene_viewport.as_ref().map(|v| v.game_state());

        if self.visibility.show_hierarchy {
            self.render_hierarchy_and_maps_combined_panel(
                ctx,
                game_state,
                context_host.project_assets.as_deref_mut(),
                config_readonly,
            );
        }

        if self.visibility.show_inspector {
            InspectorSystem::render_inspector_panel(
                self,
                ctx,
                game_state,
                context_host.project.as_deref_mut(),
                context_host.project_assets.as_deref_mut(),
                config_readonly,
            );
        }

        if self.active_tab() == CenterPanelTab::MenuEditor {
            sync_menu_editor_selection(self, context_host.project.as_deref());
        }

        // Render viewport last (mutable access)
        PanelSystem::render_viewport(self, ctx, &mut context_host);
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
