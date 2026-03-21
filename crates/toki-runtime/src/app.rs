//! Simple winit window example.
use winit::event_loop::EventLoop;

use std::path::PathBuf;
use std::time::Instant;

use toki_core::camera::{Camera, CameraController, CameraMode};
use toki_core::menu::{MenuController, MenuSettings};
use toki_core::TimingSystem;
use toki_render::RenderError;

use crate::systems::AudioManager;
use crate::systems::{
    CameraManager, DecodedProjectCache, FrameLimiter, GameManager, PerformanceMonitor,
    PlatformSystem, RenderingSystem, ResourceManager, RuntimeAssetLoadPlan,
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
#[path = "app_lifecycle.rs"]
mod app_lifecycle;
#[path = "app_splash.rs"]
mod app_splash;
#[path = "app_tick.rs"]
mod app_tick;
#[path = "app_transition.rs"]
mod app_transition;
#[path = "runtime_menu.rs"]
mod runtime_menu;

use app_splash::{ResolvedSplashConfig, SplashPolicy};
use app_transition::SceneTransitionController;
use toki_core::project_assets::first_existing_path;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDisplayOptions {
    pub show_entity_health_bars: bool,
    pub resolution_width: u32,
    pub resolution_height: u32,
    /// Zoom level as percentage (100 = 1.0x, 200 = 2.0x, etc.)
    pub zoom_percent: u32,
    /// Enable vsync (ties frame rate to display refresh rate).
    /// When enabled, frame limiter is bypassed.
    pub vsync: bool,
    /// Target frames per second when vsync is disabled.
    /// Set to 0 for unlimited frame rate.
    pub target_fps: u32,
    /// Timing mode for game logic (fixed or delta timestep).
    pub timing_mode: toki_core::TimingMode,
}

impl Default for RuntimeDisplayOptions {
    fn default() -> Self {
        Self {
            show_entity_health_bars: false,
            resolution_width: toki_core::project_runtime::default_resolution_width(),
            resolution_height: toki_core::project_runtime::default_resolution_height(),
            zoom_percent: toki_core::project_runtime::default_zoom_percent(),
            vsync: true,
            target_fps: 60,
            timing_mode: toki_core::TimingMode::default(),
        }
    }
}

impl RuntimeDisplayOptions {
    /// Returns the zoom level as a float (1.0 = 100%, 2.0 = 200%, etc.)
    pub fn zoom_factor(&self) -> f32 {
        self.zoom_percent as f32 / 100.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTransitionOptions {
    pub fade_duration_ms: u32,
}

impl Default for RuntimeTransitionOptions {
    fn default() -> Self {
        Self {
            fade_duration_ms: 250,
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
    pub audio_mix: RuntimeAudioMixOptions,
    pub display: RuntimeDisplayOptions,
    pub transition: RuntimeTransitionOptions,
    pub menu: MenuSettings,
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
    frame_limiter: FrameLimiter,
    launch_options: RuntimeLaunchOptions,
    menu_system: MenuController,
    splash_policy: SplashPolicy,
    splash_config: ResolvedSplashConfig,
    splash_active: bool,
    splash_started_at: Option<Instant>,
    splash_logo_loaded: bool,
    post_splash_sprite_texture_path: Option<PathBuf>,
    exit_requested: bool,
    pending_ui_events: Vec<String>,
    /// Last tick instant for delta time calculation in delta timing mode
    last_tick_instant: Option<Instant>,
    asset_load_plan: RuntimeAssetLoadPlan,
    scene_transition: SceneTransitionController,
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

        let resolution_width = launch_options.display.resolution_width;
        let resolution_height = launch_options.display.resolution_height;
        let zoom_factor = launch_options.display.zoom_factor();
        let mut camera =
            Camera::with_resolution_and_zoom(resolution_width, resolution_height, zoom_factor);
        camera.center_on(glam::IVec2::new(
            (resolution_width / 2) as i32,
            (resolution_height / 2) as i32,
        ));

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
        audio_system.set_channel_volume_percent("music_a", launch_options.audio_mix.music_percent);
        audio_system.set_channel_volume_percent("music_b", launch_options.audio_mix.music_percent);
        audio_system
            .set_channel_volume_percent("movement", launch_options.audio_mix.movement_percent);
        audio_system
            .set_channel_volume_percent("collision", launch_options.audio_mix.collision_percent);
        let menu_system = MenuController::new(launch_options.menu.clone());
        let mut scene_transition =
            SceneTransitionController::new(launch_options.transition.clone());
        if let Some(track_id) = game_system
            .active_scene()
            .and_then(|scene| scene.background_music_track_id.as_deref())
        {
            if let Err(error) = scene_transition.prime_scene_music(
                &mut audio_system,
                Some(track_id),
                launch_options.audio_mix.music_percent,
            ) {
                tracing::warn!(
                    "Failed to start initial scene background music '{track_id}': {error}"
                );
            }
        }

        // Frame limiter: only active when vsync is disabled
        let frame_limiter = if launch_options.display.vsync {
            FrameLimiter::new_unlimited()
        } else {
            FrameLimiter::new_with_target_fps(launch_options.display.target_fps)
        };

        Self {
            // Core systems
            game_system,
            camera_system,
            resources,
            performance: PerformanceMonitor::new(),
            audio_system,

            // Grouped systems
            platform: PlatformSystem::new(),
            rendering: RenderingSystem::new_with_desired_resolution(
                resolution_width,
                resolution_height,
            ),
            timing: TimingSystem::new(),
            frame_limiter,
            launch_options,
            menu_system,
            splash_policy,
            splash_config,
            splash_active: true,
            splash_started_at: None,
            splash_logo_loaded: false,
            post_splash_sprite_texture_path: None,
            exit_requested: false,
            pending_ui_events: Vec::new(),
            last_tick_instant: None,
            asset_load_plan,
            scene_transition,
            decoded_project_cache,
            pack_mount,
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
