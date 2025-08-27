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
        
        let egui_winit = match &mut self.egui_winit {
            Some(egui) => egui,
            None => return,
        };
        
        // Prepare egui input
        let raw_input = egui_winit.take_egui_input(window);
        
        // Run egui UI
        let egui_ctx = egui_winit.egui_ctx().clone();
        let full_output = egui_ctx.run(raw_input, |ctx| {
            // Render all UI components
            self.ui.render(ctx, self.game_state.as_ref());
        });
        
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