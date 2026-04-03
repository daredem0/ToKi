use anyhow::Result;
use egui_winit::winit;
use glam::IVec2;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use toki_core::assets::{atlas::AtlasMeta, tilemap::TileMap};
use toki_core::game::SceneSystem;
use toki_core::GameState;
use winit::application::ApplicationHandler;
use winit::event::Modifiers;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::ModifiersState;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

use crate::background_tasks::{
    BackgroundTaskManager, BackgroundTaskUpdate, ExportBundleJob, ValidateAssetsJob,
};
use crate::config::EditorConfig;
use crate::editor_services::{commands as editor_commands, graph_metadata};
use crate::editor_types::PlacementPreviewVisual;
use crate::fonts::{load_project_fonts_into_egui, menu_font_family_choices};
use crate::logging::LogCapture;
use crate::project::ProjectAssets;
use crate::project::{ProjectManager, ProjectTemplateKind};
use crate::rendering::WindowRenderer;
use crate::scene::overlays as scene_overlays;
#[cfg(test)]
use crate::scene::viewport::DragPreviewSprite;
use crate::scene::viewport::ViewportOverlayData;
use crate::scene::SceneViewport;
use crate::ui::editor_ui::EditorConfirmation;
use crate::ui::editor_ui::{CenterPanelTab, MapEditorDraft};
use crate::ui::EditorUI;

#[path = "editor_app/background_tasks.rs"]
mod background_tasks;
#[path = "editor_app/map_editor.rs"]
mod map_editor;
#[path = "editor_app/new_project.rs"]
mod new_project;
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
    /// Caches preview visuals by project and entity definition name.
    pub preview_sprite_frames: std::collections::HashMap<
        (PathBuf, String, Option<String>),
        Option<PlacementPreviewVisual>,
    >,
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
    Copy,
    Paste,
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

    fn init_viewport_with<R, V, Create, Initialize>(
        renderer: Option<&R>,
        create_viewport: Create,
        initialize_viewport: Initialize,
        viewport_label: &str,
    ) -> Option<V>
    where
        Create: FnOnce() -> Result<V>,
        Initialize: FnOnce(&R, &mut V) -> Result<()>,
    {
        let Some(renderer) = renderer else {
            tracing::error!("Cannot initialize {viewport_label}: renderer not available");
            return None;
        };

        let mut viewport = match create_viewport() {
            Ok(viewport) => viewport,
            Err(error) => {
                tracing::error!("Failed to create {viewport_label}: {error}");
                return None;
            }
        };

        match initialize_viewport(renderer, &mut viewport) {
            Ok(()) => {
                tracing::info!("{viewport_label} initialized");
                Some(viewport)
            }
            Err(error) => {
                tracing::error!("Failed to initialize {viewport_label}: {error}");
                None
            }
        }
    }

    /// Helper method to initialize a viewport with WGPU context.
    fn initialize_viewport(
        &self,
        create_viewport: impl FnOnce() -> Result<SceneViewport>,
        viewport_label: &str,
    ) -> Option<SceneViewport> {
        Self::init_viewport_with(
            self.platform.renderer.as_ref(),
            create_viewport,
            |renderer, viewport| {
                pollster::block_on(
                    viewport.initialize(renderer.device().clone(), renderer.queue().clone()),
                )?;
                Ok(())
            },
            viewport_label,
        )
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
            "c" => Some(EditorShortcutAction::Copy),
            "v" => Some(EditorShortcutAction::Paste),
            _ => None,
        }
    }

    fn handle_escape_key(&mut self) {
        if crate::ui::editor_context::scene_viewport_context(&self.core.ui)
            .placement
            .is_in_placement_mode()
        {
            crate::ui::editor_context::scene_viewport_context_mut(&mut self.core.ui)
                .placement
                .exit_placement_mode();
            tracing::info!("Exited placement mode via Escape");
            return;
        }

        if self.escape_belongs_to_active_editor() {
            tracing::info!("Escape handled by active editor transient state");
            return;
        }

        self.core.ui.project.pending_confirmation = Some(EditorConfirmation::ExitEditor);
        tracing::info!("Escape requested editor exit confirmation");
    }

    fn escape_belongs_to_active_editor(&self) -> bool {
        use crate::ui::editor_ui::CenterPanelTab;

        if self.core.ui.workspace.center_panel_tab == CenterPanelTab::SpriteEditor {
            return crate::ui::editor_context::sprite_state(&self.core.ui).has_floating()
                || crate::ui::editor_context::sprite_state(&self.core.ui)
                    .active()
                    .selection
                    .is_some();
        }

        false
    }

    fn toggled_fullscreen_state(is_currently_fullscreen: bool) -> Option<Fullscreen> {
        if is_currently_fullscreen {
            None
        } else {
            Some(Fullscreen::Borderless(None))
        }
    }

    fn toggle_window_fullscreen(window: &Window) {
        let next_state = Self::toggled_fullscreen_state(window.fullscreen().is_some());
        window.set_fullscreen(next_state);
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

        self.viewports.scene = self.initialize_viewport(
            || SceneViewport::with_game_state(GameState::new_empty()),
            "scene viewport",
        );

        self.viewports.map_editor = self.initialize_viewport(
            || SceneViewport::with_game_state_responsive(GameState::new_empty()),
            "map editor viewport",
        );

        tracing::info!("Editor initialized successfully");
        if !self.session.startup_project_auto_open_done {
            self.session.startup_project_auto_open_done = true;
            if self.core.config.has_project_path() {
                tracing::info!("Auto-opening last project from config on startup");
                self.core
                    .ui
                    .project
                    .request(crate::ui::editor_ui::ProjectRequest::OpenProject);
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
                    let active_viewport = match self.core.ui.workspace.center_panel_tab {
                        CenterPanelTab::SceneViewport => self.viewports.scene.as_mut(),
                        CenterPanelTab::MapEditor => self.viewports.map_editor.as_mut(),
                        CenterPanelTab::SceneGraph
                        | CenterPanelTab::SceneRules
                        | CenterPanelTab::MenuEditor
                        | CenterPanelTab::DialogEditor
                        | CenterPanelTab::UiEditor
                        | CenterPanelTab::SpriteEditor
                        | CenterPanelTab::AnimationEditor
                        | CenterPanelTab::EntityEditor => None,
                    };
                    if let Some(viewport) = active_viewport {
                        tracing::trace!("Passing logical key {:?} to viewport", event.logical_key);
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
                                    .core
                                    .project_manager
                                    .current_project
                                    .as_mut()
                                    .map(|project| {
                                        editor_commands::undo_with_project(
                                            &mut self.core.ui,
                                            project,
                                        )
                                    })
                                    .unwrap_or_else(|| editor_commands::undo(&mut self.core.ui));
                                if undone {
                                    tracing::debug!("Undo applied via Ctrl+Z");
                                }
                            }
                            EditorShortcutAction::Redo => {
                                let redone = self
                                    .core
                                    .project_manager
                                    .current_project
                                    .as_mut()
                                    .map(|project| {
                                        editor_commands::redo_with_project(
                                            &mut self.core.ui,
                                            project,
                                        )
                                    })
                                    .unwrap_or_else(|| editor_commands::redo(&mut self.core.ui));
                                if redone {
                                    tracing::debug!("Redo applied via Ctrl+Y/Ctrl+Shift+Z");
                                }
                            }
                            EditorShortcutAction::Copy => {
                                // Copy only applies to sprite editor
                                if self.core.ui.workspace.center_panel_tab
                                    == CenterPanelTab::SpriteEditor
                                    && crate::ui::editor_context::sprite_state_mut(
                                        &mut self.core.ui,
                                    )
                                    .copy_selection()
                                {
                                    tracing::debug!("Sprite editor: copied selection to clipboard");
                                }
                            }
                            EditorShortcutAction::Paste => {
                                // Paste only applies to sprite editor
                                if self.core.ui.workspace.center_panel_tab
                                    == CenterPanelTab::SpriteEditor
                                {
                                    let side = crate::ui::editor_context::sprite_state_mut(
                                        &mut self.core.ui,
                                    )
                                    .active_canvas;
                                    let sprite = crate::ui::editor_context::sprite_state_mut(
                                        &mut self.core.ui,
                                    );
                                    // Use (0, 0) as fallback if no cursor position
                                    if sprite.canvas_state(side).cursor_canvas_pos.is_none() {
                                        sprite.canvas_state_mut(side).cursor_canvas_pos =
                                            Some(IVec2::new(0, 0));
                                    }
                                    if crate::ui::editor_context::sprite_state_mut(
                                        &mut self.core.ui,
                                    )
                                    .paste_at_cursor(side)
                                    {
                                        tracing::info!(
                                            "Sprite editor: pasted at cursor on {:?}",
                                            side
                                        );
                                    }
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
                            KeyCode::Enter | KeyCode::NumpadEnter => {
                                if self.core.ui.workspace.center_panel_tab
                                    == CenterPanelTab::SceneViewport
                                    && self
                                        .core
                                        .ui
                                        .scene_viewport_context()
                                        .placement
                                        .is_in_placement_mode()
                                {
                                    self.core
                                        .ui
                                        .scene_viewport_context_mut()
                                        .placement
                                        .exit_placement_mode();
                                    if let Some(viewport) = &mut self.viewports.scene {
                                        viewport.mark_dirty();
                                    }
                                    if let Some(window) = &self.platform.window {
                                        window.request_redraw();
                                    }
                                    return;
                                }
                            }
                            KeyCode::Delete => {
                                if self.core.ui.workspace.center_panel_tab
                                    == CenterPanelTab::SceneViewport
                                {
                                    let deleted = crate::ui::interactions::SelectionInteraction::delete_selected_spatial(
                                        &mut self.core.ui,
                                    );
                                    if deleted {
                                        if let Some(viewport) = &mut self.viewports.scene {
                                            viewport.mark_dirty();
                                        }
                                        if let Some(window) = &self.platform.window {
                                            window.request_redraw();
                                        }
                                        return;
                                    }
                                }
                            }
                            KeyCode::Escape => {
                                self.handle_escape_key();
                                if let Some(window) = &self.platform.window {
                                    window.request_redraw();
                                }
                            }
                            KeyCode::F11 => {
                                if let Some(window) = &self.platform.window {
                                    Self::toggle_window_fullscreen(window);
                                    tracing::info!(
                                        "Toggled editor fullscreen: {}",
                                        window.fullscreen().is_some()
                                    );
                                    window.request_redraw();
                                }
                            }
                            KeyCode::F1 => {
                                self.core.ui.visibility.show_hierarchy =
                                    !self.core.ui.visibility.show_hierarchy;
                                tracing::info!(
                                    "Toggled hierarchy panel: {}",
                                    self.core.ui.visibility.show_hierarchy
                                );
                                if let Some(window) = &self.platform.window {
                                    window.request_redraw();
                                }
                            }
                            KeyCode::F2 => {
                                self.core.ui.visibility.show_inspector =
                                    !self.core.ui.visibility.show_inspector;
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
                                    toki_core::game::InputSystem::handle_key_press(
                                        viewport.game_state_mut().runtime_mut(),
                                        toki_core::InputKey::DebugToggle,
                                    );
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
        let Some((window, egui_ctx)) = self.render_window_and_ctx() else {
            return;
        };
        self.prepare_render_state(&egui_ctx);

        let project_path = self.core.config.current_project_path().cloned();
        self.ensure_placement_preview_cached_frame(project_path.as_deref());
        self.sync_indexed_palette_override_from_project();

        let raw_input = {
            let Some(egui_winit) = &mut self.platform.egui_winit else {
                return;
            };
            egui_winit.take_egui_input(&window)
        };

        self.pre_render_active_center_viewport(project_path.as_deref());

        let available_map_names =
            Self::collect_available_map_names(self.core.project_manager.get_project_assets());
        let full_output = self.run_egui_frame(&egui_ctx, raw_input, available_map_names);

        if self.handle_render_ui_requests(event_loop) {
            return;
        }

        {
            let Some(egui_winit) = &mut self.platform.egui_winit else {
                return;
            };
            egui_winit.handle_platform_output(&window, full_output.platform_output.clone());
        }

        {
            let Some(renderer) = &mut self.platform.renderer else {
                return;
            };
            if let Err(e) = renderer.render(&window, full_output, &egui_ctx) {
                tracing::error!("Render error: {e}");
            }
        }

        graph_metadata::persist_if_dirty(
            &mut self.core.ui,
            self.core.project_manager.current_project.as_mut(),
            &egui_ctx,
        );

        self.request_redraw_if_needed(&window, &egui_ctx);
        self.handle_post_render_requests(event_loop);
    }

    fn render_window_and_ctx(
        &self,
    ) -> Option<(std::sync::Arc<winit::window::Window>, egui::Context)> {
        let window = self.platform.window.as_ref()?.clone();
        let egui_ctx = self.platform.egui_winit.as_ref()?.egui_ctx().clone();
        Some((window, egui_ctx))
    }

    fn prepare_render_state(&mut self, egui_ctx: &egui::Context) {
        if self.core.ui.project.background_task_running {
            self.ensure_busy_logo_texture(egui_ctx);
        }
        self.sync_project_menu_preview_fonts(egui_ctx);
    }

    fn ensure_placement_preview_cached_frame(&mut self, project_path: Option<&std::path::Path>) {
        if !(crate::ui::editor_context::scene_viewport_context(&self.core.ui)
            .placement
            .is_in_placement_mode()
            && crate::ui::editor_context::scene_viewport_context(&self.core.ui)
                .placement
                .preview_cached_frame
                .is_none())
        {
            return;
        }

        let placement_ctx =
            &crate::ui::editor_context::scene_viewport_context(&self.core.ui).placement;
        let placement_entity_definition = placement_ctx.entity_definition().map(str::to_string);
        let placement_decoration = crate::ui::editor_context::scene_viewport_context(&self.core.ui)
            .placement
            .decoration_draft()
            .cloned();
        if let (Some(entity_def), Some(project_path), Some(project_assets)) = (
            placement_entity_definition.as_deref(),
            project_path,
            self.core.project_manager.get_project_assets(),
        ) {
            let cached_frame = scene_overlays::cached_preview_sprite_frame(
                &mut self.resources.preview_sprite_frames,
                entity_def,
                project_path,
                project_assets,
                &self.core.ui.project.available_palettes,
                Self::project_indexed_palette_override(
                    self.core.project_manager.current_project.as_ref(),
                )
                .as_deref(),
            );
            crate::ui::editor_context::scene_viewport_context_mut(&mut self.core.ui)
                .placement
                .preview_cached_frame = cached_frame;
        } else if let (Some(draft), Some(project_path)) = (placement_decoration, project_path) {
            let cached_frame = scene_overlays::cached_decoration_preview_sprite_frame(
                &mut self.resources.preview_sprite_frames,
                &draft.sheet,
                &draft.object_name,
                project_path,
            );
            crate::ui::editor_context::scene_viewport_context_mut(&mut self.core.ui)
                .placement
                .preview_cached_frame = cached_frame;
        }
    }

    fn sync_indexed_palette_override_from_project(&mut self) {
        self.core.ui.project.indexed_palette_override = Self::project_indexed_palette_override(
            self.core.project_manager.current_project.as_ref(),
        );
    }

    fn project_indexed_palette_override(
        project: Option<&crate::project::Project>,
    ) -> Option<String> {
        project.and_then(|project| {
            project
                .metadata
                .runtime
                .display
                .indexed_palette_override
                .clone()
        })
    }

    fn pre_render_active_center_viewport(&mut self, project_path: Option<&std::path::Path>) {
        let Some(project_path) = project_path else {
            return;
        };
        let Some(project_assets) = self.core.project_manager.get_project_assets() else {
            if self.core.project_manager.current_project.is_some() {
                tracing::warn!(
                    "No project assets available for viewport rendering {:?}",
                    self.core.project_manager.current_project
                );
            }
            return;
        };
        let Some(renderer) = &mut self.platform.renderer else {
            return;
        };

        match self.core.ui.workspace.center_panel_tab {
            CenterPanelTab::SceneViewport => {
                let scene_player_overlay_sprites =
                    scene_overlays::build_scene_player_overlay_sprites(
                        self.core.ui.active_scene.as_deref(),
                        &self.core.ui.scenes,
                        project_path,
                        project_assets,
                        &mut self.resources.preview_sprite_frames,
                        &self.core.ui.project.available_palettes,
                        Self::project_indexed_palette_override(
                            self.core.project_manager.current_project.as_ref(),
                        )
                        .as_deref(),
                    );
                let overlay_data = self.viewports.scene.as_ref().map(|scene_viewport| {
                    Self::build_scene_viewport_overlay_data(
                        &self.core.ui,
                        &self.core.config,
                        scene_viewport,
                        scene_player_overlay_sprites,
                    )
                });
                if let (Some(scene_viewport), Some(overlay_data)) =
                    (&mut self.viewports.scene, overlay_data)
                {
                    Self::pre_render_scene_viewport(
                        scene_viewport,
                        project_path,
                        project_assets,
                        renderer,
                        Self::project_indexed_palette_override(
                            self.core.project_manager.current_project.as_ref(),
                        ),
                        overlay_data,
                    );
                }
            }
            CenterPanelTab::MapEditor => {
                if let Some(map_editor_viewport) = &mut self.viewports.map_editor {
                    Self::pre_render_map_editor_viewport(
                        map_editor_viewport,
                        project_path,
                        project_assets,
                        renderer,
                        Self::project_indexed_palette_override(
                            self.core.project_manager.current_project.as_ref(),
                        ),
                    );
                }
            }
            CenterPanelTab::SceneGraph
            | CenterPanelTab::SceneRules
            | CenterPanelTab::MenuEditor
            | CenterPanelTab::DialogEditor
            | CenterPanelTab::UiEditor
            | CenterPanelTab::SpriteEditor
            | CenterPanelTab::AnimationEditor
            | CenterPanelTab::EntityEditor => {}
        }
    }

    fn pre_render_scene_viewport(
        scene_viewport: &mut crate::scene::SceneViewport,
        project_path: &std::path::Path,
        project_assets: &ProjectAssets,
        renderer: &mut crate::rendering::window::WindowRenderer,
        indexed_palette_override: Option<String>,
        overlay_data: ViewportOverlayData,
    ) {
        scene_viewport.set_indexed_palette_override(indexed_palette_override);
        if let Err(e) = scene_viewport.render_to_texture(
            project_path,
            project_assets,
            renderer.egui_renderer_mut(),
            &overlay_data,
        ) {
            tracing::error!("Failed to render scene to texture: {}", e);
        }
    }

    fn build_scene_viewport_overlay_data(
        ui_state: &crate::ui::EditorUI,
        config: &crate::config::EditorConfig,
        scene_viewport: &crate::scene::SceneViewport,
        scene_player_overlay_sprites: Vec<crate::scene::viewport::OverlaySpriteInstance>,
    ) -> ViewportOverlayData {
        let placement_preview = if crate::ui::editor_context::scene_viewport_context(ui_state)
            .placement
            .is_in_placement_mode()
            && crate::ui::editor_context::scene_viewport_context(ui_state)
                .placement
                .entity_move_drag
                .is_none()
            && crate::ui::editor_context::scene_viewport_context(ui_state)
                .placement
                .scene_anchor_draft()
                .is_none()
        {
            let is_valid = crate::ui::editor_context::scene_viewport_context(ui_state)
                .placement
                .preview_valid
                .unwrap_or(true);
            match (
                crate::ui::editor_context::scene_viewport_context(ui_state)
                    .placement
                    .kind
                    .as_ref(),
                &crate::ui::editor_context::scene_viewport_context(ui_state)
                    .placement
                    .preview_position,
                &crate::ui::editor_context::scene_viewport_context(ui_state)
                    .placement
                    .preview_cached_frame,
            ) {
                (Some(_), Some(position), Some(cached_frame)) => {
                    Some((*position, cached_frame.clone(), is_valid))
                }
                _ => None,
            }
        } else {
            None
        };

        let dragged_anchor = crate::ui::editor_context::scene_viewport_context(ui_state)
            .placement
            .scene_anchor_move_drag
            .as_ref()
            .map(|drag| (drag.scene_name.as_str(), drag.anchor.id.as_str()));
        let anchor_overlay_lines = scene_overlays::build_scene_anchor_overlay_lines(
            scene_overlays::SceneAnchorOverlayRequest {
                active_scene_name: ui_state.active_scene.as_deref(),
                scenes: &ui_state.scenes,
                dragged_anchor,
                preview_position: crate::ui::editor_context::scene_viewport_context(ui_state)
                    .placement
                    .preview_position,
                preview_valid: crate::ui::editor_context::scene_viewport_context(ui_state)
                    .placement
                    .preview_valid
                    .unwrap_or(true),
                draft_active: crate::ui::editor_context::scene_viewport_context(ui_state)
                    .placement
                    .scene_anchor_draft()
                    .is_some(),
            },
            scene_viewport.tilemap(),
            Some(config),
        );
        let drag_preview_sprites = crate::ui::editor_context::scene_viewport_context(ui_state)
            .placement
            .entity_move_drag
            .as_ref()
            .and_then(|drag| {
                crate::ui::editor_context::scene_viewport_context(ui_state)
                    .placement
                    .preview_position
                    .map(|preview_position| {
                        let tilemap = scene_viewport.tilemap();
                        let terrain_atlas =
                            tilemap.map(|_| scene_viewport.resources().get_terrain_atlas());
                        scene_overlays::build_drag_preview_sprites(
                            &drag.dragged_entities,
                            drag.entity.position,
                            preview_position,
                            tilemap,
                            terrain_atlas,
                        )
                    })
            })
            .unwrap_or_default();

        ViewportOverlayData {
            placement_preview,
            drag_preview_sprites,
            overlay_sprites: scene_player_overlay_sprites,
            overlay_rects: Vec::new(),
            overlay_lines: anchor_overlay_lines,
        }
    }

    fn pre_render_map_editor_viewport(
        map_editor_viewport: &mut crate::scene::SceneViewport,
        project_path: &std::path::Path,
        project_assets: &ProjectAssets,
        renderer: &mut crate::rendering::window::WindowRenderer,
        indexed_palette_override: Option<String>,
    ) {
        map_editor_viewport.set_indexed_palette_override(indexed_palette_override);
        if let Err(e) = map_editor_viewport.render_to_texture(
            project_path,
            project_assets,
            renderer.egui_renderer_mut(),
            &ViewportOverlayData::default(),
        ) {
            tracing::error!("Failed to render map editor viewport to texture: {}", e);
        }
    }

    fn collect_available_map_names(project_assets: Option<&ProjectAssets>) -> Option<Vec<String>> {
        project_assets.map(|assets| {
            let mut names = assets.tilemaps.keys().cloned().collect::<Vec<_>>();
            names.sort();
            names
        })
    }

    fn run_egui_frame(
        &mut self,
        egui_ctx: &egui::Context,
        raw_input: egui::RawInput,
        available_map_names: Option<Vec<String>>,
    ) -> egui::FullOutput {
        let (current_project, project_assets) =
            self.core.project_manager.current_project_and_assets_mut();
        let mut current_project = current_project;
        let mut project_assets = project_assets;
        egui_ctx.run(raw_input, |ctx| {
            self.core.ui.render(
                ctx,
                crate::ui::editor_ui::EditorRenderContext {
                    scene_viewport: self.viewports.scene.as_mut(),
                    map_editor_viewport: self.viewports.map_editor.as_mut(),
                    project: current_project.as_deref_mut(),
                    project_assets: project_assets.as_deref_mut(),
                    available_map_names: available_map_names.clone(),
                    config: Some(&mut self.core.config),
                    log_capture: self.log_capture.as_ref(),
                    renderer: None,
                    busy_logo_texture: self.resources.busy_logo_texture.as_ref(),
                },
            );
        })
    }

    fn handle_render_ui_requests(&mut self, event_loop: &ActiveEventLoop) -> bool {
        if self.core.ui.visibility.should_exit {
            event_loop.exit();
            return true;
        }
        if self.core.ui.visibility.create_test_entities {
            if let Some(viewport) = &mut self.viewports.scene {
                let game_state = viewport.game_state_mut();
                let _player_id = SceneSystem::spawn_player_at(game_state, glam::IVec2::new(80, 72));
                let _npc_id =
                    SceneSystem::spawn_player_like_npc(game_state, glam::IVec2::new(120, 72));
                tracing::info!("Created test entities");
            }
            self.core.ui.visibility.create_test_entities = false;
        }
        false
    }

    fn request_redraw_if_needed(&self, window: &winit::window::Window, egui_ctx: &egui::Context) {
        if egui_ctx.has_requested_repaint()
            || self
                .viewports
                .scene
                .as_ref()
                .is_some_and(crate::scene::SceneViewport::needs_render)
            || self
                .viewports
                .map_editor
                .as_ref()
                .is_some_and(crate::scene::SceneViewport::needs_render)
        {
            window.request_redraw();
        }
    }

    fn handle_post_render_requests(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_project_requests(event_loop);
        self.handle_play_scene_request();
        self.handle_active_scene_map_loading();
        self.handle_map_requests();
        self.handle_new_map_editor_requests();
        self.handle_pending_map_editor_tilemap_sync();
        self.handle_save_map_editor_request();
        self.handle_map_editor_map_requests();
        self.handle_sprite_asset_rescan();
    }

    /// Handle sprite asset rescan request (after saving new sprites)
    fn handle_sprite_asset_rescan(&mut self) {
        if crate::ui::editor_context::sprite_state_mut(&mut self.core.ui).needs_asset_rescan {
            if let Err(e) = self.core.project_manager.rescan_assets() {
                tracing::error!("Failed to rescan assets: {}", e);
            } else {
                self.refresh_project_assets_after_rescan();
            }
            crate::ui::editor_context::sprite_state_mut(&mut self.core.ui).needs_asset_rescan =
                false;
        }
    }
}

#[cfg(test)]
impl EditorApp {
    fn load_preview_sprite_frame_static(
        entity_def_name: &str,
        project_path: &std::path::Path,
        project_assets: &ProjectAssets,
    ) -> Option<PlacementPreviewVisual> {
        scene_overlays::load_preview_sprite_frame(
            entity_def_name,
            project_path,
            project_assets,
            &toki_core::palette::builtin_palettes(),
            None,
        )
    }

    fn cached_preview_sprite_frame(
        preview_sprite_frames: &mut std::collections::HashMap<
            (PathBuf, String, Option<String>),
            Option<PlacementPreviewVisual>,
        >,
        entity_def_name: &str,
        project_path: &std::path::Path,
        project_assets: &ProjectAssets,
        indexed_palette_override: Option<&str>,
    ) -> Option<PlacementPreviewVisual> {
        scene_overlays::cached_preview_sprite_frame(
            preview_sprite_frames,
            entity_def_name,
            project_path,
            project_assets,
            &toki_core::palette::builtin_palettes(),
            indexed_palette_override,
        )
    }

    fn build_scene_player_overlay_sprites(
        ui_state: &crate::ui::EditorUI,
        project_path: &std::path::Path,
        project_assets: &ProjectAssets,
        preview_cache: &mut std::collections::HashMap<
            (PathBuf, String, Option<String>),
            Option<PlacementPreviewVisual>,
        >,
    ) -> Vec<crate::scene::viewport::OverlaySpriteInstance> {
        scene_overlays::build_scene_player_overlay_sprites(
            ui_state.active_scene.as_deref(),
            &ui_state.scenes,
            project_path,
            project_assets,
            preview_cache,
            &ui_state.project.available_palettes,
            None,
        )
    }

    fn build_scene_anchor_overlay_lines(
        ui_state: &crate::ui::EditorUI,
        tilemap: Option<&toki_core::assets::tilemap::TileMap>,
        config: Option<&EditorConfig>,
    ) -> Vec<crate::scene::viewport::OverlayLineInstance> {
        let dragged_anchor = crate::ui::editor_context::scene_viewport_context(ui_state)
            .placement
            .scene_anchor_move_drag
            .as_ref()
            .map(|drag| (drag.scene_name.as_str(), drag.anchor.id.as_str()));
        scene_overlays::build_scene_anchor_overlay_lines(
            scene_overlays::SceneAnchorOverlayRequest {
                active_scene_name: ui_state.active_scene.as_deref(),
                scenes: &ui_state.scenes,
                dragged_anchor,
                preview_position: crate::ui::editor_context::scene_viewport_context(ui_state)
                    .placement
                    .preview_position,
                preview_valid: crate::ui::editor_context::scene_viewport_context(ui_state)
                    .placement
                    .preview_valid
                    .unwrap_or(true),
                draft_active: crate::ui::editor_context::scene_viewport_context(ui_state)
                    .placement
                    .scene_anchor_draft()
                    .is_some(),
            },
            tilemap,
            config,
        )
    }

    fn build_drag_preview_sprites(
        drag_state: &crate::ui::editor_ui::EntityMoveDragState,
        preview_position: glam::Vec2,
        tilemap: Option<&toki_core::assets::tilemap::TileMap>,
        terrain_atlas: Option<&toki_core::assets::atlas::AtlasMeta>,
    ) -> Vec<DragPreviewSprite> {
        scene_overlays::build_drag_preview_sprites(
            &drag_state.dragged_entities,
            drag_state.entity.position,
            preview_position,
            tilemap,
            terrain_atlas,
        )
    }
}

#[cfg(test)]
#[path = "editor_app_tests.rs"]
mod tests;
