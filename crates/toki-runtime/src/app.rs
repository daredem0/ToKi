//! Simple winit window example.
// winit imports
use winit::application::ApplicationHandler; // Trait that defines app lifecycle hooks (resumed, event handling, etc.)
use winit::event::WindowEvent; // Enum of possible window-related events (resize, input, close, etc.)
use winit::event_loop::{ActiveEventLoop, EventLoop}; // ActiveEventLoop is used inside lifecycle methods; EventLoop creates and runs the app
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowId;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use toki_core::camera::{Camera, CameraController, CameraMode, RuntimeState};
use toki_core::graphics::image::load_image_rgba8_from_bytes;
use toki_core::math::projection::ProjectionParameter;
use toki_core::text::{TextAnchor, TextItem, TextStyle, TextWeight};
use toki_core::{EventHandler, GameState, Scene, TimingSystem};
use toki_render::RenderError;

use crate::systems::resources::resolve_project_resource_paths;
use crate::systems::AudioManager;
use crate::systems::{
    CameraManager, DecodedProjectCache, GameManager, PerformanceMonitor, PlatformSystem,
    RenderingSystem, ResourceManager, RuntimeAssetLoadPlan,
};
use toki_core::serialization::{load_game, save_game};

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

    fn splash_branding_positions(
        view_size: glam::Vec2,
        splash_logo_loaded: bool,
        logo_origin: glam::IVec2,
        logo_size: glam::UVec2,
        branding_style: &TextStyle,
        version_style: &TextStyle,
    ) -> (glam::Vec2, glam::Vec2) {
        let branding_height = branding_style.size_px * SPLASH_TEXT_LINE_HEIGHT_MULTIPLIER;
        let version_height = version_style.size_px * SPLASH_TEXT_LINE_HEIGHT_MULTIPLIER;
        let total_block_height = branding_height + SPLASH_BRANDING_VERSION_GAP_PX + version_height;
        let max_branding_top = (view_size.y - total_block_height - 4.0).max(0.0);
        let branding_position = if splash_logo_loaded {
            glam::Vec2::new(
                view_size.x * 0.5,
                (logo_origin.y as f32 + logo_size.y as f32 + 8.0).min(max_branding_top),
            )
        } else {
            glam::Vec2::new(view_size.x * 0.5, (view_size.y * 0.5).min(max_branding_top))
        };
        let version_position = glam::Vec2::new(
            branding_position.x,
            branding_position.y + branding_height + SPLASH_BRANDING_VERSION_GAP_PX,
        );
        (branding_position, version_position)
    }

    fn fitted_splash_version_style(view_width: f32, content: &str) -> TextStyle {
        let available_width = (view_width - SPLASH_TEXT_HORIZONTAL_PADDING_PX).max(1.0);
        let char_count = content.chars().count().max(1) as f32;
        let max_size_for_width = available_width / (char_count * 0.55);
        let size_px =
            max_size_for_width.clamp(SPLASH_VERSION_MIN_SIZE_PX, SPLASH_VERSION_DEFAULT_SIZE_PX);
        TextStyle {
            font_family: "Sans".to_string(),
            size_px,
            weight: TextWeight::Normal,
            ..TextStyle::default()
        }
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

    fn build_startup_state(
        launch_options: &RuntimeLaunchOptions,
    ) -> (
        ResourceManager,
        GameState,
        Option<tempfile::TempDir>,
        RuntimeAssetLoadPlan,
        DecodedProjectCache,
    ) {
        if let Some(pack_path) = &launch_options.pack_path {
            return Self::build_startup_state_from_pack(launch_options, pack_path).unwrap_or_else(
                |error| {
                    panic!(
                        "Failed to initialize runtime from pack '{}': {}",
                        pack_path.display(),
                        error
                    )
                },
            );
        }

        let mut decoded_project_cache = DecodedProjectCache::default();
        if let Some(project_path) = &launch_options.project_path {
            let scene = launch_options.scene_name.as_deref().and_then(|scene_name| {
                Self::load_project_scene_with_cache(
                    project_path,
                    scene_name,
                    &mut decoded_project_cache,
                )
                .ok()
            });

            let map_name = launch_options.map_name.clone().or_else(|| {
                scene
                    .as_ref()
                    .and_then(|loaded_scene| loaded_scene.maps.first().cloned())
            });

            match Self::load_project_resources_with_cache(
                project_path,
                launch_options.scene_name.as_deref(),
                map_name.as_deref(),
                &mut decoded_project_cache,
            ) {
                Ok((resources, asset_load_plan)) => {
                    let game_state = if let Some(scene) = scene {
                        Self::game_state_from_scene(scene)
                    } else {
                        Self::fallback_game_state()
                    };
                    return (
                        resources,
                        game_state,
                        None,
                        asset_load_plan,
                        decoded_project_cache,
                    );
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
            Ok(resources) => (
                resources,
                Self::fallback_game_state(),
                None,
                RuntimeAssetLoadPlan {
                    scene_name: launch_options.scene_name.clone(),
                    map_name: launch_options.map_name.clone(),
                    tilemap_texture_path: None,
                    sprite_texture_path: None,
                    preloaded_sfx_names: crate::systems::asset_loading::common_preloaded_sfx_names(
                    ),
                    stream_music: true,
                },
                decoded_project_cache,
            ),
            Err(error) => {
                panic!("Failed to initialize runtime resources: {error}");
            }
        }
    }

    fn build_startup_state_from_pack(
        launch_options: &RuntimeLaunchOptions,
        pack_path: &std::path::Path,
    ) -> anyhow::Result<(
        ResourceManager,
        GameState,
        Option<tempfile::TempDir>,
        RuntimeAssetLoadPlan,
        DecodedProjectCache,
    )> {
        let mount = crate::pack::extract_pak_to_tempdir(pack_path)?;
        let mount_path = mount.path().to_path_buf();
        let mut decoded_project_cache = DecodedProjectCache::default();
        let scene = launch_options
            .scene_name
            .as_deref()
            .map(|scene_name| {
                Self::load_project_scene_with_cache(
                    &mount_path,
                    scene_name,
                    &mut decoded_project_cache,
                )
            })
            .transpose()
            .map_err(anyhow::Error::msg)?;
        let map_name = launch_options.map_name.clone().or_else(|| {
            scene
                .as_ref()
                .and_then(|loaded_scene| loaded_scene.maps.first().cloned())
        });
        let (resources, asset_load_plan) = Self::load_project_resources_with_cache(
            &mount_path,
            launch_options.scene_name.as_deref(),
            map_name.as_deref(),
            &mut decoded_project_cache,
        )?;
        let game_state = if let Some(scene) = scene {
            Self::game_state_from_scene(scene)
        } else {
            Self::fallback_game_state()
        };
        Ok((
            resources,
            game_state,
            Some(mount),
            asset_load_plan,
            decoded_project_cache,
        ))
    }

    fn load_project_resources_with_cache(
        project_path: &std::path::Path,
        scene_name: Option<&str>,
        map_name: Option<&str>,
        decoded_project_cache: &mut DecodedProjectCache,
    ) -> Result<(ResourceManager, RuntimeAssetLoadPlan), RenderError> {
        let resolved = resolve_project_resource_paths(project_path, map_name)?;
        let tilemap = decoded_project_cache.load_tilemap_from_path(&resolved.tilemap_path)?;
        tilemap.validate()?;
        let terrain_atlas =
            decoded_project_cache.load_atlas_from_path(&resolved.terrain_atlas_path)?;
        let mut sprite_atlases = std::collections::HashMap::new();
        let mut sprite_texture_paths = std::collections::HashMap::new();
        let mut object_sheets = std::collections::HashMap::new();
        let mut object_texture_paths = std::collections::HashMap::new();
        for atlas_path in &resolved.sprite_atlas_paths {
            let atlas = decoded_project_cache.load_atlas_from_path(atlas_path)?;
            let texture_path = crate::systems::resources::resolve_atlas_texture_path(atlas_path)?;
            if let Some(file_name) = atlas_path.file_name().and_then(|name| name.to_str()) {
                sprite_atlases.insert(file_name.to_string(), atlas.clone());
                sprite_texture_paths.insert(file_name.to_string(), texture_path.clone());
            }
            if let Some(stem) = atlas_path.file_stem().and_then(|name| name.to_str()) {
                sprite_atlases.insert(stem.to_string(), atlas);
                sprite_texture_paths.insert(stem.to_string(), texture_path);
            }
        }
        for object_sheet_path in &resolved.object_sheet_paths {
            let object_sheet = toki_core::assets::object_sheet::ObjectSheetMeta::load_from_file(
                object_sheet_path,
            )?;
            let texture_path =
                crate::systems::resources::resolve_object_sheet_texture_path(object_sheet_path)?;
            if let Some(file_name) = object_sheet_path.file_name().and_then(|name| name.to_str()) {
                object_sheets.insert(file_name.to_string(), object_sheet.clone());
                object_texture_paths.insert(file_name.to_string(), texture_path.clone());
            }
            if let Some(stem) = object_sheet_path.file_stem().and_then(|name| name.to_str()) {
                object_sheets.insert(stem.to_string(), object_sheet);
                object_texture_paths.insert(stem.to_string(), texture_path);
            }
        }
        let resources = ResourceManager::from_preloaded(
            terrain_atlas,
            sprite_atlases,
            sprite_texture_paths,
            object_sheets,
            object_texture_paths,
            tilemap,
        );
        let asset_load_plan = RuntimeAssetLoadPlan::from_resolved_paths(
            scene_name.map(str::to_string),
            map_name.map(str::to_string),
            &resolved,
        );
        Ok((resources, asset_load_plan))
    }

    fn load_project_scene_with_cache(
        project_path: &std::path::Path,
        scene_name: &str,
        decoded_project_cache: &mut DecodedProjectCache,
    ) -> Result<Scene, String> {
        let scene_path = project_path
            .join("scenes")
            .join(format!("{scene_name}.json"));
        decoded_project_cache.load_scene_from_path(&scene_path)
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
                false,
            );
        }
        if self.splash_config.show_branding {
            let branding_style = TextStyle {
                font_family: "Sans".to_string(),
                size_px: 16.0,
                weight: TextWeight::Bold,
                ..TextStyle::default()
            };
            let version_style =
                Self::fitted_splash_version_style(view_size.x, COMMUNITY_SPLASH_VERSION_TEXT);
            let (branding_position, version_position) = Self::splash_branding_positions(
                view_size,
                self.splash_logo_loaded,
                logo_origin,
                logo_size,
                &branding_style,
                &version_style,
            );
            self.rendering.add_text_item(
                TextItem::new_screen(
                    COMMUNITY_SPLASH_BRANDING_TEXT,
                    branding_position,
                    branding_style,
                )
                .with_anchor(TextAnchor::TopCenter)
                .with_layer(10),
            );
            self.rendering.add_text_item(
                TextItem::new_screen(
                    COMMUNITY_SPLASH_VERSION_TEXT,
                    version_position,
                    version_style,
                )
                .with_max_width((view_size.x - SPLASH_TEXT_HORIZONTAL_PADDING_PX).max(1.0))
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
#[path = "app_tests.rs"]
mod tests;
