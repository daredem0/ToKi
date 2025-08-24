//! Simple winit window example.
// winit imports
use winit::application::ApplicationHandler; // Trait that defines app lifecycle hooks (resumed, event handling, etc.)
use winit::event::WindowEvent; // Enum of possible window-related events (resize, input, close, etc.)
use winit::event_loop::{ActiveEventLoop, EventLoop}; // ActiveEventLoop is used inside lifecycle methods; EventLoop creates and runs the app
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowId;

use std::time::Instant;

use toki_core::camera::{Camera, CameraController, CameraMode, RuntimeState};
use toki_core::{GameState, TimingSystem};
use toki_render::RenderError;

use crate::systems::{
    CameraSystem, GameSystem, PerformanceMonitor, PlatformSystem, RenderingSystem, ResourceManager,
};

#[derive(Debug)]
struct App {
    // Core systems
    game_system: GameSystem,
    camera_system: CameraSystem,
    resources: ResourceManager,
    performance: PerformanceMonitor,

    // Grouped systems
    platform: PlatformSystem,
    rendering: RenderingSystem,
    timing: TimingSystem,
}

impl App {
    fn new() -> Self {
        let resources = ResourceManager::load_all().expect("Failed to load resources");

        let mut game_state = GameState::new_empty();
        let _player_id = game_state.spawn_player_at(glam::IVec2::new(80, 72));
        let game_system = GameSystem::new(game_state);

        let mut camera = Camera {
            position: glam::IVec2::ZERO,
            viewport_size: glam::UVec2::new(160, 144),
            scale: 1,
        };
        camera.center_on(glam::IVec2::new(80, 72));

        // Use the player entity ID from the GameState for camera following
        let player_id = game_system.player_id().expect("Player should exist");
        let cam_controller = CameraController {
            mode: CameraMode::FollowEntity(player_id),
        };
        let camera_system = CameraSystem::new(camera, cam_controller);

        Self {
            // Core systems
            game_system,
            camera_system,
            resources,
            performance: PerformanceMonitor::new(),

            // Grouped systems
            platform: PlatformSystem::new(),
            rendering: RenderingSystem::new(),
            timing: TimingSystem::new(),
        }
    }

    fn tick(&mut self) {
        let tick_start = std::time::Instant::now();
        tracing::trace!("TICK @ {:?}", tick_start);

        // Update game state (handles input, animation, etc.)
        let world_bounds = glam::UVec2::new(
            self.resources.tilemap_size().x * self.resources.tilemap_tile_size().x,
            self.resources.tilemap_size().y * self.resources.tilemap_tile_size().y,
        );
        let player_moved = self.game_system.update(
            world_bounds,
            self.resources.get_tilemap(),
            self.resources.get_terrain_atlas(),
        );

        // Update camera based on game state
        let entities = self.game_system.entities_for_camera();
        let runtime = RuntimeState {
            entities: &entities,
        };
        let world_size = world_bounds;
        let cam_changed = self.camera_system.update(&runtime, world_size) || player_moved;

        if self.rendering.has_gpu() {
            if cam_changed {
                let view = self.camera_system.view_matrix();
                self.rendering.update_projection(view);

                // Only update tilemap if visible chunks changed
                if self
                    .camera_system
                    .update_chunk_cache(self.resources.get_tilemap())
                {
                    let atlas_size = self.resources.terrain_image_size().unwrap();
                    let verts = self.resources.get_tilemap().generate_vertices_for_chunks(
                        self.resources.get_terrain_atlas(),
                        atlas_size,
                        self.camera_system.cached_visible_chunks(),
                    );

                    if let Some(gpu) = self.rendering.gpu_mut() {
                        gpu.update_tilemap_vertices(&verts);
                    }
                }
            }
            let creature_atlas = self.resources.get_creature_atlas();
            let texture_size = creature_atlas.image_size().unwrap_or(glam::UVec2::new(64, 16)); // fallback
            let frame = self.game_system.current_sprite_frame(creature_atlas, texture_size);
            if let Some(gpu) = self.rendering.gpu_mut() {
                gpu.clear_sprites(); // Clear previous frame's sprites
                gpu.add_sprite(
                    frame,
                    self.game_system.player_position(),
                    glam::UVec2::new(16, 16),
                );

                // Add debug collision rendering
                gpu.clear_debug_shapes();
                if self.game_system.is_debug_collision_rendering_enabled() {
                    // Get debug data from game system
                    let entity_boxes = self.game_system.get_entity_collision_boxes();
                    let solid_tiles = self.game_system.get_solid_tile_positions(
                        self.resources.get_tilemap(),
                        self.resources.get_terrain_atlas(),
                    );
                    let trigger_tiles = self.game_system.get_trigger_tile_positions(
                        self.resources.get_tilemap(),
                        self.resources.get_terrain_atlas(),
                    );

                    // Define colors
                    let entity_color = [1.0, 0.0, 0.0, 0.8]; // Red for entity collision boxes
                    let solid_tile_color = [0.0, 0.0, 1.0, 0.6]; // Blue for solid tiles
                    let trigger_tile_color = [1.0, 1.0, 0.0, 0.6]; // Yellow for trigger tiles

                    // Add entity collision boxes
                    for (pos, size, is_trigger) in entity_boxes {
                        let color = if is_trigger { trigger_tile_color } else { entity_color };
                        gpu.add_debug_rect(
                            pos.x as f32,
                            pos.y as f32,
                            size.x as f32,
                            size.y as f32,
                            color,
                        );
                    }

                    // Add solid tile debug boxes
                    let tilemap = self.resources.get_tilemap();
                    for (tile_x, tile_y) in solid_tiles {
                        let world_x = tile_x * tilemap.tile_size.x;
                        let world_y = tile_y * tilemap.tile_size.y;
                        gpu.add_debug_rect(
                            world_x as f32,
                            world_y as f32,
                            tilemap.tile_size.x as f32,
                            tilemap.tile_size.y as f32,
                            solid_tile_color,
                        );
                    }

                    // Add trigger tile debug boxes
                    for (tile_x, tile_y) in trigger_tiles {
                        let world_x = tile_x * tilemap.tile_size.x;
                        let world_y = tile_y * tilemap.tile_size.y;
                        gpu.add_debug_rect(
                            world_x as f32,
                            world_y as f32,
                            tilemap.tile_size.x as f32,
                            tilemap.tile_size.y as f32,
                            trigger_tile_color,
                        );
                    }
                }
                
                // Finalize debug shapes
                gpu.finalize_debug_shapes();
            }
        }

        self.platform.request_redraw();
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
        // Update rendering system with new size
        self.rendering.resize(new_size);

        // Update projection with current view
        let view = self.camera_system.view_matrix();
        self.rendering.update_projection(view);

        self.platform.request_redraw();
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

        // Notify that you're about to draw.
        // This is necessary for some platforms (like X11) to ensure that the window is
        // ready to be drawn to.
        self.platform.pre_present_notify();

        // Wayland needs something to actually be drawn to even show the window
        // so were just filling it up for now.
        //fill::fill_window(window);
        if self.rendering.has_gpu() {
            if let Some(size) = self.platform.inner_size() {
                self.rendering.update_window_size(size);
            }
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
            tracing::trace!("Window size: {:?}", self.platform.inner_size());
            tracing::trace!(
                "Camera projection: {:?}",
                self.camera_system.projection_matrix()
            );
            tracing::trace!("Window Scale Factor: {:?}", self.platform.scale_factor());

            // Measure CPU work time (everything before GPU draw)
            let cpu_work_time = frame_start.elapsed();

            // Measure GPU draw time
            let draw_start = Instant::now();
            self.rendering.draw();
            let draw_time = draw_start.elapsed();

            // Record performance breakdown
            let total_frame_time = frame_start.elapsed();
            self.performance.record_performance_breakdown(
                cpu_work_time,
                draw_time,
                total_frame_time,
            );
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Initialize platform system (window)
        self.platform.initialize_window(event_loop);

        // Initialize rendering system (GPU)
        if let Some(window) = self.platform.window_for_gpu() {
            self.rendering.initialize_gpu(window);
        }

        // Update rendering size
        if let Some(size) = self.platform.inner_size() {
            self.rendering.update_window_size(size);
        }

        // Set up initial projection
        let view = self.camera_system.view_matrix();
        self.rendering.update_projection(view);

        self.platform.request_redraw();

        // Load initially visible chunks
        if self.rendering.has_gpu() {
            // Generate vertices for chunks visible at startup
            self.camera_system
                .update_chunk_cache(self.resources.get_tilemap());
            let atlas_size = self.resources.terrain_image_size().unwrap();
            let verts = self.resources.get_tilemap().generate_vertices_for_chunks(
                self.resources.get_terrain_atlas(),
                atlas_size,
                self.camera_system.cached_visible_chunks(),
            );
            if let Some(gpu) = self.rendering.gpu_mut() {
                gpu.update_tilemap_vertices(&verts);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Process timing updates manually to avoid borrowing issues
        let mut tick_count = 0;
        while self.timing.should_tick() {
            let tick_start = Instant::now();
            self.tick();
            let tick_time = tick_start.elapsed();
            self.performance.record_tick_time(tick_time);
            self.timing.consume_timestep();
            tick_count += 1;
            // Safety valve to prevent infinite loops
            if tick_count > 10 {
                break;
            }
        }
        self.platform.request_redraw();
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
