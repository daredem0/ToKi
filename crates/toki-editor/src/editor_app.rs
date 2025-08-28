use anyhow::Result;
use egui_winit::winit;
use std::sync::Arc;
use toki_core::GameState;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::rendering::WindowRenderer;
use crate::ui::EditorUI;
use crate::scene::SceneViewport;

pub fn run_editor() -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut editor_app = EditorApp::new();
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
    
}

impl EditorApp {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            ui: EditorUI::new(),
            egui_winit: None,
            scene_viewport: None,
        }
    }
}

impl ApplicationHandler for EditorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window
        let window_attributes = winit::window::Window::default_attributes()
            .with_title("ToKi Editor")
            .with_inner_size(winit::dpi::PhysicalSize::new(1200, 800));
            
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
        
        // Initialize scene viewport with empty game state
        let game_state = GameState::new_empty();
        match SceneViewport::with_game_state(game_state) {
            Ok(mut viewport) => {
                // Initialize the scene viewport
                viewport.initialize();
                self.scene_viewport = Some(viewport);
                tracing::info!("Scene viewport initialized");
            }
            Err(e) => {
                tracing::error!("Failed to initialize scene viewport: {e}");
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
        if let Some(egui_winit) = &mut self.egui_winit {
            if let Some(window) = &self.window {
                let _ = egui_winit.on_window_event(window, &event);
            }
        }
        
        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("Close requested, shutting down editor");
                event_loop.exit();
            }
            
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    if let PhysicalKey::Code(key_code) = event.physical_key {
                        match key_code {
                            KeyCode::Escape => event_loop.exit(),
                            KeyCode::F1 => {
                                self.ui.show_hierarchy = !self.ui.show_hierarchy;
                                tracing::info!("Toggled hierarchy panel: {}", self.ui.show_hierarchy);
                            }
                            KeyCode::F2 => {
                                self.ui.show_inspector = !self.ui.show_inspector;
                                tracing::info!("Toggled inspector panel: {}", self.ui.show_inspector);
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
            }
            
            WindowEvent::RedrawRequested => {
                self.render(event_loop);
            }
            
            _ => {}
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
        
        // Run egui UI
        let egui_ctx = egui_winit.egui_ctx().clone();
        let full_output = egui_ctx.run(raw_input, |ctx| {
            // Render all UI components - we'll handle game state inside UI render method
            self.ui.render(ctx, self.scene_viewport.as_mut());
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
        
        // Request next frame
        window.request_redraw();
    }
}