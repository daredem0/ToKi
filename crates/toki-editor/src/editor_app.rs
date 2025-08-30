use anyhow::Result;
use egui_winit::winit;
use std::sync::Arc;
use toki_core::GameState;
use winit::application::ApplicationHandler;
use winit::event::Modifiers;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::ModifiersState;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::config::EditorConfig;
use crate::logging::LogCapture;
use crate::project::ProjectManager;
use crate::rendering::WindowRenderer;
use crate::scene::SceneViewport;
use crate::ui::EditorUI;
use crate::validation::AssetValidator;

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
        }
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
                    // Try viewport keyboard input first (for zoom controls) using logical keys
                    if let Some(viewport) = &mut self.scene_viewport {
                        tracing::debug!("Passing logical key {:?} to viewport", event.logical_key);
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
        let (window, renderer) = match (&self.window, &mut self.renderer) {
            (Some(w), Some(r)) => (w, r),
            _ => return, // Not initialized yet
        };

        let egui_winit = match &mut self.egui_winit {
            Some(egui) => egui,
            None => return,
        };

        // Prepare egui input
        let raw_input = egui_winit.take_egui_input(window);

        // Pre-render scene to texture before egui UI
        let project_path = self.config.current_project_path();
        if let Some(scene_viewport) = &mut self.scene_viewport {
            if let Some(project_path) = &project_path {
                match scene_viewport
                    .render_to_texture(project_path.as_path(), renderer.egui_renderer_mut())
                {
                    Ok(()) => {
                        // Reduce log spam - render_to_texture already handles its own logging
                        // tracing::debug!("Scene rendered to offscreen texture successfully");
                    }
                    Err(e) => {
                        tracing::error!("Failed to render scene to texture: {}", e);
                    }
                }
            }
        }

        // Run egui UI
        let egui_ctx = egui_winit.egui_ctx().clone();
        let full_output = egui_ctx.run(raw_input, |ctx| {
            // Render UI - viewport will use the pre-rendered texture
            self.ui.render(
                ctx,
                self.scene_viewport.as_mut(),
                Some(&self.config),
                self.log_capture.as_ref(),
                None, // Can't pass renderer due to borrow issues
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
        egui_winit.handle_platform_output(window, full_output.platform_output.clone());

        // Render frame
        if let Err(e) = renderer.render(window, full_output, &egui_ctx) {
            tracing::error!("Render error: {e}");
        }

        // Request redraw if egui wants a repaint
        if egui_ctx.has_requested_repaint() {
            window.request_redraw();
        }

        // Handle project management requests and other actions after rendering is done
        self.handle_project_requests(event_loop);
        self.handle_active_scene_map_loading();
        self.handle_map_requests();
    }

    fn handle_new_project_requested(&mut self) {
        self.ui.new_project_requested = false;

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

            tracing::info!("Creating project '{}' in {:?}", project_name, parent_path);
            match self
                .project_manager
                .create_new_project(project_name.clone(), parent_path.clone())
            {
                Ok(game_state) => {
                    // Update scene viewport with new game state
                    match SceneViewport::with_game_state(game_state) {
                        Ok(viewport) => {
                            self.scene_viewport = self.initialize_viewport(viewport);

                            // Update config with new project path
                            let project_path = parent_path.join(&project_name);
                            self.config.set_project_path(project_path);
                            if let Err(e) = self.config.save() {
                                tracing::warn!(
                                    "Failed to save config after creating project: {}",
                                    e
                                );
                            }

                            tracing::info!("Created new project '{}' successfully", project_name);
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
                                    tracing::info!("Loaded scenes into UI hierarchy from browsed project");
                                }
                                Err(e) => {
                                    tracing::error!("Failed to load scenes into UI from browsed project: {}", e);
                                }
                            }

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

        // Get scenes from UI
        let scenes = &self.ui.scenes;
        match self.project_manager.save_current_project(scenes) {
            Ok(_) => {
                tracing::info!("Project saved successfully");
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
        if self.ui.new_project_requested {
            self.handle_new_project_requested();
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

        if self.ui.init_config_requested {
            self.handle_init_project_request();
        }

        if self.ui.validate_assets_requested {
            self.handle_validate_assets_request();
        }
    }

    fn handle_active_scene_map_loading(&mut self) {
        // Load if active scene has changed OR if scene content has changed
        let current_active_scene = self.ui.active_scene.clone();

        if current_active_scene != self.last_loaded_active_scene || self.ui.scene_content_changed {
            self.last_loaded_active_scene = current_active_scene.clone();
            self.ui.scene_content_changed = false; // Clear the flag
            tracing::info!(
                "Active scene or content changed, reloading map for scene: {:?}",
                current_active_scene
            );

            // Mark viewport as needing re-render since scene changed
            if let Some(viewport) = &mut self.scene_viewport {
                viewport.mark_dirty();
            }

            if let Some(active_scene_name) = &current_active_scene {
                // Find the active scene
                if let Some(active_scene) =
                    self.ui.scenes.iter().find(|s| &s.name == active_scene_name)
                {
                    tracing::info!(
                        "Found active scene '{}' with {} maps: {:?}",
                        active_scene_name,
                        active_scene.maps.len(),
                        active_scene.maps
                    );
                    // If the scene has at least one map, load it
                    if let Some(map_name) = active_scene.maps.first() {
                        if let Some(config) = self.config.current_project_path() {
                            let map_file = config
                                .join("assets")
                                .join("tilemaps")
                                .join(format!("{}.json", map_name));
                            if let Some(viewport) = &mut self.scene_viewport {
                                match viewport.scene_manager_mut().load_tilemap(&map_file) {
                                    Ok(()) => {
                                        tracing::info!(
                                            "Loaded active scene '{}' map '{}' into viewport",
                                            active_scene_name,
                                            map_name
                                        );
                                        // Mark viewport as needing re-render
                                        viewport.mark_dirty();
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to load active scene '{}' map '{}': {}",
                                            active_scene_name,
                                            map_name,
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    } else {
                        // No maps in active scene - clear viewport
                        if let Some(viewport) = &mut self.scene_viewport {
                            viewport.scene_manager_mut().clear_tilemap();
                        }
                        tracing::debug!(
                            "Active scene '{}' has no maps, cleared viewport",
                            active_scene_name
                        );
                    }
                }
            } else {
                // No active scene - clear viewport
                if let Some(viewport) = &mut self.scene_viewport {
                    viewport.scene_manager_mut().clear_tilemap();
                }
                tracing::debug!("No active scene set, cleared viewport");
            }
        }
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
        
        if let Some(project_assets) = self.project_manager.get_project_assets() {
            tracing::info!("Starting asset validation");
            
            match AssetValidator::new() {
                Ok(validator) => {
                    if let Err(e) = validator.validate_project_assets(project_assets) {
                        tracing::error!("Asset validation failed: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create asset validator: {}", e);
                }
            }
        } else {
            tracing::warn!("No project loaded - cannot validate assets");
        }
    }
}
