//! Simple winit window example.
// winit imports
use winit::application::ApplicationHandler; // Trait that defines app lifecycle hooks (resumed, event handling, etc.)
use winit::event::WindowEvent; // Enum of possible window-related events (resize, input, close, etc.)
use winit::event_loop::{ActiveEventLoop, EventLoop}; // ActiveEventLoop is used inside lifecycle methods; EventLoop creates and runs the app
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowId;

use std::time::{Duration, Instant};
use std::{fs, path::PathBuf};

use toki_core::camera::{Camera, CameraController, CameraMode, RuntimeState};
use toki_core::graphics::image::load_image_rgba8_from_bytes;
use toki_core::math::projection::ProjectionParameter;
use toki_core::text::{TextAnchor, TextItem, TextStyle, TextWeight};
use toki_core::{EventHandler, GameState, Scene, TimingSystem};
use toki_render::RenderError;

use crate::systems::AudioManager;
use crate::systems::{
    CameraManager, GameManager, PerformanceMonitor, PlatformSystem, RenderingSystem,
    ResourceManager,
};
use toki_core::serialization::{load_game, save_game};

const COMMUNITY_SPLASH_MIN_DURATION_MS: u64 = 3000;
const COMMUNITY_SPLASH_MAX_DURATION_MS: u64 = 4000;
const COMMUNITY_SPLASH_DEFAULT_DURATION_MS: u64 = 3000;
const COMMUNITY_SPLASH_BRANDING_TEXT: &str = "Powered by ToKi";
const SPLASH_LOGO_WIDTH: u32 = 128;
const SPLASH_LOGO_HEIGHT: u32 = 108;
const COMMUNITY_SPLASH_LOGO_PNG: &[u8] = include_bytes!("../../../assets/TokiLogo.png");

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeLaunchOptions {
    pub project_path: Option<PathBuf>,
    pub pack_path: Option<PathBuf>,
    pub scene_name: Option<String>,
    pub map_name: Option<String>,
    pub splash: RuntimeSplashOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SplashPolicy {
    Community,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedSplashConfig {
    duration: Duration,
    show_branding: bool,
}

impl SplashPolicy {
    fn resolve(self, requested: &RuntimeSplashOptions) -> ResolvedSplashConfig {
        match self {
            Self::Community => {
                if !requested.show_branding {
                    tracing::warn!(
                        "Splash branding cannot be disabled in Community bundle; forcing branding ON"
                    );
                }
                let clamped_duration = requested.duration_ms.clamp(
                    COMMUNITY_SPLASH_MIN_DURATION_MS,
                    COMMUNITY_SPLASH_MAX_DURATION_MS,
                );
                ResolvedSplashConfig {
                    duration: Duration::from_millis(clamped_duration),
                    show_branding: true,
                }
            }
        }
    }
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

    fn projection_view_size(parameters: ProjectionParameter) -> glam::Vec2 {
        let aspect = parameters.width as f32 / parameters.height as f32;
        let desired_aspect = parameters.desired_width as f32 / parameters.desired_height as f32;

        if aspect > desired_aspect {
            let height = parameters.desired_height as f32;
            let width = height * aspect;
            glam::Vec2::new(width, height)
        } else {
            let width = parameters.desired_width as f32;
            let height = width / aspect;
            glam::Vec2::new(width, height)
        }
    }

    fn centered_logo_origin_for_view(view_size: glam::Vec2, logo_size: glam::UVec2) -> glam::IVec2 {
        let x = ((view_size.x - logo_size.x as f32) * 0.5).floor() as i32;
        let y = ((view_size.y - logo_size.y as f32) * 0.5).floor() as i32;
        glam::IVec2::new(x, y)
    }

    fn new(launch_options: RuntimeLaunchOptions) -> Self {
        let splash_policy = SplashPolicy::Community;
        let splash_config = splash_policy.resolve(&launch_options.splash);
        let (resources, game_state, pack_mount) = Self::build_startup_state(&launch_options);
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
        let audio_system =
            AudioManager::new_with_assets_root(audio_root).expect("Failed to initialize audio system");

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
            pack_mount,
        }
    }

    fn build_startup_state(
        launch_options: &RuntimeLaunchOptions,
    ) -> (ResourceManager, GameState, Option<tempfile::TempDir>) {
        if let Some(pack_path) = &launch_options.pack_path {
            return Self::build_startup_state_from_pack(launch_options, pack_path)
                .unwrap_or_else(|error| {
                    panic!(
                        "Failed to initialize runtime from pack '{}': {}",
                        pack_path.display(),
                        error
                    )
                });
        }

        if let Some(project_path) = &launch_options.project_path {
            let scene = launch_options
                .scene_name
                .as_deref()
                .and_then(|scene_name| Self::load_project_scene(project_path, scene_name).ok());

            let map_name = launch_options.map_name.clone().or_else(|| {
                scene
                    .as_ref()
                    .and_then(|loaded_scene| loaded_scene.maps.first().cloned())
            });

            match ResourceManager::load_for_project(project_path, map_name.as_deref()) {
                Ok(resources) => {
                    let game_state = if let Some(scene) = scene {
                        Self::game_state_from_scene(scene)
                    } else {
                        Self::fallback_game_state()
                    };
                    return (resources, game_state, None);
                }
                Err(error) => {
                    tracing::error!(
                        "Failed to load project resources for '{}': {}",
                        project_path.display(),
                        error
                    );
                }
            }
        }

        match ResourceManager::load_all() {
            Ok(resources) => (resources, Self::fallback_game_state(), None),
            Err(error) => {
                panic!("Failed to initialize runtime resources: {error}");
            }
        }
    }

    fn build_startup_state_from_pack(
        launch_options: &RuntimeLaunchOptions,
        pack_path: &std::path::Path,
    ) -> anyhow::Result<(ResourceManager, GameState, Option<tempfile::TempDir>)> {
        let mount = crate::pack::extract_pak_to_tempdir(pack_path)?;
        let mount_path = mount.path().to_path_buf();
        let scene = launch_options
            .scene_name
            .as_deref()
            .map(|scene_name| Self::load_project_scene(&mount_path, scene_name))
            .transpose()
            .map_err(anyhow::Error::msg)?;
        let map_name = launch_options.map_name.clone().or_else(|| {
            scene
                .as_ref()
                .and_then(|loaded_scene| loaded_scene.maps.first().cloned())
        });
        let resources = ResourceManager::load_for_project(&mount_path, map_name.as_deref())?;
        let game_state = if let Some(scene) = scene {
            Self::game_state_from_scene(scene)
        } else {
            Self::fallback_game_state()
        };
        Ok((resources, game_state, Some(mount)))
    }

    fn load_project_scene(
        project_path: &std::path::Path,
        scene_name: &str,
    ) -> Result<Scene, String> {
        let scene_path = project_path
            .join("scenes")
            .join(format!("{scene_name}.json"));
        let json = fs::read_to_string(&scene_path).map_err(|error| {
            format!(
                "Could not read scene file '{}': {}",
                scene_path.display(),
                error
            )
        })?;
        serde_json::from_str::<Scene>(&json).map_err(|error| {
            format!(
                "Could not parse scene file '{}': {}",
                scene_path.display(),
                error
            )
        })
    }

    fn game_state_from_scene(scene: Scene) -> GameState {
        let scene_name = scene.name.clone();
        let mut game_state = GameState::new_empty();
        game_state.add_scene(scene);
        if let Err(error) = game_state.load_scene(&scene_name) {
            tracing::error!(
                "Failed to load startup scene '{}' into game state: {}",
                scene_name,
                error
            );
            return Self::fallback_game_state();
        }
        game_state
    }

    fn fallback_game_state() -> GameState {
        let mut game_state = GameState::new_empty();
        let _player_id = game_state.spawn_player_at(glam::IVec2::new(80, 72));
        let _npc_id = game_state.spawn_player_like_npc(glam::IVec2::new(120, 72));
        game_state
    }

    fn project_texture_paths(project_path: &std::path::Path) -> (Option<PathBuf>, Option<PathBuf>) {
        let tilemap_texture = first_existing_path(&[
            project_path
                .join("assets")
                .join("sprites")
                .join("terrain.png"),
            project_path.join("assets").join("terrain.png"),
        ]);
        let sprite_texture = first_existing_path(&[
            project_path
                .join("assets")
                .join("sprites")
                .join("creatures.png"),
            project_path.join("assets").join("creatures.png"),
        ]);
        (tilemap_texture, sprite_texture)
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
            let creature_atlas = self.resources.get_creature_atlas();
            let texture_size = creature_atlas
                .image_size()
                .unwrap_or(glam::UVec2::new(64, 16)); // fallback

            self.rendering.clear_sprites(); // Clear previous frame's sprites
            self.rendering.clear_text_items();

            // Render all visible entities with animation controllers
            let renderable_entities = self.game_system.get_renderable_entities();
            for (entity_id, position, size) in renderable_entities {
                if let Some(frame) = self.game_system.get_entity_sprite_frame(
                    entity_id,
                    creature_atlas,
                    texture_size,
                ) {
                    self.rendering.add_sprite(frame, position, size);
                }
            }

            // Add debug collision rendering
            self.rendering.clear_debug_shapes();
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

    fn render_startup_splash(&mut self) {
        let logo_size = glam::UVec2::new(SPLASH_LOGO_WIDTH, SPLASH_LOGO_HEIGHT);
        let view_size = Self::projection_view_size(self.rendering.projection_params());
        let logo_origin = Self::centered_logo_origin_for_view(view_size, logo_size);
        self.rendering.update_projection(glam::Mat4::IDENTITY);
        self.rendering.set_tilemap_render_enabled(false);
        self.rendering.clear_sprites();
        self.rendering.clear_text_items();
        self.rendering.clear_debug_shapes();
        self.rendering.finalize_debug_shapes();
        if self.splash_logo_loaded {
            self.rendering.add_sprite(
                toki_core::sprite::SpriteFrame {
                    u0: 0.0,
                    v0: 0.0,
                    u1: 1.0,
                    v1: 1.0,
                },
                logo_origin,
                logo_size,
            );
        }
        if self.splash_config.show_branding {
            let branding_style = TextStyle {
                font_family: "Sans".to_string(),
                size_px: 16.0,
                weight: TextWeight::Bold,
                ..TextStyle::default()
            };
            let branding_position = if self.splash_logo_loaded {
                glam::Vec2::new(
                    view_size.x * 0.5,
                    (logo_origin.y as f32 + logo_size.y as f32 + 8.0).min(view_size.y - 4.0),
                )
            } else {
                glam::Vec2::new(view_size.x * 0.5, view_size.y * 0.5)
            };
            self.rendering.add_text_item(
                TextItem::new_screen(
                    COMMUNITY_SPLASH_BRANDING_TEXT,
                    branding_position,
                    branding_style,
                )
                .with_anchor(TextAnchor::TopCenter)
                .with_layer(10),
            );
        }
        self.rendering.draw();
        self.platform.request_redraw();
    }

    fn restore_runtime_sprite_texture_after_splash(&mut self) {
        if let Some(path) = &self.post_splash_sprite_texture_path {
            if let Err(error) = self.rendering.load_sprite_texture(path.clone()) {
                tracing::warn!(
                    "Failed to restore sprite texture '{}' after splash: {}",
                    path.display(),
                    error
                );
            }
            return;
        }

        tracing::warn!(
            "No post-splash sprite texture path available; keeping current sprite texture"
        );
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

    fn resolve_post_splash_sprite_texture_path(
        launch_options: &RuntimeLaunchOptions,
        content_root: Option<&std::path::Path>,
    ) -> Option<PathBuf> {
        if let Some(root) = content_root {
            let (_, sprite_texture) = Self::project_texture_paths(root);
            if sprite_texture.is_some() {
                return sprite_texture;
            }
        }

        if let Some(project_path) = &launch_options.project_path {
            let (_, sprite_texture) = Self::project_texture_paths(project_path);
            if sprite_texture.is_some() {
                return sprite_texture;
            }
        }

        first_existing_path(&[
            PathBuf::from("assets/creatures.png"),
            PathBuf::from("assets/sprites/creatures.png"),
        ])
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let content_root = self.content_root_path().map(std::path::Path::to_path_buf);

        // Initialize platform system (window)
        self.platform.initialize_window(event_loop);

        // Initialize rendering system (GPU)
        if let Some(window) = self.platform.window_for_gpu() {
            if let Some(content_root) = content_root.as_deref() {
                let (tilemap_texture, sprite_texture) = Self::project_texture_paths(content_root);
                if let Err(error) = self.rendering.initialize_gpu_with_textures(
                    window.clone(),
                    tilemap_texture.clone(),
                    sprite_texture.clone(),
                ) {
                    tracing::error!(
                        "Failed to initialize GPU with project textures from '{}': {}",
                        content_root.display(),
                        error
                    );
                    self.rendering.initialize_gpu(window);
                }
            } else {
                self.rendering.initialize_gpu(window);
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

        self.post_splash_sprite_texture_path = Self::resolve_post_splash_sprite_texture_path(
            &self.launch_options,
            content_root.as_deref(),
        );
        self.splash_config = self.splash_policy.resolve(&self.launch_options.splash);
        if let Ok(decoded_logo) = load_image_rgba8_from_bytes(COMMUNITY_SPLASH_LOGO_PNG) {
            if let Err(error) = self.rendering.load_sprite_texture_rgba8(&decoded_logo) {
                tracing::warn!(
                    "Failed to load embedded startup logo texture (splash will render branding-only): {}",
                    error
                );
                self.splash_logo_loaded = false;
            } else {
                self.splash_logo_loaded = true;
            }
        } else {
            tracing::warn!(
                "Failed to decode embedded startup logo bytes; splash will render branding-only"
            );
            self.splash_logo_loaded = false;
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

fn first_existing_path(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|path| path.exists()).cloned()
}

#[cfg(test)]
mod tests {
    use super::{
        first_existing_path, App, RuntimeLaunchOptions, RuntimeSplashOptions, SplashPolicy,
    };
    use std::fs;
    use std::io::{Seek, Write};
    use std::path::PathBuf;
    use std::time::Duration;
    use toki_core::math::projection::ProjectionParameter;
    use toki_core::rules::{
        Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleTrigger,
    };
    use toki_core::Scene;

    fn make_unique_temp_dir() -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("toki_runtime_app_tests_{nanos}"));
        std::fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn write_test_pak(pak_path: &std::path::Path, entries: &[(String, Vec<u8>)]) {
        let mut file = fs::File::create(pak_path).expect("create pak");
        file.write_all(b"TOKIPAK1").expect("magic");
        file.write_all(&0u64.to_le_bytes())
            .expect("offset placeholder");
        file.write_all(&0u64.to_le_bytes())
            .expect("size placeholder");

        let mut manifest_entries = Vec::new();
        for (path, payload) in entries {
            let offset = file.stream_position().expect("offset");
            file.write_all(payload).expect("payload");
            manifest_entries.push(serde_json::json!({
                "path": path,
                "offset": offset,
                "size": payload.len(),
                "compression": "none"
            }));
        }

        let index_offset = file.stream_position().expect("index offset");
        let index_bytes = serde_json::to_vec_pretty(&serde_json::json!({
            "version": 1,
            "entries": manifest_entries
        }))
        .expect("manifest");
        file.write_all(&index_bytes).expect("index");
        let index_size = index_bytes.len() as u64;
        file.seek(std::io::SeekFrom::Start(8)).expect("seek header");
        file.write_all(&index_offset.to_le_bytes())
            .expect("write offset");
        file.write_all(&index_size.to_le_bytes())
            .expect("write size");
    }

    #[test]
    fn first_existing_path_returns_first_match() {
        let dir = make_unique_temp_dir();
        let missing = dir.join("missing.txt");
        let first = dir.join("a.txt");
        let second = dir.join("b.txt");
        fs::write(&first, "a").expect("first file write");
        fs::write(&second, "b").expect("second file write");

        let resolved = first_existing_path(&[missing, first.clone(), second.clone()]);
        assert_eq!(resolved, Some(first));
    }

    #[test]
    fn first_existing_path_returns_none_when_no_candidate_exists() {
        let dir = make_unique_temp_dir();
        let missing_a = dir.join("missing_a.txt");
        let missing_b = dir.join("missing_b.txt");
        let resolved = first_existing_path(&[missing_a, missing_b]);
        assert!(resolved.is_none());
    }

    #[test]
    fn project_texture_paths_prefers_assets_sprites_files() {
        let project_dir = make_unique_temp_dir();
        let sprites_dir = project_dir.join("assets").join("sprites");
        fs::create_dir_all(&sprites_dir).expect("sprites dir");
        let terrain = sprites_dir.join("terrain.png");
        let creatures = sprites_dir.join("creatures.png");
        fs::write(&terrain, "terrain").expect("terrain write");
        fs::write(&creatures, "creatures").expect("creatures write");

        let (tilemap_texture, sprite_texture) = App::project_texture_paths(&project_dir);
        assert_eq!(tilemap_texture, Some(terrain));
        assert_eq!(sprite_texture, Some(creatures));
    }

    #[test]
    fn project_texture_paths_falls_back_to_assets_root() {
        let project_dir = make_unique_temp_dir();
        let assets_dir = project_dir.join("assets");
        fs::create_dir_all(&assets_dir).expect("assets dir");
        let terrain = assets_dir.join("terrain.png");
        let creatures = assets_dir.join("creatures.png");
        fs::write(&terrain, "terrain").expect("terrain write");
        fs::write(&creatures, "creatures").expect("creatures write");

        let (tilemap_texture, sprite_texture) = App::project_texture_paths(&project_dir);
        assert_eq!(tilemap_texture, Some(terrain));
        assert_eq!(sprite_texture, Some(creatures));
    }

    #[test]
    fn load_project_scene_reads_valid_scene_file() {
        let project_dir = make_unique_temp_dir();
        let scenes_dir = project_dir.join("scenes");
        fs::create_dir_all(&scenes_dir).expect("scenes dir");

        let mut scene = Scene::new("Main Scene".to_string());
        scene.maps.push("main_map".to_string());
        scene.rules = RuleSet {
            rules: vec![Rule {
                id: "rule_1".to_string(),
                enabled: true,
                priority: 1,
                once: false,
                trigger: RuleTrigger::OnStart,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::PlaySound {
                    channel: RuleSoundChannel::Movement,
                    sound_id: "sfx_start".to_string(),
                }],
            }],
        };
        let scene_json =
            serde_json::to_string_pretty(&scene).expect("scene should serialize to json");
        fs::write(scenes_dir.join("Main Scene.json"), scene_json).expect("scene write");

        let loaded = App::load_project_scene(&project_dir, "Main Scene")
            .expect("scene should load from project");
        assert_eq!(loaded.name, "Main Scene");
        assert_eq!(loaded.maps, vec!["main_map".to_string()]);
        assert_eq!(loaded.rules, scene.rules);
    }

    #[test]
    fn load_project_scene_returns_error_for_invalid_json() {
        let project_dir = make_unique_temp_dir();
        let scenes_dir = project_dir.join("scenes");
        fs::create_dir_all(&scenes_dir).expect("scenes dir");
        fs::write(scenes_dir.join("Broken.json"), "{ invalid json").expect("scene write");

        let error = App::load_project_scene(&project_dir, "Broken")
            .expect_err("invalid scene json should fail");
        assert!(error.contains("Could not parse scene file"));
    }

    #[test]
    fn load_project_scene_returns_error_for_missing_scene_file() {
        let project_dir = make_unique_temp_dir();
        fs::create_dir_all(project_dir.join("scenes")).expect("scenes dir");

        let error = App::load_project_scene(&project_dir, "DoesNotExist")
            .expect_err("missing scene file should fail");
        assert!(error.contains("Could not read scene file"));
    }

    #[test]
    fn game_state_from_scene_uses_scene_data_without_fallback_entities() {
        let mut scene = Scene::new("Gameplay".to_string());
        scene.rules = RuleSet {
            rules: vec![Rule {
                id: "rule_1".to_string(),
                enabled: true,
                priority: 3,
                once: false,
                trigger: RuleTrigger::OnUpdate,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::PlayMusic {
                    track_id: "lavandia".to_string(),
                }],
            }],
        };

        let game_state = App::game_state_from_scene(scene.clone());
        assert_eq!(
            game_state.scene_manager().active_scene_name(),
            Some("Gameplay")
        );
        assert_eq!(game_state.rules(), &scene.rules);
        assert_eq!(game_state.entity_manager().active_entities().len(), 0);
    }

    #[test]
    fn fallback_game_state_spawns_player_and_npc() {
        let game_state = App::fallback_game_state();
        assert!(game_state.player_id().is_some(), "player should exist");
        assert_eq!(
            game_state.entity_manager().active_entities().len(),
            2,
            "fallback state should spawn player and one npc"
        );
    }

    #[test]
    fn resolve_post_splash_sprite_texture_path_prefers_project_creatures_texture() {
        let project_dir = make_unique_temp_dir()
            .join("example_project")
            .join("MyGame");
        let sprites_dir = project_dir.join("assets").join("sprites");
        fs::create_dir_all(&sprites_dir).expect("sprites dir");
        let creatures_path = sprites_dir.join("creatures.png");
        fs::write(&creatures_path, "creatures").expect("creatures write");

        let options = RuntimeLaunchOptions {
            project_path: Some(project_dir),
            pack_path: None,
            scene_name: None,
            map_name: None,
            splash: RuntimeSplashOptions::default(),
        };

        let resolved = App::resolve_post_splash_sprite_texture_path(&options, None);
        assert_eq!(resolved, Some(creatures_path));
    }

    #[test]
    fn resolve_post_splash_sprite_texture_path_prefers_content_root_over_project_path() {
        let project_dir = make_unique_temp_dir()
            .join("example_project")
            .join("MyGame");
        let project_sprites_dir = project_dir.join("assets").join("sprites");
        fs::create_dir_all(&project_sprites_dir).expect("project sprites dir");
        fs::write(project_sprites_dir.join("creatures.png"), "project-creatures")
            .expect("project sprite write");

        let mount_dir = make_unique_temp_dir().join("mount");
        let mount_sprites_dir = mount_dir.join("assets").join("sprites");
        fs::create_dir_all(&mount_sprites_dir).expect("mount sprites dir");
        let mount_creatures = mount_sprites_dir.join("creatures.png");
        fs::write(&mount_creatures, "mount-creatures").expect("mount sprite write");

        let options = RuntimeLaunchOptions {
            project_path: Some(project_dir),
            pack_path: None,
            scene_name: None,
            map_name: None,
            splash: RuntimeSplashOptions::default(),
        };

        let resolved =
            App::resolve_post_splash_sprite_texture_path(&options, Some(&mount_dir));
        assert_eq!(resolved, Some(mount_creatures));
    }

    #[test]
    fn build_startup_state_loads_resources_and_scene_from_pack_mount() {
        let temp = tempfile::tempdir().expect("temp dir");
        let pack_path = temp.path().join("game.toki.pak");

        let mut scene = Scene::new("Main Scene".to_string());
        scene.maps.push("demo_map".to_string());
        let scene_json = serde_json::to_vec_pretty(&scene).expect("scene json");

        let creatures_atlas = br#"{
  "image": "creatures.png",
  "tile_size": [16, 16],
  "tiles": {
    "idle": { "position": [0, 0], "properties": { "solid": false } }
  }
}"#
        .to_vec();

        let terrain_atlas_path = PathBuf::from("terrain.json");
        let map_json = format!(
            r#"{{
  "size": [1, 1],
  "tile_size": [16, 16],
  "atlas": "{}",
  "tiles": ["floor"]
}}"#,
            terrain_atlas_path.display()
        )
        .into_bytes();

        let terrain_atlas = br#"{
  "image": "terrain.png",
  "tile_size": [16, 16],
  "tiles": {
    "floor": { "position": [0, 0], "properties": { "solid": false } }
  }
}"#
        .to_vec();

        write_test_pak(
            &pack_path,
            &[
                ("scenes/Main Scene.json".to_string(), scene_json),
                ("assets/sprites/creatures.json".to_string(), creatures_atlas),
                ("assets/tilemaps/demo_map.json".to_string(), map_json),
                ("assets/tilemaps/terrain.json".to_string(), terrain_atlas),
            ],
        );

        let launch_options = RuntimeLaunchOptions {
            project_path: None,
            pack_path: Some(pack_path),
            scene_name: Some("Main Scene".to_string()),
            map_name: None,
            splash: RuntimeSplashOptions::default(),
        };

        let (resources, game_state, pack_mount) = App::build_startup_state(&launch_options);

        assert!(pack_mount.is_some(), "pack mount should be retained");
        assert_eq!(
            game_state.scene_manager().active_scene_name(),
            Some("Main Scene")
        );
        assert_eq!(game_state.entity_manager().active_entities().len(), 0);
        assert_eq!(resources.get_tilemap().size, glam::UVec2::new(1, 1));
        assert_eq!(resources.get_tilemap().atlas, terrain_atlas_path);
    }

    #[test]
    fn build_startup_state_from_pack_returns_error_when_required_assets_are_missing() {
        let temp = tempfile::tempdir().expect("temp dir");
        let pack_path = temp.path().join("broken.toki.pak");

        let mut scene = Scene::new("Main Scene".to_string());
        scene.maps.push("demo_map".to_string());
        let scene_json = serde_json::to_vec_pretty(&scene).expect("scene json");

        write_test_pak(
            &pack_path,
            &[
                ("scenes/Main Scene.json".to_string(), scene_json),
                ("assets/tilemaps/demo_map.json".to_string(), b"{}".to_vec()),
            ],
        );

        let launch_options = RuntimeLaunchOptions {
            project_path: None,
            pack_path: Some(pack_path.clone()),
            scene_name: Some("Main Scene".to_string()),
            map_name: None,
            splash: RuntimeSplashOptions::default(),
        };

        let error = App::build_startup_state_from_pack(&launch_options, &pack_path)
            .expect_err("missing pack assets should fail startup");
        let text = error.to_string();
        assert!(
            text.contains("atlas") || text.contains("resources") || text.contains("Core error"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn community_splash_policy_forces_branding_on() {
        let requested = RuntimeSplashOptions {
            duration_ms: 1200,
            show_branding: false,
        };
        let resolved = SplashPolicy::Community.resolve(&requested);
        assert!(resolved.show_branding);
    }

    #[test]
    fn community_splash_policy_clamps_duration_bounds() {
        let below_min = SplashPolicy::Community.resolve(&RuntimeSplashOptions {
            duration_ms: 200,
            show_branding: true,
        });
        assert_eq!(
            below_min.duration,
            Duration::from_millis(super::COMMUNITY_SPLASH_MIN_DURATION_MS)
        );

        let above_max = SplashPolicy::Community.resolve(&RuntimeSplashOptions {
            duration_ms: 9_999,
            show_branding: true,
        });
        assert_eq!(
            above_max.duration,
            Duration::from_millis(super::COMMUNITY_SPLASH_MAX_DURATION_MS)
        );
    }

    #[test]
    fn centered_logo_origin_matches_previous_default_layout() {
        let view = App::projection_view_size(ProjectionParameter {
            width: 160,
            height: 144,
            desired_width: 160,
            desired_height: 144,
        });
        let origin = App::centered_logo_origin_for_view(view, glam::UVec2::new(128, 108));
        assert_eq!(origin, glam::IVec2::new(16, 18));
    }

    #[test]
    fn centered_logo_origin_is_centered_for_wide_window() {
        let view = App::projection_view_size(ProjectionParameter {
            width: 320,
            height: 144,
            desired_width: 160,
            desired_height: 144,
        });
        let origin = App::centered_logo_origin_for_view(view, glam::UVec2::new(128, 108));
        assert_eq!(origin, glam::IVec2::new(96, 18));
    }

    #[test]
    fn centered_logo_origin_is_centered_for_tall_window() {
        let view = App::projection_view_size(ProjectionParameter {
            width: 160,
            height: 320,
            desired_width: 160,
            desired_height: 144,
        });
        let origin = App::centered_logo_origin_for_view(view, glam::UVec2::new(128, 108));
        assert_eq!(origin, glam::IVec2::new(16, 106));
    }
}
