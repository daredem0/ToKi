use anyhow::Result;
use egui_winit::winit;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use toki_core::assets::{atlas::AtlasMeta, tilemap::TileMap};
use toki_core::GameState;
use winit::application::ApplicationHandler;
use winit::event::Modifiers;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::ModifiersState;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::background_tasks::{
    BackgroundTaskManager, BackgroundTaskUpdate, ExportBundleJob, ValidateAssetsJob,
};
use crate::config::EditorConfig;
use crate::fonts::{load_project_fonts_into_egui, menu_font_family_choices};
use crate::logging::LogCapture;
use crate::project::ProjectAssets;
use crate::project::{ProjectManager, ProjectTemplateKind};
use crate::rendering::WindowRenderer;
use crate::scene::viewport::DragPreviewSprite;
use crate::scene::SceneViewport;
use crate::ui::editor_ui::{CenterPanelTab, MapEditorDraft};
use crate::ui::EditorUI;

#[path = "editor_app/background_tasks.rs"]
mod background_tasks;
#[path = "editor_app/map_editor.rs"]
mod map_editor;
#[path = "editor_app/new_project.rs"]
mod new_project;
#[path = "editor_app/previews.rs"]
mod previews;
#[path = "editor_app/project_requests.rs"]
mod project_requests;
#[path = "editor_app/runtime.rs"]
mod runtime;
#[path = "editor_app/session.rs"]
mod session;

pub fn run_editor(log_capture: Option<LogCapture>) -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut editor_app = EditorApp::new(log_capture);
    event_loop.run_app(&mut editor_app)?;
    Ok(())
}

/// Session state: tracks loaded scenes and maps across the editor session.
#[derive(Default)]
pub(crate) struct EditorSessionState {
    /// Track last loaded active scene to avoid unnecessary reloading.
    pub last_loaded_active_scene: Option<String>,
    /// Remembers the currently loaded map per scene for viewport reloads.
    pub loaded_scene_maps: HashMap<String, String>,
    /// Ensures startup auto-open from config only runs once.
    pub startup_project_auto_open_done: bool,
}

/// Resource cache: lazily loaded editor resources and their tracking state.
#[derive(Default)]
pub(crate) struct EditorResourceCache {
    /// Lazily loaded ToKi logo texture used for background task activity feedback.
    pub busy_logo_texture: Option<egui::TextureHandle>,
    /// Caches which project's menu preview fonts have been registered with egui.
    pub menu_font_project_path: Option<PathBuf>,
}

/// Platform layer: window, renderer, and egui integration.
/// These are initialized together during application startup.
#[derive(Default)]
pub(crate) struct EditorPlatform {
    pub window: Option<Arc<Window>>,
    pub renderer: Option<WindowRenderer>,
    pub egui_winit: Option<egui_winit::State>,
}

/// Viewport management: scene preview and map editor viewports.
#[derive(Default)]
pub(crate) struct EditorViewports {
    pub scene: Option<SceneViewport>,
    pub map_editor: Option<SceneViewport>,
}

/// Editor core: project management, UI state, and configuration.
pub(crate) struct EditorCore {
    pub project_manager: ProjectManager,
    pub ui: EditorUI,
    pub config: EditorConfig,
}

impl Default for EditorCore {
    fn default() -> Self {
        Self {
            project_manager: ProjectManager::new(),
            ui: EditorUI::new(),
            config: EditorConfig::default(),
        }
    }
}

struct EditorApp {
    /// Platform layer: window, renderer, egui integration.
    platform: EditorPlatform,

    /// Viewport management: scene and map editor viewports.
    viewports: EditorViewports,

    /// Editor core: project management, UI state, configuration.
    core: EditorCore,

    /// Logging
    log_capture: Option<LogCapture>,

    /// Keyboard modifiers state
    modifiers: ModifiersState,

    /// Session state: loaded scenes, maps, and startup flags.
    session: EditorSessionState,

    /// Runs long-running editor operations off the UI thread.
    background_tasks: BackgroundTaskManager,

    /// Resource cache: lazily loaded editor resources.
    resources: EditorResourceCache,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorShortcutAction {
    Undo,
    Redo,
}

impl EditorApp {
    fn new(log_capture: Option<LogCapture>) -> Self {
        // Load or create config
        let config = EditorConfig::load().unwrap_or_else(|e| {
            tracing::warn!("Failed to load config: {}, using defaults", e);
            EditorConfig::default()
        });

        let mut ui = EditorUI::new();
        ui.apply_config(&config);

        Self {
            platform: EditorPlatform::default(),
            viewports: EditorViewports::default(),
            core: EditorCore {
                project_manager: ProjectManager::new(),
                ui,
                config,
            },
            log_capture,
            modifiers: ModifiersState::default(),
            session: EditorSessionState::default(),
            background_tasks: BackgroundTaskManager::default(),
            resources: EditorResourceCache::default(),
        }
    }

    fn sync_project_menu_preview_fonts(&mut self, ctx: &egui::Context) {
        let current_project_path = self.core.config.current_project_path().cloned();
        if self.resources.menu_font_project_path == current_project_path {
            return;
        }

        let registry = load_project_fonts_into_egui(ctx, current_project_path.as_deref());
        self.core.ui.menu_preview_font_families = menu_font_family_choices(&registry);
        self.resources.menu_font_project_path = current_project_path;
    }

    fn busy_logo_path() -> Option<std::path::PathBuf> {
        let candidates = [
            std::env::current_dir()
                .ok()
                .map(|dir| dir.join("assets").join("TokiLogo.png")),
            Some(Self::workspace_root().join("assets").join("TokiLogo.png")),
        ];
        candidates.into_iter().flatten().find(|path| path.exists())
    }

    fn ensure_busy_logo_texture(&mut self, ctx: &egui::Context) {
        if self.resources.busy_logo_texture.is_some() {
            return;
        }

        let Some(logo_path) = Self::busy_logo_path() else {
            tracing::warn!("Could not resolve ToKi logo path for editor task indicator");
            return;
        };

        let decoded = match toki_core::graphics::image::load_image_rgba8(&logo_path) {
            Ok(decoded) => decoded,
            Err(error) => {
                tracing::warn!(
                    "Failed to load ToKi logo texture '{}' for editor task indicator: {}",
                    logo_path.display(),
                    error
                );
                return;
            }
        };

        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.data,
        );
        self.resources.busy_logo_texture =
            Some(ctx.load_texture("toki_busy_logo", color_image, egui::TextureOptions::LINEAR));
    }

    /// Helper method to initialize a viewport with WGPU context
    fn initialize_viewport(&self, mut viewport: SceneViewport) -> Option<SceneViewport> {
        if let Some(renderer) = &self.platform.renderer {
            match pollster::block_on(
                viewport.initialize(renderer.device().clone(), renderer.queue().clone()),
            ) {
                Ok(()) => {
                    tracing::info!("Scene viewport initialized with unified rendering");
                    Some(viewport)
                }
                Err(e) => {
                    tracing::error!("Failed to initialize scene viewport with WGPU: {e}");
                    None
                }
            }
        } else {
            tracing::error!("Cannot initialize scene viewport: renderer not available");
            None
        }
    }

    fn parse_legacy_graph_layout_key(key: &str) -> Option<(String, String, String)> {
        let mut parts = key.rsplitn(3, "::");
        let node_key = parts.next()?.to_string();
        let scene_name = parts.next()?.to_string();
        let project_key = parts.next()?.to_string();
        Some((project_key, scene_name, node_key))
    }

    fn editor_shortcut_action(
        logical_key: &winit::keyboard::Key,
        modifiers: ModifiersState,
    ) -> Option<EditorShortcutAction> {
        if !modifiers.control_key() {
            return None;
        }

        let winit::keyboard::Key::Character(ch) = logical_key else {
            return None;
        };
        let normalized = ch.to_ascii_lowercase();
        match normalized.as_str() {
            "z" if modifiers.shift_key() => Some(EditorShortcutAction::Redo),
            "z" => Some(EditorShortcutAction::Undo),
            "y" => Some(EditorShortcutAction::Redo),
            _ => None,
        }
    }

    fn sync_ui_graph_layouts_from_project(&mut self) {
        let (graph_layouts, rule_graph_drafts) = self
            .core.project_manager
            .current_project
            .as_ref()
            .map(|project| {
                (
                    project.metadata.editor.graph_layouts.clone(),
                    project.metadata.editor.rule_graph_drafts.clone(),
                )
            })
            .unwrap_or_default();
        self.core.ui.load_graph_layouts_from_project(&graph_layouts);
        self.core.ui
            .load_rule_graph_drafts_from_project(&rule_graph_drafts);
    }

    fn migrate_legacy_graph_layouts_into_project(&mut self) {
        let Some(project) = self.core.project_manager.current_project.as_mut() else {
            return;
        };

        let config_path = match std::env::current_dir() {
            Ok(dir) => dir.join("toki_editor_config.json"),
            Err(error) => {
                tracing::warn!(
                    "Cannot determine current directory for legacy graph layout migration: {}",
                    error
                );
                return;
            }
        };

        let raw_config = match std::fs::read_to_string(&config_path) {
            Ok(raw_config) => raw_config,
            Err(_) => return,
        };
        let mut config_json = match serde_json::from_str::<serde_json::Value>(&raw_config) {
            Ok(json) => json,
            Err(error) => {
                tracing::warn!(
                    "Failed to parse config for legacy graph layout migration: {}",
                    error
                );
                return;
            }
        };
        let Some(layouts_object) = config_json
            .get("graph_layouts")
            .and_then(|value| value.as_object())
            .cloned()
        else {
            return;
        };

        let project_key = project.path.to_string_lossy().to_string();
        let mut migrated_any = false;

        for (key, value) in layouts_object {
            let Some((entry_project_key, scene_name, node_key)) =
                Self::parse_legacy_graph_layout_key(&key)
            else {
                continue;
            };
            let Some(position_values) = value.as_array() else {
                continue;
            };
            if position_values.len() != 2 {
                continue;
            }
            let Some(x) = position_values[0].as_f64() else {
                continue;
            };
            let Some(y) = position_values[1].as_f64() else {
                continue;
            };
            let position = [x as f32, y as f32];

            if entry_project_key == project_key {
                project
                    .metadata
                    .editor
                    .graph_layouts
                    .entry(scene_name)
                    .or_default()
                    .node_positions
                    .insert(node_key, position);
                migrated_any = true;
            }
        }

        if migrated_any {
            if let Err(error) = project.save_metadata() {
                tracing::warn!(
                    "Failed to persist migrated graph layout metadata: {}",
                    error
                );
            }
            tracing::info!(
                "Migrated legacy scene graph layout entries from global config into project metadata"
            );
        }

        if let Some(config_object) = config_json.as_object_mut() {
            config_object.remove("graph_layouts");
            match serde_json::to_string_pretty(&config_json) {
                Ok(serialized) => {
                    if let Err(error) = std::fs::write(&config_path, serialized) {
                        tracing::warn!(
                            "Failed to remove legacy graph layouts from config file: {}",
                            error
                        );
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        "Failed to serialize config after removing legacy graph layouts: {}",
                        error
                    );
                }
            }
        }
    }

    fn persist_graph_layout_metadata_if_needed(&mut self, egui_ctx: &egui::Context) {
        if !self.core.ui.is_graph_layout_dirty() {
            return;
        }
        if egui_ctx.input(|input| input.pointer.any_down()) {
            return;
        }

        let Some(project) = self.core.project_manager.current_project.as_mut() else {
            return;
        };

        project.metadata.editor.graph_layouts = self.core.ui.export_graph_layouts_for_project();
        project.metadata.editor.rule_graph_drafts = self.core.ui.export_rule_graph_drafts_for_project();
        match project.save_metadata() {
            Ok(()) => self.core.ui.clear_graph_layout_dirty(),
            Err(error) => tracing::warn!(
                "Failed to persist scene graph layout to project metadata: {}",
                error
            ),
        }
    }
}

impl ApplicationHandler for EditorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window using config settings
        let [width, height] = self.core.config.editor_settings.window_size;
        let window_attributes = winit::window::Window::default_attributes()
            .with_title("ToKi Editor")
            .with_inner_size(winit::dpi::PhysicalSize::new(width, height));

        let window = match event_loop.create_window(window_attributes) {
            Ok(window) => Arc::new(window),
            Err(e) => {
                tracing::error!("Failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };

        // Initialize renderer (async, but we block here since we're in resumed)
        let renderer = match pollster::block_on(WindowRenderer::new(window.clone())) {
            Ok(renderer) => renderer,
            Err(e) => {
                tracing::error!("Failed to initialize renderer: {e}");
                event_loop.exit();
                return;
            }
        };

        // Initialize egui
        let egui_context = egui::Context::default();
        let egui_winit = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            event_loop,
            Some(window.scale_factor() as f32),
            None,
            Some(2048),
        );

        // Store components
        self.platform.window = Some(window.clone());
        self.platform.renderer = Some(renderer);
        self.platform.egui_winit = Some(egui_winit);

        // Initialize scene viewport with empty game state and WGPU context
        let game_state = GameState::new_empty();
        match SceneViewport::with_game_state(game_state) {
            Ok(mut viewport) => {
                // Initialize the scene viewport with WGPU context from renderer
                if let Some(renderer) = &self.platform.renderer {
                    match pollster::block_on(
                        viewport.initialize(renderer.device().clone(), renderer.queue().clone()),
                    ) {
                        Ok(()) => {
                            self.viewports.scene = Some(viewport);
                            tracing::info!("Scene viewport initialized with unified rendering");
                        }
                        Err(e) => {
                            tracing::error!("Failed to initialize scene viewport with WGPU: {e}");
                        }
                    }
                } else {
                    tracing::error!("Cannot initialize scene viewport: renderer not available");
                }
            }
            Err(e) => {
                tracing::error!("Failed to create scene viewport: {e}");
            }
        }

        let map_editor_state = GameState::new_empty();
        match SceneViewport::with_game_state_responsive(map_editor_state) {
            Ok(mut viewport) => {
                if let Some(renderer) = &self.platform.renderer {
                    match pollster::block_on(
                        viewport.initialize(renderer.device().clone(), renderer.queue().clone()),
                    ) {
                        Ok(()) => {
                            self.viewports.map_editor = Some(viewport);
                            tracing::info!("Map editor viewport initialized");
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to initialize map editor viewport with WGPU: {e}"
                            );
                        }
                    }
                } else {
                    tracing::error!(
                        "Cannot initialize map editor viewport: renderer not available"
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to create map editor viewport: {e}");
            }
        }

        tracing::info!("Editor initialized successfully");
        if !self.session.startup_project_auto_open_done {
            self.session.startup_project_auto_open_done = true;
            if self.core.config.has_project_path() {
                tracing::info!("Auto-opening last project from config on startup");
                self.core.ui.project.open_project_requested = true;
            }
        }
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Handle egui events first
        let mut needs_repaint = false;
        if let Some(egui_winit) = &mut self.platform.egui_winit {
            if let Some(window) = &self.platform.window {
                let event_response = egui_winit.on_window_event(window, &event);
                if event_response.repaint {
                    needs_repaint = true;
                }
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("Close requested, shutting down editor");
                event_loop.exit();
            }

            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    let active_viewport = match self.core.ui.center_panel_tab {
                        CenterPanelTab::SceneViewport => self.viewports.scene.as_mut(),
                        CenterPanelTab::MapEditor => self.viewports.map_editor.as_mut(),
                        CenterPanelTab::SceneGraph
                        | CenterPanelTab::SceneRules
                        | CenterPanelTab::MenuEditor => None,
                    };
                    if let Some(viewport) = active_viewport {
                        tracing::debug!("Passing logical key {:?} to viewport", event.logical_key);
                        if viewport.handle_keyboard_input(
                            &event.logical_key,
                            Modifiers::from(self.modifiers),
                            true,
                        ) {
                            if let Some(window) = &self.platform.window {
                                window.request_redraw();
                            }
                            return;
                        }
                    }

                    // Layout-aware editor shortcuts use logical key values.
                    if let Some(shortcut) =
                        Self::editor_shortcut_action(&event.logical_key, self.modifiers)
                    {
                        match shortcut {
                            EditorShortcutAction::Undo => {
                                let undone = self
                                    .core.project_manager
                                    .current_project
                                    .as_mut()
                                    .map(|project| self.core.ui.undo_with_project(project))
                                    .unwrap_or_else(|| self.core.ui.undo());
                                if undone {
                                    tracing::info!("Undo applied via Ctrl+Z");
                                }
                            }
                            EditorShortcutAction::Redo => {
                                let redone = self
                                    .core.project_manager
                                    .current_project
                                    .as_mut()
                                    .map(|project| self.core.ui.redo_with_project(project))
                                    .unwrap_or_else(|| self.core.ui.redo());
                                if redone {
                                    tracing::info!("Redo applied via Ctrl+Y/Ctrl+Shift+Z");
                                }
                            }
                        }
                        if let Some(window) = &self.platform.window {
                            window.request_redraw();
                        }
                        return;
                    }

                    // Fallback: try physical key for other editor shortcuts
                    if let PhysicalKey::Code(key_code) = event.physical_key {
                        // Handle other editor keyboard shortcuts
                        match key_code {
                            KeyCode::Escape => event_loop.exit(),
                            KeyCode::F1 => {
                                self.core.ui.visibility.show_hierarchy = !self.core.ui.visibility.show_hierarchy;
                                tracing::info!(
                                    "Toggled hierarchy panel: {}",
                                    self.core.ui.visibility.show_hierarchy
                                );
                                if let Some(window) = &self.platform.window {
                                    window.request_redraw();
                                }
                            }
                            KeyCode::F2 => {
                                self.core.ui.visibility.show_inspector = !self.core.ui.visibility.show_inspector;
                                tracing::info!(
                                    "Toggled inspector panel: {}",
                                    self.core.ui.visibility.show_inspector
                                );
                                if let Some(window) = &self.platform.window {
                                    window.request_redraw();
                                }
                            }
                            KeyCode::F4 => {
                                // Toggle debug collision rendering (same as toki-runtime)
                                if let Some(viewport) = &mut self.viewports.scene {
                                    viewport
                                        .scene_manager_mut()
                                        .game_state_mut()
                                        .handle_key_press(toki_core::InputKey::DebugToggle);
                                    tracing::info!("Toggled debug collision rendering via F4");
                                    if let Some(window) = &self.platform.window {
                                        window.request_redraw();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            WindowEvent::Resized(new_size) => {
                if let Some(renderer) = &mut self.platform.renderer {
                    renderer.resize(new_size);
                }
                if let Some(window) = &self.platform.window {
                    window.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                self.render(event_loop);
            }

            _ => {}
        }

        // Request repaint if egui or our events need it
        if needs_repaint {
            if let Some(window) = &self.platform.window {
                window.request_redraw();
            }
        }
    }
}

impl EditorApp {
    fn render(&mut self, event_loop: &ActiveEventLoop) {
        let window = match &self.platform.window {
            Some(window) => window.clone(),
            None => return, // Not initialized yet
        };
        let egui_ctx = match self.platform.egui_winit.as_ref() {
            Some(egui) => egui.egui_ctx().clone(),
            None => return, // Not initialized yet
        };
        if self.core.ui.project.background_task_running {
            self.ensure_busy_logo_texture(&egui_ctx);
        }
        self.sync_project_menu_preview_fonts(&egui_ctx);

        let egui_winit = match &mut self.platform.egui_winit {
            Some(egui) => egui,
            None => return, // Not initialized yet
        };

        // Prepare egui input
        let raw_input = egui_winit.take_egui_input(&window);

        let renderer = match &mut self.platform.renderer {
            Some(renderer) => renderer,
            None => return, // Not initialized yet
        };

        // Load sprite frame cache if needed (before render loop to avoid borrowing issues)
        let project_path = self.core.config.current_project_path();
        if self.core.ui.is_in_placement_mode() && self.core.ui.placement.preview_cached_frame.is_none() {
            if let (Some(entity_def), Some(project_path), Some(project_assets)) = (
                &self.core.ui.placement.entity_definition,
                &project_path,
                self.core.project_manager.get_project_assets(),
            ) {
                let cached_frame = EditorApp::load_preview_sprite_frame_static(
                    entity_def,
                    project_path.as_path(),
                    project_assets,
                );
                self.core.ui.placement.preview_cached_frame = cached_frame;
            }
        }

        // Pre-render active center viewport to texture before egui UI.
        if let Some(project_path) = &project_path {
            if let Some(project_assets) = self.core.project_manager.get_project_assets() {
                match self.core.ui.center_panel_tab {
                    CenterPanelTab::SceneViewport => {
                        if let Some(scene_viewport) = &mut self.viewports.scene {
                            let preview_data = if self.core.ui.is_in_placement_mode() {
                                if self.core.ui.placement.entity_move_drag.is_none() {
                                    if let (Some(_entity_def), Some(position), Some(cached_frame)) = (
                                        &self.core.ui.placement.entity_definition,
                                        &self.core.ui.placement.preview_position,
                                        &self.core.ui.placement.preview_cached_frame,
                                    ) {
                                        let is_valid =
                                            self.core.ui.placement.preview_valid.unwrap_or(true);
                                        Some((*position, cached_frame.clone(), is_valid))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            let drag_preview_data =
                                self.core.ui.placement.entity_move_drag.as_ref().and_then(|drag| {
                                    self.core.ui.placement.preview_position.map(|preview_position| {
                                        let tilemap = scene_viewport.scene_manager().tilemap();
                                        let terrain_atlas = tilemap.map(|_| {
                                            scene_viewport
                                                .scene_manager()
                                                .resources()
                                                .get_terrain_atlas()
                                        });
                                        Self::build_drag_preview_sprites(
                                            drag,
                                            preview_position,
                                            tilemap,
                                            terrain_atlas,
                                        )
                                    })
                                });

                            if let Err(e) = scene_viewport.render_to_texture(
                                project_path.as_path(),
                                project_assets,
                                renderer.egui_renderer_mut(),
                                preview_data,
                                drag_preview_data.as_deref(),
                            ) {
                                tracing::error!("Failed to render scene to texture: {}", e);
                            }
                        }
                    }
                    CenterPanelTab::MapEditor => {
                        if let Some(map_editor_viewport) = &mut self.viewports.map_editor {
                            if let Err(e) = map_editor_viewport.render_to_texture(
                                project_path.as_path(),
                                project_assets,
                                renderer.egui_renderer_mut(),
                                None,
                                None,
                            ) {
                                tracing::error!(
                                    "Failed to render map editor viewport to texture: {}",
                                    e
                                );
                            }
                        }
                    }
                    CenterPanelTab::SceneGraph
                    | CenterPanelTab::SceneRules
                    | CenterPanelTab::MenuEditor => {}
                }
            } else if self.core.project_manager.current_project.is_some() {
                tracing::warn!(
                    "No project assets available for viewport rendering {:?}",
                    self.core.project_manager.current_project
                );
            }
        }

        // Run egui UI
        let available_map_names = self.core.project_manager.get_project_assets().map(|assets| {
            let mut names = assets.tilemaps.keys().cloned().collect::<Vec<_>>();
            names.sort();
            names
        });
        let full_output = egui_ctx.run(raw_input, |ctx| {
            // Render UI - viewport will use the pre-rendered texture
            self.core.ui.render(
                ctx,
                self.viewports.scene.as_mut(),
                self.viewports.map_editor.as_mut(),
                self.core.project_manager.current_project.as_mut(),
                available_map_names.clone(),
                Some(&mut self.core.config),
                self.log_capture.as_ref(),
                None, // Can't pass renderer due to borrow issues
                self.resources.busy_logo_texture.as_ref(),
            );
        });

        // Handle UI requests
        if self.core.ui.visibility.should_exit {
            event_loop.exit();
            return;
        }

        if self.core.ui.visibility.create_test_entities {
            if let Some(viewport) = &mut self.viewports.scene {
                let game_state = viewport.scene_manager_mut().game_state_mut();
                let _player_id = game_state.spawn_player_at(glam::IVec2::new(80, 72));
                let _npc_id = game_state.spawn_player_like_npc(glam::IVec2::new(120, 72));
                tracing::info!("Created test entities");
            }
            self.core.ui.visibility.create_test_entities = false;
        }

        // Handle platform output (cursor, clipboard, etc.)
        egui_winit.handle_platform_output(&window, full_output.platform_output.clone());

        // Render frame
        if let Err(e) = renderer.render(&window, full_output, &egui_ctx) {
            tracing::error!("Render error: {e}");
        }

        self.persist_graph_layout_metadata_if_needed(&egui_ctx);

        // Request redraw if egui wants a repaint
        if egui_ctx.has_requested_repaint() {
            window.request_redraw();
        }

        // Handle project management requests and other actions after rendering is done
        self.handle_project_requests(event_loop);
        self.handle_play_scene_request();
        self.handle_active_scene_map_loading();
        self.handle_map_requests();
        self.handle_new_map_editor_requests();
        self.handle_pending_map_editor_tilemap_sync();
        self.handle_save_map_editor_request();
        self.handle_map_editor_map_requests();

        if self
            .viewports.scene
            .as_ref()
            .is_some_and(crate::scene::SceneViewport::needs_render)
        {
            window.request_redraw();
        }
        if self
            .viewports.map_editor
            .as_ref()
            .is_some_and(crate::scene::SceneViewport::needs_render)
        {
            window.request_redraw();
        }
    }
}

#[cfg(test)]
#[path = "editor_app_tests.rs"]
mod tests;
