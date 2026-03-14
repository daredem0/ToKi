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
use toki_core::text::{TextAnchor, TextItem, TextStyle, TextWeight};
use toki_core::{EventHandler, GameState, Scene, TimingSystem};
use toki_render::RenderError;

use crate::systems::AudioManager;
use crate::systems::{
    CameraManager, GameManager, PerformanceMonitor, PlatformSystem, RenderingSystem,
    ResourceManager,
};
use toki_core::serialization::{load_game, save_game};

#[derive(Debug, Clone, Default)]
pub struct RuntimeLaunchOptions {
    pub project_path: Option<PathBuf>,
    pub scene_name: Option<String>,
    pub map_name: Option<String>,
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
    splash_active: bool,
    splash_started_at: Option<Instant>,
    splash_duration: Duration,
    splash_logo_path: Option<PathBuf>,
    post_splash_sprite_texture_path: Option<PathBuf>,
}

impl App {
    fn new(launch_options: RuntimeLaunchOptions) -> Self {
        let (resources, game_state) = Self::build_startup_state(&launch_options);
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
        let audio_system = AudioManager::new().expect("Failed to initialize audio system");

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
            splash_active: true,
            splash_started_at: None,
            splash_duration: Duration::from_millis(1200),
            splash_logo_path: None,
            post_splash_sprite_texture_path: None,
        }
    }

    fn build_startup_state(launch_options: &RuntimeLaunchOptions) -> (ResourceManager, GameState) {
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
                    return (resources, game_state);
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
            Ok(resources) => (resources, Self::fallback_game_state()),
            Err(error) => {
                panic!("Failed to initialize runtime resources: {error}");
            }
        }
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
                            self.performance.toggle_display();
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

                if started_at.elapsed() < self.splash_duration {
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
        self.rendering.update_projection(glam::Mat4::IDENTITY);
        self.rendering.set_tilemap_render_enabled(false);
        self.rendering.clear_sprites();
        self.rendering.clear_debug_shapes();
        self.rendering.finalize_debug_shapes();
        self.rendering.add_sprite(
            toki_core::sprite::SpriteFrame {
                u0: 0.0,
                v0: 0.0,
                u1: 1.0,
                v1: 1.0,
            },
            glam::IVec2::new(16, 18),
            glam::UVec2::new(128, 108),
        );
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

    fn resolve_logo_path(launch_options: &RuntimeLaunchOptions) -> Option<PathBuf> {
        let mut candidates = Vec::new();

        if let Some(project_path) = &launch_options.project_path {
            candidates.push(project_path.join("assets").join("TokiLogo.png"));
            if let Some(parent) = project_path.parent() {
                candidates.push(parent.join("assets").join("TokiLogo.png"));
                if let Some(grand_parent) = parent.parent() {
                    candidates.push(grand_parent.join("assets").join("TokiLogo.png"));
                }
            }
        }

        if let Ok(current_dir) = std::env::current_dir() {
            candidates.push(current_dir.join("assets").join("TokiLogo.png"));
        }

        candidates
            .push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/TokiLogo.png"));

        first_existing_path(&candidates)
    }

    fn resolve_post_splash_sprite_texture_path(
        launch_options: &RuntimeLaunchOptions,
    ) -> Option<PathBuf> {
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
        // Initialize platform system (window)
        self.platform.initialize_window(event_loop);

        // Initialize rendering system (GPU)
        if let Some(window) = self.platform.window_for_gpu() {
            if let Some(project_path) = &self.launch_options.project_path {
                let (tilemap_texture, sprite_texture) = Self::project_texture_paths(project_path);
                if let Err(error) = self.rendering.initialize_gpu_with_textures(
                    window.clone(),
                    tilemap_texture.clone(),
                    sprite_texture.clone(),
                ) {
                    tracing::error!(
                        "Failed to initialize GPU with project textures from '{}': {}",
                        project_path.display(),
                        error
                    );
                    self.rendering.initialize_gpu(window);
                }
            } else {
                self.rendering.initialize_gpu(window);
            }
        }

        if self.rendering.has_gpu() {
            if let Some(project_path) = &self.launch_options.project_path {
                if let Err(error) = self.rendering.load_project_textures(project_path) {
                    tracing::warn!(
                        "Failed to load project textures from '{}': {}",
                        project_path.display(),
                        error
                    );
                }
            }
        }

        self.splash_logo_path = Self::resolve_logo_path(&self.launch_options);
        self.post_splash_sprite_texture_path =
            Self::resolve_post_splash_sprite_texture_path(&self.launch_options);
        if let Some(path) = &self.splash_logo_path {
            if let Err(error) = self.rendering.load_sprite_texture(path.clone()) {
                tracing::warn!(
                    "Failed to load startup logo texture '{}' (disabling splash): {}",
                    path.display(),
                    error
                );
                self.splash_active = false;
            }
        } else {
            tracing::warn!("No startup logo found at assets/TokiLogo.png candidate paths");
            self.splash_active = false;
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
    use super::{first_existing_path, App, RuntimeLaunchOptions};
    use std::fs;
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
    fn resolve_logo_path_prefers_project_local_assets_logo() {
        let project_dir = make_unique_temp_dir()
            .join("example_project")
            .join("MyGame");
        fs::create_dir_all(project_dir.join("assets")).expect("assets dir");
        let logo_path = project_dir.join("assets").join("TokiLogo.png");
        fs::write(&logo_path, "logo").expect("logo write");

        let options = RuntimeLaunchOptions {
            project_path: Some(project_dir),
            scene_name: None,
            map_name: None,
        };

        let resolved = App::resolve_logo_path(&options);
        assert_eq!(resolved, Some(logo_path));
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
            scene_name: None,
            map_name: None,
        };

        let resolved = App::resolve_post_splash_sprite_texture_path(&options);
        assert_eq!(resolved, Some(creatures_path));
    }
}
