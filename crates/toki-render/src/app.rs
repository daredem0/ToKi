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

use crate::errors::RenderError;
use crate::gpu::GpuState;
use toki_core::assets::{atlas::AtlasMeta, tilemap::TileMap};
use toki_core::camera::{Camera, CameraController, CameraMode, Entity, RuntimeState};
use toki_core::math::projection::{calculate_projection, ProjectionParameter};
use toki_core::sprite::{self, Animation, Frame, SpriteInstance, SpriteSheetMeta};

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
}

impl Assets {
    pub fn load() -> Result<Self, RenderError> {
        let terrain_atlas = AtlasMeta::load_from_file("assets/terrain.json")?;
        let creature_atlas = AtlasMeta::load_from_file("assets/creatures.json")?;
        let tilemap = TileMap::load_from_file("assets/maps/test_map.json")?;
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
        }
    }

    fn tick(&mut self) {
        tracing::trace!("TICK @ {:?}", Instant::now());

        // Movement speed in pixels per krey press
        let step = 0.5;
        let sprite_size = 16.0; // your sprite is 16×16 pixels
        let screen_width = 160.0;
        let screen_height = 144.0;
        // Track if we _moved at all this tick
        let mut _moved = false;

        self.handle_input(step, sprite_size, screen_width, screen_height, _moved);
        if true {
            // this point can be used to differentiate between idle and moving animations later
            // Update animation
            self.sprite.tick(17);
            let runtime = RuntimeState {
                entities: &[Entity {
                    id: 1,
                    position: self.sprite.position,
                }],
            };
            self.cam_controller.update(&mut self.camera, &runtime);
            if let Some(gpu) = &mut self.gpu {
                let frame = self.sprite.current_frame();
                gpu.update_vertex_buffer(frame, self.sprite.position);
            }
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn handle_input(
        &mut self,
        step: f32,
        sprite_size: f32,
        screen_width: f32,
        screen_height: f32,
        mut _moved: bool,
    ) {
        for key in &self.keys_held {
            match key {
                KeyCode::KeyW | KeyCode::ArrowUp => {
                    tracing::trace!("Move forward");
                    self.sprite.position.y = (self.sprite.position.y - step).max(0.0);
                    _moved = true;
                }
                KeyCode::KeyA | KeyCode::ArrowLeft => {
                    tracing::trace!("Move left");
                    self.sprite.position.x = (self.sprite.position.x - step).max(0.0);
                    _moved = true;
                }
                KeyCode::KeyS | KeyCode::ArrowDown => {
                    tracing::trace!("Move backward");
                    self.sprite.position.y =
                        (self.sprite.position.y + step).min(screen_height - sprite_size);
                    _moved = true;
                }
                KeyCode::KeyD | KeyCode::ArrowRight => {
                    tracing::trace!("Move right");
                    self.sprite.position.x =
                        (self.sprite.position.x + step).min(screen_width - sprite_size);
                    _moved = true;
                }
                // Ignore all other events
                _ => (),
            }
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
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        const TIMESTEP: Duration = Duration::from_nanos(16_666_667); // ~16.67ms -> 60fps
        let now = Instant::now();
        let dt = now - self.last_update;
        self.last_update = now;

        self.accumulator += dt;
        while self.accumulator >= TIMESTEP {
            self.tick();
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
                use winit::event::ElementState;
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            self.keys_held.insert(keycode);
                        }
                        ElementState::Released => {
                            self.keys_held.remove(&keycode);
                        }
                    }
                }
            }

            // If the window was closed, stop the event loop
            WindowEvent::CloseRequested => {
                tracing::info!("Close was requested; stopping");
                event_loop.exit();
            }
            // If the window was resized, request a redraw
            WindowEvent::Resized(_) => {
                // Get the window from self.window
                let window = self.window.as_ref().expect("resize event without a window");
                let size = window.inner_size();
                self.projection_params.height = size.height;
                self.projection_params.width = size.width;
                let projection = calculate_projection(self.projection_params);
                if let Some(gpu) = &mut self.gpu {
                    gpu.update_projection(projection);
                }
                window.request_redraw();
            }
            // If the window needs to be redrawn, redraw it
            WindowEvent::RedrawRequested => {
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

                    tracing::debug!(
                        "Camera Viewport in world space: left={}, right={}, top={}, bottom={}",
                        left,
                        right,
                        top,
                        bottom
                    );
                    tracing::trace!("Redrawing projection");
                    tracing::debug!("Camera position: {:?}", self.camera.position);
                    tracing::debug!(
                        "Window size: {:?}",
                        self.window.as_ref().unwrap().inner_size()
                    );
                    tracing::debug!(
                        "Camera projection: {:?}",
                        self.camera.calculate_projection()
                    );
                    tracing::debug!("Window Scale Factor: {:?}", window.scale_factor());

                    // Also draw the map
                    let atlas_size = self.assets.terrain_atlas.image_size().unwrap();
                    let verts = self
                        .assets
                        .tilemap
                        .generate_vertices(&self.assets.terrain_atlas, atlas_size);
                    gpu.update_tilemap_vertex_buffer(&verts);
                    gpu.draw(&self.camera);
                }
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
