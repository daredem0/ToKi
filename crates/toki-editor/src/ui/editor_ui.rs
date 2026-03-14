use super::inspector::InspectorSystem;
use super::menus::MenuSystem;
use super::panels::PanelSystem;
use super::rule_graph::RuleGraph;
use crate::project::SceneGraphLayout;
use crate::scene::SceneViewport;
use std::collections::HashMap;
use toki_core::{
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
}

#[derive(Debug, Clone)]
pub struct EntityMoveDragState {
    pub scene_name: String,
    pub entity: Entity,
    pub grab_offset: glam::Vec2, // Cursor world position offset from entity top-left at drag start
}

#[derive(Debug, Clone, Copy)]
pub struct MarqueeSelectionState {
    pub start_screen: egui::Pos2,
    pub current_screen: egui::Pos2,
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
    pub open_project_requested: bool,
    pub browse_for_project_requested: bool,
    pub save_project_requested: bool,
    pub play_scene_requested: bool,
    pub init_config_requested: bool,
    pub window_title: Option<String>,

    // Map loading request
    pub map_load_requested: Option<(String, String)>, // (scene_name, map_name)

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
            open_project_requested: false,
            browse_for_project_requested: false,
            save_project_requested: false,
            play_scene_requested: false,
            init_config_requested: false,
            window_title: Some("No project open".to_string()),

            // Map loading request
            map_load_requested: None,

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

        // Set the first scene as active if we have scenes and no active scene is set
        if !self.scenes.is_empty() && self.active_scene.is_none() {
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
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        scene_viewport: Option<&mut SceneViewport>,
        config: Option<&mut crate::config::EditorConfig>,
        log_capture: Option<&crate::logging::LogCapture>,
        renderer: Option<&mut egui_wgpu::Renderer>,
    ) {
        let config_readonly = config.as_deref();
        MenuSystem::render_top_menu(self, ctx, config_readonly);

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
            InspectorSystem::render_inspector_panel(self, ctx, game_state, config_readonly);
        }

        // Render viewport last (mutable access)
        PanelSystem::render_viewport(self, ctx, scene_viewport, config, renderer);
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

    pub fn set_graph_layout_position(
        &mut self,
        scene_name: &str,
        node_key: &str,
        position: [f32; 2],
    ) {
        let layout = self
            .graph_layouts_by_scene
            .entry(scene_name.to_string())
            .or_default();
        let changed = layout
            .node_positions
            .get(node_key)
            .is_none_or(|existing| *existing != position);
        if changed {
            layout.node_positions.insert(node_key.to_string(), position);
            self.graph_layout_dirty = true;
        }
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
                                            let entity_display = match entity.entity_type {
                                                toki_core::entity::EntityType::Player => format!("👤 Player (ID: {})", entity.id),
                                                toki_core::entity::EntityType::Npc => format!("🧙 NPC (ID: {})", entity.id),
                                                toki_core::entity::EntityType::Item => format!("📦 Item (ID: {})", entity.id),
                                                toki_core::entity::EntityType::Decoration => format!("🎨 Decoration (ID: {})", entity.id),
                                                toki_core::entity::EntityType::Trigger => format!("⚡ Trigger (ID: {})", entity.id),
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
                        if let Some(scene) = self.scenes.iter_mut().find(|s| s.name == scene_name) {
                            if let Some(index) = scene.entities.iter().position(|e| e.id == entity_id) {
                                scene.entities.remove(index);
                                tracing::info!("Removed entity {} from scene {}", entity_id, scene_name);

                                // Clear selection if it was the removed entity
                                if matches!(&self.selection, Some(Selection::Entity(id)) if id == &entity_id) {
                                    self.clear_selection();
                                }

                                self.scene_content_changed = true;
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
                });

                ui.separator();

                // Add new scene button
                if ui.button("+ Add Scene").clicked() {
                    let new_scene_name = format!("Scene {}", self.scenes.len() + 1);
                    self.add_scene(new_scene_name.clone());
                    tracing::info!("Created new scene: {}", new_scene_name);
                }

                // Add Maps section if enabled
                if self.show_maps {
                    ui.add_space(10.0);
                    ui.heading("🗺️ Maps");
                    ui.separator();

                    if let Some(config) = config {
                        if let Some(project_path) = config.current_project_path() {
                            let tilemaps_path = project_path.join("assets").join("tilemaps");

                            if tilemaps_path.exists() {
                                // Discover tilemap files
                                if let Ok(entries) = std::fs::read_dir(&tilemaps_path) {
                                    let mut found_maps = false;

                                    // Collect actions to perform after UI iteration
                                    let mut map_selections: Vec<String> = Vec::new();
                                    let mut scene_map_additions: Vec<(String, String)> = Vec::new(); // (scene_name, map_name)

                                    egui::ScrollArea::vertical()
                                        .id_salt("maps_scroll")
                                        .max_height(150.0) // Limit height so hierarchy doesn't get too tall
                                        .show(ui, |ui| {
                                            for entry in entries.flatten() {
                                                if let Some(name) = entry.file_name().to_str() {
                                                    if name.ends_with(".json") {
                                                        let map_name = name.trim_end_matches(".json").to_string();
                                                        found_maps = true;

                                                        let is_selected = matches!(
                                                            &self.selection,
                                                            Some(Selection::StandaloneMap(name)) if name == &map_name
                                                        );

                                                        let response = ui.selectable_label(is_selected, &map_name);

                                                        if response.clicked() {
                                                            tracing::info!("Map selected: {}", map_name);
                                                            map_selections.push(map_name.clone());
                                                        }

                                                        // Right-click context menu for "Add to Scene"
                                                        response.context_menu(|ui| {
                                                            ui.label("Add to Scene:");
                                                            ui.separator();

                                                            // Show available scenes - create a copy to avoid borrowing issues
                                                            let scene_names: Vec<(String, bool)> = self.scenes.iter()
                                                                .map(|s| (s.name.clone(), s.maps.contains(&map_name)))
                                                                .collect();

                                                            for (scene_name, already_added) in scene_names {
                                                                if !already_added {
                                                                    if ui.button(&scene_name).clicked() {
                                                                        scene_map_additions.push((scene_name.clone(), map_name.clone()));
                                                                        ui.close();
                                                                    }
                                                                } else {
                                                                    ui.add_enabled(false, egui::Button::new(format!("{} (already added)", scene_name)));
                                                                }
                                                            }

                                                            if self.scenes.is_empty() {
                                                                ui.label("No scenes available");
                                                            }
                                                        });
                                                    }
                                                }
                                            }
                                        });

                                    // Apply collected actions
                                    for map_name in map_selections {
                                        self.set_selection(Selection::StandaloneMap(map_name));
                                    }

                                    for (scene_name, map_name) in scene_map_additions {
                                        if let Some(target_scene) = self.scenes.iter_mut().find(|s| s.name == scene_name) {
                                            target_scene.maps.push(map_name.clone());
                                            tracing::info!("Added map '{}' to scene '{}'", map_name, scene_name);
                                            // Signal that scene content changed by setting a flag
                                            self.scene_content_changed = true;
                                        }
                                    }

                                    if !found_maps {
                                        tracing::info!("No tilemap (.json) files found in assets/tilemaps/");
                                    }
                                } else {
                                    tracing::warn!("Could not read tilemaps directory");
                                }
                            } else {
                                tracing::info!("No tilemaps directory found, expected: assets/tilemaps/");
                            }
                        } else {
                            tracing::info!("No project loaded for Maps panel");
                        }
                    } else {
                        tracing::warn!("No project configuration available for Maps panel");
                    }
                }

                // Add Entity Palette section
                ui.add_space(10.0);
                ui.heading("🧙 Entities");
                ui.separator();

                if let Some(config) = config {
                    if let Some(project_path) = config.current_project_path() {
                        let (selected_entity, entity_additions, placement_request) = super::hierarchy::HierarchySystem::render_entity_palette(ui, project_path, &self.selection, &self.scenes);

                        // Handle entity selection
                        if let Some(selected_entity) = selected_entity {
                            self.set_selection(Selection::EntityDefinition(selected_entity));
                        }

                        // Handle placement mode request
                        if let Some(entity_definition) = placement_request {
                            self.enter_placement_mode(entity_definition);
                        }

                        // Process entity additions to scenes
                        for (scene_name, entity_name) in entity_additions {
                            if let Some(target_scene) = self.scenes.iter_mut().find(|s| s.name == scene_name) {
                                // Try to load and create entity from definition
                                if let Some(project_path) = config.current_project_path() {
                                    let entity_file = project_path
                                        .join("entities")
                                        .join(format!("{}.json", entity_name));

                                    if entity_file.exists() {
                                        match std::fs::read_to_string(&entity_file) {
                                            Ok(content) => {
                                                match serde_json::from_str::<toki_core::entity::EntityDefinition>(&content) {
                                                    Ok(entity_def) => {
                                                        // Generate a new entity ID (simple increment from existing entities)
                                                        let new_id = target_scene.entities.iter()
                                                            .map(|e| e.id)
                                                            .max()
                                                            .unwrap_or(0) + 1;

                                                        // Default position at (100, 100) - user can move it later
                                                        let default_position = glam::IVec2::new(100, 100);

                                                        match entity_def.create_entity(default_position, new_id) {
                                                            Ok(entity) => {
                                                                target_scene.entities.push(entity);
                                                                tracing::info!("Successfully added entity '{}' (ID: {}) to scene '{}' at position ({}, {})",
                                                                    entity_name, new_id, scene_name, default_position.x, default_position.y);
                                                                self.scene_content_changed = true;
                                                            }
                                                            Err(e) => {
                                                                tracing::error!("Failed to create entity '{}': {}", entity_name, e);
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        tracing::error!("Failed to parse entity definition '{}': {}", entity_name, e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                tracing::error!("Failed to read entity file '{}': {}", entity_name, e);
                                            }
                                        }
                                    } else {
                                        tracing::error!("Entity definition file not found: {:?}", entity_file);
                                    }
                                } else {
                                    tracing::error!("No project path available for entity creation");
                                }
                            }
                        }
                    } else {
                        ui.label("No project loaded for Entity palette");
                    }
                } else {
                    ui.label("No project configuration available for Entity palette");
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use toki_core::rules::{
        Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleTrigger,
    };

    use super::EditorUI;
    use crate::ui::rule_graph::RuleGraph;

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
}
