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

pub fn run_editor() -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut editor_app = EditorApp::new();
    event_loop.run_app(&mut editor_app)?;
    Ok(())
}

struct EditorApp {
    // Window and graphics state
    window: Option<Arc<Window>>,
    
    // egui integration
    egui_winit: Option<egui_winit::State>,
    egui_wgpu: Option<egui_wgpu::Renderer>,
    wgpu_device: Option<wgpu::Device>,
    wgpu_queue: Option<wgpu::Queue>,
    wgpu_surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    
    // Game state that we'll be editing
    game_state: Option<GameState>,
    
    // Editor UI state
    selected_entity_id: Option<EntityId>,
    show_hierarchy: bool,
    show_inspector: bool,
    
    // Timing for the game loop
    timing: TimingSystem,
    last_frame: Instant,
}

impl EditorApp {
    fn new() -> Self {
        Self {
            window: None,
            egui_winit: None,
            egui_wgpu: None,
            wgpu_device: None,
            wgpu_queue: None,
            wgpu_surface: None,
            surface_config: None,
            game_state: None,
            selected_entity_id: None,
            show_hierarchy: true,
            show_inspector: true,
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
        
        // Initialize wgpu
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            ..Default::default()
        });
        
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();
        
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })).unwrap();
        
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor::default(),
        )).unwrap();
        
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        
        let size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);
        
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
        
        let egui_wgpu = egui_wgpu::Renderer::new(&device, surface_format, None, 1, false);
        
        // Store everything
        self.window = Some(window.clone());
        self.wgpu_device = Some(device);
        self.wgpu_queue = Some(queue);
        self.wgpu_surface = Some(surface);
        self.surface_config = Some(surface_config);
        self.egui_winit = Some(egui_winit);
        self.egui_wgpu = Some(egui_wgpu);
        
        // Initialize game state when app starts
        let mut game_state = GameState::new_empty();
        let _player_id = game_state.spawn_player_at(glam::IVec2::new(80, 72));
        let _npc_id = game_state.spawn_player_like_npc(glam::IVec2::new(120, 72));
        
        self.game_state = Some(game_state);
        
        tracing::info!("Editor started with window and game state initialized");
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
                                self.show_hierarchy = !self.show_hierarchy;
                                tracing::info!("Toggled hierarchy panel: {}", self.show_hierarchy);
                            }
                            KeyCode::F2 => {
                                self.show_inspector = !self.show_inspector;
                                tracing::info!("Toggled inspector panel: {}", self.show_inspector);
                            }
                            _ => {}
                        }
                    }
                }
            }
            
            WindowEvent::Resized(new_size) => {
                if let (Some(surface), Some(device), Some(surface_config)) = 
                    (&self.wgpu_surface, &self.wgpu_device, &mut self.surface_config) {
                    surface_config.width = new_size.width.max(1);
                    surface_config.height = new_size.height.max(1);
                    surface.configure(device, surface_config);
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
        // Just render a basic colored surface to show the window is working
        if self.wgpu_surface.is_none() || self.wgpu_device.is_none() {
            return;
        }
        
        let surface = self.wgpu_surface.as_ref().unwrap();
        let surface_texture = match surface.get_current_texture() {
            Ok(texture) => texture,
            Err(_) => return,
        };
        
        let surface_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let device = self.wgpu_device.as_ref().unwrap();
        let queue = self.wgpu_queue.as_ref().unwrap();
        
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Simple Render Encoder"),
        });
        
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.2,
                            g: 0.3,
                            b: 0.4,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        
        queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        
        // Request next frame
        self.window.as_ref().unwrap().request_redraw();
    }
    
    fn draw_ui(&mut self, ctx: &egui::Context) {
        // Top menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project").clicked() {
                        tracing::info!("New Project clicked");
                    }
                    if ui.button("Open Project").clicked() {
                        tracing::info!("Open Project clicked");
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        tracing::info!("Exit clicked");
                        std::process::exit(0);
                    }
                });
                
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_hierarchy, "Hierarchy");
                    ui.checkbox(&mut self.show_inspector, "Inspector");
                });
            });
        });
        
        // Left side panel - Hierarchy
        if self.show_hierarchy {
            egui::SidePanel::left("hierarchy_panel")
                .default_width(250.0)
                .show(ctx, |ui| {
                    ui.heading("Hierarchy");
                    ui.separator();
                    
                    if let Some(game_state) = &self.game_state {
                        let entity_manager = game_state.entity_manager();
                        let active_entities = entity_manager.active_entities();
                        
                        for entity_id in active_entities {
                            if let Some(entity) = entity_manager.get_entity(entity_id) {
                                let entity_name = format!("{:?} (ID: {})", entity.entity_type, entity_id);
                                
                                let response = ui.selectable_label(
                                    self.selected_entity_id == Some(entity_id),
                                    entity_name
                                );
                                
                                if response.clicked() {
                                    self.selected_entity_id = Some(entity_id);
                                    tracing::info!("Selected entity: {:?}", entity_id);
                                }
                            }
                        }
                    } else {
                        ui.label("No game state loaded");
                    }
                });
        }
        
        // Right side panel - Inspector  
        if self.show_inspector {
            egui::SidePanel::right("inspector_panel")
                .default_width(300.0)
                .show(ctx, |ui| {
                    ui.heading("Inspector");
                    ui.separator();
                    
                    if let Some(selected_id) = self.selected_entity_id {
                        if let Some(game_state) = &self.game_state {
                            if let Some(entity) = game_state.entity_manager().get_entity(selected_id) {
                                ui.label(format!("Entity ID: {}", selected_id));
                                ui.label(format!("Type: {:?}", entity.entity_type));
                                ui.label(format!("Position: {:?}", entity.position));
                                ui.label(format!("Size: {:?}", entity.size));
                                ui.label(format!("Visible: {}", entity.attributes.visible));
                                
                                if let Some(collision_box) = &entity.collision_box {
                                    ui.separator();
                                    ui.label("Collision Box:");
                                    ui.label(format!("  Offset: {:?}", collision_box.offset));
                                    ui.label(format!("  Size: {:?}", collision_box.size));
                                }
                                
                                if entity.attributes.animation_controller.is_some() {
                                    ui.separator();
                                    ui.label("Has Animation Controller");
                                }
                            }
                        }
                    } else {
                        ui.label("No entity selected");
                        ui.label("Click an entity in the Hierarchy to inspect it");
                    }
                });
        }
        
        // Central panel - Game viewport (placeholder for now)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Game Viewport");
            ui.separator();
            
            let available_size = ui.available_size();
            ui.allocate_response(available_size, egui::Sense::click())
                .on_hover_text("Game will render here");
                
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.label("📊 Stats:");
                    if let Some(game_state) = &self.game_state {
                        ui.label(format!("Entities: {}", game_state.entity_manager().active_entities().len()));
                    }
                    ui.label("Press F1/F2 to toggle panels");
                });
            });
        });
    }
}