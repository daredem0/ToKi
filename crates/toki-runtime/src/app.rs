//! Simple winit window example.
// winit imports
use winit::application::ApplicationHandler; // Trait that defines app lifecycle hooks (resumed, event handling, etc.)
use winit::dpi::LogicalSize;
use winit::event::WindowEvent; // Enum of possible window-related events (resize, input, close, etc.)
use winit::event_loop::{ActiveEventLoop, EventLoop}; // ActiveEventLoop is used inside lifecycle methods; EventLoop creates and runs the app
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId}; // Window: window handle; Attributes: window config; ID: unique per window

use std::sync::Arc;
use std::time::{Duration, Instant};

use toki_core::camera::{Camera, CameraController, CameraMode, Entity};
use toki_core::math::projection::{calculate_projection, ProjectionParameter};
use toki_core::sprite::{Animation, Frame, SpriteInstance, SpriteSheetMeta};
use toki_core::GameState;
use toki_render::GpuState;
use toki_render::RenderError;

use crate::systems::{CameraSystem, GameSystem, PerformanceMonitor, ResourceManager};

#[derive(Debug)]
struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
    last_update: Instant,
    accumulator: Duration,
    game_system: GameSystem,
    projection_params: ProjectionParameter,
    resources: ResourceManager,
    camera_system: CameraSystem,
    // Performance monitoring
    performance: PerformanceMonitor,
}


impl App {
    fn new() -> Self {
        let resources = ResourceManager::load_all().expect("Failed to load resources");
        let animation = Animation {
            name: "slime_bounce".into(),
            looped: true,
            frames: vec![
                Frame {
                    index: 0,
                    duration_ms: 150,
                },
                Frame {
                    index: 1,
                    duration_ms: 150,
                },
                Frame {
                    index: 2,
                    duration_ms: 150,
                },
                Frame {
                    index: 3,
                    duration_ms: 150,
                },
            ],
        };
        let sprite_sheet = SpriteSheetMeta {
            frame_size: (
                resources.creature_tile_size().x,
                resources.creature_tile_size().y,
            ),
            frame_count: 4,
            sheet_size: (
                resources
                    .creature_image_size()
                    .expect("Cannot derive image size")
                    .x,
                resources
                    .creature_image_size()
                    .expect("Cannot derive image size")
                    .y,
            ),
        };
        let sprite_instance =
            SpriteInstance::new(glam::Vec2::new(80.0, 72.0), animation, sprite_sheet);
        let game_state = GameState::new(sprite_instance);
        let game_system = GameSystem::new(game_state);
        
        let mut camera = Camera {
            position: glam::IVec2::ZERO,
            viewport_size: glam::UVec2::new(160, 144),
            scale: 1,
        };
        camera.center_on(glam::Vec2::new(80.0, 72.0).as_ivec2());
        let slime_entity = Entity {
            id: 1,
            position: glam::vec2(80.0, 72.0),
        };
        let cam_controller = CameraController {
            mode: CameraMode::FollowEntity(slime_entity.id),
        };
        let camera_system = CameraSystem::new(camera, cam_controller);
        // let runtime = RuntimeState {
        //     entities: &[slime_entity],
        // };

        Self {
            window: None,
            gpu: None,
            last_update: Instant::now(),
            accumulator: Duration::ZERO,
            game_system,
            projection_params: ProjectionParameter {
                width: 160,
                height: 144,
                desired_width: 160,
                desired_height: 144,
            },
            resources,
            camera_system,
            // Performance monitoring
            performance: PerformanceMonitor::new(),
        }
    }

    fn tick(&mut self) {
        let tick_start = std::time::Instant::now();
        tracing::trace!("TICK @ {:?}", tick_start);

        // Update game state (handles input, animation, etc.)
        let world_bounds = glam::Vec2::new(
            (self.resources.tilemap_size().x * self.resources.tilemap_tile_size().x) as f32,
            (self.resources.tilemap_size().y * self.resources.tilemap_tile_size().y) as f32,
        );
        let player_moved = self.game_system.update(world_bounds);
        
        // Update camera based on game state
        let runtime = self.game_system.create_runtime_state();
        let world_size = glam::UVec2::new(world_bounds.x as u32, world_bounds.y as u32);
        let cam_changed = self.camera_system.update(&runtime, world_size) || player_moved;

        if let Some(gpu) = &mut self.gpu {
            if cam_changed {
                let projection = calculate_projection(self.projection_params);
                let view = self.camera_system.view_matrix();
                gpu.update_projection(projection * view);

                // Only update tilemap if visible chunks changed
                if self.camera_system.update_chunk_cache(self.resources.get_tilemap()) {
                    let atlas_size = self.resources.terrain_image_size().unwrap();
                    let verts = self.resources.get_tilemap().generate_vertices_for_chunks(
                        self.resources.get_terrain_atlas(),
                        atlas_size,
                        self.camera_system.cached_visible_chunks(),
                    );

                    gpu.update_tilemap_vertices(&verts);
                }
            }
            let frame = self.game_system.current_sprite_frame();
            gpu.clear_sprites(); // Clear previous frame's sprites
            gpu.add_sprite(frame, self.game_system.player_position(), glam::Vec2::new(16.0, 16.0));
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }


    fn handle_keyboard_input_event(&mut self, event: winit::event::KeyEvent) {
        use winit::event::ElementState;
        if let PhysicalKey::Code(keycode) = event.physical_key {
            match event.state {
                ElementState::Pressed => {
                    // Handle special keys that trigger on press
                    match keycode {
                        KeyCode::F3 => {
                            self.performance.toggle_display();
                        }
                        _ => {
                            // Delegate game input to GameSystem
                            self.game_system.handle_keyboard_input(keycode, true);
                        }
                    }
                }
                ElementState::Released => {
                    // Delegate game input to GameSystem
                    self.game_system.handle_keyboard_input(keycode, false);
                }
            }
        }
    }

    fn handle_resize_event(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // Get the window from self.window
        let window = self.window.as_ref().expect("resize event without a window");
        let size = window.inner_size();
        self.projection_params.height = size.height;
        self.projection_params.width = size.width;
        let projection = calculate_projection(self.projection_params);
        let view = self.camera_system.view_matrix();
        if let Some(gpu) = &mut self.gpu {
            gpu.resize(new_size);
            gpu.update_projection(projection * view);
        }
        window.request_redraw();
    }

    fn handle_redraw_request_event(&mut self) {
        let frame_start = Instant::now();
        
        // Record frame interval timing
        self.performance.record_frame_interval(frame_start);

        // Redraw the application.
        //
        // It's preferable for applications that do not render continuously to render in
        // this event rather than in AboutToWait, since rendering in here allows
        // the program to gracefully handle redraws requested by the OS.

        // Get the window from self.window
        let window = self
            .window
            .as_ref()
            .expect("redraw request without a window");

        // Notify that you're about to draw.
        // This is necessary for some platforms (like X11) to ensure that the window is
        // ready to be drawn to.
        window.pre_present_notify();

        // Wayland needs something to actually be drawn to even show the window
        // so were just filling it up for now.
        //fill::fill_window(window);
        if let Some(gpu) = &mut self.gpu {
            let size = self
                .window
                .as_ref()
                .expect("redraw request without a window")
                .inner_size();
            self.projection_params.height = size.height;
            self.projection_params.width = size.width;
            // let projection = calculate_projection(self.projection_params);
            // let model = glam::Mat4::from_translation(self.sprite.position.extend(0.0));

            // let mvp = projection * model;

            // gpu.update_projection(mvp);
            let left = self.camera_system.position().x;
            let top = self.camera_system.position().y;
            let right = left + self.camera_system.viewport_size().x as i32;
            let bottom = top + self.camera_system.viewport_size().y as i32;

            tracing::trace!(
                "Camera Viewport in world space: left={}, right={}, top={}, bottom={}",
                left,
                right,
                top,
                bottom
            );
            tracing::trace!("Camera position: {:?}", self.camera_system.position());
            tracing::trace!(
                "Window size: {:?}",
                self.window.as_ref().unwrap().inner_size()
            );
            tracing::trace!(
                "Camera projection: {:?}",
                self.camera_system.projection_matrix()
            );
            tracing::trace!("Window Scale Factor: {:?}", window.scale_factor());

            // Also draw the map
            // let atlas_size = self.assets.terrain_atlas.image_size().unwrap();
            // let verts = self
            //     .assets
            //     .tilemap
            //     .generate_vertices(&self.assets.terrain_atlas, atlas_size);
            // gpu.update_tilemap_vertex_buffer(&verts);
            
            // Measure CPU work time (everything before GPU draw)
            let cpu_work_time = frame_start.elapsed();
            
            // Measure GPU draw time
            let draw_start = Instant::now();
            gpu.draw();
            let draw_time = draw_start.elapsed();
            
            // Record performance breakdown
            let total_frame_time = frame_start.elapsed();
            self.performance.record_performance_breakdown(cpu_work_time, draw_time, total_frame_time);
        }
    }

}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Initialize default window attributes
        let window_attributes =
            WindowAttributes::default().with_inner_size(LogicalSize::new(160.0, 144.0));

        // Attempt to create a window with the given attributes
        // This has to be done before the GPU state is initialized to ensure
        // its lifetime is longer than that of GPU state
        let raw_window = event_loop.create_window(window_attributes).unwrap();
        let window = Arc::new(raw_window);

        // Now we can safely initialize GPU state
        let gpu = GpuState::new(Arc::clone(&window));
        window.request_redraw();
        self.window = Some(window);
        self.gpu = Some(gpu);

        let size = self.window.as_ref().unwrap().inner_size();
        self.projection_params.height = size.height;
        self.projection_params.width = size.width;

        let projection = calculate_projection(self.projection_params);
        let view = self.camera_system.view_matrix();

        // Load initially visible chunks
        if let Some(gpu) = &mut self.gpu {
            // Generate vertices for chunks visible at startup
            self.camera_system.update_chunk_cache(self.resources.get_tilemap());
            let atlas_size = self.resources.terrain_image_size().unwrap();
            let verts = self.resources.get_tilemap().generate_vertices_for_chunks(
                self.resources.get_terrain_atlas(),
                atlas_size,
                self.camera_system.cached_visible_chunks(),
            );
            gpu.update_tilemap_vertices(&verts);

            gpu.update_projection(projection * view);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        const TIMESTEP: Duration = Duration::from_nanos(16_666_667); // ~16.67ms -> 60fps
        let now = Instant::now();
        let dt = now - self.last_update;
        self.last_update = now;

        self.accumulator += dt;
        while self.accumulator >= TIMESTEP {
            let tick_start = Instant::now();
            self.tick();
            let tick_time = tick_start.elapsed();
            self.performance.record_tick_time(tick_time);
            self.accumulator -= TIMESTEP;
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        tracing::trace!("{event:?}");

        match event {
            // Handle keyboard inputs
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input_event(event);
            }

            // If the window was closed, stop the event loop
            WindowEvent::CloseRequested => {
                tracing::info!("Close was requested; stopping");
                event_loop.exit();
            }
            // If the window was resized, request a redraw
            WindowEvent::Resized(new_size) => {
                self.handle_resize_event(new_size);
            }
            // If the window needs to be redrawn, redraw it
            WindowEvent::RedrawRequested => {
                self.handle_redraw_request_event();
            }
            // Ignore all other events
            _ => (),
        }
    }
}
/// Runs a minimal window using the winit library.
pub fn run_minimal_window() -> Result<(), RenderError> {
    let event_loop = EventLoop::new()?;

    // Create an instance of the App struct
    let mut app = App::new();

    // Run the application
    event_loop.run_app(&mut app)?;

    // Return Ok if the application was closed successfully
    Ok(())
}
