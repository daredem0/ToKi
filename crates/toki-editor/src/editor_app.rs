use anyhow::Result;
use egui_winit::winit;
use std::sync::Arc;
use std::time::Instant;
use toki_core::{GameState, TimingSystem};
use toki_core::entity::EntityId;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::renderer::EditorRenderer;
use crate::ui::EditorUI;

pub fn run_editor() -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut editor_app = EditorApp::new();
    event_loop.run_app(&mut editor_app)?;
    Ok(())
}

struct EditorApp {
    // Core components
    window: Option<Arc<Window>>,
    renderer: Option<EditorRenderer>,
    ui: EditorUI,
    
    // egui integration
    egui_winit: Option<egui_winit::State>,
    
    // Game state that we're editing
    game_state: Option<GameState>,
    
    // Timing
    timing: TimingSystem,
    last_frame: Instant,
}

impl EditorApp {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            ui: EditorUI::new(),
            egui_winit: None,
            game_state: None,
            timing: TimingSystem::new(),
            last_frame: Instant::now(),
        }
    }
}

impl ApplicationHandler for EditorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window
        let window_attributes = winit::window::Window::default_attributes()
            .with_title("ToKi Editor")
            .with_inner_size(winit::dpi::PhysicalSize::new(1200, 800));
            
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        
        // Initialize renderer (async, but we block here since we're in resumed)
        let renderer = pollster::block_on(EditorRenderer::new(window.clone()))
            .expect("Failed to initialize renderer");
        
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
        
        // Initialize game state
        let mut game_state = GameState::new_empty();
        let _player_id = game_state.spawn_player_at(glam::IVec2::new(80, 72));
        let _npc_id = game_state.spawn_player_like_npc(glam::IVec2::new(120, 72));
        self.game_state = Some(game_state);
        
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
            let _ = egui_winit.on_window_event(self.window.as_ref().unwrap(), &event);
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
                self.render();
            }
            
            _ => {}
        }
    }
}

impl EditorApp {
    fn render(&mut self) {
        let (window, renderer) = match (&self.window, &mut self.renderer) {
            (Some(w), Some(r)) => (w, r),
            _ => return, // Not initialized yet
        };
        
        // For now, just render with empty egui output (no UI yet)
        let empty_output = egui::FullOutput {
            platform_output: egui::PlatformOutput::default(),
            textures_delta: egui::TexturesDelta::default(),
            shapes: Vec::new(),
            pixels_per_point: window.scale_factor() as f32,
            viewport_output: std::collections::HashMap::default(),
        };
        
        if let Err(e) = renderer.render(window, empty_output) {
            tracing::error!("Render error: {e}");
        }
        
        // Request next frame
        window.request_redraw();
    }
}