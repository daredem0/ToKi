use anyhow::Result;
use egui_winit::winit;
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
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
use crate::logging::LogCapture;
use crate::project::{ProjectManager, ProjectTemplateKind};
use crate::rendering::WindowRenderer;
use crate::scene::viewport::DragPreviewSprite;
use crate::scene::SceneViewport;
use crate::ui::editor_ui::CenterPanelTab;
use crate::ui::EditorUI;

pub fn run_editor(log_capture: Option<LogCapture>) -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut editor_app = EditorApp::new(log_capture);
    event_loop.run_app(&mut editor_app)?;
    Ok(())
}

struct EditorApp {
    // Core components
    window: Option<Arc<Window>>,
    renderer: Option<WindowRenderer>,
    ui: EditorUI,

    // egui integration
    egui_winit: Option<egui_winit::State>,

    // Scene viewport integration
    scene_viewport: Option<SceneViewport>,

    // Project management
    project_manager: ProjectManager,

    // Editor configuration
    config: EditorConfig,

    /// Logging
    log_capture: Option<LogCapture>,

    /// Keyboard modifiers state
    modifiers: ModifiersState,

    /// Track last loaded active scene to avoid unnecessary reloading
    last_loaded_active_scene: Option<String>,

    /// Remembers the currently loaded map per scene for viewport reloads.
    loaded_scene_maps: HashMap<String, String>,
    /// Ensures startup auto-open from config only runs once.
    startup_project_auto_open_done: bool,
    /// Runs long-running editor operations off the UI thread.
    background_tasks: BackgroundTaskManager,
    /// Lazily loaded ToKi logo texture used for background task activity feedback.
    busy_logo_texture: Option<egui::TextureHandle>,
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
            window: None,
            renderer: None,
            ui,
            egui_winit: None,
            scene_viewport: None,
            project_manager: ProjectManager::new(),
            config,
            log_capture,
            modifiers: ModifiersState::default(),
            last_loaded_active_scene: None,
            loaded_scene_maps: HashMap::new(),
            startup_project_auto_open_done: false,
            background_tasks: BackgroundTaskManager::default(),
            busy_logo_texture: None,
        }
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
        if self.busy_logo_texture.is_some() {
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
        self.busy_logo_texture =
            Some(ctx.load_texture("toki_busy_logo", color_image, egui::TextureOptions::LINEAR));
    }

    /// Helper method to initialize a viewport with WGPU context
    fn initialize_viewport(&self, mut viewport: SceneViewport) -> Option<SceneViewport> {
        if let Some(renderer) = &self.renderer {
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

    fn resolve_scene_map_to_load(
        scene: &toki_core::Scene,
        preferred_map: Option<&str>,
    ) -> Option<String> {
        if let Some(preferred_map) = preferred_map {
            if scene.maps.iter().any(|map| map == preferred_map) {
                return Some(preferred_map.to_string());
            }
        }

        scene.maps.first().cloned()
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
            .project_manager
            .current_project
            .as_ref()
            .map(|project| {
                (
                    project.metadata.editor.graph_layouts.clone(),
                    project.metadata.editor.rule_graph_drafts.clone(),
                )
            })
            .unwrap_or_default();
        self.ui.load_graph_layouts_from_project(&graph_layouts);
        self.ui
            .load_rule_graph_drafts_from_project(&rule_graph_drafts);
    }

    fn migrate_legacy_graph_layouts_into_project(&mut self) {
        let Some(project) = self.project_manager.current_project.as_mut() else {
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
        if !self.ui.is_graph_layout_dirty() {
            return;
        }
        if egui_ctx.input(|input| input.pointer.any_down()) {
            return;
        }

        let Some(project) = self.project_manager.current_project.as_mut() else {
            return;
        };

        project.metadata.editor.graph_layouts = self.ui.export_graph_layouts_for_project();
        project.metadata.editor.rule_graph_drafts = self.ui.export_rule_graph_drafts_for_project();
        match project.save_metadata() {
            Ok(()) => self.ui.clear_graph_layout_dirty(),
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
        let [width, height] = self.config.editor_settings.window_size;
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
        self.window = Some(window.clone());
        self.renderer = Some(renderer);
        self.egui_winit = Some(egui_winit);

        // Initialize scene viewport with empty game state and WGPU context
        let game_state = GameState::new_empty();
        match SceneViewport::with_game_state(game_state) {
            Ok(mut viewport) => {
                // Initialize the scene viewport with WGPU context from renderer
                if let Some(renderer) = &self.renderer {
                    match pollster::block_on(
                        viewport.initialize(renderer.device().clone(), renderer.queue().clone()),
                    ) {
                        Ok(()) => {
                            self.scene_viewport = Some(viewport);
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

        tracing::info!("Editor initialized successfully");
        if !self.startup_project_auto_open_done {
            self.startup_project_auto_open_done = true;
            if self.config.has_project_path() {
                tracing::info!("Auto-opening last project from config on startup");
                self.ui.open_project_requested = true;
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
        if let Some(egui_winit) = &mut self.egui_winit {
            if let Some(window) = &self.window {
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
                    // Route viewport zoom keys only while Scene Viewport is the active center tab.
                    if self.ui.center_panel_tab == CenterPanelTab::SceneViewport {
                        if let Some(viewport) = &mut self.scene_viewport {
                            tracing::debug!(
                                "Passing logical key {:?} to viewport",
                                event.logical_key
                            );
                            if viewport.handle_keyboard_input(
                                &event.logical_key,
                                Modifiers::from(self.modifiers),
                                true,
                            ) {
                                // Viewport handled the input, request redraw
                                tracing::debug!(
                                    "Viewport consumed key {:?}, requesting redraw",
                                    event.logical_key
                                );
                                if let Some(window) = &self.window {
                                    window.request_redraw();
                                }
                                return; // Input consumed by viewport
                            }
                        }
                    }

                    // Layout-aware editor shortcuts use logical key values.
                    if let Some(shortcut) =
                        Self::editor_shortcut_action(&event.logical_key, self.modifiers)
                    {
                        match shortcut {
                            EditorShortcutAction::Undo => {
                                if self.ui.undo() {
                                    tracing::info!("Undo applied via Ctrl+Z");
                                }
                            }
                            EditorShortcutAction::Redo => {
                                if self.ui.redo() {
                                    tracing::info!("Redo applied via Ctrl+Y/Ctrl+Shift+Z");
                                }
                            }
                        }
                        if let Some(window) = &self.window {
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
                                self.ui.show_hierarchy = !self.ui.show_hierarchy;
                                tracing::info!(
                                    "Toggled hierarchy panel: {}",
                                    self.ui.show_hierarchy
                                );
                                if let Some(window) = &self.window {
                                    window.request_redraw();
                                }
                            }
                            KeyCode::F2 => {
                                self.ui.show_inspector = !self.ui.show_inspector;
                                tracing::info!(
                                    "Toggled inspector panel: {}",
                                    self.ui.show_inspector
                                );
                                if let Some(window) = &self.window {
                                    window.request_redraw();
                                }
                            }
                            KeyCode::F4 => {
                                // Toggle debug collision rendering (same as toki-runtime)
                                if let Some(viewport) = &mut self.scene_viewport {
                                    viewport
                                        .scene_manager_mut()
                                        .game_state_mut()
                                        .handle_key_press(toki_core::InputKey::DebugToggle);
                                    tracing::info!("Toggled debug collision rendering via F4");
                                    if let Some(window) = &self.window {
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
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(new_size);
                }
                if let Some(window) = &self.window {
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
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}

impl EditorApp {
    fn render(&mut self, event_loop: &ActiveEventLoop) {
        let window = match &self.window {
            Some(window) => window.clone(),
            None => return, // Not initialized yet
        };
        let egui_ctx = match self.egui_winit.as_ref() {
            Some(egui) => egui.egui_ctx().clone(),
            None => return, // Not initialized yet
        };
        if self.ui.background_task_running {
            self.ensure_busy_logo_texture(&egui_ctx);
        }

        let egui_winit = match &mut self.egui_winit {
            Some(egui) => egui,
            None => return, // Not initialized yet
        };

        // Prepare egui input
        let raw_input = egui_winit.take_egui_input(&window);

        let renderer = match &mut self.renderer {
            Some(renderer) => renderer,
            None => return, // Not initialized yet
        };

        // Load sprite frame cache if needed (before render loop to avoid borrowing issues)
        let project_path = self.config.current_project_path();
        if self.ui.is_in_placement_mode() && self.ui.placement_preview_cached_frame.is_none() {
            if let (Some(entity_def), Some(project_path), Some(project_assets)) = (
                &self.ui.placement_entity_definition,
                &project_path,
                self.project_manager.get_project_assets(),
            ) {
                let cached_frame = EditorApp::load_preview_sprite_frame_static(
                    entity_def,
                    project_path.as_path(),
                    project_assets,
                );
                self.ui.placement_preview_cached_frame = cached_frame;
            }
        }

        // Pre-render scene to texture before egui UI
        if let Some(scene_viewport) = &mut self.scene_viewport {
            if let Some(project_path) = &project_path {
                if let Some(project_assets) = self.project_manager.get_project_assets() {
                    // Prepare preview data for entity placement
                    let preview_data = if self.ui.is_in_placement_mode() {
                        if self.ui.entity_move_drag.is_none() {
                            if let (Some(entity_def), Some(position), Some(cached_frame)) = (
                                &self.ui.placement_entity_definition,
                                &self.ui.placement_preview_position,
                                &self.ui.placement_preview_cached_frame,
                            ) {
                                let is_valid = self.ui.placement_preview_valid.unwrap_or(true);
                                Some((entity_def.as_str(), *position, *cached_frame, is_valid))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let drag_preview_data = self.ui.entity_move_drag.as_ref().and_then(|drag| {
                        self.ui.placement_preview_position.map(|preview_position| {
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

                    match scene_viewport.render_to_texture(
                        project_path.as_path(),
                        project_assets,
                        renderer.egui_renderer_mut(),
                        preview_data,
                        drag_preview_data.as_deref(),
                    ) {
                        Ok(()) => {
                            // Reduce log spam - render_to_texture already handles its own logging
                            // tracing::debug!("Scene rendered to offscreen texture successfully");
                        }
                        Err(e) => {
                            tracing::error!("Failed to render scene to texture: {}", e);
                        }
                    }
                } else if self.project_manager.current_project.is_some() {
                    tracing::warn!(
                        "No project assets available for scene rendering {:?}",
                        self.project_manager.current_project
                    );
                }
            }
        }

        // Run egui UI
        let full_output = egui_ctx.run(raw_input, |ctx| {
            // Render UI - viewport will use the pre-rendered texture
            self.ui.render(
                ctx,
                self.scene_viewport.as_mut(),
                Some(&mut self.config),
                self.log_capture.as_ref(),
                None, // Can't pass renderer due to borrow issues
                self.busy_logo_texture.as_ref(),
            );
        });

        // Handle UI requests
        if self.ui.should_exit {
            event_loop.exit();
            return;
        }

        if self.ui.create_test_entities {
            if let Some(viewport) = &mut self.scene_viewport {
                let game_state = viewport.scene_manager_mut().game_state_mut();
                let _player_id = game_state.spawn_player_at(glam::IVec2::new(80, 72));
                let _npc_id = game_state.spawn_player_like_npc(glam::IVec2::new(120, 72));
                tracing::info!("Created test entities");
            }
            self.ui.create_test_entities = false;
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
    }

    fn handle_new_project_requested(&mut self, template: ProjectTemplateKind) {
        self.ui.new_project_requested = false;
        self.ui.new_top_down_project_requested = false;

        // Use project_path from config if available, otherwise ask user
        let folder_path = if let Some(config_path) = &self.config.project_path {
            tracing::info!(
                "Using project path from config as parent: {:?}",
                config_path
            );
            Some(config_path.clone())
        } else {
            tracing::info!("No project path in config, asking user to select folder");
            rfd::FileDialog::new()
                .set_title("Select folder for new project")
                .pick_folder()
        };

        if let Some(parent_path) = folder_path {
            // Generate a unique project name
            let mut project_name = "NewProject".to_string();
            let mut counter = 1;

            while parent_path.join(&project_name).exists() {
                project_name = format!("NewProject{}", counter);
                counter += 1;
            }

            tracing::info!(
                "Creating project '{}' from template '{}' in {:?}",
                project_name,
                template.label(),
                parent_path
            );
            let create_result = match template {
                ProjectTemplateKind::Empty => self
                    .project_manager
                    .create_new_project(project_name.clone(), parent_path.clone()),
                ProjectTemplateKind::TopDownStarter => self
                    .project_manager
                    .create_new_project_with_template(
                        project_name.clone(),
                        parent_path.clone(),
                        template,
                    ),
            };
            match create_result {
                Ok(game_state) => {
                    // Update scene viewport with new game state
                    match SceneViewport::with_game_state(game_state) {
                        Ok(viewport) => {
                            self.scene_viewport = self.initialize_viewport(viewport);
                            self.last_loaded_active_scene = None;
                            self.loaded_scene_maps.clear();

                            // Update config with new project path
                            let project_path = parent_path.join(&project_name);
                            self.config.set_project_path(project_path);
                            if let Err(e) = self.config.save() {
                                tracing::warn!(
                                    "Failed to save config after creating project: {}",
                                    e
                                );
                            }

                            self.ui.set_title(
                                &self
                                    .project_manager
                                    .current_project
                                    .as_ref()
                                    .unwrap()
                                    .name
                                    .to_string(),
                            );

                            match self.project_manager.load_scenes() {
                                Ok(loaded_scenes) => {
                                    self.ui.load_scenes_from_project(loaded_scenes);
                                    tracing::info!("Loaded scenes into UI hierarchy");
                                }
                                Err(e) => {
                                    tracing::error!("Failed to load scenes into UI: {}", e);
                                }
                            }

                            self.sync_ui_graph_layouts_from_project();

                            tracing::info!(
                                "Created '{}' project '{}' successfully",
                                template.label(),
                                project_name
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to initialize scene viewport for new project: {}",
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create new project: {}", e);
                }
            }
        }
    }

    fn handle_open_project_request(&mut self) {
        self.ui.open_project_requested = false;

        // Try to open project from config first
        let project_path = if let Some(config_path) = &self.config.project_path {
            tracing::info!("Opening project from config: {:?}", config_path);
            Some(config_path.clone())
        } else {
            tracing::info!("No project path in config, asking user to select folder");
            rfd::FileDialog::new()
                .set_title("Open ToKi Project")
                .add_filter("ToKi Project", &["toki"])
                .pick_folder()
        };

        if let Some(project_path) = project_path {
            match self.project_manager.open_project(project_path.clone()) {
                Ok(game_state) => {
                    // Update scene viewport with loaded game state
                    match SceneViewport::with_game_state(game_state) {
                        Ok(viewport) => {
                            self.scene_viewport = self.initialize_viewport(viewport);
                            // Reset last loaded scene to force map loading for active scene
                            self.last_loaded_active_scene = None;
                            self.loaded_scene_maps.clear();

                            // Update config with opened project path
                            self.config.set_project_path(project_path);
                            if let Err(e) = self.config.save() {
                                tracing::warn!(
                                    "Failed to save config after opening project: {}",
                                    e
                                );
                            }
                            self.ui.set_title(
                                &self
                                    .project_manager
                                    .current_project
                                    .as_ref()
                                    .unwrap()
                                    .name
                                    .to_string(),
                            );

                            // Load scenes from project into UI
                            match self.project_manager.load_scenes() {
                                Ok(loaded_scenes) => {
                                    self.ui.load_scenes_from_project(loaded_scenes);
                                    tracing::info!("Loaded scenes into UI hierarchy");
                                }
                                Err(e) => {
                                    tracing::error!("Failed to load scenes into UI: {}", e);
                                }
                            }

                            self.migrate_legacy_graph_layouts_into_project();
                            self.sync_ui_graph_layouts_from_project();

                            tracing::info!("Opened project successfully");
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to initialize scene viewport for opened project: {}",
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to open project: {}", e);
                }
            }
        }
    }

    fn handle_browse_for_project_request(&mut self) {
        self.ui.browse_for_project_requested = false;

        if let Some(project_path) = rfd::FileDialog::new()
            .set_title("Browse for ToKi Project")
            .add_filter("ToKi Project", &["toki"])
            .pick_folder()
        {
            match self.project_manager.open_project(project_path.clone()) {
                Ok(game_state) => {
                    // Update scene viewport with loaded game state
                    match SceneViewport::with_game_state(game_state) {
                        Ok(viewport) => {
                            self.scene_viewport = self.initialize_viewport(viewport);
                            // Reset last loaded scene to force map loading for active scene
                            self.last_loaded_active_scene = None;
                            self.loaded_scene_maps.clear();

                            // Update config with opened project path
                            self.config.set_project_path(project_path);
                            if let Err(e) = self.config.save() {
                                tracing::warn!(
                                    "Failed to save config after browsing for project: {}",
                                    e
                                );
                            }

                            // Load scenes from project into UI
                            match self.project_manager.load_scenes() {
                                Ok(loaded_scenes) => {
                                    self.ui.load_scenes_from_project(loaded_scenes);
                                    tracing::info!(
                                        "Loaded scenes into UI hierarchy from browsed project"
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to load scenes into UI from browsed project: {}",
                                        e
                                    );
                                }
                            }

                            self.migrate_legacy_graph_layouts_into_project();
                            self.sync_ui_graph_layouts_from_project();

                            tracing::info!("Opened browsed project successfully");
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to initialize scene viewport for browsed project: {}",
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to open browsed project: {}", e);
                }
            }
        }
    }

    fn handle_save_project_request(&mut self) {
        self.ui.save_project_requested = false;

        if let Some(project) = self.project_manager.current_project.as_mut() {
            project.metadata.editor.graph_layouts = self.ui.export_graph_layouts_for_project();
            project.metadata.editor.rule_graph_drafts =
                self.ui.export_rule_graph_drafts_for_project();
        }

        // Get scenes from UI
        let scenes = &self.ui.scenes;
        match self.project_manager.save_current_project(scenes) {
            Ok(_) => {
                tracing::info!("Project saved successfully");
                self.ui.clear_graph_layout_dirty();
            }
            Err(e) => {
                tracing::error!("Failed to save project: {}", e);
            }
        }
    }

    fn handle_init_project_request(&mut self) {
        self.ui.init_config_requested = false;

        match EditorConfig::init_default_config() {
            Ok(new_config) => {
                self.config = new_config;
                tracing::info!("Config initialized successfully");
            }
            Err(e) => {
                tracing::error!("Failed to initialize config: {}", e);
            }
        }
    }

    fn handle_project_requests(&mut self, _event_loop: &ActiveEventLoop) {
        self.poll_background_task_updates();

        if self.ui.cancel_background_task_requested {
            self.ui.cancel_background_task_requested = false;
            if self.background_tasks.request_cancel() {
                tracing::info!("Background task cancellation requested");
            }
        }

        if self.ui.new_project_requested {
            self.handle_new_project_requested(ProjectTemplateKind::Empty);
        }

        if self.ui.new_top_down_project_requested {
            self.handle_new_project_requested(ProjectTemplateKind::TopDownStarter);
        }

        if self.ui.open_project_requested {
            self.handle_open_project_request();
        }

        // Handle Browse for Project request (file dialog)
        if self.ui.browse_for_project_requested {
            self.handle_browse_for_project_request();
        }

        if self.ui.save_project_requested {
            self.handle_save_project_request();
        }

        if self.ui.export_project_requested {
            self.handle_export_project_request();
        }

        if self.ui.init_config_requested {
            self.handle_init_project_request();
        }

        if self.ui.validate_assets_requested {
            self.handle_validate_assets_request();
        }
    }

    fn poll_background_task_updates(&mut self) {
        for update in self.background_tasks.poll_updates() {
            self.apply_background_task_update(update);
        }
    }

    fn apply_background_task_update(&mut self, update: BackgroundTaskUpdate) {
        match update {
            BackgroundTaskUpdate::Started { kind, message } => {
                self.ui.background_task_running = true;
                self.ui.background_task_status = Some(format!("{}: {}", kind.label(), message));
                tracing::info!(
                    "{}",
                    self.ui.background_task_status.as_deref().unwrap_or("")
                );
            }
            BackgroundTaskUpdate::Progress { kind, message } => {
                self.ui.background_task_running = true;
                self.ui.background_task_status = Some(format!("{}: {}", kind.label(), message));
            }
            BackgroundTaskUpdate::Completed { kind, message } => {
                self.ui.background_task_running = false;
                self.ui.background_task_status = None;
                tracing::info!("{} completed: {}", kind.label(), message);
            }
            BackgroundTaskUpdate::Failed { kind, message } => {
                self.ui.background_task_running = false;
                self.ui.background_task_status = None;
                tracing::error!("{} failed: {}", kind.label(), message);
            }
            BackgroundTaskUpdate::Cancelled { kind } => {
                self.ui.background_task_running = false;
                self.ui.background_task_status = None;
                tracing::info!("{} cancelled", kind.label());
            }
        }
    }

    fn handle_play_scene_request(&mut self) {
        if !self.ui.play_scene_requested {
            return;
        }
        self.ui.play_scene_requested = false;

        let Some(project_path) = self.config.current_project_path().cloned() else {
            tracing::warn!("Cannot play scene: no project is currently open");
            return;
        };
        let Some(active_scene_name) = self.ui.active_scene.clone() else {
            tracing::warn!("Cannot play scene: no active scene is selected");
            return;
        };

        if let Err(error) = self.project_manager.save_current_project(&self.ui.scenes) {
            tracing::error!(
                "Cannot play scene '{}': failed to save current project state: {}",
                active_scene_name,
                error
            );
            return;
        }

        let map_name = self
            .find_scene_by_name(&active_scene_name)
            .and_then(|scene| {
                self.loaded_scene_maps
                    .get(&active_scene_name)
                    .cloned()
                    .filter(|map| scene.maps.iter().any(|scene_map| scene_map == map))
                    .or_else(|| scene.maps.first().cloned())
            });

        let splash_duration_ms = self
            .project_manager
            .current_project
            .as_ref()
            .map(|project| project.metadata.runtime.splash.duration_ms);

        if let Err(error) = Self::launch_runtime_process(
            &project_path,
            &active_scene_name,
            map_name.as_deref(),
            splash_duration_ms,
        ) {
            tracing::error!(
                "Failed to launch runtime for scene '{}' from '{}': {}",
                active_scene_name,
                project_path.display(),
                error
            );
            return;
        }

        tracing::info!(
            "Launched runtime for scene '{}' (map: {})",
            active_scene_name,
            map_name.as_deref().unwrap_or("<auto>")
        );
    }

    fn handle_export_project_request(&mut self) {
        if !self.ui.export_project_requested {
            return;
        }
        self.ui.export_project_requested = false;

        if self.background_tasks.is_running() {
            tracing::warn!("Cannot export game: another background task is running");
            return;
        }

        let Some(project_path) = self.config.current_project_path().cloned() else {
            tracing::warn!("Cannot export game: no project is currently open");
            return;
        };

        if let Err(error) = self.project_manager.save_current_project(&self.ui.scenes) {
            tracing::error!(
                "Cannot export game: failed to save current project state: {}",
                error
            );
            return;
        }

        let default_export_root = project_path
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(Self::workspace_root);
        let export_root = match rfd::FileDialog::new()
            .set_title("Select export destination directory")
            .set_directory(default_export_root)
            .pick_folder()
        {
            Some(path) => path,
            None => {
                tracing::info!("Game export cancelled by user");
                return;
            }
        };

        let startup_scene = self.ui.active_scene.as_deref();
        let splash_duration_ms = self
            .project_manager
            .current_project
            .as_ref()
            .map(|project| project.metadata.runtime.splash.duration_ms)
            .unwrap_or(3000);

        let Some(project) = self.project_manager.current_project.as_ref().cloned() else {
            tracing::warn!("Cannot export game: no project is currently open");
            return;
        };

        let job = ExportBundleJob {
            project,
            workspace_root: Self::workspace_root(),
            export_root,
            startup_scene: startup_scene.map(str::to_string),
            splash_duration_ms,
        };

        if let Err(error) = self.background_tasks.start_export_bundle(job) {
            tracing::error!("Failed to start game export job: {}", error);
        } else {
            self.poll_background_task_updates();
        }
    }

    fn handle_active_scene_map_loading(&mut self) {
        let current_active_scene = self.ui.active_scene.clone();

        if !self.should_reload_scene(&current_active_scene) {
            return;
        }

        self.update_scene_state(&current_active_scene);

        match &current_active_scene {
            Some(scene_name) => self.load_active_scene(scene_name),
            None => self.clear_viewport_scene(),
        }
    }

    fn should_reload_scene(&self, current_scene: &Option<String>) -> bool {
        *current_scene != self.last_loaded_active_scene || self.ui.scene_content_changed
    }

    fn update_scene_state(&mut self, current_scene: &Option<String>) {
        self.last_loaded_active_scene = current_scene.clone();
        self.ui.scene_content_changed = false;

        tracing::info!(
            "Active scene or content changed, reloading map for scene: {:?}",
            current_scene
        );

        if let Some(viewport) = &mut self.scene_viewport {
            viewport.mark_dirty();
        }
    }

    fn load_active_scene(&mut self, scene_name: &str) {
        let Some(active_scene) = self.find_scene_by_name(scene_name).cloned() else {
            tracing::warn!("Active scene '{}' not found in scenes list", scene_name);
            return;
        };

        tracing::info!(
            "Found active scene '{}' with {} maps: {:?}",
            scene_name,
            active_scene.maps.len(),
            active_scene.maps
        );

        let Some(viewport) = &mut self.scene_viewport else {
            return;
        };

        let project_path = self.config.current_project_path().cloned();
        let preferred_map = self.loaded_scene_maps.get(scene_name).map(String::as_str);
        let map_to_load = Self::resolve_scene_map_to_load(&active_scene, preferred_map);

        Self::load_scene_into_gamestate(viewport, &active_scene, scene_name);
        Self::load_scene_tilemap(
            viewport,
            scene_name,
            map_to_load.as_deref(),
            project_path.as_deref(),
        );

        if preferred_map.is_some() && map_to_load.as_deref() != preferred_map {
            self.loaded_scene_maps.remove(scene_name);
        }
    }

    fn find_scene_by_name(&self, scene_name: &str) -> Option<&toki_core::Scene> {
        self.ui.scenes.iter().find(|s| s.name == scene_name)
    }

    fn launch_runtime_process(
        project_path: &std::path::Path,
        scene_name: &str,
        map_name: Option<&str>,
        splash_duration_ms: Option<u64>,
    ) -> Result<()> {
        let runtime_args =
            Self::build_runtime_launch_args(project_path, scene_name, map_name, splash_duration_ms);

        let mut cargo_command = Command::new("cargo");
        cargo_command
            .current_dir(Self::workspace_root())
            .arg("run")
            .arg("-p")
            .arg("toki-runtime")
            .arg("--")
            .args(&runtime_args);

        match cargo_command.spawn() {
            Ok(_) => return Ok(()),
            Err(cargo_error) => {
                tracing::warn!(
                    "Failed to launch runtime via cargo ({}), trying direct binary fallback",
                    cargo_error
                );
            }
        }

        let runtime_bin_name = Self::runtime_binary_name();
        let runtime_bin_path = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|parent| parent.join(runtime_bin_name)))
            .ok_or_else(|| anyhow::anyhow!("Could not resolve runtime binary path"))?;

        Command::new(runtime_bin_path).args(&runtime_args).spawn()?;
        Ok(())
    }

    fn build_runtime_launch_args(
        project_path: &std::path::Path,
        scene_name: &str,
        map_name: Option<&str>,
        splash_duration_ms: Option<u64>,
    ) -> Vec<String> {
        let mut runtime_args = vec![
            "--project".to_string(),
            project_path.display().to_string(),
            "--scene".to_string(),
            scene_name.to_string(),
        ];
        if let Some(map_name) = map_name {
            runtime_args.push("--map".to_string());
            runtime_args.push(map_name.to_string());
        }
        if let Some(duration_ms) = splash_duration_ms {
            runtime_args.push("--splash-duration-ms".to_string());
            runtime_args.push(duration_ms.to_string());
        }
        runtime_args
    }

    fn runtime_binary_name() -> &'static str {
        if cfg!(target_os = "windows") {
            "toki-runtime.exe"
        } else {
            "toki-runtime"
        }
    }

    fn workspace_root() -> std::path::PathBuf {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .map(std::path::Path::to_path_buf)
            .unwrap_or(manifest_dir)
    }

    fn load_scene_into_gamestate(
        viewport: &mut crate::scene::SceneViewport,
        scene: &toki_core::Scene,
        scene_name: &str,
    ) {
        viewport
            .scene_manager_mut()
            .game_state_mut()
            .add_scene(scene.clone());

        match viewport
            .scene_manager_mut()
            .game_state_mut()
            .load_scene(scene_name)
        {
            Ok(()) => {
                tracing::info!(
                    "Loaded active scene '{}' with {} entities into GameState",
                    scene_name,
                    scene.entities.len()
                );
            }
            Err(e) => {
                tracing::error!(
                    "Failed to load active scene '{}' into GameState: {}",
                    scene_name,
                    e
                );
            }
        }
    }

    fn load_scene_tilemap(
        viewport: &mut crate::scene::SceneViewport,
        scene_name: &str,
        map_name: Option<&str>,
        project_path: Option<&std::path::Path>,
    ) {
        let Some(map_name) = map_name else {
            // Even if there's no map, mark dirty to show entities
            viewport.mark_dirty();
            return;
        };

        let Some(project_path) = project_path else {
            tracing::warn!("No project path available for loading tilemap");
            return;
        };

        let map_file = project_path
            .join("assets")
            .join("tilemaps")
            .join(format!("{}.json", map_name));

        match viewport.scene_manager_mut().load_tilemap(&map_file) {
            Ok(()) => {
                tracing::info!(
                    "Loaded active scene '{}' map '{}' into viewport",
                    scene_name,
                    map_name
                );
                viewport.mark_dirty();
            }
            Err(e) => {
                tracing::error!(
                    "Failed to load active scene '{}' map '{}': {}",
                    scene_name,
                    map_name,
                    e
                );
            }
        }
    }

    fn clear_viewport_scene(&mut self) {
        if let Some(viewport) = &mut self.scene_viewport {
            viewport.scene_manager_mut().clear_tilemap();
        }
        tracing::debug!("No active scene set, cleared viewport");
    }

    fn handle_map_requests(&mut self) {
        // Handle Map Loading request
        if let Some((scene_name, map_name)) = self.ui.map_load_requested.take() {
            if let Some(config) = self.config.current_project_path() {
                let map_file = config
                    .join("assets")
                    .join("tilemaps")
                    .join(format!("{}.json", map_name));

                if let Some(viewport) = &mut self.scene_viewport {
                    match viewport.scene_manager_mut().load_tilemap(&map_file) {
                        Ok(()) => {
                            tracing::info!(
                                "Successfully loaded map '{}' from scene '{}' into viewport",
                                map_name,
                                scene_name
                            );
                            self.loaded_scene_maps
                                .insert(scene_name.clone(), map_name.clone());
                            // Mark viewport as needing re-render
                            viewport.mark_dirty();
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to load map '{}' from scene '{}': {}",
                                map_name,
                                scene_name,
                                e
                            );
                        }
                    }
                } else {
                    tracing::warn!(
                        "No scene viewport available for loading map '{}' from scene '{}'",
                        map_name,
                        scene_name
                    );
                }
            } else {
                tracing::warn!(
                    "No project loaded for map loading request: '{}' from scene '{}'",
                    map_name,
                    scene_name
                );
            }
        }
    }

    fn handle_validate_assets_request(&mut self) {
        self.ui.validate_assets_requested = false;

        if self.background_tasks.is_running() {
            tracing::warn!("Cannot validate assets: another background task is running");
            return;
        }

        let Some(project_path) = self.config.current_project_path().cloned() else {
            tracing::warn!("No project loaded - cannot validate assets");
            return;
        };

        tracing::info!("Starting asset validation task");
        if let Err(error) = self
            .background_tasks
            .start_validate_assets(ValidateAssetsJob { project_path })
        {
            tracing::error!("Failed to start asset validation task: {}", error);
        } else {
            self.poll_background_task_updates();
        }
    }

    /// Load sprite frame for preview (cached) - static version
    fn load_preview_sprite_frame_static(
        entity_def_name: &str,
        project_path: &std::path::Path,
        project_assets: &crate::project::ProjectAssets,
    ) -> Option<toki_core::sprite::SpriteFrame> {
        tracing::info!(
            "Loading preview sprite frame for entity '{}' (one-time cache)",
            entity_def_name
        );

        // Load entity definition
        let entity_file = project_path
            .join("entities")
            .join(format!("{}.json", entity_def_name));
        if !entity_file.exists() {
            tracing::warn!(
                "Entity definition file not found for preview: {:?}",
                entity_file
            );
            return None;
        }

        let entity_def = match std::fs::read_to_string(&entity_file).and_then(|content| {
            serde_json::from_str::<toki_core::entity::EntityDefinition>(&content)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }) {
            Ok(def) => def,
            Err(e) => {
                tracing::warn!("Failed to load entity definition for preview: {}", e);
                return None;
            }
        };

        // Get atlas name from entity definition
        let atlas_name = &entity_def.animations.atlas_name;
        let atlas_name_clean = atlas_name.strip_suffix(".json").unwrap_or(atlas_name);

        // Find the atlas in project assets
        let atlas_asset = project_assets.sprite_atlases.get(atlas_name_clean)?;

        // Load the sprite atlas
        let sprite_atlas =
            match toki_core::assets::atlas::AtlasMeta::load_from_file(&atlas_asset.path) {
                Ok(atlas) => atlas,
                Err(e) => {
                    tracing::warn!("Failed to load sprite atlas for preview: {}", e);
                    return None;
                }
            };

        let sprite_texture_size = sprite_atlas
            .image_size()
            .unwrap_or(glam::UVec2::new(64, 16));

        // Get the default animation frame (usually idle state, first frame)
        if let Some(clip_def) = entity_def.animations.clips.first() {
            if let Some(first_tile_name) = clip_def.frame_tiles.first() {
                // Look up the tile in the atlas to get UV coordinates
                if let Some(uvs) = sprite_atlas.get_tile_uvs(first_tile_name, sprite_texture_size) {
                    return Some(toki_core::sprite::SpriteFrame {
                        u0: uvs[0],
                        v0: uvs[1],
                        u1: uvs[2],
                        v1: uvs[3],
                    });
                } else {
                    tracing::warn!(
                        "Failed to get UV coordinates for tile '{}' in preview",
                        first_tile_name
                    );
                }
            } else {
                tracing::warn!("No frame tiles found in first animation clip for preview");
            }
        } else {
            tracing::warn!("No animation clips found for preview");
        }

        None
    }

    fn build_drag_preview_sprites(
        drag_state: &crate::ui::editor_ui::EntityMoveDragState,
        preview_position: glam::Vec2,
        tilemap: Option<&toki_core::assets::tilemap::TileMap>,
        terrain_atlas: Option<&toki_core::assets::atlas::AtlasMeta>,
    ) -> Vec<DragPreviewSprite> {
        let anchor_preview = glam::IVec2::new(
            preview_position.x.floor() as i32,
            preview_position.y.floor() as i32,
        );
        let delta = anchor_preview - drag_state.entity.position;

        drag_state
            .dragged_entities
            .iter()
            .map(|entity| {
                let world_position = entity.position + delta;
                let is_valid = match (tilemap, terrain_atlas) {
                    (Some(tilemap), Some(terrain_atlas)) => {
                        toki_core::collision::can_entity_move_to_position(
                            entity,
                            world_position,
                            tilemap,
                            terrain_atlas,
                        )
                    }
                    _ => true,
                };

                DragPreviewSprite {
                    entity_id: entity.id,
                    world_position,
                    is_valid,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::EditorApp;
    use crate::ui::editor_ui::EntityMoveDragState;
    use glam::{IVec2, UVec2, Vec2};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
    use toki_core::assets::tilemap::TileMap;
    use toki_core::collision::CollisionBox;
    use toki_core::entity::{Entity, EntityAttributes, EntityType};
    use winit::keyboard::ModifiersState;

    #[test]
    fn resolve_scene_map_to_load_prefers_previously_loaded_map() {
        let scene = toki_core::Scene::with_maps(
            "Test Scene".to_string(),
            vec!["map_a".to_string(), "map_b".to_string()],
        );

        let chosen = EditorApp::resolve_scene_map_to_load(&scene, Some("map_b"));
        assert_eq!(chosen.as_deref(), Some("map_b"));
    }

    #[test]
    fn resolve_scene_map_to_load_falls_back_to_first_map_when_preferred_missing() {
        let scene = toki_core::Scene::with_maps(
            "Test Scene".to_string(),
            vec!["map_a".to_string(), "map_b".to_string()],
        );

        let chosen = EditorApp::resolve_scene_map_to_load(&scene, Some("map_missing"));
        assert_eq!(chosen.as_deref(), Some("map_a"));
    }

    #[test]
    fn resolve_scene_map_to_load_returns_none_when_scene_has_no_maps() {
        let scene = toki_core::Scene::new("Empty Scene".to_string());
        let chosen = EditorApp::resolve_scene_map_to_load(&scene, Some("any_map"));
        assert_eq!(chosen, None);
    }

    #[test]
    fn parse_legacy_graph_layout_key_splits_project_scene_and_node() {
        let key = "/tmp/project::Main Scene::rule_1:action:0";
        let parsed = EditorApp::parse_legacy_graph_layout_key(key)
            .expect("legacy graph layout key should parse");
        assert_eq!(parsed.0, "/tmp/project");
        assert_eq!(parsed.1, "Main Scene");
        assert_eq!(parsed.2, "rule_1:action:0");
    }

    #[test]
    fn editor_shortcut_action_maps_ctrl_z_to_undo() {
        let action = EditorApp::editor_shortcut_action(
            &winit::keyboard::Key::Character("z".into()),
            ModifiersState::CONTROL,
        );
        assert_eq!(action, Some(super::EditorShortcutAction::Undo));
    }

    #[test]
    fn editor_shortcut_action_maps_ctrl_y_and_ctrl_shift_z_to_redo() {
        let redo_y = EditorApp::editor_shortcut_action(
            &winit::keyboard::Key::Character("y".into()),
            ModifiersState::CONTROL,
        );
        assert_eq!(redo_y, Some(super::EditorShortcutAction::Redo));

        let redo_shift_z = EditorApp::editor_shortcut_action(
            &winit::keyboard::Key::Character("z".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        assert_eq!(redo_shift_z, Some(super::EditorShortcutAction::Redo));
    }

    #[test]
    fn editor_shortcut_action_ignores_non_ctrl_sequences() {
        let no_ctrl = EditorApp::editor_shortcut_action(
            &winit::keyboard::Key::Character("z".into()),
            ModifiersState::default(),
        );
        assert_eq!(no_ctrl, None);

        let other_key = EditorApp::editor_shortcut_action(
            &winit::keyboard::Key::Character("x".into()),
            ModifiersState::CONTROL,
        );
        assert_eq!(other_key, None);
    }

    #[test]
    fn build_runtime_launch_args_includes_optional_map_and_splash_duration() {
        let args = EditorApp::build_runtime_launch_args(
            std::path::Path::new("/tmp/project"),
            "Main Scene",
            Some("main_map"),
            Some(2600),
        );

        assert_eq!(
            args,
            vec![
                "--project",
                "/tmp/project",
                "--scene",
                "Main Scene",
                "--map",
                "main_map",
                "--splash-duration-ms",
                "2600",
            ]
        );
    }

    #[test]
    fn build_runtime_launch_args_omits_absent_optional_values() {
        let args = EditorApp::build_runtime_launch_args(
            std::path::Path::new("/tmp/project"),
            "Main Scene",
            None,
            None,
        );

        assert_eq!(
            args,
            vec!["--project", "/tmp/project", "--scene", "Main Scene",]
        );
    }

    fn collision_assets_with_center_solid_tile() -> (TileMap, AtlasMeta) {
        let mut tiles = HashMap::new();
        tiles.insert(
            "solid".to_string(),
            TileInfo {
                position: UVec2::new(0, 0),
                properties: TileProperties {
                    solid: true,
                    trigger: false,
                },
            },
        );
        tiles.insert(
            "floor".to_string(),
            TileInfo {
                position: UVec2::new(1, 0),
                properties: TileProperties {
                    solid: false,
                    trigger: false,
                },
            },
        );

        let atlas = AtlasMeta {
            image: PathBuf::from("test.png"),
            tile_size: UVec2::new(16, 16),
            tiles,
        };

        let tilemap = TileMap {
            size: UVec2::new(3, 3),
            tile_size: UVec2::new(16, 16),
            atlas: PathBuf::from("test_atlas.json"),
            tiles: vec![
                "floor".to_string(),
                "floor".to_string(),
                "floor".to_string(),
                "floor".to_string(),
                "solid".to_string(),
                "floor".to_string(),
                "floor".to_string(),
                "floor".to_string(),
                "floor".to_string(),
            ],
        };

        (tilemap, atlas)
    }

    fn solid_entity(id: u32, position: IVec2) -> Entity {
        Entity {
            id,
            position,
            size: UVec2::new(16, 16),
            entity_type: EntityType::Npc,
            definition_name: Some("test".to_string()),
            attributes: EntityAttributes::default(),
            collision_box: Some(CollisionBox::solid_box(UVec2::new(16, 16))),
        }
    }

    #[test]
    fn build_drag_preview_sprites_computes_validity_per_entity() {
        let (tilemap, atlas) = collision_assets_with_center_solid_tile();
        let first = solid_entity(1, IVec2::new(0, 0));
        let second = solid_entity(2, IVec2::new(0, 16));
        let drag_state = EntityMoveDragState {
            scene_name: "Main Scene".to_string(),
            entity: first.clone(),
            dragged_entities: vec![first.clone(), second.clone()],
            grab_offset: Vec2::ZERO,
        };

        let previews = EditorApp::build_drag_preview_sprites(
            &drag_state,
            Vec2::new(16.0, 0.0),
            Some(&tilemap),
            Some(&atlas),
        );

        let first_preview = previews
            .iter()
            .find(|preview| preview.entity_id == first.id)
            .expect("first preview should exist");
        let second_preview = previews
            .iter()
            .find(|preview| preview.entity_id == second.id)
            .expect("second preview should exist");

        assert_eq!(first_preview.world_position, IVec2::new(16, 0));
        assert_eq!(second_preview.world_position, IVec2::new(16, 16));
        assert!(first_preview.is_valid);
        assert!(!second_preview.is_valid);
    }
}
