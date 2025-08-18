//! Simple winit window example.
// winit imports
use winit::application::ApplicationHandler; // Trait that defines app lifecycle hooks (resumed, event handling, etc.)
use winit::dpi::LogicalSize;
use winit::event::WindowEvent; // Enum of possible window-related events (resize, input, close, etc.)
use winit::event_loop::{ActiveEventLoop, EventLoop}; // ActiveEventLoop is used inside lifecycle methods; EventLoop creates and runs the app
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId}; // Window: window handle; Attributes: window config; ID: unique per window

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use toki_core::assets::{atlas::AtlasMeta, tilemap::TileMap};
use toki_core::camera::{Camera, CameraController, CameraMode, Entity, RuntimeState};
use toki_core::math::projection::{calculate_projection, ProjectionParameter};
use toki_core::sprite::{Animation, Frame, SpriteInstance, SpriteSheetMeta};
use toki_render::GpuState;
use toki_render::RenderError;

#[derive(Debug)]
pub struct Assets {
    pub tilemap: TileMap,
    pub terrain_atlas: AtlasMeta,
    pub creature_atlas: AtlasMeta,
}

#[derive(Debug)]
struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
    last_update: Instant,
    accumulator: Duration,
    keys_held: HashSet<KeyCode>,
    sprite: SpriteInstance,
    projection_params: ProjectionParameter,
    pub assets: Assets,
    camera: Camera,
    cam_controller: CameraController,
    cached_visible_chunks: Vec<(u32, u32)>,
    // Performance tracking
    frame_times: Vec<Duration>,
    tick_times: Vec<Duration>,
    draw_times: Vec<Duration>,
    cpu_work_times: Vec<Duration>,
    total_frame_times: Vec<Duration>,
    last_fps_print: Instant,
    last_frame_time: Instant,
    show_fps_stats: bool,
}

impl Assets {
    pub fn load() -> Result<Self, RenderError> {
        let terrain_atlas = AtlasMeta::load_from_file("assets/terrain.json")?;
        let creature_atlas = AtlasMeta::load_from_file("assets/creatures.json")?;
        // let tilemap = TileMap::load_from_file("assets/maps/test_map.json")?;
        let tilemap = TileMap::load_from_file("assets/maps/tilemap_64x64_chunk.json")?;

        tilemap.validate()?;

        Ok(Self {
            tilemap,
            terrain_atlas,
            creature_atlas,
        })
    }
}

impl App {
    fn new() -> Self {
        let assets = Assets::load().expect("Failed to load assets");
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
                assets.creature_atlas.tile_size.x,
                assets.creature_atlas.tile_size.y,
            ),
            frame_count: 4,
            sheet_size: (
                assets
                    .creature_atlas
                    .image_size()
                    .expect("Cannot derive image size")
                    .x,
                assets
                    .creature_atlas
                    .image_size()
                    .expect("Cannot derive image size")
                    .y,
            ),
        };
        let sprite_instance =
            SpriteInstance::new(glam::Vec2::new(80.0, 72.0), animation, sprite_sheet);
        let mut camera = Camera {
            position: glam::IVec2::ZERO,
            viewport_size: glam::UVec2::new(160, 144),
            scale: 1,
        };
        camera.center_on(sprite_instance.position.as_ivec2());
        let slime_entity = Entity {
            id: 1,
            position: glam::vec2(80.0, 72.0),
        };
        let cam_controller = CameraController {
            mode: CameraMode::FollowEntity(slime_entity.id),
        };
        // let runtime = RuntimeState {
        //     entities: &[slime_entity],
        // };

        Self {
            window: None,
            gpu: None,
            last_update: Instant::now(),
            accumulator: Duration::ZERO,
            keys_held: HashSet::new(),
            sprite: sprite_instance,
            projection_params: ProjectionParameter {
                width: 160,
                height: 144,
                desired_width: 160,
                desired_height: 144,
            },
            assets,
            camera,
            cam_controller,
            cached_visible_chunks: Vec::new(),
            // Performance tracking
            frame_times: Vec::new(),
            tick_times: Vec::new(),
            draw_times: Vec::new(),
            cpu_work_times: Vec::new(),
            total_frame_times: Vec::new(),
            last_fps_print: Instant::now(),
            last_frame_time: Instant::now(),
            show_fps_stats: true, // Enable FPS display by default
        }
    }

    fn tick(&mut self) {
        let tick_start = std::time::Instant::now();
        tracing::trace!("TICK @ {:?}", tick_start);

        // Movement speed in pixels per key press
        let step = 1.0; // Move exactly 1 pixel per frame
        let sprite_size = 16.0; // your sprite is 16×16 pixels
        let world_w = (self.assets.tilemap.size.x * self.assets.tilemap.tile_size.x) as f32;
        let world_h = (self.assets.tilemap.size.y * self.assets.tilemap.tile_size.y) as f32;

        let moved = self.handle_input(step, sprite_size, world_w, world_h);
        // this point can be used to differentiate between idle and moving animations later
        // Update animation
        self.sprite.tick(17);
        let prev_cam_pos = self.camera.position;
        let runtime = RuntimeState {
            entities: &[Entity {
                id: 1,
                position: self.sprite.position,
            }],
        };
        self.cam_controller.update(&mut self.camera, &runtime);
        // Clamp camera to world bounds
        let view_w = (self.camera.viewport_size.x * self.camera.scale) as i32;
        let view_h = (self.camera.viewport_size.y * self.camera.scale) as i32;
        let world_w_i = (self.assets.tilemap.size.x * self.assets.tilemap.tile_size.x) as i32;
        let world_h_i = (self.assets.tilemap.size.y * self.assets.tilemap.tile_size.y) as i32;

        let max_cam_x = (world_w_i - view_w).max(0);
        let max_cam_y = (world_h_i - view_h).max(0);

        self.camera.position.x = self.camera.position.x.clamp(0, max_cam_x);
        self.camera.position.y = self.camera.position.y.clamp(0, max_cam_y);

        let cam_changed = prev_cam_pos != self.camera.position || moved;

        if let Some(gpu) = &mut self.gpu {
            if cam_changed {
                let projection = calculate_projection(self.projection_params);
                let view = glam::Mat4::from_translation(glam::vec3(
                    -(self.camera.position.x as f32),
                    -(self.camera.position.y as f32),
                    0.0,
                ));
                gpu.update_projection(projection * view);

                // Only update tilemap if visible chunks changed
                let current_chunks = self.assets.tilemap.visible_chunks(
                    glam::UVec2::new(self.camera.position.x as u32, self.camera.position.y as u32),
                    self.camera.viewport_size,
                );

                if current_chunks != self.cached_visible_chunks {
                    let atlas_size = self.assets.terrain_atlas.image_size().unwrap();
                    let verts = self.assets.tilemap.generate_vertices_for_chunks(
                        &self.assets.terrain_atlas,
                        atlas_size,
                        &current_chunks,
                    );

                    gpu.update_tilemap_vertices(&verts);
                    self.cached_visible_chunks = current_chunks;
                }
            }
            let frame = self.sprite.current_frame();
            gpu.clear_sprites(); // Clear previous frame's sprites
            gpu.add_sprite(frame, self.sprite.position, glam::Vec2::new(16.0, 16.0));
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn handle_input(&mut self, step: f32, sprite_size: f32, world_w: f32, world_h: f32) -> bool {
        let mut moved = false;
        for key in &self.keys_held {
            match key {
                KeyCode::KeyW | KeyCode::ArrowUp => {
                    tracing::trace!("Move forward");
                    self.sprite.position.y = (self.sprite.position.y - step).max(0.0);
                    moved = true;
                }
                KeyCode::KeyA | KeyCode::ArrowLeft => {
                    tracing::trace!("Move left");
                    self.sprite.position.x = (self.sprite.position.x - step).max(0.0);
                    moved = true;
                }
                KeyCode::KeyS | KeyCode::ArrowDown => {
                    tracing::trace!("Move backward");
                    self.sprite.position.y =
                        (self.sprite.position.y + step).min(world_h - sprite_size);
                    moved = true;
                }
                KeyCode::KeyD | KeyCode::ArrowRight => {
                    tracing::trace!("Move right");
                    self.sprite.position.x =
                        (self.sprite.position.x + step).min(world_w - sprite_size);
                    moved = true;
                }
                // Ignore all other events
                _ => (),
            }
        }
        moved
    }

    fn handle_keyboard_input_event(&mut self, event: winit::event::KeyEvent) {
        use winit::event::ElementState;
        if let PhysicalKey::Code(keycode) = event.physical_key {
            match event.state {
                ElementState::Pressed => {
                    // Handle special keys that trigger on press
                    match keycode {
                        KeyCode::F3 => {
                            self.show_fps_stats = !self.show_fps_stats;
                            println!(
                                "FPS stats display: {}",
                                if self.show_fps_stats { "ON" } else { "OFF" }
                            );
                        }
                        _ => {
                            self.keys_held.insert(keycode);
                        }
                    }
                }
                ElementState::Released => {
                    self.keys_held.remove(&keycode);
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
        let view = glam::Mat4::from_translation(glam::vec3(
            -(self.camera.position.x as f32),
            -(self.camera.position.y as f32),
            0.0,
        ));
        if let Some(gpu) = &mut self.gpu {
            gpu.resize(new_size);
            gpu.update_projection(projection * view);
        }
        window.request_redraw();
    }

    fn handle_redraw_request_event(&mut self) {
        let frame_start = Instant::now();
        
        // Measure time between actual frame renders
        let frame_time = frame_start.duration_since(self.last_frame_time);
        self.last_frame_time = frame_start;
        self.update_fps_stats(frame_time);

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
            let left = self.camera.position.x;
            let top = self.camera.position.y;
            let right = left + self.camera.viewport_size.x as i32;
            let bottom = top + self.camera.viewport_size.y as i32;

            tracing::trace!(
                "Camera Viewport in world space: left={}, right={}, top={}, bottom={}",
                left,
                right,
                top,
                bottom
            );
            tracing::trace!("Camera position: {:?}", self.camera.position);
            tracing::trace!(
                "Window size: {:?}",
                self.window.as_ref().unwrap().inner_size()
            );
            tracing::trace!(
                "Camera projection: {:?}",
                self.camera.calculate_projection()
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
            
            // Update performance stats
            let total_frame_time = frame_start.elapsed();
            self.update_performance_stats(cpu_work_time, draw_time, total_frame_time);
        }
    }

    fn update_fps_stats(&mut self, frame_time: Duration) {
        // Add current frame time to our collection
        self.frame_times.push(frame_time);

        // Keep only the last 60 frames for a rolling average
        const MAX_SAMPLES: usize = 60;
        if self.frame_times.len() > MAX_SAMPLES {
            self.frame_times.remove(0);
        }

        // Print FPS stats every second (if enabled)
        let now = Instant::now();
        if self.show_fps_stats && now.duration_since(self.last_fps_print) >= Duration::from_secs(1) {
            self.print_fps_stats();
            self.last_fps_print = now;
        }
    }

    fn update_tick_stats(&mut self, tick_time: Duration) {
        // Add tick time to our collection
        self.tick_times.push(tick_time);

        // Keep only the last 60 samples
        const MAX_SAMPLES: usize = 60;
        if self.tick_times.len() > MAX_SAMPLES {
            self.tick_times.remove(0);
        }
    }

    fn update_performance_stats(&mut self, cpu_work_time: Duration, draw_time: Duration, total_frame_time: Duration) {
        // Add performance times to our collections
        self.cpu_work_times.push(cpu_work_time);
        self.draw_times.push(draw_time);
        self.total_frame_times.push(total_frame_time);

        // Keep only the last 60 samples for each
        const MAX_SAMPLES: usize = 60;
        
        if self.cpu_work_times.len() > MAX_SAMPLES {
            self.cpu_work_times.remove(0);
        }
        if self.draw_times.len() > MAX_SAMPLES {
            self.draw_times.remove(0);
        }
        if self.total_frame_times.len() > MAX_SAMPLES {
            self.total_frame_times.remove(0);
        }
    }

    fn print_fps_stats(&self) {
        if self.frame_times.is_empty() {
            return;
        }

        // Calculate FPS from frame intervals
        let total_time: Duration = self.frame_times.iter().sum();
        let avg_frame_time = total_time / self.frame_times.len() as u32;
        let fps = if avg_frame_time.as_nanos() > 0 {
            1_000_000_000.0 / avg_frame_time.as_nanos() as f64
        } else {
            0.0
        };

        // Calculate average tick time (game logic)
        let avg_tick_time = if !self.tick_times.is_empty() {
            let total: Duration = self.tick_times.iter().sum();
            (total / self.tick_times.len() as u32).as_secs_f64() * 1000.0
        } else {
            0.0
        };

        // Calculate average draw time (GPU rendering)
        let avg_draw_time = if !self.draw_times.is_empty() {
            let total: Duration = self.draw_times.iter().sum();
            (total / self.draw_times.len() as u32).as_secs_f64() * 1000.0
        } else {
            0.0
        };

        // Calculate average CPU work time (frame preparation)
        let avg_cpu_time = if !self.cpu_work_times.is_empty() {
            let total: Duration = self.cpu_work_times.iter().sum();
            (total / self.cpu_work_times.len() as u32).as_secs_f64() * 1000.0
        } else {
            0.0
        };

        // Calculate average total frame time
        let avg_total_frame = if !self.total_frame_times.is_empty() {
            let total: Duration = self.total_frame_times.iter().sum();
            (total / self.total_frame_times.len() as u32).as_secs_f64() * 1000.0
        } else {
            0.0
        };

        // Calculate overhead (total - cpu - draw)
        let overhead = avg_total_frame - avg_cpu_time - avg_draw_time;

        // Print comprehensive performance breakdown
        println!(
            "FPS: {:.1} | Frame: {:.2}ms | Tick: {:.2}ms | Draw: {:.2}ms | CPU: {:.2}ms | Overhead: {:.2}ms",
            fps,
            avg_total_frame,
            avg_tick_time,
            avg_draw_time,
            avg_cpu_time,
            overhead.max(0.0) // Don't show negative overhead
        );
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
        let view = glam::Mat4::from_translation(glam::vec3(
            -(self.camera.position.x as f32),
            -(self.camera.position.y as f32),
            0.0,
        ));

        // Load initially visible chunks
        if let Some(gpu) = &mut self.gpu {
            // Generate vertices for chunks visible at startup
            let initial_chunks = self.assets.tilemap.visible_chunks(
                glam::UVec2::new(self.camera.position.x as u32, self.camera.position.y as u32),
                self.camera.viewport_size,
            );
            let atlas_size = self.assets.terrain_atlas.image_size().unwrap();
            let verts = self.assets.tilemap.generate_vertices_for_chunks(
                &self.assets.terrain_atlas,
                atlas_size,
                &initial_chunks,
            );
            gpu.update_tilemap_vertices(&verts);
            self.cached_visible_chunks = initial_chunks; // Cache the initial chunks

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
            self.update_tick_stats(tick_time);
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
