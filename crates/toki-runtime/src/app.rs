//! Simple winit window example.
// winit imports
use winit::application::ApplicationHandler; // Trait that defines app lifecycle hooks (resumed, event handling, etc.)
use winit::event::WindowEvent; // Enum of possible window-related events (resize, input, close, etc.)
use winit::event_loop::{ActiveEventLoop, EventLoop}; // ActiveEventLoop is used inside lifecycle methods; EventLoop creates and runs the app
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowId;

use std::path::PathBuf;
use std::time::Instant;

use toki_core::camera::{Camera, CameraController, CameraMode, RuntimeState};
use toki_core::serialization::{load_game, save_game};
use toki_core::text::{TextAnchor, TextItem, TextStyle, TextWeight};
use toki_core::{EventHandler, TimingSystem};
use toki_render::RenderError;

use crate::systems::AudioManager;
use crate::systems::{
    CameraManager, DecodedProjectCache, GameManager, PerformanceMonitor, PlatformSystem,
    RenderingSystem, ResourceManager, RuntimeAssetLoadPlan,
};

const COMMUNITY_SPLASH_MIN_DURATION_MS: u64 = 3000;
const COMMUNITY_SPLASH_MAX_DURATION_MS: u64 = 10000;
const COMMUNITY_SPLASH_DEFAULT_DURATION_MS: u64 = 3000;
const COMMUNITY_SPLASH_BRANDING_TEXT: &str = "Powered by ToKi";
const COMMUNITY_SPLASH_VERSION_TEXT: &str = env!("TOKI_VERSION");
const SPLASH_LOGO_WIDTH: u32 = 128;
const SPLASH_LOGO_HEIGHT: u32 = 108;
const SPLASH_TEXT_LINE_HEIGHT_MULTIPLIER: f32 = 1.25;
const SPLASH_BRANDING_VERSION_GAP_PX: f32 = 4.0;
const SPLASH_VERSION_DEFAULT_SIZE_PX: f32 = 11.0;
const SPLASH_VERSION_MIN_SIZE_PX: f32 = 7.0;
const SPLASH_TEXT_HORIZONTAL_PADDING_PX: f32 = 8.0;
const COMMUNITY_SPLASH_LOGO_PNG: &[u8] = include_bytes!("../../../assets/TokiLogo.png");

#[path = "app_bootstrap.rs"]
mod app_bootstrap;
#[path = "app_splash.rs"]
mod app_splash;

use app_bootstrap::first_existing_path;
use app_splash::{ResolvedSplashConfig, SplashPolicy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSplashOptions {
    pub duration_ms: u64,
    pub show_branding: bool,
}

impl Default for RuntimeSplashOptions {
    fn default() -> Self {
        Self {
            duration_ms: COMMUNITY_SPLASH_DEFAULT_DURATION_MS,
            show_branding: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAudioMixOptions {
    pub master_percent: u8,
    pub music_percent: u8,
    pub movement_percent: u8,
    pub collision_percent: u8,
}

impl Default for RuntimeAudioMixOptions {
    fn default() -> Self {
        Self {
            master_percent: 100,
            music_percent: 100,
            movement_percent: 100,
            collision_percent: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeDisplayOptions {
    pub show_entity_health_bars: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeLaunchOptions {
    pub project_path: Option<PathBuf>,
    pub pack_path: Option<PathBuf>,
    pub scene_name: Option<String>,
    pub map_name: Option<String>,
    pub splash: RuntimeSplashOptions,
    pub audio_mix: RuntimeAudioMixOptions,
    pub display: RuntimeDisplayOptions,
}

#[derive(Debug)]
struct App {
    // Core systems
    game_system: GameManager,
    camera_system: CameraManager,
    resources: ResourceManager,
    performance: PerformanceMonitor,
    audio_system: AudioManager,

    // Grouped systems
    platform: PlatformSystem,
    rendering: RenderingSystem,
    timing: TimingSystem,
    launch_options: RuntimeLaunchOptions,
    splash_policy: SplashPolicy,
    splash_config: ResolvedSplashConfig,
    splash_active: bool,
    splash_started_at: Option<Instant>,
    splash_logo_loaded: bool,
    post_splash_sprite_texture_path: Option<PathBuf>,
    asset_load_plan: RuntimeAssetLoadPlan,
    #[allow(dead_code)]
    decoded_project_cache: DecodedProjectCache,
    #[allow(dead_code)]
    pack_mount: Option<tempfile::TempDir>,
}

impl App {
    fn content_root_path(&self) -> Option<&std::path::Path> {
        self.pack_mount
            .as_ref()
            .map(tempfile::TempDir::path)
            .or(self.launch_options.project_path.as_deref())
    }

    fn new(launch_options: RuntimeLaunchOptions) -> Self {
        let splash_policy = SplashPolicy::Community;
        let splash_config = splash_policy.resolve(&launch_options.splash);
        let (resources, game_state, pack_mount, asset_load_plan, decoded_project_cache) =
            Self::build_startup_state(&launch_options);
        let game_system = GameManager::new(game_state);

        let mut camera = Camera {
            position: glam::IVec2::ZERO,
            viewport_size: glam::UVec2::new(160, 144),
            scale: 1,
        };
        camera.center_on(glam::IVec2::new(80, 72));

        let cam_controller = if let Some(player_id) = game_system.player_id() {
            CameraController {
                mode: CameraMode::FollowEntity(player_id),
            }
        } else {
            CameraController {
                mode: CameraMode::FreeScroll,
            }
        };
        let camera_system = CameraManager::new(camera, cam_controller);
        let audio_root = pack_mount
            .as_ref()
            .map(tempfile::TempDir::path)
            .or(launch_options.project_path.as_deref())
            .map(std::path::Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .expect("Failed to resolve audio root path");
        let mut audio_system = AudioManager::new_with_assets_root_and_preload_names(
            audio_root,
            &asset_load_plan.preloaded_sfx_names,
        )
        .expect("Failed to initialize audio system");
        audio_system.set_master_volume_percent(launch_options.audio_mix.master_percent);
        audio_system.set_channel_volume_percent("music", launch_options.audio_mix.music_percent);
        audio_system
            .set_channel_volume_percent("movement", launch_options.audio_mix.movement_percent);
        audio_system
            .set_channel_volume_percent("collision", launch_options.audio_mix.collision_percent);

        Self {
            // Core systems
            game_system,
            camera_system,
            resources,
            performance: PerformanceMonitor::new(),
            audio_system,

            // Grouped systems
            platform: PlatformSystem::new(),
            rendering: RenderingSystem::new(),
            timing: TimingSystem::new(),
            launch_options,
            splash_policy,
            splash_config,
            splash_active: true,
            splash_started_at: None,
            splash_logo_loaded: false,
            post_splash_sprite_texture_path: None,
            asset_load_plan,
            decoded_project_cache,
            pack_mount,
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
        let game_result = self.game_system.update(
            world_bounds,
            self.resources.get_tilemap(),
            self.resources.get_terrain_atlas(),
        );

        let listener_position = self
            .game_system
            .player_id()
            .map(|_| self.game_system.player_position());
        self.audio_system.set_listener_position(listener_position);

        // Process audio events
        for event in &game_result.events {
            self.audio_system.handle(event);
        }

        let player_moved = game_result.player_moved;

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
                    self.rendering.update_tilemap_vertices(&verts);
                }
            }
            self.rendering.clear_sprites(); // Clear previous frame's sprites
            self.rendering.clear_text_items();

            // Render all visible entities with animation controllers
            let renderable_entities = self.game_system.get_renderable_entities();
            for (entity_id, position, size) in renderable_entities {
                let Some(atlas_name) = self.game_system.get_entity_current_atlas_name(entity_id)
                else {
                    continue;
                };
                let Some(sprite_atlas) = self.resources.get_sprite_atlas(&atlas_name) else {
                    tracing::warn!(
                        "Entity {} requested missing sprite atlas '{}'",
                        entity_id,
                        atlas_name
                    );
                    continue;
                };
                let texture_size = sprite_atlas
                    .image_size()
                    .unwrap_or(glam::UVec2::new(64, 16));
                if let Some(frame) =
                    self.game_system
                        .get_entity_sprite_frame(entity_id, sprite_atlas, texture_size)
                {
                    let flip_x = self.game_system.get_entity_sprite_flip_x(entity_id);
                    if let Some(texture_path) =
                        self.resources.get_sprite_texture_path(&atlas_name).cloned()
                    {
                        self.rendering.add_sprite_with_texture(
                            texture_path,
                            frame,
                            position,
                            size,
                            flip_x,
                        );
                    } else {
                        self.rendering.add_sprite(frame, position, size, flip_x);
                    }
                }
            }

            for object in &self.resources.get_tilemap().objects {
                if !object.visible {
                    continue;
                }
                let sheet_name = object
                    .sheet
                    .file_name()
                    .and_then(|name| name.to_str())
                    .or_else(|| object.sheet.to_str());
                let Some(sheet_name) = sheet_name else {
                    continue;
                };
                let Some(object_sheet) = self.resources.get_object_sheet(sheet_name) else {
                    tracing::warn!("Map object requested missing object sheet '{}'", sheet_name);
                    continue;
                };
                let texture_size = object_sheet
                    .image_size()
                    .unwrap_or(glam::UVec2::new(16, 16));
                let Some(uv_rect) = object_sheet.get_object_uvs(&object.object_name, texture_size)
                else {
                    tracing::warn!(
                        "Map object '{}' missing from object sheet '{}'",
                        object.object_name,
                        sheet_name
                    );
                    continue;
                };
                let Some(rect) = object_sheet.get_object_rect(&object.object_name) else {
                    continue;
                };
                let frame = toki_core::sprite::SpriteFrame {
                    u0: uv_rect[0],
                    v0: uv_rect[1],
                    u1: uv_rect[2],
                    v1: uv_rect[3],
                };
                let size = glam::UVec2::new(rect[2], rect[3]);
                let position = object.position.as_ivec2();
                if let Some(texture_path) =
                    self.resources.get_object_texture_path(sheet_name).cloned()
                {
                    self.rendering.add_sprite_with_texture(
                        texture_path,
                        frame,
                        position,
                        size,
                        false,
                    );
                } else {
                    self.rendering.add_sprite(frame, position, size, false);
                }
            }

            // Add debug collision rendering
            self.rendering.clear_debug_shapes();
            if self.launch_options.display.show_entity_health_bars {
                self.render_entity_health_bars();
            }
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
                    let color = if is_trigger {
                        trigger_tile_color
                    } else {
                        entity_color
                    };
                    self.rendering.add_debug_rect(
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
                    self.rendering.add_debug_rect(
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
                    self.rendering.add_debug_rect(
                        world_x as f32,
                        world_y as f32,
                        tilemap.tile_size.x as f32,
                        tilemap.tile_size.y as f32,
                        trigger_tile_color,
                    );
                }
            }
            self.rendering.finalize_debug_shapes();

            if let Some(stats_line) = self.performance.stats_line() {
                let hud_style = TextStyle {
                    font_family: "Sans".to_string(),
                    size_px: 14.0,
                    weight: TextWeight::Bold,
                    ..TextStyle::default()
                };
                let hud_text =
                    TextItem::new_screen(stats_line, glam::Vec2::new(8.0, 8.0), hud_style)
                        .with_anchor(TextAnchor::TopLeft)
                        .with_layer(1);
                self.rendering.add_text_item(hud_text);
            }
        }

        self.platform.request_redraw();
    }

    fn render_entity_health_bars(&mut self) {
        for health_bar in self.game_system.get_entity_health_bars() {
            let bar_width = health_bar.size.x.max(16) as f32;
            let bar_height = 3.0;
            let bar_x = health_bar.position.x as f32;
            let bar_y = health_bar.position.y as f32 - 6.0;
            let fill_ratio = (health_bar.current as f32 / health_bar.max as f32).clamp(0.0, 1.0);
            let fill_color = Self::health_bar_fill_color(fill_ratio);

            self.rendering.add_filled_debug_rect(
                bar_x,
                bar_y,
                bar_width,
                bar_height,
                [0.1, 0.1, 0.1, 0.8],
            );
            if fill_ratio > 0.0 {
                self.rendering.add_filled_debug_rect(
                    bar_x,
                    bar_y,
                    (bar_width * fill_ratio).max(1.0),
                    bar_height,
                    fill_color,
                );
            }
            self.rendering.add_debug_rect(
                bar_x,
                bar_y,
                bar_width,
                bar_height,
                [0.0, 0.0, 0.0, 1.0],
            );
        }
    }

    fn health_bar_fill_color(fill_ratio: f32) -> [f32; 4] {
        if fill_ratio > 0.6 {
            [0.2, 0.85, 0.25, 0.95]
        } else if fill_ratio > 0.3 {
            [0.95, 0.8, 0.2, 0.95]
        } else {
            [0.9, 0.2, 0.2, 0.95]
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
                            self.performance.toggle_hud_display();
                        }
                        KeyCode::F7 => {
                            self.performance.toggle_console_display();
                        }
                        KeyCode::F5 => {
                            if let Err(e) = save_game(&self.game_system.game_state, "savegame.json")
                            {
                                tracing::error!("Failed to save game: {}", e);
                            } else {
                                tracing::info!("Game saved to savegame.json");
                            }
                        }
                        KeyCode::F6 => match load_game("savegame.json") {
                            Ok(loaded_state) => {
                                self.game_system.game_state = loaded_state;
                                tracing::info!("Game loaded from savegame.json");
                            }
                            Err(e) => tracing::error!("Failed to load game: {}", e),
                        },
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
            if self.splash_active {
                let started_at = self.splash_started_at.unwrap_or_else(|| {
                    let now = Instant::now();
                    self.splash_started_at = Some(now);
                    now
                });

                if started_at.elapsed() < self.splash_config.duration {
                    self.render_startup_splash();
                    return;
                }

                self.splash_active = false;
                self.rendering.set_tilemap_render_enabled(true);
                self.restore_runtime_sprite_texture_after_splash();
                self.rendering
                    .update_projection(self.camera_system.view_matrix());
                self.refresh_tilemap_vertices_for_current_camera();
                self.tick();
                self.timing.reset();
                self.platform.request_redraw();
            }

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

    fn refresh_tilemap_vertices_for_current_camera(&mut self) {
        if !self.rendering.has_gpu() {
            return;
        }

        self.camera_system
            .update_chunk_cache(self.resources.get_tilemap());
        let atlas_size = self.resources.terrain_image_size().unwrap();
        let verts = self.resources.get_tilemap().generate_vertices_for_chunks(
            self.resources.get_terrain_atlas(),
            atlas_size,
            self.camera_system.cached_visible_chunks(),
        );
        self.rendering.update_tilemap_vertices(&verts);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let content_root = self.content_root_path().map(std::path::Path::to_path_buf);

        // Initialize platform system (window)
        self.platform.initialize_window(event_loop);

        // Initialize rendering system (GPU)
        if let Some(window) = self.platform.window_for_gpu() {
            if let Err(error) = self.rendering.initialize_gpu_with_textures(
                window.clone(),
                self.asset_load_plan.tilemap_texture_path.clone(),
                self.asset_load_plan.sprite_texture_path.clone(),
            ) {
                if let Some(content_root) = content_root.as_deref() {
                    tracing::error!(
                        "Failed to initialize GPU with runtime asset plan from '{}': {}",
                        content_root.display(),
                        error
                    );
                } else {
                    tracing::error!("Failed to initialize GPU with runtime asset plan: {error}");
                }
                self.rendering.initialize_gpu(window);
            } else {
                self.post_splash_sprite_texture_path =
                    self.asset_load_plan.sprite_texture_path.clone();
            }
        }

        if self.rendering.has_gpu() {
            if let Some(content_root) = content_root.as_deref() {
                if let Err(error) = self.rendering.load_project_textures(content_root) {
                    tracing::warn!(
                        "Failed to load project textures from '{}': {}",
                        content_root.display(),
                        error
                    );
                }
            }
        }

        self.post_splash_sprite_texture_path =
            self.post_splash_sprite_texture_path.clone().or_else(|| {
                Self::resolve_post_splash_sprite_texture_path(
                    &self.launch_options,
                    content_root.as_deref(),
                )
            });
        self.initialize_splash_resources();

        // Update rendering size
        if let Some(size) = self.platform.inner_size() {
            self.rendering.update_window_size(size);
        }

        // Set up initial projection
        let view = self.camera_system.view_matrix();
        self.rendering.update_projection(view);

        self.platform.request_redraw();

        // Load initially visible chunks
        self.refresh_tilemap_vertices_for_current_camera();

        self.audio_system.list_available_sounds();
        if self.launch_options.scene_name.is_none() {
            if let Err(e) = self.audio_system.play_background_music("lavandia", -10.0) {
                tracing::warn!("Failed to start background music: {}", e);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.splash_active && self.rendering.has_gpu() {
            self.platform.request_redraw();
            return;
        }

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
    run_minimal_window_with_options(RuntimeLaunchOptions::default())
}

pub fn run_minimal_window_with_options(
    launch_options: RuntimeLaunchOptions,
) -> Result<(), RenderError> {
    let event_loop = EventLoop::new()?;

    // Create an instance of the App struct
    let mut app = App::new(launch_options);

    // Run the application
    event_loop.run_app(&mut app)?;

    // Return Ok if the application was closed successfully
    Ok(())
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
